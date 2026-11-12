pub mod phonetic;
pub mod wijesekara;

pub use wijesekara::Order;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Wijesekara,
    /// Easy Singlish: forgiving phonetic typing that picks the likeliest word
    Singlish,
    /// Phonetic with Helakuru's keys
    Helakuru,
}

/// What the app receives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Output {
    Unicode,
    /// FM Abhaya and the other FM fonts
    Fm,
    /// Isi Abhaya and the other Isi fonts
    Isi,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub layout: Layout,
    pub output: Output,
    /// Wijesekara: kombuva before / repaya after the letter (traditional, default) or Unicode order
    pub order: Order,
    /// Pali touching letters (ම‍්බ, ඤ‍්ච). Wijesekara: Akkharapadayo's Pali mode, hal (`a`) after a letter
    /// touches the next one and `X` types ඞ. Everywhere: clusters in what is typed are written touching.
    pub pali: bool,
    /// ර් before a consonant is written as repaya (ධර්‍ම).
    pub repaya: bool,
    /// The Pali pairs (ක්‍ව ත්‍ථ න්‍ද ද්‍ධ ...) are written as joined letters.
    pub conjuncts: bool,
    /// Suggest words that are Pali only (`data/lexicon/pali.txt`).
    pub pali_words: bool,
    /// Offer word suggestions: readings in Easy Singlish, completions of the word in the other layouts.
    pub suggest: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { layout: Layout::Wijesekara, output: Output::Unicode, order: Order::Traditional, pali: false, repaya: false, conjuncts: false, pali_words: true, suggest: true }
    }
}
