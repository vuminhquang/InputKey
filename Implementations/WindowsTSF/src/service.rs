use crate::config::{EngineConfig, EngineFactory};
use inputkey_core_abstractions::TypingEnginePort;
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::rc::{Rc, Weak};
use std::sync::{
    atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering},
    Arc, Mutex, OnceLock,
};
use windows::{
    core::{implement, w, Error, Interface, Ref, Result, BOOL, GUID, HRESULT, PCWSTR},
    Win32::{
        Foundation::{
            CLASS_E_NOAGGREGATION, E_POINTER, HINSTANCE, HWND, LPARAM, LRESULT, S_FALSE, S_OK,
            WPARAM,
        },
        System::{
            Com::{IClassFactory, IClassFactory_Impl},
            LibraryLoader::GetModuleHandleW,
        },
        UI::{
            Input::KeyboardAndMouse::{
                GetKeyState, GetKeyboardLayout, GetKeyboardState, MapVirtualKeyExW, ToUnicodeEx,
                MAPVK_VK_TO_VSC, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE,
                VK_HOME, VK_INSERT, VK_LEFT, VK_LWIN, VK_MENU, VK_NEXT, VK_PRIOR, VK_RETURN,
                VK_RIGHT, VK_RWIN, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
            },
            TextServices::{
                ITfComposition, ITfCompositionSink, ITfCompositionSink_Impl, ITfContext,
                ITfContextComposition, ITfEditSession, ITfEditSession_Impl, ITfInsertAtSelection,
                ITfKeyEventSink, ITfKeyEventSink_Impl, ITfKeyTraceEventSink,
                ITfKeyTraceEventSink_Impl, ITfKeystrokeMgr, ITfMouseSink, ITfMouseSink_Impl,
                ITfMouseTracker, ITfSource, ITfTextInputProcessor, ITfTextInputProcessor_Impl,
                ITfThreadMgr, TF_AE_END, TF_ANCHOR_END, TF_ANCHOR_START, TF_DEFAULT_SELECTION,
                TF_ES_READWRITE, TF_ES_SYNC, TF_IAS_NO_DEFAULT_COMPOSITION, TF_SELECTION,
                TF_SELECTIONSTYLE,
            },
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowW, PostMessageW,
                RegisterClassW, RegisterWindowMessageW, HWND_MESSAGE, WINDOW_EX_STYLE,
                WINDOW_STYLE, WM_NCDESTROY, WNDCLASSW,
            },
        },
    },
};
use windows_core::IUnknownImpl as _;

static LIVE_OBJECTS: AtomicU32 = AtomicU32::new(0);
static SERVER_LOCKS: AtomicU32 = AtomicU32::new(0);

const TSF_CONTROL_CLASS: PCWSTR = w!("InputKeyTSFControlWindow");
const APP_CONTROL_CLASS: PCWSTR = w!("InputKeyControlWindow");

fn pointer_error() -> Error {
    Error::from_hresult(E_POINTER)
}

fn commit_then_pass_key(vk: u32) -> bool {
    [
        VK_RETURN, VK_TAB, VK_LEFT, VK_RIGHT, VK_UP, VK_DOWN, VK_HOME, VK_END, VK_PRIOR, VK_NEXT,
        VK_DELETE, VK_INSERT,
    ]
    .iter()
    .any(|key| vk == key.0 as u32)
}

fn is_raw_boundary_space(vk: u32, shift: bool, control: bool, alt: bool, win: bool) -> bool {
    vk == VK_SPACE.0 as u32 && shift && !control && !alt && !win
}

pub fn can_unload_now() -> HRESULT {
    if LIVE_OBJECTS.load(Ordering::Acquire) == 0 && SERVER_LOCKS.load(Ordering::Acquire) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

fn composition_state_message() -> u32 {
    static MESSAGE: OnceLock<u32> = OnceLock::new();
    *MESSAGE.get_or_init(|| unsafe { RegisterWindowMessageW(w!("InputKey.TSF.CompositionState")) })
}

fn reload_settings_message() -> u32 {
    static MESSAGE: OnceLock<u32> = OnceLock::new();
    *MESSAGE.get_or_init(|| unsafe { RegisterWindowMessageW(w!("InputKey.TSF.ReloadSettings")) })
}

#[derive(Clone)]
enum EditAction {
    Update(String),
    Commit(String),
    ValidateCaret(Arc<std::sync::atomic::AtomicBool>),
    TrackMouse(ITfMouseSink),
}

struct MouseTracking {
    context: ITfContext,
    tracker: ITfMouseTracker,
    cookie: u32,
}

struct Runtime {
    engine: Box<dyn TypingEnginePort>,
    composition: Option<ITfComposition>,
    swallowed: [bool; 256],
}

impl Runtime {
    fn new(engine: Box<dyn TypingEnginePort>) -> Self {
        Self {
            engine,
            composition: None,
            swallowed: [false; 256],
        }
    }

    fn active(&self) -> bool {
        self.engine.history_active() || self.composition.is_some()
    }

    fn reset(&mut self) {
        self.engine.reset();
        self.composition = None;
        self.swallowed = [false; 256];
    }
}

struct ControlState {
    runtime: Mutex<Runtime>,
    factory: EngineFactory,
    config: Mutex<EngineConfig>,
    enabled: AtomicBool,
    client_id: AtomicU32,
    control_hwnd: AtomicIsize,
    mouse_tracking: Mutex<Option<MouseTracking>>,
}

impl ControlState {
    fn new(factory: EngineFactory, mut config: EngineConfig) -> Self {
        let persisted = inputkey_windows_settings::load();
        config.language_id = persisted.language.clone();
        config.language.method = persisted.method.clone();
        config
            .language
            .toggles
            .insert("auto_restore".into(), persisted.auto_restore);
        config
            .language
            .toggles
            .insert("smart_correction".into(), persisted.smart_correction);
        let engine = factory(config.clone());
        Self {
            runtime: Mutex::new(Runtime::new(engine)),
            factory,
            config: Mutex::new(config),
            enabled: AtomicBool::new(persisted.enabled),
            client_id: AtomicU32::new(0),
            control_hwnd: AtomicIsize::new(0),
            mouse_tracking: Mutex::new(None),
        }
    }

    fn clear_mouse_tracking(&self) {
        if let Some(tracking) = self
            .mouse_tracking
            .lock()
            .expect("mouse tracking lock")
            .take()
        {
            unsafe {
                let _ = tracking.tracker.UnadviseMouseSink(tracking.cookie);
            }
        }
    }

    fn reset_runtime(&self) {
        self.clear_mouse_tracking();
        let config = self.config.lock().expect("config lock").clone();
        let mut runtime = self.runtime.lock().expect("runtime lock");
        runtime.engine = (self.factory)(config);
        runtime.composition = None;
        runtime.swallowed = [false; 256];
    }

    fn reload_settings(&self) {
        let persisted = inputkey_windows_settings::load();
        {
            let mut config = self.config.lock().expect("config lock");
            config.language_id = persisted.language.clone();
            config.language.method = persisted.method.clone();
            config
                .language
                .toggles
                .insert("auto_restore".into(), persisted.auto_restore);
            config
                .language
                .toggles
                .insert("smart_correction".into(), persisted.smart_correction);
        }
        self.enabled.store(persisted.enabled, Ordering::Release);
        self.reset_runtime();
        self.notify_composition_state(false);
    }

    fn notify_composition_state(&self, active: bool) {
        let service_hwnd = self.control_hwnd.load(Ordering::Acquire);
        if service_hwnd == 0 {
            return;
        }
        let Ok(controller) = (unsafe { FindWindowW(APP_CONTROL_CLASS, PCWSTR::null()) }) else {
            return;
        };
        unsafe {
            let _ = PostMessageW(
                Some(controller),
                composition_state_message(),
                WPARAM(usize::from(active)),
                LPARAM(service_hwnd),
            );
        }
    }
}

thread_local! {
    static CONTROL_WINDOWS: RefCell<HashMap<isize, Weak<ControlState>>> = RefCell::new(HashMap::new());
}

unsafe extern "system" fn control_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == reload_settings_message() {
        let key = hwnd.0 as isize;
        CONTROL_WINDOWS.with(|windows| {
            if let Some(state) = windows.borrow().get(&key).and_then(Weak::upgrade) {
                state.reload_settings();
            }
        });
        return LRESULT(1);
    }

    if msg == WM_NCDESTROY {
        CONTROL_WINDOWS.with(|windows| {
            windows.borrow_mut().remove(&(hwnd.0 as isize));
        });
    }

    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn create_control_window(state: &Rc<ControlState>) -> Result<HWND> {
    let module = unsafe { GetModuleHandleW(PCWSTR::null())? };
    let instance = HINSTANCE(module.0);
    let class = WNDCLASSW {
        lpfnWndProc: Some(control_window_proc),
        hInstance: instance,
        lpszClassName: TSF_CONTROL_CLASS,
        ..unsafe { std::mem::zeroed() }
    };
    unsafe {
        RegisterClassW(&class);
    }

    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            TSF_CONTROL_CLASS,
            PCWSTR::null(),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance),
            None,
        )?
    };

    state.control_hwnd.store(hwnd.0 as isize, Ordering::Release);
    CONTROL_WINDOWS.with(|windows| {
        windows
            .borrow_mut()
            .insert(hwnd.0 as isize, Rc::downgrade(state));
    });
    Ok(hwnd)
}

fn destroy_control_window(state: &ControlState) {
    let raw = state.control_hwnd.swap(0, Ordering::AcqRel);
    if raw == 0 {
        return;
    }
    let hwnd = HWND(raw as *mut core::ffi::c_void);
    CONTROL_WINDOWS.with(|windows| {
        windows.borrow_mut().remove(&raw);
    });
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

#[implement(ITfCompositionSink)]
struct CompositionSink {
    control: Rc<ControlState>,
}

impl ITfCompositionSink_Impl for CompositionSink_Impl {
    fn OnCompositionTerminated(
        &self,
        _ecwrite: u32,
        _pcomposition: Ref<ITfComposition>,
    ) -> Result<()> {
        self.control.runtime.lock().expect("runtime lock").reset();
        self.control.notify_composition_state(false);
        Ok(())
    }
}

#[implement(ITfEditSession)]
struct EditSession {
    context: ITfContext,
    control: Rc<ControlState>,
    action: EditAction,
}

impl EditSession {
    fn take_selection_range(&self, ec: u32) -> Result<windows::Win32::UI::TextServices::ITfRange> {
        let mut selection = [TF_SELECTION {
            range: ManuallyDrop::new(None),
            style: TF_SELECTIONSTYLE::default(),
        }];
        let mut fetched = 0u32;
        unsafe {
            self.context
                .GetSelection(ec, TF_DEFAULT_SELECTION, &mut selection, &mut fetched)?;
        }
        if fetched != 1 {
            return Err(pointer_error());
        }
        let range = unsafe { ManuallyDrop::take(&mut selection[0].range) };
        range.ok_or_else(pointer_error)
    }

    fn set_caret_at_end(
        &self,
        ec: u32,
        range: &windows::Win32::UI::TextServices::ITfRange,
    ) -> Result<()> {
        let caret = range.clone();
        unsafe {
            caret.Collapse(ec, TF_ANCHOR_END)?;
        }
        let mut selection = [TF_SELECTION {
            range: ManuallyDrop::new(Some(caret)),
            style: TF_SELECTIONSTYLE {
                ase: TF_AE_END,
                fInterimChar: BOOL::from(false),
            },
        }];
        let result = unsafe { self.context.SetSelection(ec, &selection) };
        unsafe {
            ManuallyDrop::drop(&mut selection[0].range);
        }
        result
    }

    fn start_composition_from_selection(
        &self,
        ec: u32,
        text: &str,
    ) -> Result<(ITfComposition, windows::Win32::UI::TextServices::ITfRange)> {
        let insertion: ITfInsertAtSelection = self.context.cast()?;
        let utf16: Vec<u16> = text.encode_utf16().collect();
        let range =
            unsafe { insertion.InsertTextAtSelection(ec, TF_IAS_NO_DEFAULT_COMPOSITION, &utf16)? };
        let context_composition: ITfContextComposition = self.context.cast()?;
        let sink: ITfCompositionSink = CompositionSink {
            control: Rc::clone(&self.control),
        }
        .into();
        let composition = unsafe { context_composition.StartComposition(ec, &range, &sink)? };
        self.control
            .runtime
            .lock()
            .expect("runtime lock")
            .composition = Some(composition.clone());
        self.control.notify_composition_state(true);
        Ok((composition, range))
    }

    fn install_mouse_tracking(&self, ec: u32, sink: &ITfMouseSink) -> Result<()> {
        let current_context = self.context.clone();
        {
            let guard = self
                .control
                .mouse_tracking
                .lock()
                .expect("mouse tracking lock");
            if guard.as_ref().is_some_and(|tracking| {
                Interface::as_raw(&tracking.context) == Interface::as_raw(&current_context)
            }) {
                return Ok(());
            }
        }

        self.control.clear_mouse_tracking();
        let range = unsafe { self.context.GetStart(ec)? };
        let end = unsafe { self.context.GetEnd(ec)? };
        unsafe {
            range.ShiftEndToRange(ec, &end, TF_ANCHOR_END)?;
        }
        let tracker: ITfMouseTracker = self.context.cast()?;
        let cookie = unsafe { tracker.AdviseMouseSink(&range, sink)? };
        *self
            .control
            .mouse_tracking
            .lock()
            .expect("mouse tracking lock") = Some(MouseTracking {
            context: current_context,
            tracker,
            cookie,
        });
        Ok(())
    }

    fn caret_is_at_composition_end(&self, ec: u32) -> Result<bool> {
        let composition = match self
            .control
            .runtime
            .lock()
            .expect("runtime lock")
            .composition
            .clone()
        {
            Some(value) => value,
            None => return Ok(true),
        };
        let selection = self.take_selection_range(ec)?;
        if !unsafe { selection.IsEmpty(ec)? }.as_bool() {
            return Ok(false);
        }
        let caret = unsafe { composition.GetRange()? };
        unsafe {
            caret.Collapse(ec, TF_ANCHOR_END)?;
        }
        let same_start = unsafe { selection.IsEqualStart(ec, &caret, TF_ANCHOR_START)? }.as_bool();
        let same_end = unsafe { selection.IsEqualEnd(ec, &caret, TF_ANCHOR_END)? }.as_bool();
        Ok(same_start && same_end)
    }

    fn terminate_stale_composition(&self, ec: u32) -> Result<()> {
        let composition = self
            .control
            .runtime
            .lock()
            .expect("runtime lock")
            .composition
            .clone();
        if let Some(composition) = composition {
            unsafe {
                composition.EndComposition(ec)?;
            }
        }
        self.control.runtime.lock().expect("runtime lock").reset();
        self.control.notify_composition_state(false);
        Ok(())
    }

    fn replace_composition(&self, ec: u32, text: &str, end: bool) -> Result<()> {
        let existing = self
            .control
            .runtime
            .lock()
            .expect("runtime lock")
            .composition
            .clone();

        let (composition, range) = if let Some(composition) = existing {
            let range = unsafe { composition.GetRange()? };
            let utf16: Vec<u16> = text.encode_utf16().collect();
            unsafe {
                range.SetText(ec, 0, &utf16)?;
            }
            (composition, range)
        } else {
            self.start_composition_from_selection(ec, text)?
        };

        self.set_caret_at_end(ec, &range)?;

        if end || text.is_empty() {
            unsafe {
                composition.EndComposition(ec)?;
            }
            self.control
                .runtime
                .lock()
                .expect("runtime lock")
                .composition = None;
            self.control.notify_composition_state(false);
        }
        Ok(())
    }
}

impl ITfEditSession_Impl for EditSession_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        match &self.action {
            EditAction::Update(text) => self.replace_composition(ec, text, false),
            EditAction::Commit(text) => self.replace_composition(ec, text, true),
            EditAction::ValidateCaret(moved) => {
                let valid = self.caret_is_at_composition_end(ec)?;
                if !valid {
                    self.terminate_stale_composition(ec)?;
                    moved.store(true, Ordering::Release);
                }
                Ok(())
            }
            EditAction::TrackMouse(sink) => self.install_mouse_tracking(ec, sink),
        }
    }
}

fn request_edit(
    control: &Rc<ControlState>,
    context: &ITfContext,
    action: EditAction,
) -> Result<()> {
    let client_id = control.client_id.load(Ordering::Acquire);
    if client_id == 0 {
        return Err(pointer_error());
    }
    let session: ITfEditSession = EditSession {
        context: context.clone(),
        control: Rc::clone(control),
        action,
    }
    .into();
    let result =
        unsafe { context.RequestEditSession(client_id, &session, TF_ES_SYNC | TF_ES_READWRITE)? };
    result.ok()
}

#[implement(
    ITfTextInputProcessor,
    ITfKeyEventSink,
    ITfKeyTraceEventSink,
    ITfMouseSink
)]
pub struct TextService {
    control: Rc<ControlState>,
    thread_mgr: Mutex<Option<ITfThreadMgr>>,
    key_trace_cookie: Mutex<Option<u32>>,
}

impl TextService {
    pub fn new(factory: EngineFactory, config: EngineConfig) -> Self {
        LIVE_OBJECTS.fetch_add(1, Ordering::AcqRel);
        Self {
            control: Rc::new(ControlState::new(factory, config)),
            thread_mgr: Mutex::new(None),
            key_trace_cookie: Mutex::new(None),
        }
    }

    fn focused_context(&self) -> Result<ITfContext> {
        let thread_mgr = self
            .thread_mgr
            .lock()
            .expect("thread manager lock")
            .clone()
            .ok_or_else(pointer_error)?;
        let document = unsafe { thread_mgr.GetFocus()? };
        unsafe { document.GetTop() }
    }

    fn commit_traced_boundary(&self, vk: u32) {
        if !commit_then_pass_key(vk) || !self.control.enabled.load(Ordering::Acquire) {
            return;
        }

        let text = {
            let mut runtime = self.control.runtime.lock().expect("runtime lock");
            if !runtime.active() {
                return;
            }
            runtime.engine.natural_boundary()
        };

        let Ok(context) = self.focused_context() else {
            self.fail_open();
            return;
        };
        if request_edit(&self.control, &context, EditAction::Commit(text)).is_err() {
            self.fail_open();
        }
    }

    fn modifier_down(vk: i32) -> bool {
        unsafe { (GetKeyState(vk) as u16 & 0x8000) != 0 }
    }

    fn raw_boundary_space(vk: u32) -> bool {
        is_raw_boundary_space(
            vk,
            Self::modifier_down(VK_SHIFT.0 as i32),
            Self::modifier_down(VK_CONTROL.0 as i32),
            Self::modifier_down(VK_MENU.0 as i32),
            Self::modifier_down(VK_LWIN.0 as i32) || Self::modifier_down(VK_RWIN.0 as i32),
        )
    }

    fn has_command_modifier() -> bool {
        Self::modifier_down(VK_CONTROL.0 as i32)
            || Self::modifier_down(VK_MENU.0 as i32)
            || Self::modifier_down(VK_LWIN.0 as i32)
            || Self::modifier_down(VK_RWIN.0 as i32)
    }

    fn key_char(&self, vk: u32) -> Option<char> {
        let ch = self.delimiter_char(vk)?;
        self.control
            .runtime
            .lock()
            .expect("runtime lock")
            .engine
            .accepts_key(ch)
            .then_some(ch)
    }

    fn delimiter_char(&self, vk: u32) -> Option<char> {
        if Self::has_command_modifier() {
            return None;
        }
        let mut keyboard = [0u8; 256];
        unsafe {
            GetKeyboardState(&mut keyboard).ok()?;
        }
        let layout = unsafe { GetKeyboardLayout(0) };
        let scan = unsafe { MapVirtualKeyExW(vk, MAPVK_VK_TO_VSC, Some(layout)) };
        let mut buffer = [0u16; 4];
        let count = unsafe { ToUnicodeEx(vk, scan, &keyboard, &mut buffer, 4, Some(layout)) };
        if count != 1 {
            return None;
        }
        char::from_u32(buffer[0] as u32).filter(|ch| !ch.is_control())
    }

    fn fail_open(&self) {
        self.control.reset_runtime();
        self.control.notify_composition_state(false);
    }

    fn validate_caret(&self, context: &ITfContext) -> Result<bool> {
        if !self.control.runtime.lock().expect("runtime lock").active() {
            return Ok(false);
        }
        let moved = Arc::new(std::sync::atomic::AtomicBool::new(false));
        request_edit(
            &self.control,
            context,
            EditAction::ValidateCaret(Arc::clone(&moved)),
        )?;
        Ok(moved.load(Ordering::Acquire))
    }

    fn ensure_mouse_tracking(&self, context: &ITfContext, sink: &ITfMouseSink) -> Result<()> {
        if !self.control.runtime.lock().expect("runtime lock").active() {
            return Ok(());
        }
        request_edit(&self.control, context, EditAction::TrackMouse(sink.clone()))
    }

    fn commit_mouse_boundary(&self) {
        let text = {
            let mut runtime = self.control.runtime.lock().expect("runtime lock");
            if !runtime.active() {
                return;
            }
            runtime.engine.mouse_boundary()
        };
        let Ok(context) = self.focused_context() else {
            self.fail_open();
            return;
        };
        if request_edit(&self.control, &context, EditAction::Commit(text)).is_err() {
            self.fail_open();
        }
    }

    fn should_offer_key(&self, vk: u32) -> bool {
        if !self.control.enabled.load(Ordering::Acquire) {
            return false;
        }
        if self.key_char(vk).is_some() {
            return true;
        }
        let active = self.control.runtime.lock().expect("runtime lock").active();
        if !active {
            return false;
        }
        if commit_then_pass_key(vk) {
            return false;
        }
        if vk == VK_BACK.0 as u32 || vk == VK_ESCAPE.0 as u32 {
            return true;
        }
        self.delimiter_char(vk).is_some()
    }

    fn handle_key_down(&self, context: &ITfContext, vk: u32) -> Result<BOOL> {
        if !self.control.enabled.load(Ordering::Acquire) {
            return Ok(BOOL::from(false));
        }

        let caret_moved = match self.validate_caret(context) {
            Ok(value) => value,
            Err(_) => {
                self.fail_open();
                return Ok(BOOL::from(false));
            }
        };

        if Self::raw_boundary_space(vk) {
            let commit = {
                let mut runtime = self.control.runtime.lock().expect("runtime lock");
                if !runtime.active() {
                    return Ok(BOOL::from(false));
                }
                runtime.engine.commit_raw_boundary()
            };
            if request_edit(&self.control, context, EditAction::Commit(commit)).is_err() {
                self.fail_open();
                return Ok(BOOL::from(false));
            }
            if vk < 256 {
                self.control.runtime.lock().expect("runtime lock").swallowed[vk as usize] = true;
            }
            return Ok(BOOL::from(true));
        }

        if let Some(ch) = self.key_char(vk) {
            let text = {
                let mut runtime = self.control.runtime.lock().expect("runtime lock");
                runtime.engine.type_key(ch)
            };
            if request_edit(&self.control, context, EditAction::Update(text)).is_err() {
                self.fail_open();
                return Ok(BOOL::from(false));
            }
            if vk < 256 {
                self.control.runtime.lock().expect("runtime lock").swallowed[vk as usize] = true;
            }
            return Ok(BOOL::from(true));
        }

        if caret_moved {
            return Ok(BOOL::from(false));
        }

        if vk == VK_BACK.0 as u32 {
            let text = {
                let mut runtime = self.control.runtime.lock().expect("runtime lock");
                if !runtime.active() {
                    return Ok(BOOL::from(false));
                }
                runtime.engine.backspace()
            };
            if request_edit(&self.control, context, EditAction::Update(text)).is_err() {
                self.fail_open();
                return Ok(BOOL::from(false));
            }
            if vk < 256 {
                self.control.runtime.lock().expect("runtime lock").swallowed[vk as usize] = true;
            }
            return Ok(BOOL::from(true));
        }

        if vk == VK_ESCAPE.0 as u32 {
            let text = {
                let mut runtime = self.control.runtime.lock().expect("runtime lock");
                if !runtime.active() {
                    return Ok(BOOL::from(false));
                }
                runtime.engine.escape()
            };
            if request_edit(&self.control, context, EditAction::Update(text)).is_err() {
                self.fail_open();
                return Ok(BOOL::from(false));
            }
            if vk < 256 {
                self.control.runtime.lock().expect("runtime lock").swallowed[vk as usize] = true;
            }
            return Ok(BOOL::from(true));
        }

        let Some(delimiter) = self.delimiter_char(vk) else {
            return Ok(BOOL::from(false));
        };
        let commit = {
            let mut runtime = self.control.runtime.lock().expect("runtime lock");
            if !runtime.active() {
                return Ok(BOOL::from(false));
            }
            runtime.engine.decision_boundary(delimiter)
        };
        if request_edit(&self.control, context, EditAction::Commit(commit)).is_err() {
            self.fail_open();
            return Ok(BOOL::from(false));
        }
        if vk < 256 {
            self.control.runtime.lock().expect("runtime lock").swallowed[vk as usize] = true;
        }
        Ok(BOOL::from(true))
    }
}

impl Drop for TextService {
    fn drop(&mut self) {
        self.control.clear_mouse_tracking();
        destroy_control_window(&self.control);
        LIVE_OBJECTS.fetch_sub(1, Ordering::AcqRel);
    }
}

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, ptim: Ref<ITfThreadMgr>, tid: u32) -> Result<()> {
        let thread_mgr = ptim.ok()?.clone();
        let key_mgr: ITfKeystrokeMgr = thread_mgr.cast()?;
        let sink: ITfKeyEventSink = self.to_interface();
        unsafe {
            key_mgr.AdviseKeyEventSink(tid, &sink, true)?;
        }

        let source: ITfSource = thread_mgr.cast()?;
        let trace_sink: ITfKeyTraceEventSink = self.to_interface();
        let trace_cookie =
            match unsafe { source.AdviseSink(&ITfKeyTraceEventSink::IID, &trace_sink) } {
                Ok(cookie) => cookie,
                Err(error) => {
                    unsafe {
                        let _ = key_mgr.UnadviseKeyEventSink(tid);
                    }
                    return Err(error);
                }
            };
        *self.key_trace_cookie.lock().expect("key trace cookie lock") = Some(trace_cookie);

        self.control.client_id.store(tid, Ordering::Release);
        if self.control.control_hwnd.load(Ordering::Acquire) == 0 {
            let _ = create_control_window(&self.control)?;
        }
        *self.thread_mgr.lock().expect("thread manager lock") = Some(thread_mgr);
        Ok(())
    }

    fn Deactivate(&self) -> Result<()> {
        self.control.notify_composition_state(false);
        let tid = self.control.client_id.swap(0, Ordering::AcqRel);
        if let Some(thread_mgr) = self.thread_mgr.lock().expect("thread manager lock").take() {
            if let Some(cookie) = self
                .key_trace_cookie
                .lock()
                .expect("key trace cookie lock")
                .take()
            {
                if let Ok(source) = thread_mgr.cast::<ITfSource>() {
                    unsafe {
                        let _ = source.UnadviseSink(cookie);
                    }
                }
            }
            if let Ok(key_mgr) = thread_mgr.cast::<ITfKeystrokeMgr>() {
                if tid != 0 {
                    unsafe {
                        let _ = key_mgr.UnadviseKeyEventSink(tid);
                    }
                }
            }
        }
        destroy_control_window(&self.control);
        self.fail_open();
        Ok(())
    }
}

impl ITfKeyEventSink_Impl for TextService_Impl {
    fn OnSetFocus(&self, _fforeground: BOOL) -> Result<()> {
        Ok(())
    }

    fn OnTestKeyDown(&self, pic: Ref<ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        let _ = pic;
        Ok(BOOL::from(self.should_offer_key(wparam.0 as u32)))
    }

    fn OnTestKeyUp(&self, _pic: Ref<ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        let vk = wparam.0 as u32;
        let eaten =
            vk < 256 && self.control.runtime.lock().expect("runtime lock").swallowed[vk as usize];
        Ok(BOOL::from(eaten))
    }

    fn OnKeyDown(&self, pic: Ref<ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        let context = pic.ok()?;
        let eaten = self.handle_key_down(context, wparam.0 as u32)?;
        if eaten.as_bool() {
            let sink: ITfMouseSink = self.to_interface();
            if self.ensure_mouse_tracking(context, &sink).is_err() {
                self.fail_open();
                return Ok(BOOL::from(false));
            }
        }
        Ok(eaten)
    }

    fn OnKeyUp(&self, _pic: Ref<ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        let vk = wparam.0 as u32;
        if vk >= 256 {
            return Ok(BOOL::from(false));
        }
        let mut runtime = self.control.runtime.lock().expect("runtime lock");
        let eaten = runtime.swallowed[vk as usize];
        runtime.swallowed[vk as usize] = false;
        Ok(BOOL::from(eaten))
    }

    fn OnPreservedKey(&self, _pic: Ref<ITfContext>, _rguid: *const GUID) -> Result<BOOL> {
        Ok(BOOL::from(false))
    }
}

impl ITfMouseSink_Impl for TextService_Impl {
    fn OnMouseEvent(&self, _uedge: u32, _uquadrant: u32, dwbtnstatus: u32) -> Result<BOOL> {
        const BUTTON_MASK: u32 = 0x0073;
        if dwbtnstatus & BUTTON_MASK != 0 {
            self.commit_mouse_boundary();
        }
        Ok(BOOL::from(false))
    }
}

impl ITfKeyTraceEventSink_Impl for TextService_Impl {
    fn OnKeyTraceDown(&self, wparam: WPARAM, _lparam: LPARAM) -> Result<()> {
        self.commit_traced_boundary(wparam.0 as u32);
        Ok(())
    }

    fn OnKeyTraceUp(&self, _wparam: WPARAM, _lparam: LPARAM) -> Result<()> {
        Ok(())
    }
}

#[implement(IClassFactory)]
pub struct TextServiceClassFactory {
    factory: EngineFactory,
    config: EngineConfig,
}

impl TextServiceClassFactory {
    pub fn new(factory: EngineFactory, config: EngineConfig) -> Self {
        LIVE_OBJECTS.fetch_add(1, Ordering::AcqRel);
        Self { factory, config }
    }
}

impl Drop for TextServiceClassFactory {
    fn drop(&mut self) {
        LIVE_OBJECTS.fetch_sub(1, Ordering::AcqRel);
    }
}

impl IClassFactory_Impl for TextServiceClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Ref<windows::core::IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut core::ffi::c_void,
    ) -> Result<()> {
        if ppvobject.is_null() || riid.is_null() {
            return Err(pointer_error());
        }
        unsafe {
            *ppvobject = std::ptr::null_mut();
        }
        if !punkouter.is_null() {
            return Err(Error::from_hresult(CLASS_E_NOAGGREGATION));
        }
        let service: ITfTextInputProcessor =
            TextService::new(Arc::clone(&self.factory), self.config.clone()).into();
        unsafe { service.query(riid, ppvobject).ok() }
    }

    fn LockServer(&self, flock: BOOL) -> Result<()> {
        if flock.as_bool() {
            SERVER_LOCKS.fetch_add(1, Ordering::AcqRel);
        } else {
            SERVER_LOCKS
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                    Some(value.saturating_sub(1))
                })
                .ok();
        }
        Ok(())
    }
}

pub fn class_factory(factory: EngineFactory, config: EngineConfig) -> IClassFactory {
    TextServiceClassFactory::new(factory, config).into()
}

#[cfg(test)]
mod tests {
    use super::{commit_then_pass_key, is_raw_boundary_space};
    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_DELETE, VK_LEFT, VK_RETURN, VK_SPACE};

    #[test]
    fn natural_boundary_keys_are_trace_commit_then_pass() {
        assert!(commit_then_pass_key(VK_RETURN.0 as u32));
        assert!(commit_then_pass_key(VK_LEFT.0 as u32));
        assert!(commit_then_pass_key(VK_DELETE.0 as u32));
        assert!(!commit_then_pass_key(VK_SPACE.0 as u32));
    }

    #[test]
    fn shift_space_is_the_only_raw_space_boundary() {
        assert!(is_raw_boundary_space(
            VK_SPACE.0 as u32,
            true,
            false,
            false,
            false
        ));
        assert!(!is_raw_boundary_space(
            VK_SPACE.0 as u32,
            false,
            false,
            false,
            false
        ));
        assert!(!is_raw_boundary_space(
            VK_SPACE.0 as u32,
            true,
            true,
            false,
            false
        ));
        assert!(!is_raw_boundary_space(
            VK_RETURN.0 as u32,
            true,
            false,
            false,
            false
        ));
    }
}
