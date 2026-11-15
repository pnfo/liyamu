//! The keyboard's button in the taskbar (next to the language indicator): its menu chooses the layout and
//! the output, turns suggestions on or off, and opens the settings program.

use crate::settings::{Settings, LAYOUTS, OUTPUTS};
use crate::{candidates, wide, CLSID};
use std::cell::RefCell;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::TextServices::*;
use windows::Win32::UI::WindowsAndMessaging::*;

const SUGGESTIONS: u32 = 20;
const SETTINGS: u32 = 30;

#[implement(ITfLangBarItemButton, ITfSource)]
pub struct Button {
    sink: RefCell<Option<ITfLangBarItemSink>>,
}

impl Button {
    pub fn new() -> Button {
        Button { sink: RefCell::new(None) }
    }
}

/// (id, text, checked) for the menu; id 0 is a separator
fn items() -> Vec<(u32, String, bool)> {
    let s = Settings::load();
    let mut v = Vec::new();
    for (i, name) in LAYOUTS.iter().enumerate() {
        v.push((1 + i as u32, name.to_string(), s.layout == i as u32));
    }
    v.push((0, String::new(), false));
    for (i, name) in OUTPUTS.iter().enumerate() {
        v.push((11 + i as u32, name.to_string(), s.output == i as u32));
    }
    v.push((0, String::new(), false));
    v.push((SUGGESTIONS, "Word suggestions".into(), s.suggest));
    v.push((0, String::new(), false));
    v.push((SETTINGS, "Settings and Help…".into(), false));
    v
}

fn choose(id: u32) {
    let mut s = Settings::load();
    match id {
        1..=3 => s.layout = id - 1,
        11..=13 => s.output = id - 11,
        SUGGESTIONS => s.suggest = !s.suggest,
        SETTINGS => {
            open_settings();
            return;
        }
        _ => return,
    }
    s.save();
}

pub fn open_settings() {
    if let Some(dir) = crate::install_dir() {
        let exe = wide(&dir.join("liyamu-settings.exe").to_string_lossy());
        unsafe { ShellExecuteW(None, w!("open"), PCWSTR(exe.as_ptr()), None, None, SW_SHOWNORMAL) };
    }
}

fn tooltip() -> String {
    let s = Settings::load();
    format!("Liyamu: {}, {}", LAYOUTS[s.layout as usize], OUTPUTS[s.output as usize])
}

/// Asks the taskbar to show the current state (after the settings changed).
pub fn update(button: &ComObject<Button>) {
    if let Some(sink) = button.sink.borrow().clone() {
        unsafe {
            let _ = sink.OnUpdate(TF_LBI_ICON | TF_LBI_TEXT | TF_LBI_TOOLTIP);
        }
    }
}

impl ITfLangBarItem_Impl for Button_Impl {
    fn GetInfo(&self, info: *mut TF_LANGBARITEMINFO) -> Result<()> {
        let mut desc = [0u16; 32];
        for (d, c) in desc.iter_mut().zip("Liyamu".encode_utf16()) {
            *d = c;
        }
        unsafe {
            *info = TF_LANGBARITEMINFO {
                clsidService: CLSID,
                guidItem: GUID_LBI_INPUTMODE,
                dwStyle: TF_LBI_STYLE_BTN_BUTTON,
                ulSort: 0,
                szDescription: desc,
            };
        }
        Ok(())
    }
    fn GetStatus(&self) -> Result<u32> {
        Ok(0)
    }
    fn Show(&self, _: BOOL) -> Result<()> {
        Ok(())
    }
    fn GetTooltipString(&self) -> Result<BSTR> {
        Ok(BSTR::from(tooltip()))
    }
}

impl ITfLangBarItemButton_Impl for Button_Impl {
    /// A click on the button: our own menu at the pointer.
    fn OnClick(&self, _click: TfLBIClick, pt: &POINT, _area: *const RECT) -> Result<()> {
        unsafe {
            let menu = CreatePopupMenu()?;
            for (id, text, on) in items() {
                if id == 0 {
                    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
                } else {
                    let t = wide(&text);
                    let flags = MF_STRING | if on { MF_CHECKED } else { MF_UNCHECKED };
                    let _ = AppendMenuW(menu, flags, id as usize, PCWSTR(t.as_ptr()));
                }
            }
            let owner = candidates::hwnd();
            let id = TrackPopupMenuEx(menu, (TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN).0,
                                      pt.x, pt.y, owner, None);
            let _ = DestroyMenu(menu);
            if id.0 > 0 {
                choose(id.0 as u32);
            }
        }
        Ok(())
    }
    /// The menu Windows draws itself (right click, or the floating language bar).
    fn InitMenu(&self, menu: Option<&ITfMenu>) -> Result<()> {
        let Some(menu) = menu else { return Ok(()) };
        for (id, text, on) in items() {
            let flags = if id == 0 { TF_LBMENUF_SEPARATOR } else if on { TF_LBMENUF_CHECKED } else { 0 };
            let t: Vec<u16> = text.encode_utf16().collect();
            unsafe { menu.AddMenuItem(id, flags, None, None, &t, std::ptr::null_mut())? };
        }
        Ok(())
    }
    fn OnMenuSelect(&self, id: u32) -> Result<()> {
        choose(id);
        Ok(())
    }
    fn GetIcon(&self) -> Result<HICON> {
        unsafe {
            let size = GetSystemMetrics(SM_CXSMICON);
            let h = LoadImageW(crate::hinstance(), PCWSTR(1 as *const u16), IMAGE_ICON, size, size, LR_DEFAULTCOLOR)?;
            Ok(HICON(h.0))
        }
    }
    fn GetText(&self) -> Result<BSTR> {
        Ok(BSTR::from("Liyamu"))
    }
}

impl ITfSource_Impl for Button_Impl {
    fn AdviseSink(&self, riid: *const GUID, punk: Option<&IUnknown>) -> Result<u32> {
        if unsafe { *riid } != ITfLangBarItemSink::IID {
            return Err(HRESULT(0x80040202u32 as i32).into()) // CONNECT_E_CANNOTCONNECT;
        }
        let sink: ITfLangBarItemSink = punk.ok_or(Error::from(E_INVALIDARG))?.cast()?;
        *self.sink.borrow_mut() = Some(sink);
        Ok(1)
    }
    fn UnadviseSink(&self, cookie: u32) -> Result<()> {
        if cookie != 1 {
            return Err(HRESULT(0x80040200u32 as i32).into()) // CONNECT_E_NOCONNECTION;
        }
        *self.sink.borrow_mut() = None;
        Ok(())
    }
}
