//! The suggestion bar under the word being typed: a popup that never takes the focus, one per UI thread.
//! Words are drawn in Abhaya (installed with the keyboard, loaded privately) or else Nirmala UI.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Once;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::*;

const CLASS: PCWSTR = w!("LiyamuSuggestions");

type Pick = Rc<dyn Fn(usize)>;

#[derive(Default)]
struct Bar {
    hwnd: HWND,
    items: Vec<String>,
    selected: usize,
    /// where each item was drawn, for clicks
    hits: Vec<RECT>,
    on_pick: Option<Pick>,
}

thread_local! {
    static BAR: RefCell<Bar> = RefCell::new(Bar::default());
}

fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF(r as u32 | (g as u32) << 8 | (b as u32) << 16)
}

// sepia, like the Android keyboard by day; the highlighted word in the logo's maroon and gold
fn background() -> COLORREF { rgb(0xFB, 0xF5, 0xE9) }
fn border() -> COLORREF { rgb(0xD9, 0xC9, 0xA8) }
fn ink() -> COLORREF { rgb(0x3A, 0x2A, 0x1E) }
fn number() -> COLORREF { rgb(0x9A, 0x84, 0x68) }
fn maroon() -> COLORREF { rgb(0x6E, 0x1C, 0x22) }
fn gold() -> COLORREF { rgb(0xFB, 0xE7, 0xB5) }

/// Loads the Abhaya font that the installer puts next to the DLL (fonts\AbhayaUni-Regular.ttf), for this
/// process only.
fn load_font_file() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        if let Some(dir) = crate::install_dir() {
            let path = dir.join("fonts").join("AbhayaUni-Regular.ttf");
            let wide = crate::wide(&path.to_string_lossy());
            unsafe { AddFontResourceExW(PCWSTR(wide.as_ptr()), FR_PRIVATE, None) };
        }
    });
}

fn font(px: i32, small: bool) -> HFONT {
    load_font_file();
    let face = if small { w!("Segoe UI") } else { w!("Abhaya Uni") };
    let height = if small { -(px * 13 / 20) } else { -px };
    unsafe {
        let f = CreateFontW(height, 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0, DEFAULT_CHARSET.0 as u32, OUT_TT_PRECIS.0 as u32,
                            CLIP_DEFAULT_PRECIS.0 as u32, CLEARTYPE_QUALITY.0 as u32, (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32, face);
        if small || has_face(f, "Abhaya Uni") {
            f
        } else {
            let _ = DeleteObject(f);
            CreateFontW(-px * 9 / 10, 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0, DEFAULT_CHARSET.0 as u32, OUT_TT_PRECIS.0 as u32,
                        CLIP_DEFAULT_PRECIS.0 as u32, CLEARTYPE_QUALITY.0 as u32, (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                        w!("Nirmala UI"))
        }
    }
}

/// Windows quietly substitutes a font it does not have; this tells whether it did.
fn has_face(f: HFONT, name: &str) -> bool {
    unsafe {
        let dc = GetDC(None);
        let old = SelectObject(dc, f);
        let mut buf = [0u16; 64];
        let n = GetTextFaceW(dc, Some(&mut buf));
        SelectObject(dc, old);
        ReleaseDC(None, dc);
        n > 0 && String::from_utf16_lossy(&buf[..(n as usize).saturating_sub(1)]).eq_ignore_ascii_case(name)
    }
}

fn register() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_DROPSHADOW,
            lpfnWndProc: Some(wndproc),
            hInstance: crate::hinstance(),
            hCursor: LoadCursorW(None, IDC_HAND).unwrap_or_default(),
            lpszClassName: CLASS,
            ..Default::default()
        };
        RegisterClassExW(&wc);
    });
}

/// The bar's window (created on first use); also the owner of the keyboard's popup menu.
/// Window calls send messages right away, which can come back here: the bar's state is never borrowed
/// across one.
pub fn hwnd() -> HWND {
    let h = BAR.with(|b| b.borrow().hwnd);
    if !h.0.is_null() && unsafe { IsWindow(h).as_bool() } {
        return h;
    }
    register();
    let h = unsafe {
        CreateWindowExW(WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW, CLASS, w!(""), WS_POPUP,
                        0, 0, 1, 1, None, None, crate::hinstance(), None)
    }
    .unwrap_or_default();
    BAR.with(|b| b.borrow_mut().hwnd = h);
    h
}

struct Metrics {
    px: i32,
    pad: i32,
    gap: i32,
}

fn metrics(hwnd: HWND) -> Metrics {
    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96) as i32;
    let s = |v: i32| v * dpi / 96;
    Metrics { px: s(22), pad: s(7), gap: s(10) }
}

/// Lays the items out from x = 0 and returns each item's rectangle and the bar's size.
fn layout(dc: HDC, m: &Metrics, items: &[String]) -> (Vec<RECT>, i32, i32) {
    let big = font(m.px, false);
    let small = font(m.px, true);
    let mut rects = Vec::new();
    let mut x = m.pad / 2;
    let h = m.px * 3 / 2 + m.pad;
    unsafe {
        for (i, w) in items.iter().enumerate() {
            let mut size = SIZE::default();
            SelectObject(dc, small);
            let n: Vec<u16> = format!("{}", i + 1).encode_utf16().collect();
            let _ = GetTextExtentPoint32W(dc, &n, &mut size);
            let nw = size.cx;
            SelectObject(dc, big);
            let t: Vec<u16> = w.encode_utf16().collect();
            let _ = GetTextExtentPoint32W(dc, &t, &mut size);
            let width = m.pad + nw + m.pad / 2 + size.cx + m.pad;
            rects.push(RECT { left: x, top: m.pad / 2, right: x + width, bottom: h - m.pad / 2 });
            x += width + m.gap / 2;
        }
        let _ = DeleteObject(big);
        let _ = DeleteObject(small);
    }
    (rects, x + m.pad / 2 - m.gap / 2, h)
}

/// Shows `items` under `anchor` (the word's rectangle on screen). `on_pick` is called for a click.
pub fn show(items: Vec<String>, selected: usize, anchor: RECT, on_pick: Pick) {
    let hwnd = hwnd();
    if hwnd.0.is_null() {
        return;
    }
    let m = metrics(hwnd);
    let (hits, w, h) = unsafe {
        let dc = GetDC(hwnd);
        let r = layout(dc, &m, &items);
        ReleaseDC(hwnd, dc);
        r
    };
    let old = BAR.with(|b| {
        let mut b = b.borrow_mut();
        b.items = items;
        b.selected = selected;
        b.hits = hits;
        b.on_pick.replace(on_pick)
    });
    drop(old);
    unsafe {
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let mon = MonitorFromRect(&anchor, MONITOR_DEFAULTTONEAREST);
        let _ = GetMonitorInfoW(mon, &mut info);
        let work = info.rcWork;
        let mut x = anchor.left;
        let mut y = anchor.bottom + m.pad / 2;
        if y + h > work.bottom {
            y = anchor.top - h - m.pad / 2;
        }
        x = x.min(work.right - w).max(work.left);
        let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
        let _ = InvalidateRect(hwnd, None, true);
    }
}

pub fn hide() {
    let (hwnd, pick) = BAR.with(|b| {
        let mut b = b.borrow_mut();
        b.items.clear();
        (b.hwnd, b.on_pick.take())
    });
    drop(pick);
    if !hwnd.0.is_null() {
        unsafe { let _ = ShowWindow(hwnd, SW_HIDE); }
    }
}

fn paint(hwnd: HWND) {
    unsafe {
        let mut ps = PAINTSTRUCT::default();
        let dc = BeginPaint(hwnd, &mut ps);
        let mut rc = RECT::default();
        let _ = GetClientRect(hwnd, &mut rc);
        // drawn off screen first, so the bar does not flicker while typing
        let mem = CreateCompatibleDC(dc);
        let bmp = CreateCompatibleBitmap(dc, rc.right, rc.bottom);
        let old_bmp = SelectObject(mem, bmp);

        let bg = CreateSolidBrush(background());
        FillRect(mem, &rc, bg);
        let _ = DeleteObject(bg);
        let edge = CreateSolidBrush(border());
        FrameRect(mem, &rc, edge);
        let _ = DeleteObject(edge);

        let m = metrics(hwnd);
        let (items, selected, hits) = BAR.with(|b| {
            let b = b.borrow();
            (b.items.clone(), b.selected, b.hits.clone())
        });
        let big = font(m.px, false);
        let small = font(m.px, true);
        SetBkMode(mem, TRANSPARENT);
        for (i, (word, r)) in items.iter().zip(hits.iter()).enumerate() {
            let on = i == selected;
            if on {
                let b = CreateSolidBrush(maroon());
                let pen = CreatePen(PS_SOLID, 1, maroon());
                let ob = SelectObject(mem, b);
                let op = SelectObject(mem, pen);
                let _ = RoundRect(mem, r.left, r.top, r.right, r.bottom, m.pad * 2, m.pad * 2);
                SelectObject(mem, ob);
                SelectObject(mem, op);
                let _ = DeleteObject(b);
                let _ = DeleteObject(pen);
            }
            let mut tr = RECT { left: r.left + m.pad, ..*r };
            SelectObject(mem, small);
            SetTextColor(mem, if on { gold() } else { number() });
            let mut n: Vec<u16> = format!("{}", i + 1).encode_utf16().collect();
            DrawTextW(mem, &mut n, &mut tr, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
            let mut size = SIZE::default();
            let _ = GetTextExtentPoint32W(mem, &n, &mut size);
            tr.left += size.cx + m.pad / 2;
            SelectObject(mem, big);
            SetTextColor(mem, if on { gold() } else { ink() });
            let mut t: Vec<u16> = word.encode_utf16().collect();
            DrawTextW(mem, &mut t, &mut tr, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        }
        let _ = DeleteObject(big);
        let _ = DeleteObject(small);

        let _ = BitBlt(dc, 0, 0, rc.right, rc.bottom, mem, 0, 0, SRCCOPY);
        SelectObject(mem, old_bmp);
        let _ = DeleteObject(bmp);
        let _ = DeleteDC(mem);
        let _ = EndPaint(hwnd, &ps);
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    crate::guard(LRESULT(0), || wndproc_(hwnd, msg, wparam, lparam))
}

unsafe fn wndproc_(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_LBUTTONUP => {
            let x = (lparam.0 & 0xFFFF) as i16 as i32;
            let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
            let hit = BAR.with(|b| {
                let b = b.borrow();
                let i = b.hits.iter().position(|r| x >= r.left && x < r.right && y >= r.top && y < r.bottom)?;
                Some((i, b.on_pick.clone()?))
            });
            if let Some((i, pick)) = hit {
                pick(i);
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
