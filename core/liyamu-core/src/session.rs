//! One typing session (one text field). Composes the current word as preedit and commits it, encoded for
//! the chosen output, at a word boundary.

use crate::chars::{is_word_char, style, HAL, ZWJ};
use crate::learn;
use crate::lexicon::user_shortcut;
use crate::suggest;
use crate::encode::Encoding;
use crate::layout::phonetic::{self, easy, helakuru};
use crate::layout::{wijesekara, Layout, Options, Output};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// A printable key, as the US QWERTY layout would type it (shift applied: 'a' or 'A').
    Char(char),
    Space,
    Backspace,
    Enter,
    Tab,
    Escape,
    /// Arrows move through the suggestions while there are some; otherwise like Other.
    Left,
    Right,
    Up,
    Down,
    /// Home, function keys ...: the word is committed and the key goes to the app.
    Other,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    /// Right Alt (AltGr) on Windows, Option on macOS
    pub altgr: bool,
    /// Ctrl, or Cmd on macOS: shortcuts are never typed
    pub ctrl: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Response {
    /// false: pass the key on to the app (after inserting `commit`)
    pub handled: bool,
    /// text to insert, already encoded for the output
    pub commit: String,
    /// the word being composed, encoded for the output; replaces the previous preedit
    pub preedit: String,
    /// the word being composed in Unicode, for the candidate window
    pub preedit_unicode: String,
    /// word suggestions (Unicode); the first is what Space commits unless another is selected
    pub candidates: Vec<String>,
    /// the highlighted suggestion (moved with the arrow keys), shown as the preedit
    pub selected: usize,
    /// some of the text could not be written in the legacy font and was approximated
    pub lossy: bool,
}

/// Previous words kept for suggestions.
const CONTEXT: usize = suggest::context::WORDS;

pub struct Session {
    opts: Options,
    /// Wijesekara: one token per key. Phonetic: one Latin letter per key.
    tokens: Vec<String>,
    /// the keys of the word being composed as typed, one per token (for the user's shortcuts)
    keys: String,
    /// the last words committed in this sentence (at most CONTEXT), for suggestions in context
    prev: Vec<String>,
    /// suggestions last shown, for pick() and the arrow keys
    candidates: Vec<String>,
    selected: usize,
    /// last character committed, for opening / closing quotes in legacy fonts
    last: Option<char>,
    /// after a word and a space, offer the words likely to come next (phone keyboards)
    predict: bool,
}

impl Session {
    pub fn new(opts: Options) -> Session {
        Session { opts, tokens: Vec::new(), keys: String::new(), prev: Vec::new(), candidates: Vec::new(), selected: 0, last: None, predict: false }
    }

    pub fn options(&self) -> &Options {
        &self.opts
    }

    /// Changes layout or output; the word being composed is committed first.
    pub fn set_options(&mut self, opts: Options) -> Response {
        let r = self.commit_word("", true);
        self.opts = opts;
        r
    }

    /// Next-word suggestions: after a word and a space (or a picked suggestion) the response carries the words
    /// likely to come next as candidates, with no preedit; `pick` takes one. Which word the user writes after
    /// which is then learned too, when learning is on.
    pub fn set_predict(&mut self, on: bool) {
        self.predict = on;
    }

    /// The response with the words likely to come next, when the shell asked for them.
    fn with_next(&mut self, mut r: Response) -> Response {
        if self.predict && self.opts.suggest {
            if let Some(prev) = self.prev.last() {
                self.candidates = suggest::next_words(prev, self.opts.pali_words);
                r.candidates = self.candidates.iter().map(|c| self.styled(c)).collect();
            }
        }
        r
    }

    /// The word being composed, in Unicode.
    pub fn word(&self) -> String {
        self.render(false)
    }

    /// `complete`: the word is being committed, so Easy Singlish judges it as a whole word.
    fn render(&self, complete: bool) -> String {
        match self.opts.layout {
            Layout::Wijesekara => wijesekara::render(&self.tokens),
            Layout::Singlish => easy::render(&self.tokens.concat(), complete),
            Layout::Helakuru => helakuru::render(&self.tokens.concat()),
        }
    }

    /// Commits suggestion `i` (from the last response) instead of the word being composed.
    pub fn pick(&mut self, i: usize) -> Response {
        match self.candidates.get(i).cloned() {
            Some(word) => {
                if i > 0 && !self.tokens.is_empty() {
                    learn::record(&self.keys, &word);
                }
                self.tokens.clear();
                self.keys.clear();
                self.remember(&word);
                let r = self.commit_text(word);
                self.with_next(r)
            }
            None => Response { handled: false, ..Default::default() },
        }
    }

    /// Commits the word being composed, e.g. when the text field loses focus or the caret is moved.
    pub fn commit(&mut self) -> Response {
        let r = self.commit_word("", true);
        self.prev.clear(); // what comes next may be somewhere else
        learn::flush();
        r
    }

    /// Adds a committed word to the context that suggestions for the next word use.
    pub fn remember(&mut self, word: &str) {
        if self.predict {
            if let Some(prev) = self.prev.last() {
                learn::record_next(prev, word);
            }
        }
        if self.prev.len() == CONTEXT {
            self.prev.remove(0);
        }
        self.prev.push(word.to_string());
    }

    /// Forgets the previous words (a new sentence, or the caret moved).
    pub fn forget(&mut self) {
        self.prev.clear();
    }

    /// Drops the word being composed without inserting it.
    pub fn reset(&mut self) {
        self.tokens.clear();
        self.keys.clear();
        self.candidates.clear();
        self.selected = 0;
        self.prev.clear();
    }

    /// A word is being composed.
    pub fn is_composing(&self) -> bool {
        !self.tokens.is_empty()
    }

    /// Whether `key` would return `handled: true`, without changing anything. Windows asks this before it
    /// sends the key (ITfKeyEventSink::OnTestKeyDown).
    pub fn handles(&self, key: Key, mods: Mods) -> bool {
        if mods.ctrl {
            return false;
        }
        match key {
            Key::Char(_) | Key::Space => true,
            Key::Backspace => !self.tokens.is_empty(),
            Key::Left | Key::Up => self.choosing() && self.selected > 0,
            Key::Right | Key::Down => self.choosing(),
            Key::Enter => self.choosing() && self.selected > 0,
            Key::Tab | Key::Escape | Key::Other => false,
        }
    }

    pub fn key(&mut self, key: Key, mods: Mods) -> Response {
        if mods.ctrl {
            return self.commit_word("", false);
        }
        if self.tokens.is_empty() {
            self.candidates.clear(); // next-word suggestions end with the next key
        }
        match key {
            Key::Char(c) => self.char_key(c, mods.altgr),
            Key::Space => {
                let r = self.commit_word(" ", true);
                self.with_next(r)
            }
            Key::Backspace => {
                if self.tokens.pop().is_some() {
                    self.keys.pop();
                    self.composing()
                } else {
                    self.prev.clear();
                    Response::default() // not handled: the app deletes
                }
            }
            // at the first suggestion Left and Up go to the app (the caret moves) like without suggestions
            Key::Left | Key::Up if self.choosing() && self.selected > 0 => self.select(self.selected - 1),
            Key::Right | Key::Down if self.choosing() => self.select(self.selected + 1),
            // a suggestion picked with the arrows is committed without a new line
            Key::Enter if self.choosing() && self.selected > 0 => self.commit_word("", true),
            Key::Enter | Key::Tab | Key::Escape | Key::Left | Key::Right | Key::Up | Key::Down | Key::Other => {
                self.commit_word("", false)
            }
        }
    }

    /// A word is being composed and there is more than one suggestion to choose from.
    fn choosing(&self) -> bool {
        !self.tokens.is_empty() && self.candidates.len() > 1
    }

    /// Highlights suggestion `i` (wrapping around) and shows it as the preedit.
    fn select(&mut self, i: usize) -> Response {
        self.selected = i % self.candidates.len();
        let word = self.candidates[self.selected].clone();
        let (preedit, _) = self.encode(&word);
        Response {
            handled: true,
            preedit,
            preedit_unicode: self.styled(&word),
            candidates: self.candidates.iter().map(|c| self.styled(c)).collect(),
            selected: self.selected,
            ..Default::default()
        }
    }

    fn char_key(&mut self, c: char, altgr: bool) -> Response {
        // 1-9 pick a suggestion while there are suggestions to pick from
        if let Some(d) = c.to_digit(10) {
            if d >= 1 && (d as usize) <= self.candidates.len() && self.choosing() && !altgr {
                return self.pick(d as usize - 1);
            }
        }
        match self.opts.layout {
            Layout::Wijesekara => match wijesekara::key_output(c, altgr, &self.word(), &self.opts) {
                Some(out) if out.chars().any(is_word_char) => {
                    self.tokens.push(out);
                    self.keys.push(c);
                    self.composing()
                }
                Some(punct) => self.commit_word(&punct, true),
                None => self.commit_word(&c.to_string(), true),
            },
            Layout::Singlish | Layout::Helakuru => {
                if phonetic::is_key(c) && !altgr {
                    self.tokens.push(c.to_string());
                    self.keys.push(c);
                    self.composing()
                } else {
                    self.commit_word(&c.to_string(), true)
                }
            }
        }
    }

    fn composing(&mut self) -> Response {
        let latin = self.tokens.concat();
        let (mut word, mut candidates) = match self.opts.layout {
            Layout::Singlish if !latin.is_empty() && self.opts.suggest => {
                // judged as a whole word, so what is shown is what a space commits
                let candidates = suggest::singlish(&latin, &self.prev, self.opts.pali_words);
                (candidates[0].clone(), candidates)
            }
            _ => {
                let word = if self.opts.layout == Layout::Singlish { self.render(true) } else { self.word() };
                let mut candidates = if self.opts.suggest { self.completions(&word) } else { Vec::new() };
                // the user's own shortcut for these keys comes first, with or without suggestions
                let own = user_shortcut(&self.keys);
                if !own.is_empty() {
                    if candidates.is_empty() {
                        candidates.push(word.clone());
                    }
                    candidates.retain(|c| !own.contains(c));
                    candidates.splice(0..0, own);
                }
                (candidates.first().cloned().unwrap_or(word), candidates)
            }
        };
        // what the user picked for these keys before comes first, after their own shortcuts
        if self.opts.suggest && !candidates.is_empty() {
            learn::apply(&self.keys, &mut candidates, user_shortcut(&self.keys).len());
            word = candidates[0].clone();
        }
        self.candidates = candidates.clone();
        self.selected = 0;
        // a word part-way through may not be writable yet (ක්‍ before its ෂ); only commits report losses
        let (preedit, _) = self.encode(&word);
        let candidates = candidates.iter().map(|c| self.styled(c)).collect();
        Response { handled: true, preedit, preedit_unicode: self.styled(&word), candidates, ..Default::default() }
    }

    /// Wijesekara and Helakuru keys: the word as typed, then likely words that start with it.
    fn completions(&self, word: &str) -> Vec<String> {
        // a consonant typed phonetically still shows its hal; the next letter may take it away
        let prefix = match self.opts.layout {
            Layout::Helakuru => word.strip_suffix(HAL).unwrap_or(word),
            _ => word,
        };
        if prefix.chars().count() < 2 || !prefix.chars().all(|c| ('\u{0D80}'..='\u{0DFF}').contains(&c) || c == ZWJ) {
            return Vec::new();
        }
        if self.opts.layout == Layout::Singlish {
            return Vec::new();
        }
        suggest::completions(word, prefix, &self.prev, self.opts.pali_words)
    }

    /// Commits the word followed by `after` (a space or punctuation, in Unicode).
    fn commit_word(&mut self, after: &str, handled: bool) -> Response {
        let mut text = if self.tokens.is_empty() {
            String::new()
        } else {
            match self.candidates.get(self.selected) {
                Some(w) => {
                    if self.selected > 0 {
                        learn::record(&self.keys, w);
                    }
                    w.clone()
                }
                None => self.render(true),
            }
        };
        self.tokens.clear();
        self.keys.clear();
        if !text.is_empty() {
            for w in text.clone().split(' ') {
                self.remember(w);
            }
        }
        // the sentence goes on after a space or a comma; anything else starts afresh
        if !(after == " " || after == "," || (after.is_empty() && handled)) {
            self.prev.clear();
        }
        text.push_str(after);
        let mut r = self.commit_text(text);
        r.handled = handled;
        r
    }

    /// Commits Unicode text, encoded for the output.
    fn commit_text(&mut self, text: String) -> Response {
        self.candidates.clear();
        self.selected = 0;
        let handled = true;
        if text.is_empty() {
            return Response { handled, ..Default::default() };
        }
        let text = if self.opts.output == Output::Unicode { text } else { self.curly_quotes(&text) };
        let (commit, lossy) = self.encode(&text);
        if let Some(c) = text.chars().last() {
            self.last = Some(c);
        }
        Response { handled, commit, lossy, ..Default::default() }
    }

    /// The text with its clusters written as the options ask (repaya, joined and touching letters).
    fn styled(&self, s: &str) -> String {
        style(s, self.opts.repaya, self.opts.conjuncts, self.opts.pali)
    }

    fn encode(&self, s: &str) -> (String, bool) {
        let s = self.styled(s);
        match self.opts.output {
            Output::Unicode => (s, false),
            Output::Fm => Encoding::fm().from_unicode_lossy(&s),
            Output::Isi => Encoding::isi().from_unicode_lossy(&s),
        }
    }

    /// Legacy fonts have no straight quotes (their keys are letters), so quotes become curly ones:
    /// opening at the start or after a space or bracket, closing elsewhere.
    fn curly_quotes(&self, s: &str) -> String {
        let mut prev = self.last;
        let mut out = String::new();
        for c in s.chars() {
            let opening = prev.map_or(true, |p| p.is_whitespace() || "([{".contains(p));
            out.push(match c {
                '"' if opening => '“',
                '"' => '”',
                '\'' if opening => '‘',
                '\'' => '’',
                _ => c,
            });
            prev = Some(c);
        }
        out
    }
}
