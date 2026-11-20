//! Legacy font encodings (FM, Isi) to Unicode and back.
//!
//! A line-by-line port of pitaka.lk `tools/unicode/lib/convert.js`. The tables themselves are exported from
//! pitaka.lk by `tools/export-encodings.mjs` into `data/encodings/*.json`, so pitaka.lk stays the single
//! source of truth. `tests/encode_fixtures.rs` checks this port against pitaka.lk's own output.

use crate::chars::{is_cons, is_tail, is_vowel_sign, HAL, KOMBU, REPAYA, ZWJ};
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct EncodingJson {
    id: String,
    label: String,
    map: Vec<(String, String)>,
    #[serde(default)]
    pre: Vec<(String, String)>,
    #[serde(default)]
    post: Vec<(String, String)>,
    #[serde(default)]
    unpost: Vec<(String, String)>,
    #[serde(default)]
    variants: Vec<VariantJson>,
    /// keys that are read but never written
    #[serde(default, rename = "readOnly")]
    read_only: Vec<String>,
    /// (sign, key, letters): the sign is written with this key after one of these letters (a consonant, or a
    /// ්‍ + consonant join). FM fonts have two papillas, ispillas and hal signs: කු is `l=`, මු is `uq`.
    #[serde(default)]
    after: Vec<(String, String, Vec<String>)>,
}

#[derive(Deserialize)]
struct VariantJson {
    #[serde(rename = "fontNames")]
    font_names: Vec<String>,
    map: Vec<(String, String)>,
}

type Rules = Vec<(Regex, String)>;

fn rules(list: &[(String, String)]) -> Rules {
    list.iter().map(|(re, to)| (Regex::new(re).expect("bad rule in encoding json"), to.clone())).collect()
}

fn apply(rules: &Rules, mut s: String) -> String {
    for (re, to) in rules {
        if re.is_match(&s) {
            s = re.replace_all(&s, to.as_str()).into_owned();
        }
    }
    s
}

/// One legacy encoding, compiled for one font (variants can change a few keys, e.g. FM Derana).
pub struct Encoding {
    pub id: String,
    pub label: String,
    map: HashMap<String, String>,
    max_len: usize,
    /// Unicode -> the first legacy key that produces it
    reverse: HashMap<String, String>,
    pre: Rules,
    post: Rules,
    unpost: Rules,
    /// (letter before, sign) → the key for the sign there
    after: HashMap<(String, String), String>,
}

impl Encoding {
    fn from_json(json: &str, font_name: Option<&str>) -> Encoding {
        let e: EncodingJson = serde_json::from_str(json).expect("bad encoding json");
        let mut pairs = e.map.clone();
        if let Some(font) = font_name {
            for v in &e.variants {
                if v.font_names.iter().any(|re| Regex::new(re).map(|r| r.is_match(font)).unwrap_or(false)) {
                    // a variant overrides keys in place (JS Map.set keeps the original position)
                    for (k, u) in &v.map {
                        match pairs.iter_mut().find(|(pk, _)| pk == k) {
                            Some(p) => p.1 = u.clone(),
                            None => pairs.push((k.clone(), u.clone())),
                        }
                    }
                }
            }
        }
        let mut map = HashMap::new();
        let mut reverse = HashMap::new();
        for (k, u) in &pairs {
            map.insert(k.clone(), u.clone());
        }
        // reverse is built from the final map in insertion order, like convert.js
        for (k, _) in &pairs {
            let u = &map[k];
            if !reverse.contains_key(u) && !u.chars().all(char::is_whitespace) && !e.read_only.contains(k) {
                reverse.insert(u.clone(), k.clone());
            }
        }
        let mut after = HashMap::new();
        for (sign, key, prevs) in &e.after {
            for p in prevs {
                after.insert((p.clone(), sign.clone()), key.clone());
            }
        }
        let max_len = map.keys().map(|k| k.chars().count()).max().unwrap_or(1);
        Encoding {
            id: e.id,
            label: e.label,
            map,
            max_len,
            reverse,
            pre: rules(&e.pre),
            post: rules(&e.post),
            unpost: rules(&e.unpost),
            after,
        }
    }

    pub fn fm() -> &'static Encoding {
        static E: OnceLock<Encoding> = OnceLock::new();
        E.get_or_init(|| Encoding::from_json(include_str!("../../../data/encodings/fm.json"), None))
    }

    pub fn isi() -> &'static Encoding {
        static E: OnceLock<Encoding> = OnceLock::new();
        E.get_or_init(|| Encoding::from_json(include_str!("../../../data/encodings/isi.json"), None))
    }

    /// The encoding as used by a particular font, e.g. "FM Derana".
    pub fn for_font(id: &str, font_name: &str) -> Option<Encoding> {
        let json = match id {
            "fm" => include_str!("../../../data/encodings/fm.json"),
            "isi" => include_str!("../../../data/encodings/isi.json"),
            _ => return None,
        };
        Some(Encoding::from_json(json, Some(font_name)))
    }

    fn tokenize(&self, text: &str) -> String {
        let s: Vec<char> = text.chars().collect();
        let mut out = String::new();
        let mut i = 0;
        while i < s.len() {
            let mut n = self.max_len.min(s.len() - i);
            while n > 0 {
                let key: String = s[i..i + n].iter().collect();
                if let Some(u) = self.map.get(&key) {
                    out.push_str(u);
                    break;
                }
                n -= 1;
            }
            if n == 0 {
                out.push(s[i]);
                n = 1;
            }
            i += n;
        }
        out
    }

    /// Legacy font text to Unicode.
    pub fn to_unicode(&self, text: &str) -> String {
        let text = apply(&self.pre, text.to_string());
        apply(&self.post, reorder(&self.tokenize(&text)))
    }

    /// Unicode to legacy. `None` when some part can't be written in the encoding.
    pub fn from_unicode(&self, text: &str) -> Option<String> {
        let text: Vec<char> = apply(&self.unpost, text.to_string()).chars().collect();
        let at = |i: usize| text.get(i).copied().unwrap_or('\0');
        let mut out = String::new();
        let mut i = 0;
        while i < text.len() {
            let mut j = i;
            let mut repaya = false;
            if at(j) == 'ර' && at(j + 1) == HAL && at(j + 2) == ZWJ && is_cons(at(j + 3)) {
                repaya = true;
                j += 3;
            }
            if !is_cons(at(j)) {
                out.push_str(&self.greedy(&text[i..i + 1])?);
                i += 1;
                continue;
            }
            let mut e = j + 1;
            while at(e) == HAL && at(e + 1) == ZWJ && is_cons(at(e + 2)) {
                e += 3;
            }
            let cluster = &text[j..e];
            let mut marks: Vec<char> = Vec::new();
            while is_vowel_sign(at(e)) || at(e) == HAL || is_tail(at(e)) {
                marks.push(at(e));
                e += 1;
            }
            // the first kombu sign becomes kombu keys before the syllable plus what remains of the sign
            let mut kombu = 0;
            if let Some(p) = marks.iter().position(|c| "ෙේොෝෞෛ".contains(*c)) {
                let (k, rest): (usize, &[char]) = match marks[p] {
                    'ෙ' => (1, &[]),
                    'ේ' => (1, &[HAL]),
                    'ො' => (1, &['ා']),
                    'ෝ' => (1, &['ා', HAL]),
                    'ෞ' => (1, &['ෟ']),
                    _ => (2, &[]), // ෛ
                };
                kombu = k;
                marks.splice(p..p + 1, rest.iter().copied());
            }
            let body = if repaya {
                let first = self.greedy(&cluster[..1])?;
                let rest: Vec<char> = cluster[1..].iter().chain(marks.iter()).copied().collect();
                let rest = self.greedy(&rest)?;
                let r = self.reverse.get(&REPAYA.to_string())?;
                format!("{first}{r}{rest}")
            } else {
                let all: Vec<char> = cluster.iter().chain(marks.iter()).copied().collect();
                self.greedy(&all)?
            };
            if kombu > 0 {
                out.push_str(&self.reverse.get(&KOMBU.to_string())?.repeat(kombu));
            }
            out.push_str(&body);
            i = e;
        }
        Some(out)
    }

    /// Units are a consonant, a ්‍ + consonant join, or one sign. A legacy key may cover several whole units.
    fn greedy(&self, s: &[char]) -> Option<String> {
        let mut units: Vec<String> = Vec::new();
        let mut i = 0;
        while i < s.len() {
            let n = if s[i] == HAL && s.get(i + 1) == Some(&ZWJ) && s.get(i + 2).is_some_and(|c| is_cons(*c)) { 3 } else { 1 };
            units.push(s[i..i + n].iter().collect());
            i += n;
        }
        let mut out = String::new();
        let mut i = 0;
        while i < units.len() {
            if let Some(k) = i.checked_sub(1).and_then(|p| self.after.get(&(units[p].clone(), units[i].clone()))) {
                out.push_str(k);
                i += 1;
                continue;
            }
            let mut n = units.len() - i;
            while n > 0 {
                if let Some(k) = self.reverse.get(&units[i..i + n].concat()) {
                    out.push_str(k);
                    break;
                }
                n -= 1;
            }
            if n == 0 {
                if units[i].chars().any(|c| ('\u{0D80}'..='\u{0DFF}').contains(&c) || c == ZWJ || c == '\u{200C}') {
                    return None;
                }
                out.push_str(&units[i]);
                n = 1;
            }
            i += n;
        }
        Some(out)
    }

    /// Like `from_unicode`, but always gives something. Syllables that can't be written are approximated:
    /// a kombuva with no letter yet (while typing) becomes the kombuva key, touching letters become plain
    /// hal, half-typed joins lose their ZWJ, and anything else is left in Unicode. The flag says whether
    /// anything was approximated.
    pub fn from_unicode_lossy(&self, text: &str) -> (String, bool) {
        if let Some(s) = self.from_unicode(text) {
            return (s, false);
        }
        let mut out = String::new();
        let mut lossy = false;
        for syl in syllables(text) {
            if let Some(s) = self.from_unicode(&syl) {
                out.push_str(&s);
                continue;
            }
            let kombu = self.reverse.get(&KOMBU.to_string());
            let lone = syl.chars().all(|c| c == 'ෙ' || c == 'ෛ');
            if let (true, Some(k)) = (lone, kombu) {
                for c in syl.chars() {
                    out.push_str(&k.repeat(if c == 'ෛ' { 2 } else { 1 }));
                }
                continue;
            }
            lossy = true;
            let plain = syl.replace("\u{200D}\u{0DCA}", "\u{0DCA}");
            let bare = plain.replace('\u{200D}', "");
            match self.from_unicode(&plain).or_else(|| self.from_unicode(&bare)) {
                Some(s) => out.push_str(&s),
                None => out.push_str(&syl),
            }
        }
        (out, lossy)
    }
}

/// Splits Unicode text before each consonant or vowel letter that starts a new syllable.
fn syllables(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut prev = '\0';
    for c in text.chars() {
        let starts = (is_cons(c) || ('\u{0D85}'..='\u{0D96}').contains(&c)) && prev != ZWJ;
        if starts || out.is_empty() {
            out.push(String::new());
        }
        out.last_mut().unwrap().push(c);
        prev = c;
    }
    out
}

/// Puts each syllable in logical order: repaya, consonant cluster, yansaya/rakaransaya, vowel sign, ං/ඃ.
/// Input may contain the KOMBU and REPAYA markers in visual (typed) positions.
pub fn reorder(s: &str) -> String {
    let s: Vec<char> = s.chars().collect();
    let at = |i: usize| s.get(i).copied().unwrap_or('\0');
    let mut out = String::new();
    let mut i = 0;
    while i < s.len() {
        let mut j = i;
        let mut kombu = 0;
        while at(j) == KOMBU {
            kombu += 1;
            j += 1;
        }
        if !is_cons(at(j)) {
            if kombu == 1 && at(j) == 'එ' {
                out.push('ඓ');
                i = j + 1;
                continue;
            }
            match kombu {
                0 => {}
                2 => out.push('ෛ'),
                k => out.push_str(&"ෙ".repeat(k)),
            }
            if j < s.len() {
                out.push(s[j]);
            }
            i = j + 1;
            continue;
        }
        let mut c = j + 1;
        while at(c) == HAL && at(c + 1) == ZWJ && is_cons(at(c + 2)) {
            c += 3; // conjuncts, yansaya, rakaransaya
        }
        let cluster: String = s[j..c].iter().collect();
        let mut repaya = false;
        let mut post = String::new();
        let mut vowels: Vec<char> = Vec::new();
        let mut tail = String::new();
        loop {
            let ch = at(c);
            if ch == REPAYA {
                repaya = true;
                c += 1;
            } else if ch == HAL && at(c + 1) == ZWJ && (at(c + 2) == 'ය' || at(c + 2) == 'ර') {
                post.extend(&s[c..c + 3]);
                c += 3;
            } else if is_vowel_sign(ch) || ch == HAL {
                vowels.push(ch);
                c += 1;
            } else if is_tail(ch) {
                tail.push(ch);
                c += 1;
            } else {
                break;
            }
        }
        let mut v = String::new();
        if kombu == 1 {
            let mut take = |m: char| match vowels.iter().position(|x| *x == m) {
                Some(k) => {
                    vowels.remove(k);
                    true
                }
                None => false,
            };
            let aa = take('ා');
            let hal = take(HAL);
            let ga = !aa && take('ෟ');
            v.push(if aa {
                if hal { 'ෝ' } else { 'ො' }
            } else if hal {
                'ේ'
            } else if ga {
                'ෞ'
            } else {
                'ෙ'
            });
        } else if kombu >= 2 {
            v.push('ෛ');
            v.push_str(&"ෙ".repeat(kombu - 2));
        }
        if repaya {
            out.push_str("ර\u{0DCA}\u{200D}");
        }
        out.push_str(&cluster);
        out.push_str(&post);
        out.push_str(&v);
        out.extend(vowels);
        out.push_str(&tail);
        i = c;
    }
    out
}
