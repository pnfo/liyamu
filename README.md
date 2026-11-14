# Liyamu ලියමු — Sinhala keyboard

A real input method for typing Sinhala on macOS and Windows (and later iOS/Android). It grew out of
Akkharapadayo, the Windows key-fixing tool by Ven. M. Pagngnaseeha Thero (sadaham.net), whose Wijesekara
extras and Pali mode it keeps.

- **Layouts:** Wijesekara (traditional or Unicode typing order, optional Pali mode), Easy Singlish, and Helakuru-style phonetic keys.
- **Pali:** repaya, joined letters (බැඳි අකුරු) and touching letters as options in every layout; Pali-only
  words in the suggestions can be switched on.
- **Your own shortcuts:** `bs=බුදු සරණයි` lines in the settings window.
- **Output:** Unicode, or FM / Isi legacy-font codes so that text typed into Word with FM Abhaya looks right.
- The word being typed is shown underlined (preedit) and committed at a space or punctuation, so no
  Backspace tricks are needed and it works the same in every app.

## Privacy first, free, open

A keyboard sees everything you type, passwords included. So:

- **Nothing leaves the device.** The keyboard has no network code at all: no accounts, no sync, no
  analytics, no crash reports, no ads. Suggestions come from word lists built into the app.
- **Nothing you type is stored.** (If learning from your own typing is added, it will be a file that stays
  on your device, off in password fields, with a switch and a delete button.)
- **Open source**, so the two points above can be checked rather than believed.
- **Free, all of it.** No paid features, no premium tier, no strings attached.

Plan and milestones: [docs/plan.md](docs/plan.md).

## Layout

```
core/liyamu-core   the engine (Rust, no OS code): layouts, FM/Isi encoders, session
core/liyamu-ffi    C API for the native shells: include/liyamu.h, libliyamu.a
core/liyamu-cli    `liyamu` command for trying the engine in a terminal
data/encodings      FM and Isi tables, exported from pitaka.lk
data/fixtures       20,000 book words converted by pitaka.lk, to check the Rust port against
data/lexicon        word and word-pair counts, everyday words, names, places, dictionary and Pali words, shortcuts
tools/              export-encodings.mjs, build-corpus.mjs, build-lexicon.mjs, build-names.mjs
macos/              the macOS input method and its settings window (Swift, InputMethodKit)
android/            the Android keyboard and its settings app (Kotlin)
design/logo/        logo concepts and the script that draws them
```

## Build and test

```sh
cargo test                     # golden keystroke tables + exact match with pitaka.lk's converter
cargo build --release -p liyamu-ffi   # target/release/libliyamu.a for the macOS shell

cargo run -q -p liyamu-cli -- -k 'fld wd'           # Wijesekara → කො ආ
cargo run -q -p liyamu-cli -- --fm -k 'wdhqfndajdoh' # FM codes
cargo run -q -p liyamu-cli -- --singlish -k 'aayuboowan'
cargo run -q -p liyamu-cli -- to-fm 'ශ්‍රී ලංකාව'
```

### macOS

```sh
macos/install.sh     # builds Liyamu.app, installs it to ~/Library/Input Methods and registers it
macos/uninstall.sh
```

Add **Liyamu** in System Settings › Keyboard › Text Input › Input Sources › Edit › + › Sinhala (macOS does
not let an installer switch input sources on; if it is not listed, log out and back in). Its menu in the
menu bar chooses the layout (**Wijesekara**, **Easy Singlish**, **Helakuru keys**) and the output (Unicode /
FM / Isi). **Settings and Help…** in that menu (or opening Liyamu.app while it is running) opens a window
with the rest: repaya, Pali joined and touching letters, Pali words in suggestions, the Wijesekara typing
order, your own shortcuts (kept in `~/Library/Application Support/Liyamu/shortcuts.txt`) and a help page.
Suggestions are drawn in Abhaya (`macos/Resources/fonts`). It is one input method without input modes
because macOS would not enable the extra modes of a multi-mode one. `build.sh` makes an ad-hoc signed build
for this Mac only; releases need a universal Rust library (arm64 + x86_64), Developer ID signing and
notarization.

Suggestions appear in a bar under the word: **→ / ↓** and **← / ↑** move the highlight, **Space** commits the
highlighted word (with a space), **Enter** commits it without a new line, **1–9** or a click commit that one. Easy Singlish suggests
readings of the word; Wijesekara and Helakuru keys suggest common words that start with what was typed.
**Word suggestions** in the menu turns them off for all layouts.

### Android

```sh
android/build-rust.sh                      # the engine for Android (needs rustup with the Android targets)
(cd android && ./gradlew assembleDebug)    # android/app/build/outputs/apk/debug/app-debug.apk
```

`android/` is the keyboard (an input method service drawing its own keys, with the suggestion row above
them) and the app that holds its settings, shortcuts and help. The engine is the same Rust library, reached
through a small C bridge (`app/src/main/cpp/liyamu_jni.c`). The app asks for no permissions at all. Password,
e-mail and web-address fields get plain letters and never reach the engine. On the emulator, run
`adb shell settings put secure show_ime_with_hard_keyboard 1` or the keyboard stays hidden.

Store texts and screenshots: [docs/store/listing.md](docs/store/listing.md). Privacy policy:
[PRIVACY.md](PRIVACY.md).

Key notation in tests and the CLI: `␣` Space, `⏎` Enter, `⌫` Backspace, `←→↑↓` arrows, `⌥x` AltGr+x.

The FM/Isi tables live in pitaka.lk (`tools/unicode/lib/encodings`). After changing them there, run
`node tools/export-encodings.mjs [path/to/pitaka.lk]` and `cargo test`.

## Wijesekara

Standard Wijesekara keys, plus:

| Typed | Gives | Notes |
|---|---|---|
| `f` before a letter | කෙ | traditional order (default); `ff` → ෛ, then `d` → ො, `a` → ේ, `A` → ෞ |
| `~` after a letter | කර්‍ම | traditional order; at the start of a word it comes first |
| `w d`, `w e`, `W A`, `t a`, `T a` | ආ ඇ ඌ ඒ ඕ | vowel letters in pieces |
| `\` between letters | ක්‍ෂ ත්‍ව | joined letters; `` ` `` rakaransaya, `H` yansaya |
| AltGr `o . v x , C` | ඳ ඟ ඬ ඞ ඏ ඦ | |
| AltGr `p P c G j O` | ඤ්ච ඤ්ඡ ඤ්ජ ට්ඨ ද්‍ව ද්‍ධ | from Akkharapadayo |
| Pali mode: `a` after a letter | ඤ‍්ච ම‍්බ | touching letters; ක/ත/න/ද pairs that are normally joined stay joined (න්‍ද, ත්‍ථ); `X` → ඞ |

Logical order (Unicode order, as the Windows layout without Akkharapadayo) is an option.

Changes from Akkharapadayo: no Win10/Win11 switch (not needed without Backspace tricks); no auto-start or
registry settings; `ඔ` followed by any key no longer turns into ඕ (a bug in the original);
`i D` → ඍ was dropped because සෘ is a real spelling (සෘජු). Auto-text shortcuts will come back as
suggestions (milestone 3).

## Singlish (two phonetic modes)

Both re-read the word as you type (`k` ක් → `ka` ක → `kaa` කා); Backspace removes one Latin letter.

### Easy Singlish

Type the way people usually spell Sinhala in English letters. No table to learn and capitals don't matter.
Many letters have several readings, and the likeliest word wins:
- `d` ද/ඩ/ධ, `t` ට/ත, `n` න/ණ/ං, `l` ල/ළ, `s` ස/ශ/ෂ, `e` එ/ඒ/ඇ, `i`/`u`/`o` short or long
- `nd` ඳ, `mb` ඹ, `ng` ංග/ඟ, a final `a` may be ා

The Sinhala word is chosen and suggestions are made from real words only (`core/liyamu-core/src/suggest`):
- **Readings**: known words the letters spell, likeliest first. If they spell no known word the letters are
  read literally (names, new words), scored with a letter model.
- **Typos**: words a small slip away. `gedar`, `gedra`, `gedr` → ගෙදර. A correction becomes what Space
  commits only when the letters spell no known word and the slip is one typists make (a vowel left out, a
  double typed single, two letters swapped); other corrections are offered, never applied by themselves.
- **Shorthand**: letters typed without their vowels find the word, however many are left out: `hri ennm` →
  හරි එන්නම්, `gdr` → ගෙදර, `khmd` → කොහොමද. ඹ ඳ ඟ may be typed `b` `d` `g` (`kolaba` → කොළඹ) and ක as `c`.
- **Shortcuts** (`data/lexicon/shortcuts.tsv`, letters<TAB>words; add your own lines): what the letters stand
  for comes first. `col`, `colombo` → කොළඹ, `kandy` → මහනුවර, `mn` → මං, `bs` → බුදු සරණයි.
- **Completions**: likely words that start with the letters so far.
- **Context**: all of these are ranked by how likely the word is after the previous words of the sentence
  (`api gedara g` → ගිහින්, ගියා).
- **English**: the letters as typed are always the last suggestion. If they are one of about 1,300 common
  English words (`data/lexicon/english.txt`) they come second, or first when no reading is a known word:
  phone, meeting, computer stay English; bath gives බත් with bath second.

A capital leans to the heavy letter (T ඨ, D ඩ, N ණ, L ළ, S ෂ, B භ).

What it learned from (`tools/build-corpus.mjs`, `tools/build-lexicon.mjs`): the pitaka.lk books (4.3 M
words, formal, with Pali) and OpenSubtitles Sinhala from OPUS (4.1 M words of everyday speech), 5% held out
for measuring. Only word counts (`data/lexicon/words.tsv`), word-pair counts (`data/lexicon/ngram.bin`) and
a hand-made list of everyday words (`data/lexicon/everyday.tsv`) are kept, never the text. Three more word
lists make names and rarer words known (`tools/build-names.mjs`): `places.tsv` (words of about 12,000
Sri Lankan place names, from the OCHA COD-AB administrative names and OpenStreetMap, and of names of people
from Wikidata), `names.tsv` (hand-made: family names, given names, titles) and `dictionary.txt` (headwords of
the Sinhala Dictionary Office's Concise and Spelling dictionaries, as on arutha.lk). The headwords also
show which corpus spellings are wrong (පුලුවන් for පුළුවන්: ල/ළ, න/ණ, ශ/ෂ, a missing joiner); those are counted as
the right word and never suggested (`data/lexicon/misspellings.tsv`, about 3,500 of them). Measurements:
[docs/suggest-eval.md](docs/suggest-eval.md), from `liyamu eval`.

### Helakuru keys

The fixed, case-sensitive table of Helakuru's phonetic keyboard, re-implemented from the keymap Bhasha Lanka
published for Linux (2018), for people who already type with it. Highlights: `d` ඩ, `dh`/`q` ද, `Dh`/`Q` ධ,
`th` ත, `t` ට, `B` ඹ, `bh` භ, `S` ෂ, `c` ක, `ch` ච, `I` `U` `O` short, `ii` `uu` `oo` `ee` long, `E` ඓ,
`kru` කෘ, `sRu` ඍ, `x` ං, `H` ඃ, `X` ඞ, `zk` ඤ, `zh` ඥ, `zq` ඳ, `zd` ඬ, `zg` ඟ, `zj` ඦ, `NG` ක්‍ෂ.
r/y after a consonant always join (`kra` ක්‍ර, `kya` ක්‍ය); `z` stops a join (`kzya` ක්ය). No automatic ං:
`laxkaa` ලංකා. Extras where Helakuru types nothing useful: `ae` ඇ, `Sh` ෂ, `jh` ඣ, `zb` ඹ.

## Licence and credits

Copyright © Path Nirvana Foundation. Free software under the GNU General Public License, version 3 or
later ([LICENSE](LICENSE)).

- Akkharapadayo by Ven. M. Pagngnaseeha Thero (sadaham.net): the Wijesekara extras and the Pali mode.
- FM / Isi conversion tables and book word counts: pitaka.lk. Dictionary headwords: Sinhala Dictionary
  Office dictionaries as published on arutha.lk. Pali word forms: Digital Pāḷi Dictionary.
- Everyday word counts: OpenSubtitles (via OPUS). Place names: OCHA COD-AB Sri Lanka and © OpenStreetMap
  contributors. People's names: Wikidata. Only word lists and counts derived from these are in the repo.
- Abhaya font: `macos/Resources/fonts`.
