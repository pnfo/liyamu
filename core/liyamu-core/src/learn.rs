//! Learning from the user's choices: the keys typed and the suggestion picked for them. A word picked
//! once for some keys moves up to second place; picked twice it comes first, until another word is picked
//! twice for the same keys. Kept in one small text file on the device, which the app names; nothing here
//! goes anywhere else, and `forget` deletes it.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

/// Picks that put a word first.
const FIRST: u32 = 2;
/// A count never goes above this, so that a change of mind takes over quickly.
const MOST: u32 = 4;
/// Keys remembered; the least recently used go first.
const ENTRIES: usize = 4000;

struct Choice {
    word: String,
    count: u32,
    /// when it was last picked: a counter, not a time
    stamp: u64,
}

#[derive(Default)]
pub struct Learned {
    map: HashMap<String, Vec<Choice>>,
    clock: u64,
}

impl Learned {
    /// Lines of `keys<TAB>word<TAB>count<TAB>stamp`.
    pub fn parse(text: &str) -> Learned {
        let mut l = Learned::default();
        for line in text.lines() {
            let f: Vec<&str> = line.split('\t').collect();
            if let [keys, word, count, stamp] = f[..] {
                if let (Ok(count), Ok(stamp)) = (count.parse::<u32>(), stamp.parse::<u64>()) {
                    if !keys.is_empty() && !word.is_empty() && count > 0 {
                        l.clock = l.clock.max(stamp);
                        l.map.entry(keys.to_string()).or_default().push(Choice { word: word.to_string(), count: count.min(MOST), stamp });
                    }
                }
            }
        }
        l
    }

    pub fn to_text(&self) -> String {
        let mut lines: Vec<(u64, String)> = Vec::new();
        for (keys, choices) in &self.map {
            for c in choices {
                lines.push((c.stamp, format!("{keys}\t{}\t{}\t{}\n", c.word, c.count, c.stamp)));
            }
        }
        lines.sort();
        lines.into_iter().map(|(_, l)| l).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// The user picked `word` for `keys` instead of what was offered first.
    pub fn record(&mut self, keys: &str, word: &str) {
        if keys.is_empty() || word.is_empty() || keys.contains(['\t', '\n']) || word.contains(['\t', '\n']) {
            return;
        }
        self.clock += 1;
        let choices = self.map.entry(keys.to_string()).or_default();
        // the other words for these keys lose a pick, so the newer habit wins
        for c in choices.iter_mut().filter(|c| c.word != word) {
            c.count -= 1;
        }
        choices.retain(|c| c.count > 0);
        match choices.iter_mut().find(|c| c.word == word) {
            Some(c) => {
                c.count = (c.count + 1).min(MOST);
                c.stamp = self.clock;
            }
            None => choices.push(Choice { word: word.to_string(), count: 1, stamp: self.clock }),
        }
        if self.map.len() > ENTRIES {
            let mut ages: Vec<(u64, String)> =
                self.map.iter().map(|(k, v)| (v.iter().map(|c| c.stamp).max().unwrap_or(0), k.clone())).collect();
            ages.sort();
            for (_, k) in ages.into_iter().take(self.map.len() - ENTRIES * 9 / 10) {
                self.map.remove(&k);
            }
        }
    }

    /// Reorders the suggestions for `keys`: the word picked at least twice comes first, other picked words
    /// right after the first. The first `fixed` suggestions (the user's own shortcuts) keep their places.
    pub fn apply(&self, keys: &str, candidates: &mut Vec<String>, fixed: usize) {
        let Some(choices) = self.map.get(keys) else { return };
        if candidates.is_empty() {
            return;
        }
        let mut order: Vec<&Choice> = choices.iter().collect();
        order.sort_by(|a, b| b.count.cmp(&a.count).then(b.stamp.cmp(&a.stamp)));
        let mut at = fixed.min(candidates.len());
        if order[0].count < FIRST {
            at = (at + 1).min(candidates.len());
        }
        for c in order {
            match candidates.iter().position(|w| *w == c.word) {
                Some(i) if i <= at => {}
                Some(i) => {
                    let w = candidates.remove(i);
                    candidates.insert(at, w);
                }
                None => candidates.insert(at, c.word.clone()),
            }
            at = (at + 1).min(candidates.len());
        }
    }
}

struct State {
    on: bool,
    path: Option<PathBuf>,
    learned: Learned,
}

fn state() -> &'static RwLock<State> {
    static S: OnceLock<RwLock<State>> = OnceLock::new();
    S.get_or_init(|| RwLock::new(State { on: false, path: None, learned: Learned::default() }))
}

/// Turns learning on or off for all sessions. `path` names the file that keeps what was learned; it is
/// read when first given. Off: nothing is recorded or used, but the file stays until `forget`.
pub fn configure(path: Option<&str>, on: bool) {
    let mut s = state().write().unwrap();
    s.on = on;
    if let Some(p) = path.map(PathBuf::from) {
        if s.path.as_ref() != Some(&p) {
            s.learned = Learned::parse(&std::fs::read_to_string(&p).unwrap_or_default());
            s.path = Some(p);
        }
    }
}

pub fn record(keys: &str, word: &str) {
    let mut s = state().write().unwrap();
    if !s.on {
        return;
    }
    s.learned.record(keys, word);
    if let Some(p) = &s.path {
        // written whole (it is small) beside the file and moved over it, so a crash leaves the old one
        let tmp = p.with_extension("tmp");
        if std::fs::write(&tmp, s.learned.to_text()).is_ok() {
            let _ = std::fs::rename(&tmp, p);
        }
    }
}

pub fn apply(keys: &str, candidates: &mut Vec<String>, fixed: usize) {
    let s = state().read().unwrap();
    if s.on {
        s.learned.apply(keys, candidates, fixed);
    }
}

/// Deletes everything learned, in memory and on disk.
pub fn forget() {
    let mut s = state().write().unwrap();
    s.learned = Learned::default();
    if let Some(p) = &s.path {
        let _ = std::fs::remove_file(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(l: &Learned, keys: &str) -> Vec<String> {
        let mut c: Vec<String> = ["a", "b", "c", "d"].iter().map(|s| s.to_string()).collect();
        l.apply(keys, &mut c, 0);
        c
    }

    #[test]
    fn once_moves_up_twice_comes_first() {
        let mut l = Learned::default();
        assert_eq!(list(&l, "mm"), ["a", "b", "c", "d"]);
        l.record("mm", "d");
        assert_eq!(list(&l, "mm"), ["a", "d", "b", "c"]);
        l.record("mm", "d");
        assert_eq!(list(&l, "mm"), ["d", "a", "b", "c"]);
        assert_eq!(list(&l, "m"), ["a", "b", "c", "d"]); // other keys are not touched
    }

    #[test]
    fn another_word_takes_over_after_two_picks() {
        let mut l = Learned::default();
        for _ in 0..6 {
            l.record("mm", "d");
        }
        l.record("mm", "c");
        assert_eq!(list(&l, "mm")[0], "d");
        l.record("mm", "c");
        assert_eq!(list(&l, "mm"), ["c", "d", "a", "b"]);
    }

    #[test]
    fn a_word_not_offered_is_added_and_shortcuts_stay_first() {
        let mut l = Learned::default();
        l.record("mm", "x");
        l.record("mm", "x");
        assert_eq!(list(&l, "mm"), ["x", "a", "b", "c", "d"]);
        let mut c = vec!["own".to_string(), "a".to_string()];
        l.apply("mm", &mut c, 1);
        assert_eq!(c, ["own", "x", "a"]);
    }

    #[test]
    fn survives_the_file() {
        let mut l = Learned::default();
        l.record("mm", "මම");
        l.record("mm", "මම");
        l.record("compute", "computer");
        let again = Learned::parse(&l.to_text());
        assert_eq!(again.to_text(), l.to_text());
        assert_eq!(list(&again, "mm")[0], "මම");
    }
}
