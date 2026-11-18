//! How likely a word is after the previous word: bigram counts from the books and subtitles
//! (`data/lexicon/ngram.bin`, built by tools/build-lexicon.mjs), backing off to the word's own frequency
//! (Witten–Bell). Costs are in half bits like the rest of the engine.

use crate::lexicon::Lexicon;
use std::sync::OnceLock;

/// Extra cost of a word that is not in the lexicon (as in Easy Singlish).
const UNKNOWN_WORD: f32 = 8.0;

/// Words of context the session keeps.
pub const WORDS: usize = 1;

pub struct Context {
    /// sorted: (48-bit hash of the key) << 16 | value
    entries: Vec<u64>,
}

fn hash(parts: &[&str]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in parts.iter().flat_map(|p| p.bytes()) {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    (h ^ (h >> 48)) & ((1 << 48) - 1)
}

impl Context {
    pub fn get() -> &'static Context {
        static C: OnceLock<Context> = OnceLock::new();
        C.get_or_init(|| Context::new(include_bytes!("../../../../data/lexicon/ngram.bin")))
    }

    fn new(b: &[u8]) -> Context {
        let entries = if b.len() < 12 || &b[..5] != b"AKNG1" {
            Vec::new()
        } else {
            let n = u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize;
            b[12..].chunks_exact(8).take(n).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect()
        };
        Context { entries }
    }

    fn value(&self, parts: &[&str]) -> u32 {
        let h = hash(parts);
        let i = self.entries.partition_point(|e| (e >> 16) < h);
        match self.entries.get(i) {
            Some(e) if e >> 16 == h => (e & 0xffff) as u32,
            _ => 0,
        }
    }

    /// How often `word` followed `prev` in the books (0: never, or too seldom to have been kept).
    pub fn pair(&self, prev: &str, word: &str) -> u32 {
        self.value(&[prev, " ", word])
    }

    /// P(word | previous word), or the word's own probability when there is no usable context.
    pub fn probability(&self, prev: Option<&str>, word: &str) -> f64 {
        let lex = Lexicon::books();
        let p_word = lex.count(word) as f64 / lex.total() as f64;
        let Some(prev) = prev else { return p_word };
        let (n, types) = (lex.count(prev) as f64, self.value(&[prev, "\t"]) as f64);
        if n == 0.0 || types == 0.0 {
            return p_word;
        }
        let pair = (self.value(&[prev, " ", word]) as f64).min(n);
        (pair + types * p_word) / (n + types)
    }

    /// Cost of `word` after `prev` (the words before it, nearest last).
    pub fn cost(&self, prev: &[String], word: &str) -> f32 {
        let lex = Lexicon::books();
        if lex.count(word) == 0 {
            return lex.spelling_cost(word, true) + UNKNOWN_WORD;
        }
        let p = self.probability(prev.last().map(String::as_str), word);
        (-2.0 * p.log2()) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_helps() {
        let c = Context::get();
        assert!(!c.entries.is_empty());
        let after = |prev: &str, w: &str| c.cost(&[prev.to_string()], w);
        // "ගෙදර ගියා" is common; ගියා is likelier after ගෙදර than on its own
        assert!(after("ගෙදර", "ගියා") < c.cost(&[], "ගියා") - 2.0);
        // an unknown previous word changes nothing
        assert_eq!(after("qqq", "ගියා"), c.cost(&[], "ගියා"));
    }
}
