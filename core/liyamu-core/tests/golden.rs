//! Keystroke tables: tests/golden/*.tsv. Each row types its keys into a fresh session and checks what
//! the text field shows.

use liyamu_core::sim::type_keys;
use liyamu_core::{Layout, Options, Order, Output, Session};

fn options(spec: &str) -> Options {
    let mut o = Options::default();
    for f in spec.split(',').filter(|f| !f.is_empty() && *f != "-") {
        match f {
            "trad" => o.order = Order::Traditional,
            "logical" => o.order = Order::Logical,
            "pali" => o.pali = true,
            "nosug" => o.suggest = false,
            "repaya" => o.repaya = true,
            "conj" => o.conjuncts = true,
            "nopali" => o.pali_words = false,
            "fm" => o.output = Output::Fm,
            "isi" => o.output = Output::Isi,
            "sing" => o.layout = Layout::Singlish,
            "hela" => o.layout = Layout::Helakuru,
            other => panic!("unknown option {other}"),
        }
    }
    o
}

fn run(table: &str) {
    let mut wrong = Vec::new();
    let mut n = 0;
    for (i, line) in table.lines().enumerate() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        let (spec, keys, want) = (f[0], f[1], f.get(2).copied().unwrap_or("").replace("\\n", "\n"));
        let mut s = Session::new(options(spec));
        let got = type_keys(&mut s, keys).shown();
        n += 1;
        if got != want {
            let codes = |s: &str| s.chars().map(|c| format!("{:04X}", c as u32)).collect::<Vec<_>>().join(" ");
            wrong.push(format!("line {}: [{spec}] {keys} → {got:?} [{}], want {want:?} [{}]", i + 1, codes(&got), codes(&want)));
        }
    }
    assert!(wrong.is_empty(), "{} of {n} rows wrong:\n{}", wrong.len(), wrong.join("\n"));
}

#[test]
fn wijesekara() {
    run(include_str!("golden/wijesekara.tsv"));
}

#[test]
fn singlish() {
    run(include_str!("golden/singlish.tsv"));
}
