//! Word suggestions: real words only, found from what was typed (readings, completions, small typos) and
//! ranked by
//!   typing cost (how far the typed letters are from the word) + context cost (how likely the word is
//!   after the previous word).
//! Costs are in half bits.

pub mod context;
pub mod index;
pub mod romanize;

use crate::layout::phonetic::easy;
use crate::lexicon::{is_english, is_pali, shortcuts, user_shortcut, Lexicon};
use context::Context;
use index::Index;
use romanize::{is_vowel, keys_of_latin};

/// Most suggestions shown (not counting the Latin letters as typed).
const MAX: usize = 6;
/// Most exact readings shown before corrections and completions get their places.
const MAX_READINGS: usize = 4;
/// Cost of a word that only matches loosely (same key, but not a reading the letters spell).
const LOOSE: f32 = 6.0;
/// Cost of taking a typo correction at all, on top of the edits.
const TYPO: f32 = 4.0;
/// Largest typing cost of a typo correction: one consonant, or two vowels.
const MAX_TYPO: f32 = 12.0;
/// Cost of a completion, and of each letter still to type.
const COMPLETION: f32 = 6.0;
const PER_LETTER: f32 = 1.0;
/// A correction replaces the letters as typed (as what Space commits) only when it is this much likelier
/// and only adds letters typists leave out (`index::slip`).
const MARGIN: f32 = 6.0;
/// Cost of reading the letters as shorthand (vowels and the like left out), and of each letter left out.
const SHORTHAND: f32 = 6.0;
const LEFT_OUT: f32 = 2.0;

fn push_best(v: &mut Vec<(String, f32)>, word: &str, cost: f32) {
    match v.iter_mut().find(|(w, _)| w == word) {
        Some(e) => e.1 = e.1.min(cost),
        None => v.push((word.to_string(), cost)),
    }
}

/// Easy Singlish: what to offer for the Latin letters typed so far, after the words in `prev`. The first
/// is what Space commits:
/// - the likeliest known word the letters spell; or, when they spell no known word, a clear typo
///   correction, else the letters read literally (names, new words);
/// - before those, what the letters are a shortcut for (`data/lexicon/shortcuts.tsv`);
/// - then other readings, corrections and completions, likeliest first;
/// - the letters as typed in English: first when they are a common English word and no reading is a known
///   word, second when they are a common English word, otherwise last.
/// `pali`: Pali-only words may be offered.
pub fn singlish(latin: &str, prev: &[String], pali: bool) -> Vec<String> {
    let (lex, idx, ctx) = (Lexicon::books(), Index::get(), Context::get());
    let beam = easy::readings_detailed(latin, true, 16);
    let known = |w: &str| lex.count(w) > 0 && (pali || !is_pali(w));
    let wanted = |id: u32| pali || !idx.pali[id as usize];
    // readings that are known words, in context
    let mut exact: Vec<(String, f32)> =
        beam.iter().filter(|(w, _, _)| known(w)).map(|(w, _, reading)| (w.clone(), reading + ctx.cost(prev, w))).collect();
    exact.sort_by(|a, b| a.1.total_cmp(&b.1));
    // the letters read literally, when that is not a known word and reads better than any known one
    let literal = beam.first().filter(|(w, _, _)| !known(w)).map(|(w, total, _)| (w.clone(), *total));

    let keys = keys_of_latin(latin);
    let mut corrections: Vec<(String, f32)> = Vec::new();
    // the best correction that only adds what typists leave out
    let mut slip: Option<(String, f32)> = None;
    let mut others: Vec<(String, f32)> = Vec::new();
    for key in keys.iter().filter(|k| !k.is_empty()) {
        let matches = idx.prefix(key);
        // completions: the best few of what can be many
        let mut done: Vec<(u32, f32)> = Vec::new();
        for &id in matches.iter().filter(|id| wanted(**id)) {
            let (word, k) = (&idx.words[id as usize], &idx.keys[id as usize]);
            if k.len() == key.len() {
                push_best(&mut others, word, LOOSE + ctx.cost(prev, word));
            } else {
                done.push((id, COMPLETION + PER_LETTER * (k.len() - key.len()) as f32 + ctx.cost(prev, word)));
            }
        }
        done.sort_by(|a, b| a.1.total_cmp(&b.1));
        for (id, cost) in done.into_iter().take(MAX) {
            push_best(&mut others, &idx.words[id as usize], cost);
        }
        // typos: not for one or two letters, and only a vowel's worth for three
        let max = match key.len() {
            0..=2 => 0.0,
            3 => 5.0,
            _ => MAX_TYPO,
        };
        for (id, edits) in idx.fuzzy(key, max).into_iter().filter(|(id, _)| wanted(*id)) {
            let word = &idx.words[id as usize];
            let cost = TYPO + edits + ctx.cost(prev, word);
            push_best(&mut corrections, word, cost);
            if index::slip(key.as_bytes(), idx.keys[id as usize].as_bytes()) && slip.as_ref().is_none_or(|(_, c)| cost < *c) {
                slip = Some((word.clone(), cost));
            }
        }
        // shorthand: the word without its vowels (hri, khmd), however many
        for (id, left_out) in idx.shorthand(key).into_iter().filter(|(id, _)| wanted(*id)) {
            let word = &idx.words[id as usize];
            let cost = SHORTHAND + LEFT_OUT * left_out as f32 + ctx.cost(prev, word);
            push_best(&mut corrections, word, cost);
            if slip.as_ref().is_none_or(|(_, c)| cost < *c) {
                slip = Some((word.clone(), cost));
            }
        }
    }
    corrections.sort_by(|a, b| a.1.total_cmp(&b.1));

    let mut out: Vec<String> = Vec::new();
    let add = |out: &mut Vec<String>, w: &str| {
        if !out.iter().any(|o| o == w) {
            out.push(w.to_string());
        }
    };
    let own = user_shortcut(latin);
    let known_first = !own.is_empty() || !shortcuts(latin).is_empty() || match (&literal, exact.first()) {
        (Some(_), _) => false,
        (None, Some(_)) => true,
        (None, None) => false,
    };
    let mut last: Option<String> = None;
    // shortcuts are what the typist asked for
    for w in own.iter().map(String::as_str).chain(shortcuts(latin).iter().copied()) {
        add(&mut out, w);
    }
    if let Some((word, cost)) = &literal {
        // a clear correction goes before the letters as typed; letters without any vowel are shorthand
        if let Some((c, c_cost)) = &slip {
            let bare = keys[0].len() >= 2 && !keys[0].bytes().any(is_vowel);
            if exact.is_empty() && (bare || (keys[0].len() >= 3 && c_cost + MARGIN < *cost)) {
                add(&mut out, c);
            }
            // bare letters read literally (ග්ද්ර්) are hardly ever meant: last
            if bare && !out.is_empty() {
                last = Some(word.clone());
            }
        }
        if last.is_none() {
            add(&mut out, word);
        }
    }
    for (w, _) in exact.iter().take(MAX_READINGS) {
        add(&mut out, w);
    }
    others.extend(corrections);
    others.sort_by(|a, b| a.1.total_cmp(&b.1));
    let max = MAX.max(out.len() + 2);
    for (w, _) in &others {
        if out.len() >= max {
            break;
        }
        add(&mut out, w);
    }
    if let Some(w) = last {
        add(&mut out, &w);
    }
    let at = if !is_english(latin) {
        out.len()
    } else if known_first {
        1
    } else {
        0
    };
    out.insert(at.min(out.len()), latin.to_string());
    out
}

/// Wijesekara and Helakuru keys: the word as typed, then likely words that start with `prefix` (the word
/// without a still-open hal). Empty when there is nothing to suggest.
pub fn completions(word: &str, prefix: &str, prev: &[String], pali: bool) -> Vec<String> {
    let (lex, ctx) = (Lexicon::books(), Context::get());
    let (words, _) = lex.entries();
    let mut found: Vec<(&str, f32)> = lex
        .range(prefix)
        .map(|i| words[i].as_str())
        .filter(|w| *w != word && (pali || !is_pali(w)))
        .map(|w| (w, PER_LETTER * (w.chars().count() - prefix.chars().count()) as f32 + ctx.cost(prev, w)))
        .collect();
    if found.is_empty() {
        return Vec::new();
    }
    found.sort_by(|a, b| a.1.total_cmp(&b.1));
    std::iter::once(word.to_string()).chain(found.into_iter().take(MAX - 1).map(|(w, _)| w.to_string())).collect()
}
