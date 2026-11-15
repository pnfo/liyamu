//! Options shared by every app, kept in HKEY_CURRENT_USER\Software\Liyamu. The settings program and the
//! keyboard menu write them and bump `Version`; each running keyboard reads `Version` before every key and
//! reloads when it changed. The user's shortcuts and what was learned are files in %APPDATA%\Liyamu.

use liyamu_core::{Layout, Options, Order, Output};
use std::path::PathBuf;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::System::Registry::{RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_DWORD, RRF_RT_REG_DWORD};

const KEY: PCWSTR = w!("Software\\Liyamu");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// 0 Wijesekara, 1 Easy Singlish, 2 Helakuru keys
    pub layout: u32,
    /// 0 Unicode, 1 FM, 2 Isi
    pub output: u32,
    /// kombuva typed before its letter (Wijesekara)
    pub traditional: bool,
    pub pali: bool,
    pub suggest: bool,
    pub repaya: bool,
    pub conjuncts: bool,
    pub pali_words: bool,
    pub learn: bool,
}

pub fn get(name: &str) -> Option<u32> {
    let mut v = 0u32;
    let mut size = 4u32;
    let r = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, KEY, &HSTRING::from(name), RRF_RT_REG_DWORD, None,
                     Some(&mut v as *mut u32 as *mut _), Some(&mut size))
    };
    r.is_ok().then_some(v)
}

pub fn set(name: &str, v: u32) {
    unsafe {
        let _ = RegSetKeyValueW(HKEY_CURRENT_USER, KEY, &HSTRING::from(name), REG_DWORD.0,
                                Some(&v as *const u32 as *const _), 4);
    }
}

/// Changes on every save, so that the keyboards in other apps reload.
pub fn version() -> u32 {
    get("Version").unwrap_or(0)
}

pub fn bump(name: &str) {
    set(name, get(name).unwrap_or(0).wrapping_add(1));
}

impl Settings {
    pub fn load() -> Settings {
        let b = |n: &str, d: bool| get(n).map(|v| v != 0).unwrap_or(d);
        Settings {
            layout: get("Layout").unwrap_or(1).min(2), // Easy Singlish, as on Android
            output: get("Output").unwrap_or(0).min(2),
            traditional: b("TraditionalOrder", true),
            pali: b("Pali", false),
            suggest: b("Suggestions", true),
            repaya: b("Repaya", false),
            conjuncts: b("Conjuncts", false),
            pali_words: b("PaliWords", false),
            learn: b("Learn", true),
        }
    }

    pub fn save(&self) {
        set("Layout", self.layout);
        set("Output", self.output);
        set("TraditionalOrder", self.traditional as u32);
        set("Pali", self.pali as u32);
        set("Suggestions", self.suggest as u32);
        set("Repaya", self.repaya as u32);
        set("Conjuncts", self.conjuncts as u32);
        set("PaliWords", self.pali_words as u32);
        set("Learn", self.learn as u32);
        bump("Version");
    }

    pub fn options(&self) -> Options {
        Options {
            layout: match self.layout {
                1 => Layout::Singlish,
                2 => Layout::Helakuru,
                _ => Layout::Wijesekara,
            },
            output: match self.output {
                1 => Output::Fm,
                2 => Output::Isi,
                _ => Output::Unicode,
            },
            order: if self.traditional { Order::Traditional } else { Order::Logical },
            pali: self.pali,
            suggest: self.suggest,
            repaya: self.repaya,
            conjuncts: self.conjuncts,
            pali_words: self.pali_words,
        }
    }
}

pub const LAYOUTS: [&str; 3] = ["Wijesekara", "Easy Singlish", "Helakuru keys"];
pub const OUTPUTS: [&str; 3] = ["Unicode", "FM fonts (FM Abhaya …)", "Isi fonts"];

/// %APPDATA%\Liyamu, created if missing (None in sandboxed apps that cannot see it)
pub fn dir() -> Option<PathBuf> {
    let d = PathBuf::from(std::env::var_os("APPDATA")?).join("Liyamu");
    std::fs::create_dir_all(&d).ok()?;
    Some(d)
}

pub fn shortcuts_file() -> Option<PathBuf> {
    dir().map(|d| d.join("shortcuts.txt"))
}

pub fn learned_file() -> Option<PathBuf> {
    dir().map(|d| d.join("learned.tsv"))
}
