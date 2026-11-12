//! The keys of Helakuru's phonetic keyboard, from the keymap Bhasha Lanka published for Linux
//! (si-helakuru-phonetic.mim, 2018), re-implemented here. Case-sensitive.
//!
//! Extras for sequences Helakuru leaves unmapped (where it would type something unusable):
//! `ae` ඇ, `Sh` ෂ, `jh` ඣ, `zb` ඹ, `ai`/`au` as letters, `Aa` as a sign. `z` stops a join (kzya → ක්ය)
//! and types nothing, where Helakuru types a ZWNJ.

use super::{render as render_toks, Join, Tok};

const fn c(s: &'static str) -> Tok {
    Tok::Cons(s)
}
const fn v(letter: &'static str, sign: &'static str) -> Tok {
    Tok::Vowel { letter, sign }
}

const KEYS: &[(&str, Tok)] = &[
    // vowels
    ("a", v("අ", "")),
    ("aa", v("ආ", "ා")),
    ("A", v("ඇ", "ැ")),
    ("ae", v("ඇ", "ැ")),
    ("AA", v("ඈ", "ෑ")),
    ("Aa", v("ඈ", "ෑ")),
    ("i", v("ඉ", "ි")),
    ("I", v("ඉ", "ි")),
    ("ii", v("ඊ", "ී")),
    ("u", v("උ", "ු")),
    ("U", v("උ", "ු")),
    ("uu", v("ඌ", "ූ")),
    ("e", v("එ", "ෙ")),
    ("ee", v("ඒ", "ේ")),
    ("E", v("ඓ", "")),
    ("ai", v("ඓ", "ෛ")),
    ("o", v("ඔ", "ො")),
    ("O", v("ඔ", "ො")),
    ("oo", v("ඕ", "ෝ")),
    ("ou", v("ඖ", "ෞ")),
    ("au", v("ඖ", "ෞ")),
    ("sRu", v("ඍ", "ෘ")),
    ("sRuu", v("ඎ", "ෲ")),
    // consonants
    ("k", c("ක")),
    ("K", c("ඛ")),
    ("kh", c("ඛ")),
    ("g", c("ග")),
    ("G", c("ඝ")),
    ("gh", c("ඝ")),
    ("ch", c("ච")),
    ("Ch", c("ඡ")),
    ("c", c("ක")),
    ("C", c("ක")),
    ("j", c("ජ")),
    ("J", c("ඣ")),
    ("jh", c("ඣ")),
    ("t", c("ට")),
    ("T", c("ඨ")),
    ("th", c("ත")),
    ("Th", c("ථ")),
    ("d", c("ඩ")),
    ("D", c("ඪ")),
    ("dh", c("ද")),
    ("q", c("ද")),
    ("Dh", c("ධ")),
    ("Q", c("ධ")),
    ("n", c("න")),
    ("N", c("ණ")),
    ("p", c("ප")),
    ("P", c("ඵ")),
    ("ph", c("ඵ")),
    ("b", c("බ")),
    ("bh", c("භ")),
    ("Bh", c("භ")),
    ("B", c("ඹ")),
    ("m", c("ම")),
    ("M", c("ම")),
    ("y", c("ය")),
    ("r", c("ර")),
    ("R", c("ර")),
    ("l", c("ල")),
    ("L", c("ළ")),
    ("v", c("ව")),
    ("V", c("ව")),
    ("w", c("ව")),
    ("W", c("ව")),
    ("sh", c("ශ")),
    ("S", c("ෂ")),
    ("Sh", c("ෂ")),
    ("s", c("ස")),
    ("h", c("හ")),
    ("f", c("ෆ")),
    ("F", c("ෆ")),
    ("X", c("ඞ")),
    ("zD", c("ඞ")),
    ("ZD", c("ඞ")),
    ("zk", c("ඤ")),
    ("Zk", c("ඤ")),
    ("zh", c("ඥ")),
    ("Zh", c("ඥ")),
    ("zg", c("ඟ")),
    ("Zg", c("ඟ")),
    ("zd", c("ඬ")),
    ("Zd", c("ඬ")),
    ("zq", c("ඳ")),
    ("Zq", c("ඳ")),
    ("zdh", c("ඳ")),
    ("Zdh", c("ඳ")),
    ("zj", c("ඦ")),
    ("Zj", c("ඦ")),
    ("zb", c("ඹ")),
    ("NG", c("ක\u{0DCA}\u{200D}ෂ")),
    // signs
    ("x", Tok::Mod("ං")),
    ("zn", Tok::Mod("ං")),
    ("Zn", Tok::Mod("ං")),
    ("H", Tok::Mod("ඃ")),
    ("z", Tok::Break),
    ("Z", Tok::Break),
];

fn tokenize(latin: &str) -> Vec<Tok> {
    let mut toks: Vec<Tok> = Vec::new();
    let mut rest = latin;
    while let Some(ch) = rest.chars().next() {
        // after a consonant, ru / ruu are the vowel signs ෘ ෲ (kru → කෘ)
        if matches!(toks.last(), Some(Tok::Cons(_))) {
            if let Some(r) = rest.strip_prefix("ruu") {
                toks.push(v("ඎ", "ෲ"));
                rest = r;
                continue;
            }
            if let Some(r) = rest.strip_prefix("ru") {
                toks.push(v("ඍ", "ෘ"));
                rest = r;
                continue;
            }
        }
        match KEYS.iter().filter(|(k, _)| rest.starts_with(k)).max_by_key(|(k, _)| k.len()) {
            Some((k, t)) => {
                toks.push(t.clone());
                rest = &rest[k.len()..];
            }
            None => rest = &rest[ch.len_utf8()..], // not a key (q-less letters are all mapped)
        }
    }
    toks
}

pub fn render(latin: &str) -> String {
    render_toks(&tokenize(latin), Join::Always)
}
