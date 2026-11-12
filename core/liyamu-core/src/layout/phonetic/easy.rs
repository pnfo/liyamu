//! Easy Singlish: type the way people spell Sinhala in Latin letters, without learning a table.
//!
//! Letters are read case-insensitively and most have several readings (d → ද ඩ ධ, t → ට ත, n → න ණ ං,
//! e → එ ඒ ඇ, lanka → ලංකා, handa → හඳ). Every way of reading the word is scored by
//!   reading cost (how unusual the readings are) + how likely the Sinhala word is,
//! where the second part is the word's frequency in the pitaka.lk books, or, for words not in the books,
//! a character model learned from them. Capital letters lean towards the "heavy" letter (T ඨ, D ඩ, N ණ,
//! L ළ, S ෂ, B භ ...). Costs are in half bits.

use super::{render as render_toks, Join, Tok};
use crate::lexicon::Lexicon;

const fn c(s: &'static str) -> Tok {
    Tok::Cons(s)
}
const fn v(letter: &'static str, sign: &'static str) -> Tok {
    Tok::Vowel { letter, sign }
}

/// One reading of a unit: tokens, cost, and whether a capital letter makes it the preferred reading.
struct Reading(&'static [Tok], f32, bool);

/// Lower-case Latin units and their readings, the usual one first.
const UNITS: &[(&str, &[Reading])] = &[
    // consonants
    ("k", &[Reading(&[c("ක")], 0.0, false), Reading(&[c("ඛ")], 8.0, true)]),
    ("kh", &[Reading(&[c("ඛ")], 0.0, false), Reading(&[c("ක")], 6.0, false)]),
    ("g", &[Reading(&[c("ග")], 0.0, false), Reading(&[c("ඝ")], 8.0, true)]),
    ("gh", &[Reading(&[c("ඝ")], 0.0, false), Reading(&[c("ග")], 6.0, false)]),
    ("ng", &[Reading(&[Tok::Mod("ං"), c("ග")], 1.0, false), Reading(&[c("ඟ")], 2.0, false)]),
    ("c", &[Reading(&[c("ච")], 0.0, false), Reading(&[c("ඡ")], 8.0, true), Reading(&[c("ක")], 6.0, false)]),
    ("ch", &[Reading(&[c("ච")], 0.0, false), Reading(&[c("ඡ")], 6.0, true)]),
    ("chh", &[Reading(&[c("ඡ")], 0.0, false)]),
    ("j", &[Reading(&[c("ජ")], 0.0, false), Reading(&[c("ඣ")], 8.0, true)]),
    ("jh", &[Reading(&[c("ඣ")], 0.0, false)]),
    ("t", &[Reading(&[c("ට")], 0.0, false), Reading(&[c("ත")], 3.0, false), Reading(&[c("ඨ")], 8.0, true)]),
    ("th", &[Reading(&[c("ත")], 0.0, false), Reading(&[c("ථ")], 6.0, true), Reading(&[c("ට")], 6.0, false)]),
    ("d", &[Reading(&[c("ද")], 0.0, false), Reading(&[c("ඩ")], 3.0, true), Reading(&[c("ධ")], 6.0, false)]),
    ("dh", &[Reading(&[c("ධ")], 1.0, false), Reading(&[c("ද")], 1.0, false), Reading(&[c("ඪ")], 8.0, true)]),
    ("nd", &[Reading(&[c("ඳ")], 2.0, false), Reading(&[c("ඬ")], 5.0, true)]),
    ("ndh", &[Reading(&[c("ඳ")], 3.0, false)]),
    ("mb", &[Reading(&[c("ඹ")], 2.0, false)]),
    ("n", &[Reading(&[c("න")], 0.0, false), Reading(&[c("ණ")], 4.0, true), Reading(&[Tok::Mod("ං")], 4.0, false)]),
    ("gn", &[Reading(&[c("ඥ")], 2.0, false)]),
    ("kn", &[Reading(&[c("ඥ")], 3.0, false)]),
    ("ny", &[Reading(&[c("ඤ")], 3.0, false)]),
    ("p", &[Reading(&[c("ප")], 0.0, false), Reading(&[c("ඵ")], 8.0, true)]),
    ("ph", &[Reading(&[c("ඵ")], 0.0, false), Reading(&[c("ප")], 6.0, false)]),
    ("f", &[Reading(&[c("ෆ")], 0.0, false)]),
    ("b", &[Reading(&[c("බ")], 0.0, false), Reading(&[c("භ")], 8.0, true)]),
    ("bh", &[Reading(&[c("භ")], 0.0, false)]),
    ("m", &[Reading(&[c("ම")], 0.0, false)]),
    ("y", &[Reading(&[c("ය")], 0.0, false)]),
    // ayya අයියා, not a yansaya
    ("yy", &[Reading(&[c("ය"), v("ඉ", "ි"), c("ය")], 1.0, false), Reading(&[c("ය"), Tok::Break, c("ය")], 2.0, false)]),
    ("r", &[Reading(&[c("ර")], 0.0, false)]),
    ("l", &[Reading(&[c("ල")], 0.0, false), Reading(&[c("ළ")], 3.0, true)]),
    ("v", &[Reading(&[c("ව")], 0.0, false)]),
    ("w", &[Reading(&[c("ව")], 0.0, false)]),
    ("s", &[Reading(&[c("ස")], 0.0, false), Reading(&[c("ශ")], 4.0, false), Reading(&[c("ෂ")], 5.0, true)]),
    ("sh", &[Reading(&[c("ශ")], 0.0, false), Reading(&[c("ෂ")], 2.0, true)]),
    ("h", &[Reading(&[c("හ")], 0.0, false)]),
    ("x", &[Reading(&[Tok::Mod("ං")], 0.0, false)]),
    ("q", &[Reading(&[c("ද")], 2.0, false)]),
    ("z", &[Reading(&[Tok::Break], 0.0, false)]),
    // vowels
    ("a", &[Reading(&[v("අ", "")], 0.0, false), Reading(&[v("ආ", "ා")], 4.0, true), Reading(&[v("ඇ", "ැ")], 6.0, false)]),
    ("aa", &[Reading(&[v("ආ", "ා")], 0.0, false)]),
    ("ae", &[Reading(&[v("ඇ", "ැ")], 0.0, false), Reading(&[v("ඈ", "ෑ")], 3.0, false)]),
    ("aae", &[Reading(&[v("ඈ", "ෑ")], 0.0, false)]),
    ("aee", &[Reading(&[v("ඈ", "ෑ")], 0.0, false)]),
    ("i", &[Reading(&[v("ඉ", "ි")], 0.0, false), Reading(&[v("ඊ", "ී")], 3.0, true)]),
    ("ii", &[Reading(&[v("ඊ", "ී")], 0.0, false)]),
    ("ee", &[Reading(&[v("ඒ", "ේ")], 0.0, false), Reading(&[v("ඊ", "ී")], 1.0, false)]),
    ("u", &[Reading(&[v("උ", "ු")], 0.0, false), Reading(&[v("ඌ", "ූ")], 3.0, true)]),
    ("uu", &[Reading(&[v("ඌ", "ූ")], 0.0, false)]),
    ("oo", &[Reading(&[v("ඕ", "ෝ")], 0.0, false), Reading(&[v("ඌ", "ූ")], 1.0, false)]),
    ("e", &[Reading(&[v("එ", "ෙ")], 0.0, false), Reading(&[v("ඒ", "ේ")], 3.0, true), Reading(&[v("ඇ", "ැ")], 4.0, false)]),
    ("o", &[Reading(&[v("ඔ", "ො")], 0.0, false), Reading(&[v("ඕ", "ෝ")], 3.0, true)]),
    ("ai", &[Reading(&[v("ඓ", "ෛ")], 1.0, false)]),
    ("au", &[Reading(&[v("ඖ", "ෞ")], 1.0, false)]),
    ("ou", &[Reading(&[v("ඖ", "ෞ")], 1.0, false)]),
    ("ru", &[Reading(&[v("ඍ", "ෘ")], 5.0, false)]),
    ("ri", &[Reading(&[v("ඍ", "ෘ")], 6.0, false)]),
];

/// Extra cost of a Sinhala spelling that is not in the books.
const UNKNOWN_WORD: f32 = 8.0;
/// Cost of reading a word-final "a" as ා: none, the books decide (අම්මා, but මට, පොත).
const FINAL_AA: f32 = 0.0;
/// Readings kept at each point of the word.
const BEAM: usize = 48;

#[derive(Clone)]
struct Hyp {
    toks: Vec<Tok>,
    cost: f32,
}

/// How likely a Sinhala word (or the start of one) is.
fn word_cost(lex: &Lexicon, word: &str, complete: bool) -> f32 {
    if complete {
        lex.word_cost(word).unwrap_or_else(|| lex.spelling_cost(word, true) + UNKNOWN_WORD)
    } else {
        // a final consonant still shows hal; the next letter may take it away
        let stem = word.strip_suffix('\u{0DCA}').unwrap_or(word);
        lex.prefix_cost(stem).unwrap_or_else(|| lex.spelling_cost(stem, false) + UNKNOWN_WORD)
    }
}

/// Readings of the Latin word, best first, as (Sinhala, cost). `complete`: the word is finished
/// (scored as a whole word) rather than still being typed (scored as the start of words).
pub fn readings(latin: &str, complete: bool, max: usize) -> Vec<(String, f32)> {
    readings_detailed(latin, complete, max).into_iter().map(|(w, total, _)| (w, total)).collect()
}

/// Like [`readings`], as (Sinhala, cost, reading cost): the last is the cost without the word's likelihood,
/// for callers that judge the word themselves (in context).
pub fn readings_detailed(latin: &str, complete: bool, max: usize) -> Vec<(String, f32, f32)> {
    let lex = Lexicon::books();
    let lower: Vec<char> = latin.chars().map(|c| c.to_ascii_lowercase()).collect();
    let orig: Vec<char> = latin.chars().collect();
    let n = lower.len();
    let mut beams: Vec<Vec<Hyp>> = vec![Vec::new(); n + 1];
    beams[0].push(Hyp { toks: Vec::new(), cost: 0.0 });
    for p in 0..n {
        if beams[p].is_empty() {
            continue;
        }
        prune(&mut beams[p], lex, false);
        let here = std::mem::take(&mut beams[p]);
        let mut matched = false;
        for (key, readings) in UNITS {
            let k: Vec<char> = key.chars().collect();
            if p + k.len() > n || lower[p..p + k.len()] != k[..] {
                continue;
            }
            matched = true;
            let caps = orig[p..p + k.len()].iter().any(|c| c.is_ascii_uppercase());
            let at_end = p + k.len() == n;
            for Reading(toks, cost, heavy) in readings.iter() {
                let mut cost = match (caps, heavy) {
                    (true, true) => 0.0,
                    (true, false) => cost + 3.0,
                    _ => *cost,
                };
                // a final "a" is often ා in casual spelling (amma අම්මා, yanawa යනවා)
                if at_end && *key == "a" && toks[0] == v("ආ", "ා") && !caps {
                    cost = FINAL_AA;
                }
                for h in &here {
                    let mut t = h.toks.clone();
                    t.extend_from_slice(toks);
                    beams[p + k.len()].push(Hyp { toks: t, cost: h.cost + cost });
                }
            }
        }
        if !matched {
            beams[p + 1].extend(here); // not a letter we know: skip it
        }
    }
    let mut last = std::mem::take(&mut beams[n]);
    let mut scored: Vec<(String, f32, f32)> = Vec::new();
    for h in last.drain(..) {
        let s = render_toks(&h.toks, Join::BeforeVowel);
        let cost = h.cost + word_cost(lex, &s, complete);
        match scored.iter_mut().find(|(w, _, _)| *w == s) {
            Some(e) if e.1 > cost => (e.1, e.2) = (cost, h.cost),
            Some(_) => {}
            None => scored.push((s, cost, h.cost)),
        }
    }
    scored.sort_by(|a, b| a.1.total_cmp(&b.1));
    scored.truncate(max);
    scored
}

/// Keeps the best BEAM readings, one per spelling.
fn prune(hyps: &mut Vec<Hyp>, lex: &Lexicon, complete: bool) {
    let mut scored: Vec<(String, f32, Hyp)> = Vec::new();
    for h in hyps.drain(..) {
        let s = render_toks(&h.toks, Join::BeforeVowel);
        let score = h.cost + word_cost(lex, &s, complete);
        match scored.iter_mut().find(|(w, _, _)| *w == s) {
            // same spelling so far, but the tokens may continue differently: keep the cheaper one
            Some(e) if e.2.toks == h.toks || e.1 <= score => {}
            Some(e) => *e = (s, score, h),
            None => scored.push((s, score, h)),
        }
    }
    scored.sort_by(|a, b| a.1.total_cmp(&b.1));
    scored.truncate(BEAM);
    hyps.extend(scored.into_iter().map(|(_, _, h)| h));
}

/// The best reading.
pub fn render(latin: &str, complete: bool) -> String {
    readings(latin, complete, 1).into_iter().next().map(|(s, _)| s).unwrap_or_default()
}
