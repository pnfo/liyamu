//! Liyamu settings and help: the same options as the keyboard's menu and more, saved in the registry for
//! the keyboards in every app. `--add-keyboard` / `--remove-keyboard` (used by the installer) add Liyamu
//! to, or take it from, the user's Sinhala keyboards.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
fn main() {
    app::main();
}

#[cfg(windows)]
mod app {
    use liyamu_ime::settings::{self, Settings, LAYOUTS, OUTPUTS};
    use std::cell::{Cell, RefCell};
    use std::os::windows::process::CommandExt;
    use windows::core::*;
    use windows::Win32::Foundation::*;
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Controls::*;
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::*;

    const CLASS: PCWSTR = w!("LiyamuSettings");
    /// text with Sinhala in it: Sinhala in Abhaya, the rest in the system font (one Win32 label has one font)
    const TEXT_CLASS: PCWSTR = w!("LiyamuText");
    /// what Windows calls the keyboard: language, text service class and profile
    const TIP: &str = "045B:{019C5475-F781-4EFE-B363-E4D22B25F970}{51ABF515-2FD5-4B67-8FAD-12C39216DDFE}";

    const ID_TABS: i32 = 10;
    const ID_LAYOUT: i32 = 100;
    const ID_OUTPUT: i32 = 101;
    const ID_SUGGEST: i32 = 110;
    const ID_LEARN: i32 = 111;
    const ID_PALI_WORDS: i32 = 112;
    const ID_REPAYA: i32 = 113;
    const ID_CONJUNCTS: i32 = 114;
    const ID_PALI: i32 = 115;
    const ID_TRADITIONAL: i32 = 116;
    const ID_FORGET: i32 = 130;
    const ID_SHORTCUTS: i32 = 140;
    const ID_SAVE: i32 = 141;
    const ID_ADD: i32 = 150;
    const ID_LINK: i32 = 151;

    enum Item {
        Heading(&'static str),
        Note(&'static str),
        Check(i32, &'static str),
        Combo(i32, &'static str, &'static [&'static str]),
        Button(i32, &'static str),
        Edit(i32),
    }
    use Item::*;

    fn tabs() -> Vec<(&'static str, Vec<Item>)> {
        vec![
            ("Typing", vec![
                Combo(ID_LAYOUT, "Layout:", &LAYOUTS),
                Combo(ID_OUTPUT, "Write as:", &OUTPUTS),
                Note("FM and Isi are for documents set in the old fonts; choose that font in the app you type in."),
                Heading("Suggestions"),
                Check(ID_SUGGEST, "Word suggestions"),
                Note("A row of words under what you type. → ← choose, Space takes the highlighted one, 1–9 take that one."),
                Check(ID_LEARN, "Learn from the words I pick"),
                Note("A word you pick twice for the same letters is offered first from then on. Kept only on this computer."),
                Button(ID_FORGET, "Forget what was learned"),
                Check(ID_PALI_WORDS, "Suggest Pali words"),
                Note("Also offer words found only in Pali: ධම්මං, භික්ඛූනං. For typing suttas and chanting."),
            ]),
            ("Letters", vec![
                Heading("How joined letters are written"),
                Check(ID_REPAYA, "Repaya"),
                Note("ර් before a letter is written above it: ධර්ම → ධර්‍ම, කර්ම → කර්‍ම"),
                Check(ID_CONJUNCTS, "Pali joined letters (බැඳි අකුරු)"),
                Note("බුද්ධ → බුද්‍ධ, තත්ථ → තත්‍ථ, නන්ද → නන්‍ද, ද්වාර → ද්‍වාර"),
                Check(ID_PALI, "Pali touching letters"),
                Note("ධම්ම → ධම‍්ම, සඤ්ඤා → සඤ‍්ඤා. In Wijesekara, hal after a letter touches the next one and X types ඞ."),
                Heading("Wijesekara"),
                Check(ID_TRADITIONAL, "Kombuva before the letter, as on paper"),
                Note("Type ෙ then ක for කෙ (as in FM fonts). Off: ක then ෙ, the Windows Unicode order."),
            ]),
            ("Shortcuts", vec![
                Note("Your own short forms, one on a line: the keys you type, =, then what they stand for. When you type \
                      exactly those keys the words are offered first and Space takes them. The keys are as typed in the \
                      layout you use (in Wijesekara, iaj for ස්ව).    Example:  bs=බුදු සරණයි"),
                Edit(ID_SHORTCUTS),
                Button(ID_SAVE, "Save shortcuts"),
            ]),
            ("Help", vec![
                Heading("Turning Liyamu on"),
                Note("Liyamu is a keyboard under the Sinhala language. The button below adds it to your keyboards (and \
                      Sinhala to your languages if it is not there). You can also do it in Settings › Time & language › \
                      Language & region › Sinhala › Language options › Add a keyboard."),
                Button(ID_ADD, "Add Liyamu to my keyboards"),
                Note("Switch between your keyboards with Windows key + Space. While Liyamu is on, its icon in the \
                      taskbar opens a menu for the layout and the output."),
                Heading("Typing"),
                Note("The word being typed is underlined and goes into the text at a space or punctuation. Easy \
                      Singlish: type as it sounds (aayuboowan → ආයුබෝවන්). Wijesekara: the keys of Sinhala typewriters. \
                      Helakuru keys: phonetic keys like Helakuru's."),
                Heading("Privacy"),
                Note("Nothing you type leaves this computer: Liyamu has no network code at all. The only thing it keeps \
                      is which suggestion you picked for which letters, in a file in your profile (never from password \
                      fields), which the Typing tab can turn off and delete. Liyamu is free and open source (GPL), \
                      from the Path Nirvana Foundation."),
                Button(ID_LINK, "arutha.lk/liyamu"),
            ]),
        ]
    }

    struct Ui {
        hwnd: HWND,
        font: HFONT,
        bold: HFONT,
        white: HBRUSH,
        /// (tab, control)
        controls: Vec<(usize, HWND)>,
        notes: Vec<HWND>,
        tab: usize,
    }

    thread_local! {
        static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
        /// (the system font, Abhaya) for the mixed text
        static FONTS: Cell<(HFONT, HFONT)> = const { Cell::new((HFONT(std::ptr::null_mut()), HFONT(std::ptr::null_mut()))) };
    }

    fn is_sinhala(c: char) -> bool {
        ('\u{0D80}'..='\u{0DFF}').contains(&c)
    }

    /// Splits a word into runs of Sinhala and of other letters (joiners stay with the letters around them).
    fn runs(word: &str) -> Vec<(bool, Vec<u16>)> {
        let mut out: Vec<(bool, Vec<u16>)> = Vec::new();
        for c in word.chars() {
            let si = match c {
                '\u{200C}' | '\u{200D}' => out.last().map_or(true, |r| r.0),
                c => is_sinhala(c),
            };
            if out.last().map_or(true, |r| r.0 != si) {
                out.push((si, Vec::new()));
            }
            let mut buf = [0u16; 2];
            out.last_mut().unwrap().1.extend_from_slice(c.encode_utf16(&mut buf));
        }
        out
    }

    struct Piece {
        x: i32,
        line: i32,
        sinhala: bool,
        text: Vec<u16>,
    }

    /// Lays `text` out in lines of `width`, wrapping at spaces: the pieces, the line height and the baseline.
    unsafe fn lay(dc: HDC, text: &str, width: i32) -> (Vec<Piece>, i32, i32, i32) {
        let (latin, sinhala) = FONTS.get();
        let old = SelectObject(dc, latin);
        let (mut asc, mut desc) = (0, 0);
        for f in [latin, sinhala] {
            SelectObject(dc, f);
            let mut tm = TEXTMETRICW::default();
            let _ = GetTextMetricsW(dc, &mut tm);
            asc = asc.max(tm.tmAscent);
            desc = desc.max(tm.tmDescent);
        }
        let measure = |si: bool, t: &[u16]| {
            SelectObject(dc, if si { sinhala } else { latin });
            let mut sz = SIZE::default();
            let _ = GetTextExtentPoint32W(dc, t, &mut sz);
            sz.cx
        };
        let space = measure(false, &[b' ' as u16]);
        let (mut pieces, mut x, mut line) = (Vec::new(), 0, 0);
        for (i, word) in text.split(' ').enumerate() {
            let mut start = if i > 0 { x + space } else { x };
            if word.is_empty() {
                x = start;
                continue;
            }
            let rs = runs(word);
            let ww: i32 = rs.iter().map(|(si, t)| measure(*si, t)).sum();
            if start + ww > width && x > 0 {
                line += 1;
                start = 0;
            }
            let mut px = start;
            for (si, t) in rs {
                let w = measure(si, &t);
                pieces.push(Piece { x: px, line, sinhala: si, text: t });
                px += w;
            }
            x = px;
        }
        SelectObject(dc, old);
        (pieces, asc + desc, asc, line + 1)
    }

    unsafe fn rich_height(hwnd: HWND, text: &str, width: i32) -> i32 {
        let dc = GetDC(hwnd);
        let (_, lh, _, lines) = lay(dc, text, width);
        ReleaseDC(hwnd, dc);
        lh * lines
    }

    /// The mixed-text control. Its user data, when set, is a check box that a click on the text toggles (the
    /// text is then the check box's label, in the label colour; else it is a grey note).
    unsafe extern "system" fn text_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let dc = BeginPaint(hwnd, &mut ps);
                let mut rc = RECT::default();
                let _ = GetClientRect(hwnd, &mut rc);
                FillRect(dc, &rc, GetSysColorBrush(COLOR_WINDOW));
                let n = GetWindowTextLengthW(hwnd) as usize;
                let mut buf = vec![0u16; n + 1];
                let got = GetWindowTextW(hwnd, &mut buf) as usize;
                let text = String::from_utf16_lossy(&buf[..got]);
                let label = GetWindowLongPtrW(hwnd, GWLP_USERDATA) != 0;
                SetBkMode(dc, TRANSPARENT);
                SetTextColor(dc, if label { COLORREF(GetSysColor(COLOR_WINDOWTEXT)) } else { COLORREF(0x00606060) });
                SetTextAlign(dc, TA_BASELINE | TA_LEFT);
                let (pieces, lh, base, _) = lay(dc, &text, rc.right);
                let (latin, sinhala) = FONTS.get();
                let old = SelectObject(dc, latin);
                for p in pieces {
                    SelectObject(dc, if p.sinhala { sinhala } else { latin });
                    let _ = TextOutW(dc, p.x, p.line * lh + base, &p.text);
                }
                SelectObject(dc, old);
                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                let check = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                if check != 0 {
                    SendMessageW(HWND(check as isize as *mut _), BM_CLICK, WPARAM(0), LPARAM(0));
                }
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    /// Abhaya, which the installer puts in fonts\ next to this program, loaded for this program only; else
    /// Nirmala UI. A little larger than the Latin text, as Sinhala letters sit smaller.
    unsafe fn sinhala_font(dpi: i32) -> HFONT {
        if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
            let path = wide(&dir.join("fonts").join("AbhayaUni-Regular.ttf").to_string_lossy());
            AddFontResourceExW(PCWSTR(path.as_ptr()), FR_PRIVATE, None);
        }
        let make = |face: PCWSTR, h: i32| {
            CreateFontW(-h, 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0, DEFAULT_CHARSET.0 as u32, OUT_TT_PRECIS.0 as u32,
                        CLIP_DEFAULT_PRECIS.0 as u32, CLEARTYPE_QUALITY.0 as u32, (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32, face)
        };
        let f = make(w!("Abhaya Uni"), 13 * dpi / 72);
        let dc = GetDC(None);
        let old = SelectObject(dc, f);
        let mut buf = [0u16; 64];
        let n = GetTextFaceW(dc, Some(&mut buf));
        SelectObject(dc, old);
        ReleaseDC(None, dc);
        if n > 0 && String::from_utf16_lossy(&buf[..(n as usize) - 1]) == "Abhaya Uni" {
            return f;
        }
        let _ = DeleteObject(f);
        make(w!("Nirmala UI"), 10 * dpi / 72)
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn checked(id: i32, s: &Settings) -> bool {
        match id {
            ID_SUGGEST => s.suggest,
            ID_LEARN => s.learn,
            ID_PALI_WORDS => s.pali_words,
            ID_REPAYA => s.repaya,
            ID_CONJUNCTS => s.conjuncts,
            ID_PALI => s.pali,
            ID_TRADITIONAL => s.traditional,
            _ => false,
        }
    }

    fn set_checked(id: i32, s: &mut Settings, on: bool) {
        match id {
            ID_SUGGEST => s.suggest = on,
            ID_LEARN => s.learn = on,
            ID_PALI_WORDS => s.pali_words = on,
            ID_REPAYA => s.repaya = on,
            ID_CONJUNCTS => s.conjuncts = on,
            ID_PALI => s.pali = on,
            ID_TRADITIONAL => s.traditional = on,
            _ => {}
        }
    }

    fn powershell(script: &str) -> bool {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .is_ok_and(|s| s.success())
    }

    /// Adds Liyamu to the user's Sinhala keyboards, adding Sinhala to the languages if needed.
    fn add_keyboard() -> bool {
        powershell(&format!(
            "$ErrorActionPreference='Stop'; $t='{TIP}'; $l=Get-WinUserLanguageList; \
             $si=$l | Where-Object {{ $_.LanguageTag -like 'si*' }} | Select-Object -First 1; \
             if (-not $si) {{ $l.Add('si-LK'); $si=$l[$l.Count-1]; $si.InputMethodTips.Clear() }}; \
             if (-not ($si.InputMethodTips -contains $t)) {{ $si.InputMethodTips.Add($t) }}; \
             Set-WinUserLanguageList $l -Force"
        ))
    }

    /// Takes Liyamu out of the user's keyboards (and Sinhala, if Liyamu was its only keyboard).
    fn remove_keyboard() -> bool {
        powershell(&format!(
            "$ErrorActionPreference='Stop'; $t='{TIP}'; $l=Get-WinUserLanguageList; \
             foreach ($x in @($l)) {{ if ($x.InputMethodTips -contains $t) {{ [void]$x.InputMethodTips.Remove($t); \
             if ($x.InputMethodTips.Count -eq 0) {{ [void]$l.Remove($x) }} }} }}; \
             if ($l.Count -gt 0) {{ Set-WinUserLanguageList $l -Force }}"
        ))
    }

    pub fn main() {
        match std::env::args().nth(1).as_deref() {
            Some("--add-keyboard") => std::process::exit(if add_keyboard() { 0 } else { 1 }),
            Some("--remove-keyboard") => std::process::exit(if remove_keyboard() { 0 } else { 1 }),
            _ => {}
        }
        unsafe {
            // one window: a second start brings the first one forward
            if let Ok(h) = FindWindowW(CLASS, None) {
                if !h.0.is_null() {
                    let _ = ShowWindow(h, SW_RESTORE);
                    let _ = SetForegroundWindow(h);
                    return;
                }
            }
            let _ = InitCommonControlsEx(&INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                dwICC: ICC_TAB_CLASSES | ICC_STANDARD_CLASSES,
            });
            let hinst = GetModuleHandleW(None).unwrap_or_default();
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(wndproc),
                hInstance: hinst.into(),
                hIcon: LoadIconW(hinst, PCWSTR(1 as *const u16)).unwrap_or_default(),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                hbrBackground: GetSysColorBrush(COLOR_WINDOW),
                lpszClassName: CLASS,
                ..Default::default()
            };
            RegisterClassExW(&wc);
            RegisterClassExW(&WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(text_proc),
                hInstance: hinst.into(),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                lpszClassName: TEXT_CLASS,
                ..Default::default()
            });
            let hwnd = CreateWindowExW(WINDOW_EX_STYLE(0), CLASS, w!("Liyamu ලියමු"),
                                       WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
                                       CW_USEDEFAULT, CW_USEDEFAULT, 100, 100, None, None, hinst, None)
                .unwrap();
            build(hwnd);
            let start = if std::env::args().any(|a| a == "--help") { 3 } else { 0 };
            select_tab(start);
            let _ = ShowWindow(hwnd, SW_SHOW);
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                if !IsDialogMessageW(hwnd, &msg).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    }

    unsafe fn make_font(dpi: i32, bold: bool) -> HFONT {
        let mut ncm = NONCLIENTMETRICSW { cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32, ..Default::default() };
        let _ = SystemParametersInfoW(SPI_GETNONCLIENTMETRICS, ncm.cbSize, Some(&mut ncm as *mut _ as *mut _),
                                      SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
        let mut lf = ncm.lfMessageFont;
        lf.lfHeight = -(10 * dpi / 72);
        if bold {
            lf.lfWeight = FW_SEMIBOLD.0 as i32;
        }
        CreateFontIndirectW(&lf)
    }

    /// Height of `text` wrapped to `width` in `font`.
    unsafe fn text_height(hwnd: HWND, font: HFONT, text: &str, width: i32) -> i32 {
        let dc = GetDC(hwnd);
        let old = SelectObject(dc, font);
        let mut w: Vec<u16> = text.encode_utf16().collect();
        let mut rc = RECT { left: 0, top: 0, right: width, bottom: 0 };
        DrawTextW(dc, &mut w, &mut rc, DT_CALCRECT | DT_WORDBREAK | DT_NOPREFIX);
        SelectObject(dc, old);
        ReleaseDC(hwnd, dc);
        rc.bottom
    }

    unsafe fn build(hwnd: HWND) {
        let dpi = GetDpiForWindow(hwnd).max(96) as i32;
        let s = |v: i32| v * dpi / 96;
        let font = make_font(dpi, false);
        let bold = make_font(dpi, true);
        let sinhala = sinhala_font(dpi);
        FONTS.set((font, sinhala));
        let hinst = GetModuleHandleW(None).unwrap_or_default();
        let (cw, ch) = (s(560), s(560));
        let margin = s(24);
        let width = cw - 2 * margin;
        let line = text_height(hwnd, font, "Ag", 1000);

        let tabs_hwnd = CreateWindowExW(WINDOW_EX_STYLE(0), WC_TABCONTROLW, w!(""), WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
                                        s(8), s(8), cw - s(16), ch - s(16), hwnd, HMENU(ID_TABS as isize as *mut _), hinst, None)
            .unwrap();
        SendMessageW(tabs_hwnd, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));

        let st = Settings::load();
        let mut controls = Vec::new();
        let mut notes = Vec::new();
        let child = |class: PCWSTR, text: &str, style: WINDOW_STYLE, x: i32, y: i32, w: i32, h: i32, id: i32, f: HFONT| {
            let t = wide(text);
            let c = CreateWindowExW(WINDOW_EX_STYLE(0), class, PCWSTR(t.as_ptr()), WS_CHILD | style, x, y, w, h, hwnd,
                                    HMENU(id as isize as *mut _), hinst, None)
                .unwrap();
            SendMessageW(c, WM_SETFONT, WPARAM(f.0 as usize), LPARAM(1));
            let _ = SetWindowPos(c, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
            c
        };
        for (i, (name, items)) in tabs().into_iter().enumerate() {
            let n = wide(name);
            let item = TCITEMW { mask: TCIF_TEXT, pszText: PWSTR(n.as_ptr() as *mut _), ..Default::default() };
            SendMessageW(tabs_hwnd, TCM_INSERTITEMW, WPARAM(i), LPARAM(&item as *const _ as isize));
            let mut y = s(52);
            for it in items {
                match it {
                    Heading(t) => {
                        y += s(6);
                        controls.push((i, child(w!("STATIC"), t, WINDOW_STYLE(0), margin, y, width, line, -1, bold)));
                        y += line + s(6);
                    }
                    Note(t) => {
                        let h = rich_height(hwnd, t, width - s(20));
                        let c = child(TEXT_CLASS, t, WINDOW_STYLE(0), margin + s(20), y, width - s(20), h, -1, font);
                        notes.push(c);
                        controls.push((i, c));
                        y += h + s(10);
                    }
                    Check(id, t) => {
                        // a label with Sinhala: the check box has the words before it, the mixed-text control the rest
                        let (head, tail) = match t.find(is_sinhala).and_then(|k| t[..k].rfind(' ')) {
                            Some(k) => (&t[..k], Some(&t[k + 1..])),
                            None => (t, None),
                        };
                        let cw = match tail {
                            Some(_) => {
                                let dc = GetDC(hwnd);
                                let old = SelectObject(dc, font);
                                let hw: Vec<u16> = head.encode_utf16().collect();
                                let mut sz = SIZE::default();
                                let _ = GetTextExtentPoint32W(dc, &hw, &mut sz);
                                SelectObject(dc, old);
                                ReleaseDC(hwnd, dc);
                                sz.cx + s(22)
                            }
                            None => width,
                        };
                        let c = child(w!("BUTTON"), head, WS_TABSTOP | WINDOW_STYLE(BS_AUTOCHECKBOX as u32), margin, y, cw, line + s(6), id, font);
                        SendMessageW(c, BM_SETCHECK, WPARAM(checked(id, &st) as usize), LPARAM(0));
                        controls.push((i, c));
                        if let Some(tail) = tail {
                            let th = rich_height(hwnd, tail, width - cw);
                            let l = child(TEXT_CLASS, tail, WINDOW_STYLE(0), margin + cw, y + (line + s(6) - th) / 2, width - cw, th, -1, font);
                            SetWindowLongPtrW(l, GWLP_USERDATA, c.0 as _);
                            controls.push((i, l));
                        }
                        y += line + s(8);
                    }
                    Combo(id, label, options) => {
                        controls.push((i, child(w!("STATIC"), label, WINDOW_STYLE(0), margin, y + s(4), s(90), line, -1, font)));
                        let c = child(w!("COMBOBOX"), "", WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
                                      margin + s(96), y, s(260), s(200), id, font);
                        for o in options {
                            let o = wide(o);
                            SendMessageW(c, CB_ADDSTRING, WPARAM(0), LPARAM(o.as_ptr() as isize));
                        }
                        let v = if id == ID_LAYOUT { st.layout } else { st.output };
                        SendMessageW(c, CB_SETCURSEL, WPARAM(v as usize), LPARAM(0));
                        controls.push((i, c));
                        y += line + s(18);
                    }
                    Button(id, t) => {
                        let bw = {
                            let dc = GetDC(hwnd);
                            let old = SelectObject(dc, font);
                            let tw: Vec<u16> = t.encode_utf16().collect();
                            let mut sz = SIZE::default();
                            let _ = GetTextExtentPoint32W(dc, &tw, &mut sz);
                            SelectObject(dc, old);
                            ReleaseDC(hwnd, dc);
                            sz.cx + s(24)
                        };
                        let x = if id == ID_FORGET { margin + s(20) } else { margin };
                        controls.push((i, child(w!("BUTTON"), t, WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32), x, y, bw, line + s(12), id, font)));
                        y += line + s(22);
                    }
                    Edit(id) => {
                        let h = ch - y - s(70);
                        let text = settings::shortcuts_file()
                            .and_then(|p| std::fs::read_to_string(p).ok())
                            .unwrap_or_default()
                            .replace("\r\n", "\n")
                            .replace('\n', "\r\n");
                        let c = CreateWindowExW(WS_EX_CLIENTEDGE, w!("EDIT"), PCWSTR(wide(&text).as_ptr()),
                                                WS_CHILD | WS_TABSTOP | WS_VSCROLL
                                                    | WINDOW_STYLE((ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN) as u32),
                                                margin, y, width, h, hwnd, HMENU(id as isize as *mut _), hinst, None)
                            .unwrap();
                        SendMessageW(c, WM_SETFONT, WPARAM(sinhala.0 as usize), LPARAM(1)); // what is typed there is Sinhala
                        controls.push((i, c));
                        y += h + s(10);
                    }
                }
            }
        }
        // only the row of tabs: the pages sit on the white window (the themed tab body is grey, the labels white)
        let mut tr = RECT::default();
        if SendMessageW(tabs_hwnd, TCM_GETITEMRECT, WPARAM(0), LPARAM(&mut tr as *mut _ as isize)).0 != 0 {
            let _ = SetWindowPos(tabs_hwnd, None, 0, 0, cw - s(16), tr.bottom + s(1), SWP_NOMOVE | SWP_NOZORDER);
        }
        // the window's size for this client area
        let mut rc = RECT { left: 0, top: 0, right: cw, bottom: ch };
        let _ = AdjustWindowRectEx(&mut rc, WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU, false, WINDOW_EX_STYLE(0));
        let _ = SetWindowPos(hwnd, None, 0, 0, rc.right - rc.left, rc.bottom - rc.top, SWP_NOMOVE | SWP_NOZORDER);
        UI.with(|u| {
            *u.borrow_mut() = Some(Ui { hwnd, font, bold, white: GetSysColorBrush(COLOR_WINDOW), controls, notes, tab: 0 });
        });
    }

    fn select_tab(tab: usize) {
        UI.with(|u| {
            let mut u = u.borrow_mut();
            let Some(ui) = u.as_mut() else { return };
            ui.tab = tab;
            unsafe {
                if let Ok(t) = GetDlgItem(ui.hwnd, ID_TABS) {
                    SendMessageW(t, TCM_SETCURSEL, WPARAM(tab), LPARAM(0));
                }
                for (i, c) in &ui.controls {
                    let _ = ShowWindow(*c, if *i == tab { SW_SHOW } else { SW_HIDE });
                }
            }
        });
    }

    fn save_shortcuts(hwnd: HWND) {
        unsafe {
            let Ok(edit) = GetDlgItem(hwnd, ID_SHORTCUTS) else { return };
            let n = GetWindowTextLengthW(edit) as usize;
            let mut buf = vec![0u16; n + 1];
            let got = GetWindowTextW(edit, &mut buf) as usize;
            let text = String::from_utf16_lossy(&buf[..got]).replace("\r\n", "\n");
            if let Some(p) = settings::shortcuts_file() {
                let old = std::fs::read_to_string(&p).unwrap_or_default();
                if old != text {
                    let _ = std::fs::write(&p, text);
                    settings::bump("Version");
                }
            }
        }
    }

    fn command(hwnd: HWND, id: i32, code: u32, ctl: HWND) {
        let mut st = Settings::load();
        unsafe {
            match id {
                ID_LAYOUT | ID_OUTPUT if code == CBN_SELCHANGE => {
                    let v = SendMessageW(ctl, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0.max(0) as u32;
                    if id == ID_LAYOUT { st.layout = v } else { st.output = v }
                    st.save();
                }
                ID_SUGGEST..=ID_TRADITIONAL if code == BN_CLICKED => {
                    let on = SendMessageW(ctl, BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == BST_CHECKED.0 as isize;
                    set_checked(id, &mut st, on);
                    st.save();
                }
                ID_FORGET if code == BN_CLICKED => {
                    if let Some(p) = settings::learned_file() {
                        let _ = std::fs::remove_file(p);
                    }
                    settings::bump("Forget");
                    settings::bump("Version");
                    let _ = SetWindowTextW(ctl, w!("Forgotten"));
                    SetTimer(hwnd, 1, 1500, None);
                }
                ID_SAVE if code == BN_CLICKED => save_shortcuts(hwnd),
                ID_SHORTCUTS if code == EN_KILLFOCUS => save_shortcuts(hwnd),
                ID_ADD if code == BN_CLICKED => {
                    let _ = SetWindowTextW(ctl, w!("Adding…"));
                    let ok = add_keyboard();
                    let _ = SetWindowTextW(ctl, if ok { w!("Added: Windows key + Space switches") } else { w!("Could not add it; see the steps above") });
                }
                ID_LINK if code == BN_CLICKED => {
                    ShellExecuteW(hwnd, w!("open"), w!("https://arutha.lk/liyamu"), None, None, SW_SHOWNORMAL);
                }
                _ => {}
            }
        }
    }

    unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_COMMAND => {
                command(hwnd, (wparam.0 & 0xFFFF) as i32, ((wparam.0 >> 16) & 0xFFFF) as u32, HWND(lparam.0 as *mut _));
                LRESULT(0)
            }
            WM_NOTIFY => {
                let h = &*(lparam.0 as *const NMHDR);
                if h.idFrom == ID_TABS as usize && h.code == TCN_SELCHANGE {
                    let i = SendMessageW(h.hwndFrom, TCM_GETCURSEL, WPARAM(0), LPARAM(0)).0.max(0) as usize;
                    select_tab(i);
                }
                LRESULT(0)
            }
            WM_TIMER => {
                let _ = KillTimer(hwnd, 1);
                if let Ok(b) = GetDlgItem(hwnd, ID_FORGET) {
                    let _ = SetWindowTextW(b, w!("Forget what was learned"));
                }
                LRESULT(0)
            }
            WM_CTLCOLORSTATIC => {
                // the tab page is white; the notes are grey
                let ctl = HWND(lparam.0 as *mut _);
                let dc = HDC(wparam.0 as *mut _);
                UI.with(|u| {
                    let u = u.borrow();
                    let Some(ui) = u.as_ref() else { return LRESULT(0) };
                    SetBkMode(dc, TRANSPARENT);
                    SetTextColor(dc, if ui.notes.contains(&ctl) { COLORREF(0x00606060) } else { COLORREF(GetSysColor(COLOR_WINDOWTEXT)) });
                    LRESULT(ui.white.0 as isize)
                })
            }
            WM_CLOSE => {
                save_shortcuts(hwnd);
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            WM_DESTROY => {
                UI.with(|u| {
                    if let Some(ui) = u.borrow_mut().take() {
                        let _ = DeleteObject(ui.font);
                        let _ = DeleteObject(ui.bold);
                    }
                });
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
