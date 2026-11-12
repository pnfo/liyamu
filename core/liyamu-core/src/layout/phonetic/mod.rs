//! Phonetic (Singlish) layouts. The Latin letters of the current word are kept and re-read on every key,
//! so Backspace removes one Latin letter and later letters can change earlier ones ("k" ක් → "ka" ක).
//!
//! - [`helakuru`]: a fixed table with the keys of Helakuru's phonetic keyboard, for people used to it.
//! - [`easy`]: forgiving, case-insensitive spelling as people actually type; picks the likeliest Sinhala word.

pub mod easy;
pub mod helakuru;

/// Is this a key phonetic layouts use inside a word?
pub fn is_key(c: char) -> bool {
    c.is_ascii_alphabetic()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tok {
    /// a consonant (or a consonant cluster such as ක්‍ෂ)
    Cons(&'static str),
    /// a vowel: the letter at the start of a word or after a vowel, the sign after a consonant
    Vowel { letter: &'static str, sign: &'static str },
    /// ං ඃ
    Mod(&'static str),
    /// stops a consonant from joining the next r / y (Helakuru's z)
    Break,
}

/// When a consonant joins a following ර / ය as rakaransaya / yansaya.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Join {
    /// always (Helakuru): kr → ක්‍ර, kaarya → කාර්‍ය
    Always,
    /// only when the ර / ය has a vowel and the first letter is not ර: kra → ක්‍ර, kaarya → කාර්ය
    BeforeVowel,
}

pub fn render(toks: &[Tok], join: Join) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < toks.len() {
        match &toks[i] {
            Tok::Cons(c) => {
                out.push_str(c);
                match toks.get(i + 1) {
                    Some(Tok::Vowel { sign, .. }) => {
                        out.push_str(sign);
                        i += 1;
                    }
                    Some(Tok::Cons(n))
                        if (*n == "ර" || *n == "ය")
                            && match join {
                                Join::Always => true,
                                Join::BeforeVowel => *c != "ර" && matches!(toks.get(i + 2), Some(Tok::Vowel { .. })),
                            } =>
                    {
                        out.push_str("\u{0DCA}\u{200D}")
                    }
                    _ => out.push('\u{0DCA}'),
                }
            }
            Tok::Vowel { letter, .. } => out.push_str(letter),
            Tok::Mod(m) => out.push_str(m),
            Tok::Break => {}
        }
        i += 1;
    }
    out
}
