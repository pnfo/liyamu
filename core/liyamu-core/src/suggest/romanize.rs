//! Loose Latin keys: what a Sinhala word and what a person typed in Singlish have in common once everything
//! typists do not reliably distinguish is collapsed (ත ට ථ ඨ → t, long and short vowels, ං න ණ → n,
//! doubled vowels ...). Two spellings with the same key are readings of each other; keys that differ a
//! little are typos of each other.

use crate::chars::{is_cons, HAL, ZWJ};

fn cons_key(c: char) -> &'static str {
    match c {
        'ක' | 'ඛ' => "k",
        'ග' | 'ඝ' => "g",
        'ඞ' | 'ණ' | 'න' | 'ඤ' => "n",
        'ඟ' => "ng",
        'ච' | 'ඡ' => "c",
        'ජ' | 'ඣ' => "j",
        'ඥ' => "gn",
        'ඦ' => "nj",
        'ට' | 'ඨ' | 'ත' | 'ථ' => "t",
        'ඩ' | 'ඪ' | 'ද' | 'ධ' => "d",
        'ඳ' | 'ඬ' => "nd",
        'ප' | 'ඵ' => "p",
        'බ' | 'භ' => "b",
        'ම' => "m",
        'ඹ' => "mb",
        'ය' => "y",
        'ර' => "r",
        'ල' | 'ළ' => "l",
        'ව' => "v",
        'ශ' | 'ෂ' | 'ස' => "s",
        'හ' => "h",
        'ෆ' => "f",
        _ => "",
    }
}

fn vowel_key(c: char) -> Option<&'static str> {
    Some(match c {
        'අ' | 'ආ' | 'ඇ' | 'ඈ' | 'ා' | 'ැ' | 'ෑ' => "a",
        'ඉ' | 'ඊ' | 'ි' | 'ී' => "i",
        'උ' | 'ඌ' | 'ු' | 'ූ' => "u",
        'එ' | 'ඒ' | 'ෙ' | 'ේ' => "e",
        'ඔ' | 'ඕ' | 'ො' | 'ෝ' => "o",
        'ඓ' | 'ෛ' => "ai",
        'ඖ' | 'ෞ' => "au",
        'ඍ' | 'ඎ' | 'ෘ' | 'ෲ' => "ru",
        'ඏ' | 'ඐ' | 'ෟ' | 'ෳ' => "lu",
        _ => return None,
    })
}

/// Appends to a key. A vowel repeated is one vowel (aa, and a long vowel, are the short one).
fn push(out: &mut String, s: &str) {
    for c in s.chars() {
        if !(is_vowel(c as u8) && out.ends_with(c)) {
            out.push(c);
        }
    }
}

/// The loose key of a Sinhala word: ගෙදර → gedara, අම්මා → amma, ලංකාව → lankava.
pub fn key_of_sinhala(word: &str) -> String {
    let cs: Vec<char> = word.chars().filter(|c| *c != ZWJ && *c != '\u{200C}').collect();
    let mut out = String::new();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if is_cons(c) {
            push(&mut out, cons_key(c));
            match cs.get(i + 1) {
                Some(&HAL) => i += 1,
                Some(&n) if vowel_key(n).is_some() && !('අ'..='ඖ').contains(&n) => {
                    push(&mut out, vowel_key(n).unwrap());
                    i += 1;
                }
                _ => push(&mut out, "a"),
            }
        } else if let Some(v) = vowel_key(c) {
            push(&mut out, v);
        } else if c == 'ං' {
            push(&mut out, "n");
        } else if c == 'ඃ' {
            push(&mut out, "h");
        }
        i += 1;
    }
    out
}

/// Latin spellings and what they become in a key, longest first. Two results: either may be meant.
const LATIN: &[(&str, &[&str])] = &[
    ("chh", &["c"]),
    ("aae", &["a"]),
    ("aee", &["a"]),
    ("ch", &["c"]),
    ("th", &["t"]),
    ("dh", &["d"]),
    ("kh", &["k"]),
    ("gh", &["g"]),
    ("bh", &["b"]),
    ("ph", &["p"]),
    ("sh", &["s"]),
    ("jh", &["j"]),
    ("aa", &["a"]),
    ("ae", &["a"]),
    ("ii", &["i"]),
    ("uu", &["u"]),
    ("ee", &["e", "i"]),
    ("oo", &["o", "u"]),
    ("ou", &["au"]),
    ("c", &["k", "c"]),
    ("w", &["v"]),
    ("x", &["n"]),
    ("q", &["d"]),
    ("z", &[""]),
];

/// The loose keys of what was typed in Singlish (a few when a spelling is ambiguous: ee is ඒ or ඊ).
pub fn keys_of_latin(latin: &str) -> Vec<String> {
    let lower = latin.to_ascii_lowercase();
    let mut keys = vec![String::new()];
    let mut rest = lower.as_str();
    while !rest.is_empty() {
        let (n, outs): (usize, &[&str]) = match LATIN.iter().find(|(k, _)| rest.starts_with(k)) {
            Some((k, outs)) => (k.len(), outs),
            None => (rest.chars().next().unwrap().len_utf8(), &[]),
        };
        if outs.is_empty() {
            let c = &rest[..n];
            if c.chars().all(|c| c.is_ascii_lowercase()) {
                keys.iter_mut().for_each(|k| push(k, c));
            }
        } else {
            let mut next = Vec::new();
            for k in &keys {
                for o in outs.iter().take(if keys.len() >= 4 { 1 } else { 2 }) {
                    let mut k = k.clone();
                    push(&mut k, o);
                    next.push(k);
                }
            }
            keys = next;
        }
        rest = &rest[n..];
    }
    keys.dedup();
    keys
}

/// The key without its vowels: gedara → gdr. Words whose keys differ only in vowels share it.
pub fn skeleton(key: &str) -> String {
    key.chars().filter(|c| !is_vowel(*c as u8)).collect()
}

pub fn is_vowel(c: u8) -> bool {
    matches!(c, b'a' | b'e' | b'i' | b'o' | b'u')
}

/// How a person might type a Sinhala word in Singlish, for measuring suggestions (`liyamu eval`).
/// `careful`: long vowels doubled (aa, ee), otherwise the casual single letters.
pub fn casual(word: &str, careful: bool) -> String {
    let cs: Vec<char> = word.chars().filter(|c| *c != ZWJ && *c != '\u{200C}').collect();
    let cons = |c: char| match c {
        'ක' => "k",
        'ඛ' => "kh",
        'ග' => "g",
        'ඝ' => "gh",
        'ඞ' | 'ණ' | 'න' | 'ඤ' => "n",
        'ඟ' => "ng",
        'ච' | 'ඡ' => "ch",
        'ජ' => "j",
        'ඣ' => "jh",
        'ඥ' => "gn",
        'ඦ' => "nj",
        'ට' | 'ඨ' => "t",
        'ත' | 'ථ' => "th",
        'ඩ' | 'ඪ' | 'ද' => "d",
        'ධ' => "dh",
        'ඳ' | 'ඬ' => "nd",
        'ප' | 'ඵ' => "p",
        'බ' => "b",
        'භ' => "bh",
        'ම' => "m",
        'ඹ' => "mb",
        'ය' => "y",
        'ර' => "r",
        'ල' | 'ළ' => "l",
        'ව' => "w",
        'ශ' | 'ෂ' => "sh",
        'ස' => "s",
        'හ' => "h",
        'ෆ' => "f",
        _ => "",
    };
    let vowel = |c: char| -> Option<&'static str> {
        Some(match c {
            'අ' => "a",
            'ආ' | 'ා' => if careful { "aa" } else { "a" },
            'ඇ' | 'ැ' => if careful { "ae" } else { "a" },
            'ඈ' | 'ෑ' => if careful { "aee" } else { "a" },
            'ඉ' | 'ි' => "i",
            'ඊ' | 'ී' => if careful { "ii" } else { "i" },
            'උ' | 'ු' => "u",
            'ඌ' | 'ූ' => if careful { "uu" } else { "u" },
            'එ' | 'ෙ' => "e",
            'ඒ' | 'ේ' => if careful { "ee" } else { "e" },
            'ඔ' | 'ො' => "o",
            'ඕ' | 'ෝ' => if careful { "oo" } else { "o" },
            'ඓ' | 'ෛ' => "ai",
            'ඖ' | 'ෞ' => "au",
            'ඍ' | 'ෘ' | 'ඎ' | 'ෲ' => "ru",
            _ => return None,
        })
    };
    let mut out = String::new();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if is_cons(c) {
            out.push_str(cons(c));
            match cs.get(i + 1) {
                Some(&HAL) => i += 1,
                Some(&n) if vowel(n).is_some() && !('අ'..='ඖ').contains(&n) => {
                    out.push_str(vowel(n).unwrap());
                    i += 1;
                }
                _ => out.push('a'),
            }
        } else if let Some(v) = vowel(c) {
            out.push_str(v);
        } else if c == 'ං' {
            out.push('n');
        } else if c == 'ඃ' {
            out.push('h');
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys() {
        assert_eq!(key_of_sinhala("ගෙදර"), "gedara");
        assert_eq!(key_of_sinhala("අම්මා"), "amma");
        assert_eq!(key_of_sinhala("ලංකාව"), "lankava");
        assert_eq!(key_of_sinhala("ශ්‍රී"), "sri");
        assert_eq!(key_of_sinhala("තාත්තා"), "tatta");
        assert_eq!(keys_of_latin("thaththa"), ["tatta"]);
        assert_eq!(keys_of_latin("gedara"), ["gedara"]);
        assert_eq!(keys_of_latin("Lankaawa"), ["lankava"]);
        assert_eq!(keys_of_latin("meeka"), ["meka", "mika"]);
        assert_eq!(skeleton("gedara"), "gdr");
        assert_eq!(casual("තියෙනවා", false), "thiyenawa");
        assert_eq!(casual("ගෙදර", true), "gedara");
    }
}
