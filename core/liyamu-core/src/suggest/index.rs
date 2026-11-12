//! Finds lexicon words from what was typed in Singlish: words with the same loose key (readings), words
//! whose key starts with it (completions) and words whose key is a small typo away.

use super::romanize::{is_vowel, key_of_sinhala, skeleton};
use crate::lexicon::{is_pali, Lexicon};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Typing-cost of edits between keys, in half bits.
const VOWEL: f32 = 5.0; // a vowel left out, added or wrong: very common in Singlish
const CLOSE_VOWEL: f32 = 3.0; // a/e, e/i, o/u
const CONS: f32 = 12.0;
const DOUBLE: f32 = 4.0; // a doubled consonant typed once, or a single one twice (ama / amma)
const SWAP: f32 = 8.0; // two neighbouring letters swapped
const FIRST: f32 = 20.0; // the first letter is rarely wrong
const NASAL: f32 = 3.0; // ඹ ඳ ඟ typed as b d g (kolaba)

/// The half nasal of ඹ ඳ ඟ in a key: m before b, n before d or g.
fn nasal(s: &[u8], i: usize) -> bool {
    matches!((s[i], s.get(i + 1)), (b'm', Some(b'b')) | (b'n', Some(b'd' | b'g')))
}

pub struct Index {
    /// the lexicon's words (same order as `Lexicon::entries`)
    pub words: &'static [String],
    pub counts: &'static [u32],
    /// loose key of each word
    pub keys: Vec<String>,
    /// the word is Pali only
    pub pali: Vec<bool>,
    /// word ids sorted by key
    by_key: Vec<u32>,
    /// skeleton, and skeletons with one letter removed → word ids
    skeletons: HashMap<String, Vec<u32>>,
}

impl Index {
    pub fn get() -> &'static Index {
        static I: OnceLock<Index> = OnceLock::new();
        I.get_or_init(|| {
            let (words, counts) = Lexicon::books().entries();
            let keys: Vec<String> = words.iter().map(|w| key_of_sinhala(w)).collect();
            let mut by_key: Vec<u32> = (0..words.len() as u32).collect();
            by_key.sort_by(|a, b| keys[*a as usize].cmp(&keys[*b as usize]));
            let mut skeletons: HashMap<String, Vec<u32>> = HashMap::new();
            for (i, k) in keys.iter().enumerate() {
                let s = skeleton(k);
                for d in deletions(&s) {
                    skeletons.entry(d).or_default().push(i as u32);
                }
                skeletons.entry(s).or_default().push(i as u32);
            }
            let pali = words.iter().map(|w| is_pali(w)).collect();
            Index { words, counts, keys, pali, by_key, skeletons }
        })
    }

    /// Ids of the words whose key starts with `key` (the words with exactly that key come first).
    pub fn prefix(&self, key: &str) -> &[u32] {
        let lo = self.by_key.partition_point(|i| self.keys[*i as usize].as_str() < key);
        let n = self.by_key[lo..].partition_point(|i| self.keys[*i as usize].starts_with(key));
        &self.by_key[lo..lo + n]
    }

    /// Words a small typo away from `key`, as (id, typing cost), cost in 0 < cost <= max.
    pub fn fuzzy(&self, key: &str, max: f32) -> Vec<(u32, f32)> {
        let s = skeleton(key);
        let mut ids: Vec<u32> = Vec::new();
        for k in std::iter::once(s.clone()).chain(deletions(&s)) {
            if let Some(v) = self.skeletons.get(&k) {
                ids.extend_from_slice(v);
            }
        }
        ids.sort_unstable();
        ids.dedup();
        let mut out = Vec::new();
        for id in ids {
            let d = distance(key.as_bytes(), self.keys[id as usize].as_bytes(), max);
            if d > 0.0 && d <= max {
                out.push((id, d));
            }
        }
        out
    }

    /// Words that `key` is shorthand for: the key plus only letters typists leave out (`slip`), as
    /// (id, letters left out). hri → හරි, khmd → කොහොමද, kolaba → කොළඹ.
    pub fn shorthand(&self, key: &str) -> Vec<(u32, usize)> {
        if key.len() < 2 {
            return Vec::new();
        }
        let s = skeleton(key);
        let mut ids: Vec<u32> = Vec::new();
        for k in std::iter::once(s.clone()).chain(deletions(&s)) {
            if let Some(v) = self.skeletons.get(&k) {
                ids.extend_from_slice(v);
            }
        }
        ids.sort_unstable();
        ids.dedup();
        ids.into_iter()
            .filter_map(|id| {
                let k = self.keys[id as usize].as_bytes();
                let left_out = k.len().checked_sub(key.len()).filter(|n| *n > 0 && *n <= key.len() + 1)?;
                slip(key.as_bytes(), k).then_some((id, left_out))
            })
            .collect()
    }
}

/// Is `word` what was typed plus only what typists leave out or mix up: vowels, the second letter of a
/// doubled consonant, the half nasal of ඹ ඳ ඟ, or two neighbouring letters swapped? Only such corrections are safe to make without
/// asking: a changed letter is as often another form of the word (කැපෙනවා / කපනවා) as a typo.
pub fn slip(typed: &[u8], word: &[u8]) -> bool {
    if typed.len() == word.len() {
        let diff: Vec<usize> = (0..typed.len()).filter(|i| typed[*i] != word[*i]).collect();
        return diff.len() == 2 && diff[1] == diff[0] + 1 && typed[diff[0]] == word[diff[1]] && typed[diff[1]] == word[diff[0]];
    }
    let mut t = 0;
    for (i, c) in word.iter().enumerate() {
        if t < typed.len() && typed[t] == *c {
            t += 1;
        } else if !(i > 0 && (is_vowel(*c) || word[i - 1] == *c || nasal(word, i))) {
            return false;
        }
    }
    t == typed.len()
}

/// The skeleton with one letter removed, each way (only for skeletons long enough to stay distinctive).
fn deletions(s: &str) -> Vec<String> {
    if s.len() < 3 {
        return Vec::new();
    }
    let mut v: Vec<String> = (0..s.len()).map(|i| format!("{}{}", &s[..i], &s[i + 1..])).collect();
    v.dedup();
    v
}

fn close(a: u8, b: u8) -> bool {
    matches!((a.min(b), a.max(b)), (b'a', b'e') | (b'e', b'i') | (b'o', b'u'))
}

/// Weighted Damerau–Levenshtein distance from the typed key `a` to a word's key `b`; gives up (returns
/// more than `max`) early.
pub fn distance(a: &[u8], b: &[u8], max: f32) -> f32 {
    // cost of a letter that only one side has: s[i]
    let gap = |s: &[u8], i: usize| {
        if i == 0 {
            FIRST
        } else if is_vowel(s[i]) {
            VOWEL
        } else if s[i - 1] == s[i] {
            DOUBLE
        } else if nasal(s, i) {
            NASAL
        } else {
            CONS
        }
    };
    if (a.len() as f32 - b.len() as f32).abs() * DOUBLE > max {
        return max + 1.0;
    }
    let w = b.len() + 1;
    let mut d = vec![0f32; (a.len() + 1) * w];
    for j in 1..=b.len() {
        d[j] = d[j - 1] + gap(b, j - 1);
    }
    for i in 1..=a.len() {
        d[i * w] = d[(i - 1) * w] + gap(a, i - 1);
        let mut best = d[i * w];
        for j in 1..=b.len() {
            let (x, y) = (a[i - 1], b[j - 1]);
            let sub = if x == y {
                0.0
            } else if i == 1 || j == 1 {
                FIRST
            } else if is_vowel(x) && is_vowel(y) {
                if close(x, y) { CLOSE_VOWEL } else { VOWEL }
            } else {
                CONS
            };
            let mut v = (d[(i - 1) * w + j - 1] + sub)
                .min(d[(i - 1) * w + j] + gap(a, i - 1))
                .min(d[i * w + j - 1] + gap(b, j - 1));
            if i > 1 && j > 1 && x == b[j - 2] && a[i - 2] == y && x != y {
                v = v.min(d[(i - 2) * w + j - 2] + SWAP);
            }
            d[i * w + j] = v;
            best = best.min(v);
        }
        if best > max {
            return max + 1.0;
        }
    }
    d[a.len() * w + b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typos() {
        assert_eq!(distance(b"gedar", b"gedara", 12.0), VOWEL);
        assert_eq!(distance(b"gedra", b"gedara", 12.0), VOWEL);
        assert_eq!(distance(b"gedr", b"gedara", 12.0), 2.0 * VOWEL);
        assert_eq!(distance(b"gedaar", b"gedara", 12.0), SWAP);
        assert!(distance(b"kedara", b"gedara", 12.0) > 12.0);
        assert!(slip(b"gedr", b"gedara") && slip(b"gedaar", b"gedara") && slip(b"ama", b"amma"));
        assert!(!slip(b"kapenava", b"kapanava") && !slip(b"edara", b"gedara") && !slip(b"gedara", b"gedar"));
        let i = Index::get();
        let has = |key: &str, word: &str| i.fuzzy(key, 12.0).iter().any(|(id, _)| i.words[*id as usize] == word);
        assert!(has("gedar", "ගෙදර") && has("gedra", "ගෙදර") && has("gedr", "ගෙදර"));
        let short = |key: &str, word: &str| i.shorthand(key).iter().any(|(id, _)| i.words[*id as usize] == word);
        assert!(short("hri", "හරි") && short("kohomd", "කොහොමද") && short("khmd", "කොහොමද") && short("kolaba", "කොළඹ"));
        assert!(!short("gedara", "ගෙදර") && !short("kdr", "ගෙදර"));
        assert!(i.prefix("gedar").iter().any(|id| i.words[*id as usize] == "ගෙදරට"));
    }
}
