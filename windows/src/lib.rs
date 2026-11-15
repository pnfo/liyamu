//! Liyamu for Windows: a Text Services Framework (TSF) text service in a COM DLL, registered as a Sinhala
//! keyboard. The installer registers the 64-bit and the 32-bit DLL (regsvr32), for apps of either kind.
#![cfg(windows)]
#![allow(non_snake_case)]

mod candidates;
mod keys;
mod langbar;
pub mod settings;
mod tip;

use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicPtr, Ordering};
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Registry::*;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows::Win32::UI::Input::KeyboardAndMouse::HKL;
use windows::Win32::UI::TextServices::*;

/// The text service's COM class
pub const CLSID: GUID = GUID::from_u128(0x019c5475_f781_4efe_b363_e4d22b25f970);
/// Its keyboard profile under Sinhala
pub const PROFILE: GUID = GUID::from_u128(0x51abf515_2fd5_4b67_8fad_12c39216ddfe);
/// The underline of the word being typed
pub const ATTR_GUID: GUID = GUID::from_u128(0x1b037100_eaf4_4ec2_8921_02e5077c982a);
/// Sinhala (Sri Lanka)
pub const LANGID: u16 = 0x045B;

static HINSTANCE_: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

pub(crate) fn hinstance() -> HINSTANCE {
    HINSTANCE(HINSTANCE_.load(Ordering::Relaxed))
}

/// Debug log, on when HKCU\\Software\\Liyamu Debug = 1 (read once per app): %TEMP%\\liyamu-debug.log
pub(crate) fn debug_log(msg: std::fmt::Arguments) {
    use std::io::Write;
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !*ON.get_or_init(|| settings::get("Debug") == Some(1)) {
        return;
    }
    let path = std::env::temp_dir().join("liyamu-debug.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let exe = std::env::current_exe().ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())).unwrap_or_default();
        let _ = writeln!(f, "{} {}: {}", exe, std::process::id(), msg);
    }
}

macro_rules! dlog {
    ($($t:tt)*) => { $crate::debug_log(format_args!($($t)*)) };
}
pub(crate) use dlog;

/// Runs `f`, turning a panic into `fallback`: a bug in the keyboard must not close the app it is typing in.
pub(crate) fn guard<T>(fallback: T, f: impl FnOnce() -> T) -> T {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or(fallback)
}

pub(crate) fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn dll_path() -> String {
    let mut buf = [0u16; 1024];
    let n = unsafe { GetModuleFileNameW(HMODULE(hinstance().0), &mut buf) } as usize;
    String::from_utf16_lossy(&buf[..n])
}

/// The folder above x64\ and x86\, which has the settings program and fonts\.
pub(crate) fn install_dir() -> Option<PathBuf> {
    Some(PathBuf::from(dll_path()).parent()?.parent()?.to_path_buf())
}

#[no_mangle]
extern "system" fn DllMain(hinst: HINSTANCE, reason: u32, _: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        HINSTANCE_.store(hinst.0, Ordering::Relaxed);
        std::panic::set_hook(Box::new(|info| dlog!("PANIC {}", info)));
    }
    TRUE
}

#[implement(IClassFactory)]
struct Factory;

impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(&self, outer: Option<&IUnknown>, riid: *const GUID, ppv: *mut *mut c_void) -> Result<()> {
        unsafe { *ppv = std::ptr::null_mut() };
        if outer.is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        let unk: IUnknown = tip::Tip::new().into();
        unsafe { unk.query(riid, ppv).ok() }
    }
    fn LockServer(&self, _: BOOL) -> Result<()> {
        Ok(())
    }
}

#[no_mangle]
unsafe extern "system" fn DllGetClassObject(rclsid: *const GUID, riid: *const GUID, ppv: *mut *mut c_void) -> HRESULT {
    *ppv = std::ptr::null_mut();
    if *rclsid != CLSID {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    let f: IUnknown = Factory.into();
    f.query(riid, ppv)
}

/// Text services are loaded into every app with a text field; staying loaded is simplest and safe.
#[no_mangle]
extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_FALSE
}

const CATEGORIES: [GUID; 4] = [
    GUID_TFCAT_TIP_KEYBOARD,
    GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT, // Store apps and the Start menu search
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,   // the button in the taskbar
    GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
];

fn clsid_key() -> String {
    format!("CLSID\\{{{:?}}}", CLSID)
}

fn set_string(key: HKEY, name: Option<&str>, value: &str) -> Result<()> {
    let v = wide(value);
    let bytes = unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 2) };
    let n = name.map(wide);
    let np = n.as_ref().map_or(PCWSTR::null(), |n| PCWSTR(n.as_ptr()));
    unsafe { RegSetValueExW(key, np, 0, REG_SZ, Some(bytes)).ok() }
}

fn create_key(parent: HKEY, path: &str) -> Result<HKEY> {
    let mut key = HKEY::default();
    let p = wide(path);
    unsafe {
        RegCreateKeyW(parent, PCWSTR(p.as_ptr()), &mut key).ok()?;
    }
    Ok(key)
}

fn register() -> Result<()> {
    let path = dll_path();
    unsafe {
        let key = create_key(HKEY_CLASSES_ROOT, &clsid_key())?;
        set_string(key, None, "Liyamu")?;
        let inproc = create_key(key, "InprocServer32")?;
        set_string(inproc, None, &path)?;
        set_string(inproc, Some("ThreadingModel"), "Apartment")?;
        let _ = RegCloseKey(inproc);
        let _ = RegCloseKey(key);

        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let profiles: ITfInputProcessorProfileMgr =
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
        let desc: Vec<u16> = "Liyamu".encode_utf16().collect();
        let icon: Vec<u16> = path.encode_utf16().collect();
        profiles.RegisterProfile(&CLSID, LANGID, &PROFILE, &desc, &icon, 0, HKL::default(), 0, TRUE, 0)?;
        let cats: ITfCategoryMgr = CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;
        for c in CATEGORIES {
            cats.RegisterCategory(&CLSID, &c, &CLSID)?;
        }
    }
    Ok(())
}

fn unregister() -> Result<()> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if let Ok(cats) = CoCreateInstance::<_, ITfCategoryMgr>(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER) {
            for c in CATEGORIES {
                let _ = cats.UnregisterCategory(&CLSID, &c, &CLSID);
            }
        }
        if let Ok(p) = CoCreateInstance::<_, ITfInputProcessorProfileMgr>(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER) {
            let _ = p.UnregisterProfile(&CLSID, LANGID, &PROFILE, 0);
        }
        let k = wide(&clsid_key());
        let _ = RegDeleteTreeW(HKEY_CLASSES_ROOT, PCWSTR(k.as_ptr()));
    }
    Ok(())
}

#[no_mangle]
extern "system" fn DllRegisterServer() -> HRESULT {
    match register() {
        Ok(()) => S_OK,
        Err(e) => {
            let _ = unregister();
            e.code()
        }
    }
}

#[no_mangle]
extern "system" fn DllUnregisterServer() -> HRESULT {
    match unregister() {
        Ok(()) => S_OK,
        Err(e) => e.code(),
    }
}
