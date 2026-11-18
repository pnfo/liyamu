//! C ABI for the native shells (Swift on macOS/iOS, C++/Rust on Windows, JNI on Android).
//! See `include/liyamu.h`. All strings are UTF-8 and NUL-terminated. Every response returned must be
//! released with `liyamu_response_free`; every session with `liyamu_session_free`.

use liyamu_core::{Key, Layout, Mods, Options, Order, Output, Response, Session};
use std::ffi::{c_char, CStr, CString};
use std::ptr;

pub const LIYAMU_KEY_CHAR: u32 = 0;
pub const LIYAMU_KEY_SPACE: u32 = 1;
pub const LIYAMU_KEY_BACKSPACE: u32 = 2;
pub const LIYAMU_KEY_ENTER: u32 = 3;
pub const LIYAMU_KEY_TAB: u32 = 4;
pub const LIYAMU_KEY_ESCAPE: u32 = 5;
pub const LIYAMU_KEY_OTHER: u32 = 6;
pub const LIYAMU_KEY_LEFT: u32 = 7;
pub const LIYAMU_KEY_RIGHT: u32 = 8;
pub const LIYAMU_KEY_UP: u32 = 9;
pub const LIYAMU_KEY_DOWN: u32 = 10;

#[repr(C)]
pub struct LiyamuOptions {
    /// 0 Wijesekara, 1 Easy Singlish, 2 Helakuru phonetic
    pub layout: u32,
    /// 0 Unicode, 1 FM, 2 Isi
    pub output: u32,
    /// 0 traditional (kombuva first), 1 logical
    pub order: u32,
    pub pali: bool,
    /// offer word suggestions
    pub suggest: bool,
    /// ර් before a consonant as repaya
    pub repaya: bool,
    /// Pali pairs as joined letters
    pub conjuncts: bool,
    /// suggest Pali-only words
    pub pali_words: bool,
}

#[repr(C)]
pub struct LiyamuResponse {
    pub handled: bool,
    pub lossy: bool,
    pub commit: *mut c_char,
    pub preedit: *mut c_char,
    pub preedit_unicode: *mut c_char,
    pub candidate_count: u32,
    pub candidates: *mut *mut c_char,
    /// the highlighted candidate
    pub selected: u32,
}

fn options(o: &LiyamuOptions) -> Options {
    Options {
        layout: match o.layout {
            1 => Layout::Singlish,
            2 => Layout::Helakuru,
            _ => Layout::Wijesekara,
        },
        output: match o.output {
            1 => Output::Fm,
            2 => Output::Isi,
            _ => Output::Unicode,
        },
        order: if o.order == 1 { Order::Logical } else { Order::Traditional },
        pali: o.pali,
        suggest: o.suggest,
        repaya: o.repaya,
        conjuncts: o.conjuncts,
        pali_words: o.pali_words,
    }
}

fn cstring(s: &str) -> *mut c_char {
    CString::new(s.replace('\0', "")).unwrap().into_raw()
}

fn response(r: Response) -> *mut LiyamuResponse {
    let mut cands: Vec<*mut c_char> = r.candidates.iter().map(|c| cstring(c)).collect();
    cands.shrink_to_fit();
    let count = cands.len() as u32;
    let cands_ptr = if count == 0 { ptr::null_mut() } else { Box::into_raw(cands.into_boxed_slice()) as *mut *mut c_char };
    Box::into_raw(Box::new(LiyamuResponse {
        handled: r.handled,
        lossy: r.lossy,
        commit: cstring(&r.commit),
        preedit: cstring(&r.preedit),
        preedit_unicode: cstring(&r.preedit_unicode),
        candidate_count: count,
        candidates: cands_ptr,
        selected: r.selected as u32,
    }))
}

#[no_mangle]
pub extern "C" fn liyamu_session_new(opts: *const LiyamuOptions) -> *mut Session {
    let o = if opts.is_null() { Options::default() } else { options(unsafe { &*opts }) };
    Box::into_raw(Box::new(Session::new(o)))
}

/// # Safety
/// `s` must come from `liyamu_session_new` and not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn liyamu_session_free(s: *mut Session) {
    if !s.is_null() {
        drop(Box::from_raw(s));
    }
}

/// Changes options; the word being composed is committed in the returned response.
/// # Safety
/// `s` must be a live session and `opts` a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn liyamu_session_set_options(s: *mut Session, opts: *const LiyamuOptions) -> *mut LiyamuResponse {
    response((*s).set_options(options(&*opts)))
}

/// Next-word suggestions (off by default): after a word and a space, or a picked suggestion, the response
/// has the words likely to come next as candidates and no preedit; `liyamu_session_pick` takes one.
/// # Safety
/// `s` must be a live session.
#[no_mangle]
pub unsafe extern "C" fn liyamu_session_set_predict(s: *mut Session, on: bool) {
    (*s).set_predict(on)
}

/// `kind` is one of the LIYAMU_KEY_* values; `ch` is the US-QWERTY character for LIYAMU_KEY_CHAR.
/// # Safety
/// `s` must be a live session.
#[no_mangle]
pub unsafe extern "C" fn liyamu_session_key(s: *mut Session, kind: u32, ch: u32, altgr: bool, ctrl: bool) -> *mut LiyamuResponse {
    let key = match kind {
        LIYAMU_KEY_CHAR => match char::from_u32(ch) {
            Some(c) => Key::Char(c),
            None => Key::Other,
        },
        LIYAMU_KEY_SPACE => Key::Space,
        LIYAMU_KEY_BACKSPACE => Key::Backspace,
        LIYAMU_KEY_ENTER => Key::Enter,
        LIYAMU_KEY_TAB => Key::Tab,
        LIYAMU_KEY_ESCAPE => Key::Escape,
        LIYAMU_KEY_LEFT => Key::Left,
        LIYAMU_KEY_RIGHT => Key::Right,
        LIYAMU_KEY_UP => Key::Up,
        LIYAMU_KEY_DOWN => Key::Down,
        _ => Key::Other,
    };
    response((*s).key(key, Mods { altgr, ctrl }))
}

/// Commits the word being composed (focus lost, caret moved, mouse click).
/// # Safety
/// `s` must be a live session.
#[no_mangle]
pub unsafe extern "C" fn liyamu_session_commit(s: *mut Session) -> *mut LiyamuResponse {
    response((*s).commit())
}

/// Commits suggestion `index` from the last response's candidates.
/// # Safety
/// `s` must be a live session.
#[no_mangle]
pub unsafe extern "C" fn liyamu_session_pick(s: *mut Session, index: u32) -> *mut LiyamuResponse {
    response((*s).pick(index as usize))
}

/// Replaces the user's own shortcuts for every session: lines of `keys=words` (NULL or "" for none).
/// # Safety
/// `text` must be NULL or a NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn liyamu_set_shortcuts(text: *const c_char) {
    let text = if text.is_null() { String::new() } else { CStr::from_ptr(text).to_string_lossy().into_owned() };
    liyamu_core::lexicon::set_user_shortcuts(&text);
}

/// Learning from the suggestions the user picks, for all sessions. `path` is the file that keeps what was
/// learned (NULL: keep the one given before); it never leaves the device.
/// # Safety
/// `path` must be NULL or a NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn liyamu_set_learning(path: *const c_char, on: bool) {
    let path = if path.is_null() { None } else { Some(CStr::from_ptr(path).to_string_lossy().into_owned()) };
    liyamu_core::learn::configure(path.as_deref(), on);
}

/// Deletes everything learned, in memory and the file.
#[no_mangle]
pub extern "C" fn liyamu_forget_learned() {
    liyamu_core::learn::forget();
}

/// Loads the word lists now (a few hundred milliseconds) instead of at the first key. Call it on a
/// background thread when the keyboard starts.
#[no_mangle]
pub extern "C" fn liyamu_warm_up() {
    let _ = liyamu_core::suggest::singlish("ka", &[], true);
    let _ = liyamu_core::suggest::next_words("මම", true);
}

/// Drops the word being composed.
/// # Safety
/// `s` must be a live session.
#[no_mangle]
pub unsafe extern "C" fn liyamu_session_reset(s: *mut Session) {
    (*s).reset()
}

/// # Safety
/// `r` must come from this library and not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn liyamu_response_free(r: *mut LiyamuResponse) {
    if r.is_null() {
        return;
    }
    let r = Box::from_raw(r);
    for p in [r.commit, r.preedit, r.preedit_unicode] {
        drop(CString::from_raw(p));
    }
    if !r.candidates.is_null() {
        let cands = Box::from_raw(ptr::slice_from_raw_parts_mut(r.candidates, r.candidate_count as usize));
        for p in cands.iter() {
            drop(CString::from_raw(*p));
        }
    }
}
