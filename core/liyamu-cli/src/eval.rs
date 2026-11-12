//! `liyamu eval [sentences-per-source] [heldout.txt]`: measures Easy Singlish suggestions on sentences the
//! models never saw (data/corpus/heldout.txt from tools/build-corpus.mjs). Each word is typed the way a
//! person might spell it in Singlish, letter by letter, clean and with typos, and three engines are
//! compared: the old one (every reading of the letters), real-word candidates without context, and with
//! the previous word as context.

use liyamu_core::layout::phonetic::easy;
use liyamu_core::lexicon::Lexicon;
use liyamu_core::suggest::{self, romanize::casual};
use std::time::Instant;

/// Small deterministic random numbers, so runs are comparable.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// One typo: a vowel left out, two neighbouring letters swapped, or a neighbouring key hit.
fn typo(latin: &str, rng: &mut Rng) -> String {
    let mut c: Vec<char> = latin.chars().collect();
    let i = 1 + rng.below(c.len() - 1);
    match rng.below(3) {
        0 => match (1..c.len()).filter(|i| "aeiou".contains(c[*i])).nth(rng.below(2)) {
            Some(v) => {
                c.remove(v);
            }
            None => c.swap(i - 1, i),
        },
        1 => c.swap(i - 1, i),
        _ => {
            const ROWS: [&str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];
            if let Some((row, at)) = ROWS.iter().find_map(|r| r.find(c[i]).map(|at| (r.as_bytes(), at))) {
                let at = if at + 1 < row.len() && rng.below(2) == 0 { at + 1 } else { at.saturating_sub(1) };
                c[i] = row[at] as char;
            }
        }
    }
    c.into_iter().collect()
}

#[derive(Default)]
struct Score {
    words: usize,
    top1: usize,
    top3: usize,
    /// asked / found in the first three after 2 and 3 letters
    after: [(usize, usize); 2],
    keys_typed: usize,
    keys_full: usize,
    /// words the lexicon does not have, typed without typos / left as typed (not "corrected")
    unknown: (usize, usize),
    calls: usize,
    secs: f64,
}

type Engine = fn(&str, &[String]) -> Vec<String>;

fn old(latin: &str, _prev: &[String]) -> Vec<String> {
    easy::readings(latin, true, 6).into_iter().map(|(w, _)| w).collect()
}
fn no_context(latin: &str, _prev: &[String]) -> Vec<String> {
    suggest::singlish(latin, &[], true)
}

fn with_context(latin: &str, prev: &[String]) -> Vec<String> {
    suggest::singlish(latin, prev, true)
}

pub fn run(n: usize, file: &str) {
    let text = std::fs::read_to_string(file).unwrap_or_else(|e| panic!("{file}: {e} (run tools/build-corpus.mjs)"));
    let engines: [(&str, Engine); 3] = [("old (all readings)", old), ("real words + typos", no_context), ("+ previous word", with_context)];
    let lex = Lexicon::books();
    println!("| typing | engine | first | in first 3 | after 2 letters | after 3 letters | keys saved | new words kept | ms/key |");
    println!("|---|---|---|---|---|---|---|---|---|");
    for (noise, label) in [(false, "clean"), (true, "1 typo in 30% of words")] {
        for (name, engine) in engines {
            let mut s = Score::default();
            let mut rng = Rng(7);
            for source in ["books", "subs"] {
                for line in text.lines().filter(|l| l.starts_with(source)).take(n) {
                    // (a word the lexicon lacks can still be typed: the letters read literally)
                    let words: Vec<&str> = line.split('\t').nth(1).unwrap_or("").split(' ').collect();
                    for (i, word) in words.iter().enumerate() {
                        let careful = rng.below(3) == 0;
                        let mut latin = casual(word, careful);
                        let noisy = noise && latin.len() >= 4 && rng.below(10) < 3;
                        if noisy {
                            latin = typo(&latin, &mut rng);
                        }
                        if latin.is_empty() || !latin.is_ascii() {
                            continue;
                        }
                        let prev: Vec<String> = words[i.saturating_sub(8)..i].iter().map(|w| w.to_string()).collect();
                        // words the engine cannot know: only that they are not replaced by a known word
                        if lex.count(word) == 0 {
                            if !noisy {
                                s.unknown.0 += 1;
                                let first = engine(&latin, &prev).into_iter().next().unwrap_or_default();
                                s.unknown.1 += (lex.count(&first) == 0) as usize;
                                if std::env::var("COUNTS").is_ok() && lex.count(&first) > 0 {
                                    eprintln!("{name}: {word} ({latin}) -> {first}");
                                }
                            }
                            continue;
                        }
                        s.words += 1;
                        s.keys_full += latin.len() + 1;
                        let mut best = usize::MAX;
                        for k in 1..=latin.len() {
                            let t = Instant::now();
                            let c = engine(&latin[..k], &prev);
                            s.secs += t.elapsed().as_secs_f64();
                            s.calls += 1;
                            let at = c.iter().position(|w| w == word);
                            if let Some(at) = at {
                                best = best.min(k + at + 1); // letters, arrows, Space
                            }
                            if k == latin.len() {
                                s.top1 += (at == Some(0)) as usize;
                                s.top3 += at.is_some_and(|a| a < 3) as usize;
                            } else if k == 2 || k == 3 {
                                s.after[k - 2].0 += 1;
                                s.after[k - 2].1 += at.is_some_and(|a| a < 3) as usize;
                            }
                        }
                        // not offered at all: typed, then fixed by hand (counted as typing it twice)
                        s.keys_typed += if best == usize::MAX { 2 * (latin.len() + 1) } else { best };
                    }
                }
            }
            let pct = |a: usize, b: usize| format!("{:.1}%", 100.0 * a as f64 / b.max(1) as f64);
            if std::env::var("COUNTS").is_ok() {
                eprintln!("known words {}, unknown clean {}", s.words, s.unknown.0);
            }
            println!(
                "| {label} | {name} | {} | {} | {} | {} | {:.1}% | {} | {:.2} |",
                pct(s.top1, s.words),
                pct(s.top3, s.words),
                pct(s.after[0].1, s.after[0].0),
                pct(s.after[1].1, s.after[1].0),
                100.0 * (1.0 - s.keys_typed as f64 / s.keys_full as f64),
                pct(s.unknown.1, s.unknown.0),
                1000.0 * s.secs / s.calls as f64
            );
        }
    }
}
