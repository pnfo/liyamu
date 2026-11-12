//! Words with their counts (`data/lexicon/`: from the books and subtitles, everyday words, names, places,
//! dictionary headwords), and a character model built from them. Costs are in half bits (−2·log2 P), like pitaka.lk's sinhala-model.js, so they add up.

use std::collections::{HashMap, HashSet};
use std::sync::{OnceLock, RwLock};

pub struct Lexicon {
    /// sorted, for prefix ranges
    words: Vec<String>,
    counts: Vec<u32>,
    /// cum[i] = sum of counts[..i]
    cum: Vec<u64>,
    total: u64,
    chars: CharModel,
}

impl Lexicon {
    pub fn books() -> &'static Lexicon {
        static L: OnceLock<Lexicon> = OnceLock::new();
        L.get_or_init(|| {
            Lexicon::from_lists(&[
                (include_str!("../../../data/lexicon/words.tsv"), 1),
                (include_str!("../../../data/lexicon/everyday.tsv"), EVERYDAY),
                (include_str!("../../../data/lexicon/places.tsv"), NAME),
                (include_str!("../../../data/lexicon/names.tsv"), NAME),
                (include_str!("../../../data/lexicon/dictionary.txt"), 1),
            ])
        })
    }

    pub fn from_tsv(tsv: &str) -> Lexicon {
        Lexicon::from_lists(&[(tsv, 1)])
    }

    /// Word lists of `word<TAB>count` lines (`#` comments). A line without a count gets `default`. A word in
    /// several lists keeps its highest count.
    pub fn from_lists(lists: &[(&str, u32)]) -> Lexicon {
        let mut map: HashMap<String, u32> = HashMap::new();
        for (tsv, default) in lists {
            for l in tsv.lines().map(str::trim_end).filter(|l| !l.starts_with('#') && !l.is_empty()) {
                let (w, c) = match l.split_once('\t') {
                    Some((w, c)) => match c.trim().parse() {
                        Ok(c) => (w, c),
                        Err(_) => continue,
                    },
                    None => (l, *default),
                };
                let e = map.entry(w.trim().to_string()).or_default();
                *e = (*e).max(c);
            }
        }
        let mut rows: Vec<(String, u32)> = map.into_iter().collect();
        rows.sort();
        let mut cum = Vec::with_capacity(rows.len() + 1);
        let mut total = 0u64;
        cum.push(0);
        for (_, c) in &rows {
            total += *c as u64;
            cum.push(total);
        }
        let chars = CharModel::train(rows.iter().map(|(w, c)| (w.as_str(), *c)));
        let (words, counts) = rows.into_iter().unzip();
        Lexicon { words, counts, cum, total, chars }
    }

    /// All the words' counts together.
    pub fn total(&self) -> u64 {
        self.total
    }

    /// All words, sorted, with their counts.
    pub fn entries(&self) -> (&[String], &[u32]) {
        (&self.words, &self.counts)
    }

    /// Index range (into [`Lexicon::entries`]) of the words that start with `prefix`.
    pub fn range(&self, prefix: &str) -> std::ops::Range<usize> {
        self.prefix_range(prefix)
    }

    pub fn count(&self, word: &str) -> u32 {
        match self.words.binary_search_by(|w| w.as_str().cmp(word)) {
            Ok(i) => self.counts[i],
            Err(_) => 0,
        }
    }

    /// Index range of the words that start with `prefix`.
    fn prefix_range(&self, prefix: &str) -> std::ops::Range<usize> {
        let lo = self.words.partition_point(|w| w.as_str() < prefix);
        let hi = lo + self.words[lo..].partition_point(|w| w.starts_with(prefix));
        lo..hi
    }

    /// Cost of the word itself, if it is in the lexicon.
    pub fn word_cost(&self, word: &str) -> Option<f32> {
        let c = self.count(word);
        (c > 0).then(|| half_bits(c as f64 / self.total as f64))
    }

    /// Cost of all words starting with `prefix` together, if there are any.
    pub fn prefix_cost(&self, prefix: &str) -> Option<f32> {
        let r = self.prefix_range(prefix);
        let n = self.cum[r.end] - self.cum[r.start];
        (n > 0).then(|| half_bits(n as f64 / self.total as f64))
    }

    /// The most common words starting with `prefix`, most common first.
    pub fn completions(&self, prefix: &str, max: usize) -> Vec<(&str, u32)> {
        let mut v: Vec<(&str, u32)> = self.prefix_range(prefix).map(|i| (self.words[i].as_str(), self.counts[i])).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(max);
        v
    }

    /// Character-model cost of a whole word (for words not in the lexicon).
    pub fn spelling_cost(&self, word: &str, complete: bool) -> f32 {
        self.chars.cost(word, complete)
    }
}

/// Count given to everyday words (`data/lexicon/everyday.tsv`) that the books rarely use: about as
/// common as අම්මා should be (the books total about four million words).
const EVERYDAY: u32 = 600;

/// Count given to a name (`data/lexicon/names.tsv`) without one.
const NAME: u32 = 80;

/// Is the word Pali only (`data/lexicon/pali.txt`)?
pub fn is_pali(word: &str) -> bool {
    static S: OnceLock<HashSet<&'static str>> = OnceLock::new();
    S.get_or_init(|| include_str!("../../../data/lexicon/pali.txt").lines().filter(|l| !l.starts_with('#')).collect()).contains(word)
}

fn user_shortcuts() -> &'static RwLock<HashMap<String, Vec<String>>> {
    static U: OnceLock<RwLock<HashMap<String, Vec<String>>>> = OnceLock::new();
    U.get_or_init(Default::default)
}

/// Replaces the user's own shortcuts: lines of `keys=words` or `keys<TAB>words` (`#` comments). The keys
/// are what is typed on the keyboard in the layout in use, exactly (capitals matter).
pub fn set_user_shortcuts(text: &str) {
    let mut m: HashMap<String, Vec<String>> = HashMap::new();
    for l in text.lines().filter(|l| !l.starts_with('#')) {
        if let Some((k, v)) = l.split_once('\t').or_else(|| l.split_once('=')) {
            if !k.trim().is_empty() && !v.trim().is_empty() {
                m.entry(k.trim().to_string()).or_default().push(v.trim().to_string());
            }
        }
    }
    *user_shortcuts().write().unwrap() = m;
}

/// What the user's own shortcuts say the typed keys stand for.
pub fn user_shortcut(keys: &str) -> Vec<String> {
    user_shortcuts().read().unwrap().get(keys).cloned().unwrap_or_default()
}

/// What the letters stand for when they are a shortcut (`data/lexicon/shortcuts.tsv`): col → කොළඹ.
pub fn shortcuts(latin: &str) -> &'static [&'static str] {
    static S: OnceLock<HashMap<&'static str, Vec<&'static str>>> = OnceLock::new();
    let map = S.get_or_init(|| {
        let mut m: HashMap<&'static str, Vec<&'static str>> = HashMap::new();
        for l in include_str!("../../../data/lexicon/shortcuts.tsv").lines().filter(|l| !l.starts_with('#')) {
            if let Some((k, v)) = l.split_once('\t') {
                m.entry(k.trim()).or_default().push(v.trim());
            }
        }
        m
    });
    map.get(latin.to_ascii_lowercase().as_str()).map_or(&[], Vec::as_slice)
}

/// Common English words, typed in the middle of Sinhala (`data/lexicon/english.txt`).
pub fn is_english(word: &str) -> bool {
    static S: OnceLock<HashSet<&'static str>> = OnceLock::new();
    let set = S.get_or_init(|| {
        include_str!("../../../data/lexicon/english.txt")
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect()
    });
    set.contains(word.to_ascii_lowercase().as_str())
}

fn half_bits(p: f64) -> f32 {
    (-2.0 * p.log2()) as f32
}

/// Interpolated character trigram model over words (Witten–Bell style smoothing), word frequency weighted.
struct CharModel {
    uni: HashMap<char, f64>,
    uni_total: f64,
    bi: HashMap<(char, char), f64>,
    bi_ctx: HashMap<char, (f64, f64)>, // (count, distinct followers)
    tri: HashMap<(char, char, char), f64>,
    tri_ctx: HashMap<(char, char), (f64, f64)>,
    vocab: f64,
}

const START: char = '^';
const END: char = '$';

impl CharModel {
    fn train<'a>(words: impl Iterator<Item = (&'a str, u32)>) -> CharModel {
        let mut m = CharModel {
            uni: HashMap::new(),
            uni_total: 0.0,
            bi: HashMap::new(),
            bi_ctx: HashMap::new(),
            tri: HashMap::new(),
            tri_ctx: HashMap::new(),
            vocab: 0.0,
        };
        for (w, c) in words {
            // damp very common words so the model learns spelling, not a few function words
            let wt = 1.0 + (c as f64).ln();
            let s: Vec<char> = [START, START].into_iter().chain(w.chars()).chain([END]).collect();
            for i in 2..s.len() {
                let (a, b, c) = (s[i - 2], s[i - 1], s[i]);
                *m.uni.entry(c).or_default() += wt;
                m.uni_total += wt;
                *m.bi.entry((b, c)).or_default() += wt;
                *m.tri.entry((a, b, c)).or_default() += wt;
            }
        }
        for (&(b, _), &n) in &m.bi {
            let e = m.bi_ctx.entry(b).or_default();
            e.0 += n;
            e.1 += 1.0;
        }
        for (&(a, b, _), &n) in &m.tri {
            let e = m.tri_ctx.entry((a, b)).or_default();
            e.0 += n;
            e.1 += 1.0;
        }
        m.vocab = m.uni.len() as f64 + 1.0;
        m
    }

    fn p(&self, a: char, b: char, c: char) -> f64 {
        let p1 = (self.uni.get(&c).copied().unwrap_or(0.0) + 1.0) / (self.uni_total + self.vocab);
        let p2 = match self.bi_ctx.get(&b) {
            Some(&(n, t)) => (self.bi.get(&(b, c)).copied().unwrap_or(0.0) + t * p1) / (n + t),
            None => p1,
        };
        match self.tri_ctx.get(&(a, b)) {
            Some(&(n, t)) => (self.tri.get(&(a, b, c)).copied().unwrap_or(0.0) + t * p2) / (n + t),
            None => p2,
        }
    }

    /// `complete`: the word ends here (adds the end-of-word step).
    fn cost(&self, word: &str, complete: bool) -> f32 {
        let s: Vec<char> = [START, START].into_iter().chain(word.chars()).chain(complete.then_some(END)).collect();
        let mut cost = 0.0;
        for i in 2..s.len() {
            cost += half_bits(self.p(s[i - 2], s[i - 1], s[i]));
        }
        cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookups() {
        let l = Lexicon::books();
        assert!(l.count("ලංකාව") > 10);
        assert_eq!(l.count("ලන්කාව"), 0);
        assert!(l.prefix_cost("ලංක").is_some());
        assert!(l.completions("ස්වාමීන්", 5).iter().any(|(w, _)| *w == "ස්වාමීන්"));
        // real spellings cost less than wrong ones
        assert!(l.spelling_cost("අම්මා", true) < l.spelling_cost("අම්ම්ා", true));
        assert!(l.spelling_cost("ලංකා", true) < l.spelling_cost("ලන්කා", true));
    }
}
