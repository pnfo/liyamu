//! `Session::handles` (asked by Windows before a key is sent) must agree with what `key` then does.

use liyamu_core::{Key, Layout, Mods, Options, Session};

#[test]
fn handles_agrees_with_key() {
    let keys: Vec<Key> = "kaDa aayuboowan 12 wdhqfnd, x.Y?"
        .chars()
        .map(|c| if c == ' ' { Key::Space } else { Key::Char(c) })
        .collect();
    let extra = [
        Key::Backspace, Key::Enter, Key::Tab, Key::Escape, Key::Left, Key::Right, Key::Up, Key::Down, Key::Other,
    ];
    for layout in [Layout::Wijesekara, Layout::Singlish, Layout::Helakuru] {
        for (i, &probe) in keys.iter().chain(extra.iter()).enumerate() {
            for mods in [Mods::default(), Mods { ctrl: true, altgr: false }, Mods { ctrl: false, altgr: true }] {
                // type a prefix, move through the suggestions a little, then compare on the probe key
                for arrows in 0..3 {
                    let mut s = Session::new(Options { layout, ..Options::default() });
                    for &k in keys.iter().take(i % 9 + 1) {
                        s.key(k, Mods::default());
                    }
                    for _ in 0..arrows {
                        s.key(Key::Right, Mods::default());
                    }
                    let expected = s.handles(probe, mods);
                    let r = s.key(probe, mods);
                    assert_eq!(expected, r.handled, "{layout:?} {probe:?} {mods:?} after {} keys, {arrows} arrows", i % 9 + 1);
                }
            }
        }
    }
}
