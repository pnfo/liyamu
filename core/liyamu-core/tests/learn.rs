//! Learning end to end, through a session. Its own test binary, because learning is switched on for the
//! whole process.

use liyamu_core::{learn, Key, Layout, Mods, Options, Session};

fn type_keys(s: &mut Session, keys: &str) -> Vec<String> {
    let mut last = Vec::new();
    for c in keys.chars() {
        last = s.key(Key::Char(c), Mods::default()).candidates;
    }
    last
}

#[test]
fn a_word_picked_twice_comes_first_until_forgotten() {
    let file = std::env::temp_dir().join(format!("liyamu-learn-{}.tsv", std::process::id()));
    let _ = std::fs::remove_file(&file);
    learn::configure(file.to_str(), true);
    let mut s = Session::new(Options { layout: Layout::Singlish, ..Default::default() });

    let before = type_keys(&mut s, "compute");
    let i = before.iter().position(|w| w == "compute").expect("the letters as typed are offered");
    assert!(i > 0);
    assert_eq!(s.pick(i).commit, "compute");

    // once: second place
    let once = type_keys(&mut s, "compute");
    assert_eq!(once[0], before[0]);
    assert_eq!(once[1], "compute");
    s.pick(1);

    // twice: first, and Space commits it
    assert_eq!(type_keys(&mut s, "compute")[0], "compute");
    assert_eq!(s.key(Key::Space, Mods::default()).commit, "compute ");
    assert!(std::fs::read_to_string(&file).unwrap().contains("compute\tcompute\t2\t"));

    // a choice made with the arrow keys and Space counts too
    let mm = type_keys(&mut s, "mm");
    let second = mm[1].clone();
    s.key(Key::Right, Mods::default());
    s.key(Key::Space, Mods::default());
    assert_eq!(type_keys(&mut s, "mm")[1], second);
    s.reset();

    // switched off: as before, but nothing is lost
    learn::configure(None, false);
    assert_eq!(type_keys(&mut s, "compute"), before);
    s.reset();
    learn::configure(None, true);
    assert_eq!(type_keys(&mut s, "compute")[0], "compute");
    s.reset();

    learn::forget();
    assert_eq!(type_keys(&mut s, "compute"), before);
    assert!(!file.exists());
}
