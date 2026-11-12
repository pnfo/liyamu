//! Liyamu: a Sinhala input engine shared by the macOS, Windows and mobile input methods.
//!
//! The shells translate OS key events into [`Key`]s, call [`Session::key`], and show the [`Response`]:
//! `preedit` as underlined text being composed, `commit` as text to insert, and pass the key on to the app
//! when `handled` is false.

pub mod chars;
pub mod encode;
pub mod layout;
pub mod lexicon;
mod session;
pub mod sim;
pub mod suggest;

pub use encode::Encoding;
pub use layout::{Layout, Options, Order, Output};
pub use session::{Key, Mods, Response, Session};
