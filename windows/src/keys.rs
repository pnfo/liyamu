//! Maps a key press to what the US QWERTY layout would type, by the key's position (scan code), whatever
//! keyboard layout Windows has, so that Wijesekara and Singlish always follow the physical keys.

use liyamu_core::{Key, Mods};
use windows::Win32::UI::Input::KeyboardAndMouse::*;

/// Set-1 scan codes of the main block → (plain, shifted)
fn by_scan(scan: u32) -> Option<(char, char)> {
    const DIGITS: &[u8] = b"1234567890-=";
    const DIGITS_SHIFT: &[u8] = b"!@#$%^&*()_+";
    const TOP: &[u8] = b"qwertyuiop[]";
    const TOP_SHIFT: &[u8] = b"QWERTYUIOP{}";
    const HOME: &[u8] = b"asdfghjkl;'`";
    const HOME_SHIFT: &[u8] = b"ASDFGHJKL:\"~";
    const BOTTOM: &[u8] = b"zxcvbnm,./";
    const BOTTOM_SHIFT: &[u8] = b"ZXCVBNM<>?";
    let pick = |a: &[u8], b: &[u8], i: u32| Some((a[i as usize] as char, b[i as usize] as char));
    match scan {
        0x02..=0x0D => pick(DIGITS, DIGITS_SHIFT, scan - 0x02),
        0x10..=0x1B => pick(TOP, TOP_SHIFT, scan - 0x10),
        0x1E..=0x29 => pick(HOME, HOME_SHIFT, scan - 0x1E),
        0x2B | 0x56 => Some(('\\', '|')), // 0x56: the extra key of ISO keyboards
        0x2C..=0x35 => pick(BOTTOM, BOTTOM_SHIFT, scan - 0x2C),
        _ => None,
    }
}

/// For keys sent by programs (on-screen keyboards) that carry no scan code.
fn by_vk(vk: u16) -> Option<(char, char)> {
    let v = VIRTUAL_KEY(vk);
    if (b'A' as u16..=b'Z' as u16).contains(&vk) {
        let c = (vk as u8 as char).to_ascii_lowercase();
        return Some((c, c.to_ascii_uppercase()));
    }
    if (b'0' as u16..=b'9' as u16).contains(&vk) {
        let i = (vk - b'0' as u16) as usize;
        return Some((vk as u8 as char, b")!@#$%^&*("[i] as char));
    }
    Some(match v {
        VK_OEM_1 => (';', ':'),
        VK_OEM_PLUS => ('=', '+'),
        VK_OEM_COMMA => (',', '<'),
        VK_OEM_MINUS => ('-', '_'),
        VK_OEM_PERIOD => ('.', '>'),
        VK_OEM_2 => ('/', '?'),
        VK_OEM_3 => ('`', '~'),
        VK_OEM_4 => ('[', '{'),
        VK_OEM_5 | VK_OEM_102 => ('\\', '|'),
        VK_OEM_6 => (']', '}'),
        VK_OEM_7 => ('\'', '"'),
        _ => return None,
    })
}

/// After Right Alt + a key that the keyboard used, the app has seen Alt go down and nothing else, and would
/// open its menu (the key tips of a ribbon) when Alt goes up. A tap of Ctrl in between tells it that Alt
/// was used with something. (The unassigned key 0xE8, which some programs use for this, does not stop
/// WordCraft's key tips.)
pub fn mask_alt() {
    unsafe {
        keybd_event(VK_CONTROL.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(VK_CONTROL.0 as u8, 0, KEYEVENTF_KEYUP, 0);
    }
}

fn down(vk: VIRTUAL_KEY) -> bool {
    unsafe { GetKeyState(vk.0 as i32) < 0 }
}

/// The key for the engine, or None for keys that are no business of the keyboard (Shift, Ctrl, Alt,
/// Caps Lock, Windows key ...): those must not even commit the word.
pub fn translate(vk: u16, lparam: isize) -> Option<(Key, Mods)> {
    let v = VIRTUAL_KEY(vk);
    match v {
        VK_SHIFT | VK_LSHIFT | VK_RSHIFT | VK_CONTROL | VK_LCONTROL | VK_RCONTROL | VK_MENU | VK_LMENU
        | VK_RMENU | VK_CAPITAL | VK_LWIN | VK_RWIN | VK_NUMLOCK | VK_SCROLL | VK_KANA | VK_PROCESSKEY
        | VK_PACKET => return None,
        _ => {}
    }
    // AltGr is right Alt (which arrives with a left Ctrl on layouts that have AltGr) or, as in every
    // Windows layout, Ctrl+Alt. The left Alt alone is for menus, so like Ctrl.
    let alt = down(VK_MENU);
    let altgr = down(VK_RMENU) || (alt && down(VK_CONTROL));
    let ctrl = (down(VK_CONTROL) && !altgr) || (alt && !altgr) || down(VK_LWIN) || down(VK_RWIN);
    let mods = Mods { altgr, ctrl };
    let key = match v {
        VK_SPACE => Key::Space,
        VK_BACK => Key::Backspace,
        VK_RETURN => Key::Enter,
        VK_TAB => Key::Tab,
        VK_ESCAPE => Key::Escape,
        VK_LEFT => Key::Left,
        VK_RIGHT => Key::Right,
        VK_UP => Key::Up,
        VK_DOWN => Key::Down,
        _ if (VK_NUMPAD0.0..=VK_NUMPAD9.0).contains(&vk) => Key::Char((b'0' + (vk - VK_NUMPAD0.0) as u8) as char),
        VK_DECIMAL => Key::Char('.'),
        VK_ADD => Key::Char('+'),
        VK_SUBTRACT => Key::Char('-'),
        VK_MULTIPLY => Key::Char('*'),
        VK_DIVIDE => Key::Char('/'),
        _ => {
            let scan = ((lparam >> 16) & 0xFF) as u32;
            let extended = (lparam >> 24) & 1 == 1;
            let pair = if scan != 0 && !extended { by_scan(scan) } else { None }.or_else(|| by_vk(vk));
            match pair {
                Some((plain, shifted)) => Key::Char(if down(VK_SHIFT) { shifted } else { plain }),
                None => Key::Other,
            }
        }
    };
    Some((key, mods))
}

/// The keys that can have an AltGr letter, as (virtual key on the US layout, plain, shifted). Apps do not
/// pass Alt+key to keyboards, so the text service asks for these with ITfKeystrokeMgr::PreserveKey.
pub const ALT_KEYS: &[(u16, char, char)] = &[
    (b'A' as u16, 'a', 'A'), (b'B' as u16, 'b', 'B'), (b'C' as u16, 'c', 'C'), (b'D' as u16, 'd', 'D'),
    (b'E' as u16, 'e', 'E'), (b'F' as u16, 'f', 'F'), (b'G' as u16, 'g', 'G'), (b'H' as u16, 'h', 'H'),
    (b'I' as u16, 'i', 'I'), (b'J' as u16, 'j', 'J'), (b'K' as u16, 'k', 'K'), (b'L' as u16, 'l', 'L'),
    (b'M' as u16, 'm', 'M'), (b'N' as u16, 'n', 'N'), (b'O' as u16, 'o', 'O'), (b'P' as u16, 'p', 'P'),
    (b'Q' as u16, 'q', 'Q'), (b'R' as u16, 'r', 'R'), (b'S' as u16, 's', 'S'), (b'T' as u16, 't', 'T'),
    (b'U' as u16, 'u', 'U'), (b'V' as u16, 'v', 'V'), (b'W' as u16, 'w', 'W'), (b'X' as u16, 'x', 'X'),
    (b'Y' as u16, 'y', 'Y'), (b'Z' as u16, 'z', 'Z'),
    (VK_OEM_1.0, ';', ':'), (VK_OEM_7.0, '\'', '"'), (VK_OEM_COMMA.0, ',', '<'), (VK_OEM_PERIOD.0, '.', '>'),
];
