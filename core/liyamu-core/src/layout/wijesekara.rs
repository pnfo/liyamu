//! Wijesekara layout (the Sri Lankan standard), as on Windows and in FM fonts.
//!
//! Every keystroke is kept as a token so that Backspace undoes exactly one key. The word is rendered from the
//! tokens each time: traditional order is resolved by `encode::reorder`, then letters and signs typed in pieces
//! are combined (අ + ා → ආ, ෙ + ් → ේ, ...).
//!
//! Base keys: pitaka.lk `tools/unicode/keyboard.html` (Wijesekara column). Extra behaviour: the monk's
//! Akkharapadayo (sadaham.net).

use super::Options;
use crate::chars::{is_cons, HAL, KOMBU, REPAYA, ZWJ};
use crate::encode::reorder;
use regex::Regex;
use std::sync::OnceLock;

/// Typing order of kombuva and repaya.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    /// As written on paper and in FM: kombuva before its letter, repaya after it.
    Traditional,
    /// Unicode order: kombuva after its letter, repaya (ර්‍) before it.
    Logical,
}

/// Output of a key without AltGr. `None` = not a layout key (digits, `/`, `?`, `-`, `=` ... pass through).
fn base(key: char) -> Option<&'static str> {
    Some(match key {
        '`' => "\u{0DCA}\u{200D}ර", // rakaransaya
        '~' => "ර\u{0DCA}\u{200D}", // repaya
        'q' => "ු",
        'Q' => "ූ",
        'w' => "අ",
        'W' => "උ",
        'e' => "ැ",
        'E' => "ෑ",
        'r' => "ර",
        'R' => "ඍ",
        't' => "එ",
        'T' => "ඔ",
        'y' => "හ",
        'Y' => "ශ",
        'u' => "ම",
        'U' => "ඹ",
        'i' => "ස",
        'I' => "ෂ",
        'o' => "ද",
        'O' => "ධ",
        'p' => "ච",
        'P' => "ඡ",
        '[' => "ඤ",
        '{' => "ඥ",
        ']' => ";",
        '}' => ":",
        '\\' => "\u{0DCA}\u{200D}", // joins the next letter: ක්‍ෂ ත්‍ව
        '|' => "\u{200D}\u{0DCA}",  // touching letters: ඤ‍්ච
        'a' => "්",
        'A' => "ෟ",
        's' => "ි",
        'S' => "ී",
        'd' => "ා",
        'D' => "ෘ",
        'f' => "ෙ",
        'F' => "ෆ",
        'g' => "ට",
        'G' => "ඨ",
        'h' => "ය",
        'H' => "\u{0DCA}\u{200D}ය", // yansaya
        'j' => "ව",
        'J' => "ළු",
        'k' => "න",
        'K' => "ණ",
        'l' => "ක",
        'L' => "ඛ",
        ';' => "ත",
        ':' => "ථ",
        '\'' => ".",
        '"' => ",",
        'z' => "\"",
        'Z' => "'",
        'x' => "ං",
        'X' => "ඃ",
        'c' => "ජ",
        'C' => "ඣ",
        'v' => "ඩ",
        'V' => "ඪ",
        'b' => "ඉ",
        'B' => "ඊ",
        'n' => "බ",
        'N' => "භ",
        'm' => "ප",
        'M' => "ඵ",
        ',' => "ල",
        '<' => "ළ",
        '.' => "ග",
        '>' => "ඝ",
        _ => return None,
    })
}

/// AltGr (right Alt / Option) keys: sanyaka letters, rare letters, and the old ligatures from Akkharapadayo.
fn alt(key: char) -> Option<&'static str> {
    Some(match key {
        'o' => "ඳ",
        '.' => "ඟ",
        'v' => "ඬ",
        'x' => "ඞ",
        ',' => "ඏ",
        'a' => "ෳ",
        '\'' => "෴",
        'C' => "ඦ",
        'j' => "ද\u{0DCA}\u{200D}ව", // ද්‍ව
        'O' => "ද\u{0DCA}\u{200D}ධ", // ද්‍ධ
        'p' => "ඤ\u{0DCA}ච",         // ඤ්ච
        'P' => "ඤ\u{0DCA}ඡ",         // ඤ්ඡ
        'c' => "ඤ\u{0DCA}ජ",         // ඤ්ජ
        'G' => "ට\u{0DCA}ඨ",         // ට්ඨ
        _ => return None,
    })
}

/// What a key types, given the word so far (rendered). `None` = not a layout key.
pub fn key_output(key: char, altgr: bool, rendered: &str, opts: &Options) -> Option<String> {
    let out = if altgr { alt(key).or_else(|| base(key))? } else { base(key)? };
    let trad = opts.order == Order::Traditional;
    let last = rendered.chars().last();
    Some(match key {
        'f' if trad && !altgr => KOMBU.to_string(),
        // repaya after its letter; at the start of a word there is no letter yet, so it stays logical
        '~' if trad && last.is_some_and(|c| c != ZWJ && c != HAL) => REPAYA.to_string(),
        // Pali: hal after a bare letter waits to touch the next letter (ම‍්බ), see render()
        'a' if opts.pali && !altgr && last.is_some_and(is_cons) => "\u{200D}\u{0DCA}".to_string(),
        'X' if opts.pali && !altgr => "ඞ".to_string(),
        _ => out.to_string(),
    })
}

struct Fixes {
    signs: Vec<(Regex, &'static str)>,
    conj: Vec<(Regex, &'static str)>,
    touching_end: Regex,
}

fn fixes() -> &'static Fixes {
    static F: OnceLock<Fixes> = OnceLock::new();
    F.get_or_init(|| {
        let r = |p: &str| Regex::new(p).unwrap();
        Fixes {
            signs: vec![
                // signs typed in pieces, any order (pitaka.lk keyboard.html)
                (r("\u{0DD9}\u{0DCA}|\u{0DCA}\u{0DD9}"), "\u{0DDA}"),                               // ේ
                (r("\u{0DD9}\u{0DCF}|\u{0DCF}\u{0DD9}"), "\u{0DDC}"),                               // ො
                (r("\u{0DDA}\u{0DCF}|\u{0DCF}\u{0DDA}|\u{0DDC}\u{0DCA}|\u{0DCA}\u{0DDC}"), "\u{0DDD}"), // ෝ
                (r("\u{0DD9}\u{0DDF}|\u{0DDF}\u{0DD9}"), "\u{0DDE}"),                               // ෞ
                (r("\u{0DD9}\u{0DD9}"), "\u{0DDB}"),                                                // ෛ
                (r("\u{0DD8}\u{0DD8}"), "\u{0DF2}"),                                                // ෲ
                // vowel letters typed as a letter + sign
                (r("අා"), "ආ"),
                (r("අැ"), "ඇ"),
                (r("අෑ"), "ඈ"),
                (r("උෟ|උූ"), "ඌ"), // උූ: Akkharapadayo's W Q
                (r("ඍෘ"), "ඎ"),
                (r("ඏෟ"), "ඐ"),
                (r("එ්"), "ඒ"),
                (r("එෙ"), "ඓ"),
                (r("ඔ්"), "ඕ"),
                (r("ඔෟ"), "ඖ"),
            ],
            // touching letters that are written as conjuncts (pitaka.lk customSing, Akkharapadayo's Pali rules)
            conj: vec![
                (r("([කනත])\u{200D}\u{0DCA}([ෂදධවථ])"), "$1\u{0DCA}\u{200D}$2"),
                (r("(ද)\u{200D}\u{0DCA}([ධව])"), "$1\u{0DCA}\u{200D}$2"),
            ],
            // ‍් that touches nothing becomes a plain hal
            touching_end: r("\u{200D}\u{0DCA}([^\u{0D9A}-\u{0DC6}]|$)"),
        }
    })
}

/// The word typed so far, from the keystroke tokens.
pub fn render(tokens: &[String]) -> String {
    let raw: String = tokens.concat();
    let mut s = if raw.contains(KOMBU) || raw.contains(REPAYA) { reorder(&raw) } else { raw };
    // markers that found no letter
    s = s.replace(KOMBU, "ෙ").replace(REPAYA, "ර\u{0DCA}\u{200D}");
    let f = fixes();
    for (re, to) in &f.conj {
        s = re.replace_all(&s, *to).into_owned();
    }
    s = f.touching_end.replace_all(&s, "\u{0DCA}$1").into_owned();
    for (re, to) in &f.signs {
        s = re.replace_all(&s, *to).into_owned();
    }
    s
}
