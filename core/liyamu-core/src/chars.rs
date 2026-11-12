//! Sinhala character classes shared by the layouts and the encoders.

/// Kombuva typed before its consonant (visual order). Same private-use marker as pitaka.lk's convert.js.
pub const KOMBU: char = '\u{E000}';
/// Repaya typed after its consonant (visual order).
pub const REPAYA: char = '\u{E001}';
pub const HAL: char = '\u{0DCA}'; // ්
pub const ZWJ: char = '\u{200D}';

/// ක .. ෆ
pub fn is_cons(c: char) -> bool {
    ('\u{0D9A}'..='\u{0DC6}').contains(&c)
}

/// ා .. ෟ and ෲ ෳ (ෘ ෟ included)
pub fn is_vowel_sign(c: char) -> bool {
    ('\u{0DCF}'..='\u{0DDF}').contains(&c) || c == '\u{0DF2}' || c == '\u{0DF3}'
}

/// ං ඃ come after the vowel sign
pub fn is_tail(c: char) -> bool {
    c == '\u{0D82}' || c == '\u{0D83}'
}

/// Letters and signs that make up a word (not ෴ punctuation).
pub fn is_word_char(c: char) -> bool {
    ('\u{0D80}'..='\u{0DF3}').contains(&c) || c == ZWJ || c == '\u{200C}' || c == KOMBU || c == REPAYA
}

/// Pairs written as one joined letter in Pali books (බැඳි අකුරු).
const CONJUNCTS: &[(char, char)] = &[
    ('ක', 'ව'), ('ක', 'ෂ'), ('ග', 'ධ'), ('ට', 'ඨ'), ('ත', 'ථ'), ('ත', 'ව'),
    ('න', 'ථ'), ('න', 'ද'), ('න', 'ධ'), ('න', 'ව'), ('ද', 'ධ'), ('ද', 'ව'),
];

/// Writes consonant clusters the way the options ask:
/// - `repaya`: ර් before a consonant as repaya (ධර්ම → ධර්‍ම);
/// - `conjuncts`: the Pali pairs as joined letters (බුද්ධ → බුද්‍ධ);
/// - `touching`: every other cluster as touching letters (ධම්ම → ධම‍්ම).
/// Clusters already written with a joiner (rakaransaya, yansaya, typed touching letters) are left alone,
/// except that touching letters become a joined letter where `conjuncts` asks for one.
pub fn style(text: &str, repaya: bool, conjuncts: bool, touching: bool) -> String {
    if !(repaya || conjuncts || touching) {
        return text.to_string();
    }
    let cs: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() + 8);
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        out.push(c);
        if is_cons(c) {
            // c ් x  or  c ZWJ ් x (touching)
            let plain = cs.get(i + 1) == Some(&HAL) && cs.get(i + 2).copied().is_some_and(is_cons);
            let touch = cs.get(i + 1) == Some(&ZWJ) && cs.get(i + 2) == Some(&HAL) && cs.get(i + 3).copied().is_some_and(is_cons);
            let next = if plain { cs[i + 2] } else if touch { cs[i + 3] } else { ' ' };
            let joined = conjuncts && CONJUNCTS.contains(&(c, next));
            if plain && ((repaya && c == 'ර') || joined) {
                out.push(HAL);
                out.push(ZWJ);
                i += 2;
                continue;
            } else if plain && touching && c != 'ර' {
                out.push(ZWJ);
                out.push(HAL);
                i += 2;
                continue;
            } else if touch && joined {
                out.push(HAL);
                out.push(ZWJ);
                i += 3;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Independent vowel for a dependent sign, used when a sign starts a word.
pub fn independent_of(sign: char) -> Option<char> {
    Some(match sign {
        'ා' => 'ආ',
        'ැ' => 'ඇ',
        'ෑ' => 'ඈ',
        'ි' => 'ඉ',
        'ී' => 'ඊ',
        'ු' => 'උ',
        'ූ' => 'ඌ',
        'ෙ' => 'එ',
        'ේ' => 'ඒ',
        'ෛ' => 'ඓ',
        'ො' => 'ඔ',
        'ෝ' => 'ඕ',
        'ෞ' => 'ඖ',
        'ෘ' => 'ඍ',
        'ෲ' => 'ඎ',
        'ෟ' => 'ඏ',
        'ෳ' => 'ඐ',
        _ => return None,
    })
}
