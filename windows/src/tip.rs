//! The text service: one per UI thread of each app. It sends each key to an engine session and shows the
//! response: `commit` is inserted, `preedit` is the underlined composition, suggestions go in the bar.

use crate::settings::{self, Settings};
use crate::{candidates, dlog, keys, langbar, ATTR_GUID};
use liyamu_core::{Key, Mods, Options, Response, Session};
use std::cell::RefCell;
use std::mem::ManuallyDrop;
use std::rc::Rc;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::TextServices::*;
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetGUIThreadInfo, GUITHREADINFO};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows_core::IUnknownImpl;

struct State {
    thread_mgr: Option<ITfThreadMgr>,
    tid: u32,
    thread_mgr_cookie: u32,
    session: Session,
    composition: Option<ITfComposition>,
    /// the text field the last key went to, for clicks on a suggestion
    context: Option<ITfContext>,
    /// display attribute (underline) for the composition
    attr: u32,
    /// registry Version last read; None: not yet
    version: Option<u32>,
    forget: Option<u32>,
    langbar: Option<ComObject<langbar::Button>>,
    /// the text field watched for the caret leaving the word (ITfTextEditSink), and its cookie
    watched: Option<(ITfSource, u32)>,
}

#[implement(ITfTextInputProcessorEx, ITfThreadMgrEventSink, ITfKeyEventSink, ITfCompositionSink, ITfTextEditSink, ITfDisplayAttributeProvider)]
pub struct Tip {
    st: RefCell<State>,
}

impl Tip {
    pub fn new() -> Tip {
        Tip {
            st: RefCell::new(State {
                thread_mgr: None,
                tid: 0,
                thread_mgr_cookie: TF_INVALID_COOKIE,
                session: Session::new(Options::default()),
                composition: None,
                context: None,
                attr: 0,
                version: None,
                forget: None,
                langbar: None,
                watched: None,
            }),
        }
    }

    /// Picks up changes from the settings program or the menu. Returns the response of committing the
    /// word being typed when the options changed.
    fn sync(&self) -> Option<Response> {
        let v = settings::version();
        let mut st = self.st.borrow_mut();
        if st.version == Some(v) {
            return None;
        }
        st.version = Some(v);
        let s = Settings::load();
        let shortcuts = settings::shortcuts_file().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
        liyamu_core::lexicon::set_user_shortcuts(&shortcuts);
        let learned = settings::learned_file().map(|p| p.to_string_lossy().into_owned());
        liyamu_core::learn::configure(learned.as_deref(), s.learn && learned.is_some());
        let forget = settings::get("Forget").unwrap_or(0);
        if st.forget.is_some_and(|f| f != forget) {
            liyamu_core::learn::forget();
        }
        st.forget = Some(forget);
        let r = st.session.set_options(s.options());
        let button = st.langbar.clone();
        drop(st);
        if let Some(item) = &button {
            langbar::update(item);
        }
        Some(r)
    }

    fn wants(&self, key: Key, mods: Mods) -> bool {
        let st = self.st.borrow();
        // while a word is being typed every key goes through the engine, which commits the word first
        st.session.handles(key, mods) || st.session.is_composing()
    }
}

/// Runs `f` with a write lock on the text field.
#[implement(ITfEditSession)]
struct Edit {
    f: RefCell<Option<Box<dyn FnOnce(u32) -> Result<()>>>>,
}

impl ITfEditSession_Impl for Edit_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        crate::guard(Err(E_UNEXPECTED.into()), || {
        match self.f.borrow_mut().take() {
            Some(f) => f(ec),
            None => Ok(()),
        }
    })
    }
}

/// Runs `f` in an edit session on `ctx`: now (inside a key event) or when the app allows.
fn edit(tip: &ComObject<Tip>, ctx: &ITfContext, sync: bool, f: impl FnOnce(u32) -> Result<()> + 'static) -> Result<()> {
    let tid = tip.st.borrow().tid;
    let es: ITfEditSession = Edit { f: RefCell::new(Some(Box::new(f))) }.into();
    let flags = if sync { TF_ES_SYNC | TF_ES_READWRITE } else { TF_ES_ASYNCDONTCARE | TF_ES_READWRITE };
    let r = unsafe { ctx.RequestEditSession(tid, &es, flags) };
    dlog!("edit session sync={} -> {:?}", sync, r);
    r?.ok()
}

fn request(tip: &ComObject<Tip>, ctx: &ITfContext, responses: Vec<Response>, sync: bool) -> Result<()> {
    let t = tip.clone();
    let c = ctx.clone();
    edit(tip, ctx, sync, move |ec| {
        for r in &responses {
            let res = apply(&t, ec, &c, r);
            dlog!("apply commit={:?} preedit={:?} handled={} -> {:?}", r.commit, r.preedit, r.handled, res);
            res?;
        }
        Ok(())
    })
}

fn set_caret(ec: u32, ctx: &ITfContext, range: &ITfRange) -> Result<()> {
    unsafe {
        let end = range.Clone()?;
        end.Collapse(ec, TF_ANCHOR_END)?;
        let [sel] = [TF_SELECTION {
            range: ManuallyDrop::new(Some(end)),
            style: TF_SELECTIONSTYLE { ase: TF_AE_NONE, fInterimChar: FALSE },
        }];
        let r = ctx.SetSelection(ec, std::slice::from_ref(&sel));
        drop(ManuallyDrop::into_inner(sel.range));
        r
    }
}

fn underline(tip: &Tip, ec: u32, ctx: &ITfContext, range: &ITfRange, on: bool) {
    let atom = tip.st.borrow().attr;
    unsafe {
        if let Ok(prop) = ctx.GetProperty(&GUID_PROP_ATTRIBUTE) {
            let _ = if on && atom != 0 { prop.SetValue(ec, range, &windows_core::VARIANT::from(atom as i32)) } else { prop.Clear(ec, range) };
        }
    }
}

/// Shows an engine response in the text field (inside an edit session).
fn apply(tip: &ComObject<Tip>, ec: u32, ctx: &ITfContext, r: &Response) -> Result<()> {
    unsafe {
        if !r.commit.is_empty() {
            let text = wide_no_nul(&r.commit);
            let comp = tip.st.borrow_mut().composition.take();
            match comp {
                Some(c) => {
                    let range = c.GetRange()?;
                    range.SetText(ec, 0, &text)?;
                    underline(tip, ec, ctx, &range, false);
                    set_caret(ec, ctx, &range)?;
                    c.EndComposition(ec)?;
                }
                None => {
                    let ins: ITfInsertAtSelection = ctx.cast()?;
                    let range = ins.InsertTextAtSelection(ec, INSERT_TEXT_AT_SELECTION_FLAGS(0), &text)?;
                    set_caret(ec, ctx, &range)?;
                }
            }
        }
        let comp = tip.st.borrow().composition.clone();
        if !r.preedit.is_empty() {
            let comp = match comp {
                Some(c) => c,
                None => {
                    let ins: ITfInsertAtSelection = ctx.cast()?;
                    let range = ins.InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])?;
                    let cc: ITfContextComposition = ctx.cast()?;
                    let sink: ITfCompositionSink = tip.to_interface();
                    let c = cc.StartComposition(ec, &range, &sink)?;
                    tip.st.borrow_mut().composition = Some(c.clone());
                    c
                }
            };
            let range = comp.GetRange()?;
            range.SetText(ec, TF_ST_CORRECTION, &wide_no_nul(&r.preedit))?;
            underline(tip, ec, ctx, &range, true);
            set_caret(ec, ctx, &range)?;
        } else if let Some(c) = comp {
            // the last letter was deleted
            let range = c.GetRange()?;
            range.SetText(ec, 0, &[])?;
            c.EndComposition(ec)?;
            tip.st.borrow_mut().composition = None;
        }
        show_candidates(tip, ec, ctx, r);
    }
    Ok(())
}

fn wide_no_nul(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn show_candidates(tip: &ComObject<Tip>, ec: u32, ctx: &ITfContext, r: &Response) {
    let comp = tip.st.borrow().composition.clone();
    let Some(comp) = comp.filter(|_| r.candidates.len() > 1) else {
        candidates::hide();
        return;
    };
    let mut rc = RECT::default();
    unsafe {
        let mut clipped = FALSE;
        if let (Ok(view), Ok(range)) = (ctx.GetActiveView(), comp.GetRange()) {
            let _ = view.GetTextExt(ec, &range, &mut rc, &mut clipped);
        }
        if rc.bottom <= rc.top && rc.right <= rc.left && rc.left == 0 && rc.top == 0 {
            rc = caret_rect();
        }
    }
    let t = tip.clone();
    candidates::show(r.candidates.clone(), r.selected, rc, Rc::new(move |i| pick(&t, i)));
}

/// Where the app's caret is, for apps that do not say where the text is; else under the mouse pointer.
fn caret_rect() -> RECT {
    unsafe {
        let mut gti = GUITHREADINFO { cbSize: std::mem::size_of::<GUITHREADINFO>() as u32, ..Default::default() };
        if GetGUIThreadInfo(0, &mut gti).is_ok() && !gti.hwndCaret.0.is_null() {
            let mut tl = POINT { x: gti.rcCaret.left, y: gti.rcCaret.top };
            let mut br = POINT { x: gti.rcCaret.right, y: gti.rcCaret.bottom };
            let _ = ClientToScreen(gti.hwndCaret, &mut tl);
            let _ = ClientToScreen(gti.hwndCaret, &mut br);
            return RECT { left: tl.x, top: tl.y, right: br.x, bottom: br.y };
        }
        let mut p = POINT::default();
        let _ = GetCursorPos(&mut p);
        RECT { left: p.x, top: p.y, right: p.x + 1, bottom: p.y + 20 }
    }
}

/// A suggestion was clicked.
fn pick(tip: &ComObject<Tip>, i: usize) {
    let ctx = tip.st.borrow().context.clone();
    let Some(ctx) = ctx else { return };
    let r = tip.st.borrow_mut().session.pick(i);
    let _ = request(tip, &ctx, vec![r], false);
}

/// Watches the top text field of `dim` for edits (the caret leaving the word); stops watching the last one.
fn watch(tip: &ComObject<Tip>, dim: Option<&ITfDocumentMgr>) {
    let old = tip.st.borrow_mut().watched.take();
    if let Some((src, cookie)) = old {
        unsafe { let _ = src.UnadviseSink(cookie); }
    }
    let Some(dim) = dim else { return };
    unsafe {
        let Ok(ctx) = dim.GetTop() else { return };
        let Ok(src) = ctx.cast::<ITfSource>() else { return };
        let sink: ITfTextEditSink = tip.to_interface();
        if let Ok(cookie) = src.AdviseSink(&ITfTextEditSink::IID, &sink) {
            tip.st.borrow_mut().watched = Some((src, cookie));
        }
    }
}

/// Ends the composition where it is, keeping the word as shown (the caret went elsewhere).
fn finish_in_place(tip: &ComObject<Tip>, ec: u32, ctx: &ITfContext) {
    let comp = tip.st.borrow_mut().composition.take();
    if let Some(c) = comp {
        unsafe {
            if let Ok(range) = c.GetRange() {
                underline(tip, ec, ctx, &range, false);
            }
            let _ = c.EndComposition(ec);
        }
    }
    tip.st.borrow_mut().session.reset();
    candidates::hide();
}

impl ITfTextEditSink_Impl for Tip_Impl {
    fn OnEndEdit(&self, pic: Option<&ITfContext>, ec: u32, record: Option<&ITfEditRecord>) -> Result<()> {
        crate::guard(Ok(()), || {
        let comp = self.st.borrow().composition.clone();
        let (Some(ctx), Some(record), Some(comp)) = (pic, record, comp) else { return Ok(()) };
        unsafe {
            if !record.GetSelectionStatus()?.as_bool() {
                return Ok(());
            }
            let mut sel = [TF_SELECTION::default()];
            let mut n = 0;
            ctx.GetSelection(ec, TF_DEFAULT_SELECTION, &mut sel, &mut n)?;
            let [sel] = sel;
            let range = ManuallyDrop::into_inner(sel.range);
            let (Some(range), Ok(word)) = (range, comp.GetRange()) else { return Ok(()) };
            // our own edits leave the caret at the end of the word, inside it
            let inside = range.CompareStart(ec, &word, TF_ANCHOR_START)? >= 0 && range.CompareEnd(ec, &word, TF_ANCHOR_END)? <= 0;
            dlog!("OnEndEdit inside={}", inside);
            if !inside {
                let me = self.to_object();
                let t = me.clone();
                let c = ctx.clone();
                edit(&me, ctx, false, move |ec| {
                    finish_in_place(&t, ec, &c);
                    Ok(())
                })?;
            }
        }
        Ok(())
    })
    }
}

impl ITfTextInputProcessor_Impl for Tip_Impl {
    fn Activate(&self, ptim: Option<&ITfThreadMgr>, tid: u32) -> Result<()> {
        self.ActivateEx(ptim, tid, 0)
    }

    fn Deactivate(&self) -> Result<()> {
        crate::guard(Ok(()), || {
        candidates::hide();
        watch(&self.to_object(), None);
        let mut st = self.st.borrow_mut();
        if let Some(tm) = st.thread_mgr.take() {
            unsafe {
                if let Ok(src) = tm.cast::<ITfSource>() {
                    let _ = src.UnadviseSink(st.thread_mgr_cookie);
                }
                if let Ok(km) = tm.cast::<ITfKeystrokeMgr>() {
                    let _ = km.UnadviseKeyEventSink(st.tid);
                }
                if let (Some(button), Ok(lbm)) = (st.langbar.take(), tm.cast::<ITfLangBarItemMgr>()) {
                    let _ = lbm.RemoveItem(&button.to_interface::<ITfLangBarItemButton>());
                }
            }
        }
        st.thread_mgr_cookie = TF_INVALID_COOKIE;
        st.composition = None;
        st.context = None;
        st.session.reset();
        Ok(())
    })
    }
}

impl ITfTextInputProcessorEx_Impl for Tip_Impl {
    fn ActivateEx(&self, ptim: Option<&ITfThreadMgr>, tid: u32, _flags: u32) -> Result<()> {
        crate::guard(Err(E_UNEXPECTED.into()), || {
        dlog!("ActivateEx tid={}", tid);
        let tm = ptim.ok_or(Error::from(E_INVALIDARG))?.clone();
        let me = self.to_object();
        unsafe {
            let src: ITfSource = tm.cast()?;
            let sink: ITfThreadMgrEventSink = me.to_interface();
            let cookie = src.AdviseSink(&ITfThreadMgrEventSink::IID, &sink)?;
            let km: ITfKeystrokeMgr = tm.cast()?;
            let keys: ITfKeyEventSink = me.to_interface();
            km.AdviseKeyEventSink(tid, &keys, TRUE)?;
            let attr = CoCreateInstance::<_, ITfCategoryMgr>(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)
                .and_then(|c| c.RegisterGUID(&ATTR_GUID))
                .unwrap_or(0);
            let button = ComObject::new(langbar::Button::new());
            let item: ITfLangBarItemButton = button.to_interface();
            let added = tm.cast::<ITfLangBarItemMgr>().and_then(|lbm| lbm.AddItem(&item)).is_ok();
            let mut st = self.st.borrow_mut();
            st.thread_mgr = Some(tm.clone());
            st.tid = tid;
            st.thread_mgr_cookie = cookie;
            st.attr = attr;
            st.langbar = added.then_some(button);
            st.version = None;
        }
        self.sync();
        watch(&me, unsafe { tm.GetFocus() }.ok().as_ref());
        // the word lists take a moment to load: do it now rather than at the first key
        std::thread::spawn(|| {
            let _ = liyamu_core::suggest::singlish("ka", &[], true);
        });
        Ok(())
    })
    }
}

impl ITfThreadMgrEventSink_Impl for Tip_Impl {
    fn OnInitDocumentMgr(&self, _: Option<&ITfDocumentMgr>) -> Result<()> {
        Ok(())
    }
    fn OnUninitDocumentMgr(&self, _: Option<&ITfDocumentMgr>) -> Result<()> {
        Ok(())
    }
    fn OnSetFocus(&self, focus: Option<&ITfDocumentMgr>, _prev: Option<&ITfDocumentMgr>) -> Result<()> {
        crate::guard(Ok(()), || {
        dlog!("ThreadMgr OnSetFocus {}", focus.is_some());
        candidates::hide();
        watch(&self.to_object(), focus);
        let mut st = self.st.borrow_mut();
        if st.composition.is_none() {
            st.session.reset(); // the previous words were somewhere else
        }
        Ok(())
    })
    }
    fn OnPushContext(&self, pic: Option<&ITfContext>) -> Result<()> {
        let dim = pic.and_then(|c| unsafe { c.GetDocumentMgr() }.ok());
        watch(&self.to_object(), dim.as_ref());
        Ok(())
    }
    fn OnPopContext(&self, pic: Option<&ITfContext>) -> Result<()> {
        let dim = pic.and_then(|c| unsafe { c.GetDocumentMgr() }.ok());
        watch(&self.to_object(), dim.as_ref());
        Ok(())
    }
}

impl ITfKeyEventSink_Impl for Tip_Impl {
    fn OnSetFocus(&self, foreground: BOOL) -> Result<()> {
        crate::guard(Ok(()), || {
        if !foreground.as_bool() {
            candidates::hide();
        }
        Ok(())
    })
    }

    fn OnTestKeyDown(&self, _pic: Option<&ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        crate::guard(Ok(FALSE), || {
        let r = match keys::translate(wparam.0 as u16, lparam.0) {
            Some((key, mods)) => self.wants(key, mods).into(),
            None => FALSE,
        };
        dlog!("OnTestKeyDown vk={:#x} lparam={:#x} -> {:?}", wparam.0, lparam.0, r);
        Ok(r)
    })
    }

    fn OnKeyDown(&self, pic: Option<&ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        crate::guard(Ok(FALSE), || {
        let (Some(ctx), Some((key, mods))) = (pic, keys::translate(wparam.0 as u16, lparam.0)) else {
            return Ok(FALSE);
        };
        let changed = self.sync();
        if !self.wants(key, mods) && changed.as_ref().map_or(true, |r| r.commit.is_empty()) {
            return Ok(FALSE);
        }
        let r = {
            let mut st = self.st.borrow_mut();
            st.context = Some(ctx.clone());
            st.session.key(key, mods)
        };
        let handled = r.handled;
        dlog!("OnKeyDown {:?} {:?} -> {:?}", key, mods, r);
        let responses = changed.into_iter().chain(std::iter::once(r)).collect();
        if request(&self.to_object(), ctx, responses, true).is_err() {
            // the text field cannot be written now: drop the word rather than lose the key
            self.st.borrow_mut().session.reset();
            candidates::hide();
            return Ok(FALSE);
        }
        Ok(handled.into())
    })
    }

    fn OnTestKeyUp(&self, _: Option<&ITfContext>, _: WPARAM, _: LPARAM) -> Result<BOOL> {
        Ok(FALSE)
    }
    fn OnKeyUp(&self, _: Option<&ITfContext>, _: WPARAM, _: LPARAM) -> Result<BOOL> {
        Ok(FALSE)
    }
    fn OnPreservedKey(&self, _: Option<&ITfContext>, _: *const GUID) -> Result<BOOL> {
        Ok(FALSE)
    }
}

impl ITfCompositionSink_Impl for Tip_Impl {
    /// The app ended the composition (a click elsewhere, focus moved): the word stays as it is shown.
    fn OnCompositionTerminated(&self, ec: u32, comp: Option<&ITfComposition>) -> Result<()> {
        crate::guard(Ok(()), || {
        dlog!("OnCompositionTerminated");
        let ctx = self.st.borrow().context.clone();
        if let (Some(c), Some(ctx)) = (comp, ctx) {
            if let Ok(range) = unsafe { c.GetRange() } {
                underline(self, ec, &ctx, &range, false);
            }
        }
        let mut st = self.st.borrow_mut();
        st.composition = None;
        st.session.reset();
        drop(st);
        candidates::hide();
        Ok(())
    })
    }
}

// The underline under the word being typed.

impl ITfDisplayAttributeProvider_Impl for Tip_Impl {
    fn EnumDisplayAttributeInfo(&self) -> Result<IEnumTfDisplayAttributeInfo> {
        Ok(EnumAttr { at: RefCell::new(0) }.into())
    }
    fn GetDisplayAttributeInfo(&self, guid: *const GUID) -> Result<ITfDisplayAttributeInfo> {
        if unsafe { *guid } == ATTR_GUID {
            Ok(AttrInfo.into())
        } else {
            Err(E_INVALIDARG.into())
        }
    }
}

#[implement(ITfDisplayAttributeInfo)]
struct AttrInfo;

impl ITfDisplayAttributeInfo_Impl for AttrInfo_Impl {
    fn GetGUID(&self) -> Result<GUID> {
        Ok(ATTR_GUID)
    }
    fn GetDescription(&self) -> Result<BSTR> {
        Ok(BSTR::from("Liyamu: the word being typed"))
    }
    fn GetAttributeInfo(&self, pda: *mut TF_DISPLAYATTRIBUTE) -> Result<()> {
        unsafe {
            *pda = TF_DISPLAYATTRIBUTE {
                crText: TF_DA_COLOR { r#type: TF_CT_NONE, ..Default::default() },
                crBk: TF_DA_COLOR { r#type: TF_CT_NONE, ..Default::default() },
                lsStyle: TF_LS_SOLID,
                fBoldLine: FALSE,
                crLine: TF_DA_COLOR { r#type: TF_CT_NONE, ..Default::default() },
                bAttr: TF_ATTR_INPUT,
            };
        }
        Ok(())
    }
    fn SetAttributeInfo(&self, _: *const TF_DISPLAYATTRIBUTE) -> Result<()> {
        Err(E_NOTIMPL.into())
    }
    fn Reset(&self) -> Result<()> {
        Ok(())
    }
}

#[implement(IEnumTfDisplayAttributeInfo)]
struct EnumAttr {
    at: RefCell<u32>,
}

impl IEnumTfDisplayAttributeInfo_Impl for EnumAttr_Impl {
    fn Clone(&self) -> Result<IEnumTfDisplayAttributeInfo> {
        Ok(EnumAttr { at: RefCell::new(*self.at.borrow()) }.into())
    }
    fn Next(&self, count: u32, info: *mut Option<ITfDisplayAttributeInfo>, fetched: *mut u32) -> Result<()> {
        let mut at = self.at.borrow_mut();
        let n = if count > 0 && *at == 0 {
            unsafe { *info = Some(AttrInfo.into()) };
            *at = 1;
            1
        } else {
            0
        };
        if !fetched.is_null() {
            unsafe { *fetched = n };
        }
        if n == count { Ok(()) } else { Err(S_FALSE.into()) }
    }
    fn Reset(&self) -> Result<()> {
        *self.at.borrow_mut() = 0;
        Ok(())
    }
    fn Skip(&self, count: u32) -> Result<()> {
        let mut at = self.at.borrow_mut();
        *at = (*at + count).min(1);
        Ok(())
    }
}
