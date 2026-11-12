# Liyamu — a real Sinhala input method for macOS and Windows

## Context

The monk's Akkharapadayo is a Windows-only PyQt overlay that watches every keystroke and repairs text with
Backspace + retype. It is fragile (timing, per-app behaviour, Win10/Win11 switch), cannot run on macOS, and
its FM mode is hundreds of hand-written glyph rules. We are replacing it with a proper OS-level input method
(Option B): one shared, tested core engine with thin native shells.

Decisions from the user:
- Layouts: **Wijesekara** (with the monk's extra key orders) and **Singlish** (phonetic).
- Output: **Unicode** and **FM** required, **Isi** optional (comes nearly free from the same table mechanism).
  In FM mode people type into Word with the font set to FM Abhaya and must see correct text as they type.
- Completion: mostly Sinhala with Pali words inside. Dictionary from `pitaka.lk/books`, plus learning from
  what the user types, plus optional bootstrap from the user's own files.
- Release pipelines for mac/windows/ios/android already exist; desktop first, core kept portable for mobile.

## Architecture

```
liyamu-core (Rust, no OS deps)
  key event ─► layout (wijesekara | singlish) ─► Unicode word being composed
            ─► predictor (base dict + user model + auto-text) ─► candidates
            ─► encoder (unicode | fm | isi) ─► preedit / commit text
liyamu-ffi  (C ABI, cbindgen header)  ── used by ──►  macOS shell (Swift, InputMethodKit)
                                                       Windows shell (Rust, TSF COM DLL)
liyamu-cli  (REPL: type keys, see preedit/commit/candidates — for tests and demos)
```

**Composition model.** The unit being composed is the current word, shown as underlined uncommitted text
(preedit). It is committed on space, punctuation, Enter, or picking a candidate. This removes every Backspace
trick: kombuva-before-consonant, `w`+`d` → ආ, conjuncts etc. are just edits to the preedit. The engine keeps
the word as Unicode internally in all modes.

**FM / Isi mode.** Same typing, but preedit and commit text are passed through the legacy encoder, so the
app receives FM codes and Word renders them in FM Abhaya while typing. The candidate window is our own and
always shows Unicode. If a word can't be written in the encoding (`fromUnicode` returns null), commit what can
be encoded and flash the candidate bar — never emit mixed garbage silently.

**Session API (core).** `Session::key(key, modifiers) -> Response { handled, preedit, commit, candidates }`,
plus `pick(i)`, `reset()`, `set_mode(layout, output)`. Shells only translate OS key events and draw.

## Repo layout (this directory, `git init`)

```
(the monk's original files are kept outside the repo)
core/                        Rust workspace: liyamu-core, liyamu-ffi, liyamu-cli
  liyamu-core/src/layout/{wijesekara,singlish}.rs
  liyamu-core/src/encode/{mod,reorder}.rs     port of pitaka.lk convert.js
  liyamu-core/src/predict/{dict,user,autotext}.rs
  liyamu-core/src/session.rs
  liyamu-core/tests/golden/*.tsv              keystrokes ⇒ expected text
data/                        generated: encodings/*.json, dict.fst
tools/                       node scripts: export-encodings.mjs, build-dict.mjs
macos/                       Xcode project: Liyamu.app input method + settings
windows/                     TSF text service crate + installer script
```

## Milestones

### M1 — Core typing engine (no completion yet)
1. Scaffold workspace; move monk's files into `reference/`.
2. **Golden tests first.** Turn every rule in `Akkharapadayo.py:632-900` (Unicode + Pali) into rows of
   `layout  keys  expected`, e.g. `wije  f l a  ⇒ කේ`, `wije  w d ⇒ ආ`, Pali `a` ⇒ ්+ZWJ join, `X` ⇒ ඞ,
   `\` conjunct key, AltGr ඟ ඳ ඬ. Fix the monk's bugs in the spec (line 784 precedence, etc.) rather than
   reproduce them.
3. `wijesekara.rs`: base key table from the `keymap` in `pitaka.lk/tools/unicode/keyboard.html`
   (Wijesekara column), plus a syllable composer that accepts both logical order and the old visual order
   (kombuva first, `ff` → ෛ, vowel-letter compositions). Pali conjunct behaviour is an option flag, not a
   separate language mode.
4. `singlish.rs`: phonetic transliteration over the Latin buffer of the current word; take the rules from
   the "එසැණ සිංග්ලිෂ්" implementation in the same `keyboard.html`.
5. `encode/`: `tools/export-encodings.mjs` imports `pitaka.lk/tools/unicode/lib/encodings/{fm,isi}.js` and
   writes JSON, so pitaka.lk stays the single source of truth. Port `reorder`, `toUnicode`, `fromUnicode`
   from `pitaka.lk/tools/unicode/lib/convert.js` (~140 lines) to Rust. `toUnicode` is needed for importing
   the user's old FM documents later.
6. `liyamu-cli` REPL.

### M2 — macOS input method
- Swift `IMKInputController` subclass calling `liyamu-ffi` (static lib + header via cbindgen).
- Preedit via `setMarkedText`, commit via `insertText`; pass through anything with Cmd/Ctrl.
- Input-menu items: layout (Wijesekara/Singlish), output (Unicode/FM/Isi), settings window. Each
  layout registered as its own input mode so the OS switcher (and its hotkeys) replaces the monk's hotkey UI.
- Installs to `~/Library/Input Methods`; `.pkg` built, signed and notarised by the existing pipeline.

### M3 — Word completion
- `tools/build-dict.mjs`: full pass over `pitaka.lk/books/**/*.html` (reuse the extraction in
  `pitaka.lk/dev/unicode/corpus.mjs`, without its file sampling), normalise (ZWJ forms, NFC), drop
  count < 3, keep top ~150k words → `data/dict.fst` (`fst` crate: word → frequency, prefix search).
- Ranking: prefix matches scored by base frequency + user frequency boost + recency; optional previous-word
  (bigram) boost from the user model. `pitaka.lk/tools/unicode/lib/sinhala-model.js` is available to
  reject implausible learned "words" (typos).
- User model: small local file in the app-support dir; learns committed words; never learns in secure
  (password) fields; "forget word" and "clear learned data" in settings. Nothing leaves the machine.
- Bootstrap import: settings → "Learn from my files" (.txt/.html/.docx; FM-font text detected and run
  through `toUnicode`).
- Auto-text: import `auto_text.txt` format (`key=phrase`); shortcuts appear as top candidates instead of
  the `'` trigger hack.
- Candidate UI: horizontal bar under the caret, pick with number keys or Tab; Esc dismisses; can be turned off.

### M4 — Windows input method
- TSF text service as a COM DLL in Rust (`windows` crate): `ITfTextInputProcessor`, key event sink,
  composition, own candidate window. Same core, same golden tests.
- Language-bar/tray menu for layout and output mode; settings exe.
- Installer (registers the DLL for x64 + arm64), signed by the existing pipeline.

### M5 — Extras
- "Convert selection/clipboard Unicode ⇄ FM" command in the input menu (answers the user's question: yes,
  the IME's menu can host this; clipboard version works everywhere, selection version where the app allows).
- Isi output switched on once FM is solid.
- Later: iOS keyboard extension / Android IME over the same core via FFI.

## Risks to check early
- Word + FM font + preedit on both OSes: confirm the underlined preedit renders in the document font
  (spike in M2 before investing in FM polish). Fallback: commit per syllable in FM mode.
- Apps with poor IME support (some Electron/WhatsApp builds): test early; fallback is a "direct mode" that
  commits per syllable with no preedit.
- Singlish ambiguity vs completion: candidates should come from the transliterated Unicode prefix.

## Verification
- `cargo test` — golden keystroke tables for both layouts; FM/Isi round-trip (`toUnicode(fromUnicode(w)) == w`)
  over the corpus word list, matching the checks in `pitaka.lk/dev/unicode/test`.
- `liyamu-cli` — type sample sentences from the monk's rules by hand.
- macOS: install locally, type in TextEdit, Word (Unicode font and FM Abhaya), Safari, WhatsApp, Terminal;
  check password fields don't learn.
- Windows (VM or the monk's machine): same matrix in Notepad, Word, Chrome, WhatsApp.
- Give the M2 build to the monk for a week of real use before starting M4.
