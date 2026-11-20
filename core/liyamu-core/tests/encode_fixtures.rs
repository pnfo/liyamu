//! The Rust port of convert.js must give exactly what pitaka.lk gives, on the 20,000 most common book words.
//! Fixtures are made by `node tools/export-encodings.mjs`.

use liyamu_core::Encoding;

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('t') => out.push('\t'),
                Some('n') => out.push('\n'),
                Some(o) => out.push(o),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn check(enc: &Encoding, fixture: &str) {
    let mut rows = 0;
    let mut wrong = Vec::new();
    for line in fixture.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<String> = line.split('\t').map(unescape).collect();
        let (uni, legacy, back) = (&f[0], &f[1], &f[2]);
        rows += 1;
        let got = enc.from_unicode(uni).unwrap_or_default();
        if &got != legacy {
            wrong.push(format!("from_unicode({uni}) = {got:?}, pitaka.lk {legacy:?}"));
            continue;
        }
        if !legacy.is_empty() {
            let got = enc.to_unicode(legacy);
            if &got != back {
                wrong.push(format!("to_unicode({legacy}) = {got}, pitaka.lk {back}"));
            }
        }
    }
    assert!(rows > 10_000, "fixture too small: {rows}");
    assert!(wrong.is_empty(), "{} of {rows} differ:\n{}", wrong.len(), wrong.iter().take(30).cloned().collect::<Vec<_>>().join("\n"));
}

#[test]
fn fm_matches_pitaka() {
    check(Encoding::fm(), include_str!("../../../data/fixtures/fm-words.tsv"));
}

/// The key that draws each letter correctly (found with tools/fm-forms-check.py, kept in pitaka.lk's table).
#[test]
fn fm_letter_forms() {
    let cases = [
        ("කුකූ ගුගූ ඟු තුතූ භුභූ ශුශූ", "l=l+ .=.+ `.= ;=;+ N=N+ Y=Y+"),
        ("මු නූ සු", "uq kQ iq"),
        ("ක්‍රු ප්‍රූ රු", "l%e m%E re"),
        ("ඦ ඣ ඵි ඵී ඨ් ත්‍ථි විද්‍යා", "`c ® Ý Ú GA †‘ úoHd"),
    ];
    for (uni, legacy) in cases {
        assert_eq!(Encoding::fm().from_unicode(uni).as_deref(), Some(legacy), "{uni}");
    }
}

#[test]
fn isi_matches_pitaka() {
    check(Encoding::isi(), include_str!("../../../data/fixtures/isi-words.tsv"));
}

#[test]
fn fm_typing_orders() {
    // from pitaka.lk dev/unicode/test/convert.test.mjs
    let cases = [
        ("fldrKh", "කොරණය"),
        ("fla%", "ක්‍රේ"),
        ("fl%a", "ක්‍රේ"),
        ("ls%", "ක්‍රි"),
        ("ff;%", "ත්‍රෛ"),
        ("hH_", "ර්‍ය්‍ය"),
        ("l_", "ර්‍ක"),
        ("fød", "ද්‍රො"),
        ("fm%!", "ප්‍රෞ"),
        ("fkHda", "න්‍යෝ"),
        ("ft", "ඓ"),
        ("Y%S", "ශ්‍රී"),
        ("CI", "ක්‍ෂ"),
        ("wd", "ආ"),
        ("T!", "ඖ"),
        ("wdhqfndajdoh", "ආයුබෝවාදය"),
    ];
    for (legacy, uni) in cases {
        assert_eq!(Encoding::fm().to_unicode(legacy), uni, "{legacy}");
    }
    assert_eq!(Encoding::for_font("fm", "FM Derana").unwrap().to_unicode("µ"), "i");
    assert_eq!(Encoding::fm().to_unicode("µ"), "ද්‍ය");
}
