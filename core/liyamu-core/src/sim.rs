//! Simulates typing into a plain text field, for tests, the CLI and demos.
//!
//! Key notation: each character is a key, except `␣` Space, `⏎` Enter, `⇥` Tab, `⎋` Escape, `⌫` Backspace, `←` `→` `↑` `↓` arrows,
//! and `⌥` which adds AltGr to the next key (`⌥o` → ඳ).

use crate::{Key, Mods, Response, Session};

pub fn parse_keys(s: &str) -> Vec<(Key, Mods)> {
    let mut out = Vec::new();
    let mut altgr = false;
    for c in s.chars() {
        let key = match c {
            '⌥' => {
                altgr = true;
                continue;
            }
            '␣' | ' ' => Key::Space,
            '⏎' => Key::Enter,
            '⇥' => Key::Tab,
            '⎋' => Key::Escape,
            '⌫' => Key::Backspace,
            '←' => Key::Left,
            '→' => Key::Right,
            '↑' => Key::Up,
            '↓' => Key::Down,
            c => Key::Char(c),
        };
        out.push((key, Mods { altgr, ctrl: false }));
        altgr = false;
    }
    out
}

/// A text field: committed text plus the preedit shown at the caret.
#[derive(Default)]
pub struct Field {
    pub text: String,
    pub preedit: String,
    pub lossy: bool,
    /// suggestions shown with the last key
    pub candidates: Vec<String>,
}

impl Field {
    pub fn apply(&mut self, key: Key, r: &Response) {
        self.text.push_str(&r.commit);
        self.preedit = r.preedit.clone();
        self.lossy |= r.lossy;
        self.candidates = r.candidates.clone();
        if !r.handled {
            match key {
                Key::Backspace => {
                    self.text.pop();
                }
                Key::Enter => self.text.push('\n'),
                Key::Tab => self.text.push('\t'),
                _ => {}
            }
        }
    }

    /// What the user sees: committed text and the word being composed.
    pub fn shown(&self) -> String {
        format!("{}{}", self.text, self.preedit)
    }
}

/// Types the keys and returns what the field shows.
pub fn type_keys(session: &mut Session, keys: &str) -> Field {
    let mut field = Field::default();
    for (key, mods) in parse_keys(keys) {
        let r = session.key(key, mods);
        field.apply(key, &r);
    }
    field
}
