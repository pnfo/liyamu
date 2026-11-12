//! liyamu — try the engine without installing an input method.
//!
//!   liyamu [options]                 read key lines from stdin, print what the field shows
//!   liyamu [options] -k 'fldaw'      type one line of keys
//!   liyamu to-fm|to-isi 'text'       Unicode → legacy font text
//!   liyamu from-fm|from-isi 'text'   legacy font text → Unicode
//!   liyamu eval [n] [heldout.txt]    measure Easy Singlish suggestions on held-out sentences
//!
//! options: --singlish  --helakuru  --logical  --pali  --fm  --isi  --no-suggestions  --repaya  --conjuncts  --no-pali-words
//! keys: ␣ Space, ⏎ Enter, ⌫ Backspace, ⌥ AltGr for the next key (and a plain space is Space)

mod eval;

use liyamu_core::sim::type_keys;
use liyamu_core::{Encoding, Layout, Options, Order, Output, Session};
use std::io::BufRead;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("eval") {
        let n = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(300);
        eval::run(n, args.get(2).map(String::as_str).unwrap_or("data/corpus/heldout.txt"));
        return;
    }
    if let Some(cmd) = args.first() {
        let text = args.get(1).cloned().unwrap_or_default();
        let done = match cmd.as_str() {
            "to-fm" => Some(Encoding::fm().from_unicode_lossy(&text).0),
            "to-isi" => Some(Encoding::isi().from_unicode_lossy(&text).0),
            "from-fm" => Some(Encoding::fm().to_unicode(&text)),
            "from-isi" => Some(Encoding::isi().to_unicode(&text)),
            _ => None,
        };
        if let Some(out) = done {
            println!("{out}");
            return;
        }
    }
    let mut opts = Options::default();
    let mut keys: Option<String> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--singlish" => opts.layout = Layout::Singlish,
            "--helakuru" => opts.layout = Layout::Helakuru,
            "--logical" => opts.order = Order::Logical,
            "--pali" => opts.pali = true,
            "--no-suggestions" => opts.suggest = false,
            "--repaya" => opts.repaya = true,
            "--conjuncts" => opts.conjuncts = true,
            "--no-pali-words" => opts.pali_words = false,
            "--fm" => opts.output = Output::Fm,
            "--isi" => opts.output = Output::Isi,
            "-k" => keys = it.next().cloned(),
            other => {
                eprintln!("unknown argument {other}");
                std::process::exit(2);
            }
        }
    }
    let run = |line: &str| {
        let mut s = Session::new(opts.clone());
        let f = type_keys(&mut s, line);
        let shown = f.shown();
        let codes: Vec<String> = shown.chars().map(|c| format!("{:04X}", c as u32)).collect();
        println!("{shown}\t[{}]{}", codes.join(" "), if f.lossy { "\t(lossy)" } else { "" });
        if !f.candidates.is_empty() {
            println!("\tsuggestions: {}", f.candidates.join("  "));
        }
    };
    match keys {
        Some(k) => run(&k),
        None => {
            for line in std::io::stdin().lock().lines() {
                run(&line.unwrap());
            }
        }
    }
}
