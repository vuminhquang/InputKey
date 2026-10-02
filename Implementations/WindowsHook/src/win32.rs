use crate::{transition, Config, EngineConfig, EngineFactory, Event, EventKind, SpscRing};
use inputkey_windows_clipboard::ClipboardPaste;
use std::sync::atomic::{
    AtomicBool, AtomicPtr, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering,
};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const INPUT_MOD_CTRL: u8 = 1;
const INPUT_MOD_ALT: u8 = 2;
const INPUT_MOD_SHIFT: u8 = 4;
const INPUT_MOD_WIN: u8 = 8;

struct Shared {
    queue: SpscRing<Event, 256>,
    work: usize,
    capture_enabled: AtomicBool,
    enabled: AtomicBool,
    active: AtomicBool,
    active_target: AtomicUsize,
    epoch: AtomicU64,
    modifiers: AtomicU8,
    engine_config: Mutex<EngineConfig>,
    accepted_ascii: [AtomicU64; 2],
    suppress_up: [AtomicU64; 4],
    stop: AtomicBool,
    hook_thread_id: AtomicU32,
    installed: AtomicBool,
    automation_ready: AtomicBool,
}

static SHARED: AtomicPtr<Shared> = AtomicPtr::new(std::ptr::null_mut());

fn state() -> Option<&'static Shared> {
    let ptr = SHARED.load(Ordering::Acquire);
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { &*ptr })
    }
}

fn modifier(vk: u32) -> u8 {
    match vk {
        0x10 | 0xa0 | 0xa1 => INPUT_MOD_SHIFT,
        0x11 | 0xa2 | 0xa3 => INPUT_MOD_CTRL,
        0x12 | 0xa4 | 0xa5 => INPUT_MOD_ALT,
        0x5b | 0x5c => INPUT_MOD_WIN,
        _ => 0,
    }
}

fn update_modifiers(shared: &Shared, vk: u32, down: bool) -> u8 {
    let bit = modifier(vk);
    if bit == 0 {
        return shared.modifiers.load(Ordering::Acquire);
    }
    if down {
        shared.modifiers.fetch_or(bit, Ordering::AcqRel);
    } else {
        shared.modifiers.fetch_and(!bit, Ordering::AcqRel);
    }
    shared.modifiers.load(Ordering::Acquire)
}

fn caps_on() -> bool {
    unsafe { GetKeyState(VK_CAPITAL as i32) & 1 != 0 }
}

fn accepts_char(shared: &Shared, ch: char) -> bool {
    let value = ch as u32;
    if value >= 128 {
        return false;
    }
    let slot = (value / 64) as usize;
    let bit = 1u64 << (value % 64);
    shared.accepted_ascii[slot].load(Ordering::Acquire) & bit != 0
}

fn printable_char(vk: u32, mods: u8) -> Option<char> {
    if mods & (INPUT_MOD_CTRL | INPUT_MOD_ALT | INPUT_MOD_WIN) != 0 {
        return None;
    }
    let shift = mods & INPUT_MOD_SHIFT != 0;
    match vk {
        0x41..=0x5a => {
            let upper = shift ^ caps_on();
            Some(if upper {
                vk as u8 as char
            } else {
                (vk as u8 + 32) as char
            })
        }
        0x30..=0x39 => {
            if shift {
                Some(match vk {
                    0x30 => ')',
                    0x31 => '!',
                    0x32 => '@',
                    0x33 => '#',
                    0x34 => '$',
                    0x35 => '%',
                    0x36 => '^',
                    0x37 => '&',
                    0x38 => '*',
                    0x39 => '(',
                    _ => unreachable!(),
                })
            } else {
                Some(vk as u8 as char)
            }
        }
        0x20 => Some(' '),
        0xba => Some(if shift { ':' } else { ';' }),
        0xbb => Some(if shift { '+' } else { '=' }),
        0xbc => Some(if shift { '<' } else { ',' }),
        0xbd => Some(if shift { '_' } else { '-' }),
        0xbe => Some(if shift { '>' } else { '.' }),
        0xbf => Some(if shift { '?' } else { '/' }),
        0xc0 => Some(if shift { '~' } else { '\u{0060}' }),
        0xdb => Some(if shift { '{' } else { '[' }),
        0xdc => Some(if shift { '|' } else { '\\' }),
        0xdd => Some(if shift { '}' } else { ']' }),
        0xde => Some(if shift { '"' } else { '\'' }),
        _ => None,
    }
}

fn natural_boundary_key(vk: u32) -> bool {
    [
        VK_RETURN, VK_TAB, VK_LEFT, VK_RIGHT, VK_UP, VK_DOWN, VK_HOME, VK_END, VK_PRIOR, VK_NEXT,
        VK_DELETE, VK_INSERT,
    ]
    .iter()
    .any(|key| vk == u32::from(*key))
}

fn mark_suppressed_up(shared: &Shared, vk: u32) {
    if vk < 256 {
        shared.suppress_up[(vk / 64) as usize].fetch_or(1u64 << (vk % 64), Ordering::AcqRel);
    }
}

fn take_suppressed_up(shared: &Shared, vk: u32) -> bool {
    vk < 256
        && shared.suppress_up[(vk / 64) as usize].fetch_and(!(1u64 << (vk % 64)), Ordering::AcqRel)
            & (1u64 << (vk % 64))
            != 0
}

fn focused_target() -> HWND {
    let foreground = unsafe { GetForegroundWindow() };
    if foreground.is_null() {
        return std::ptr::null_mut();
    }
    let thread_id = unsafe { GetWindowThreadProcessId(foreground, std::ptr::null_mut()) };
    if thread_id == 0 {
        return foreground;
    }
    let mut info: GUITHREADINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
    if unsafe { GetGUIThreadInfo(thread_id, &mut info) } != 0 && !info.hwndFocus.is_null() {
        info.hwndFocus
    } else {
        foreground
    }
}

fn push(shared: &Shared, kind: EventKind, vk: u32, mods: u8, target: HWND) -> bool {
    let event = Event {
        kind,
        vk: vk as u16,
        modifiers: mods,
        target: target as usize,
        epoch: shared.epoch.load(Ordering::Acquire),
        scan_code: 0,
        extended: false,
    };
    if !shared.queue.push(event) {
        shared.epoch.fetch_add(1, Ordering::AcqRel);
        shared.active.store(false, Ordering::Release);
        shared.active_target.store(0, Ordering::Release);
        unsafe {
            SetEvent(shared.work as HANDLE);
        }
        false
    } else {
        unsafe {
            SetEvent(shared.work as HANDLE);
        }
        true
    }
}

unsafe extern "system" fn mouse_proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code < 0 {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
    }
    let Some(shared) = state() else {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
    };
    let message = wp as u32;
    if !matches!(
        message,
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
    ) {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
    }
    let data = unsafe { &*(lp as *const MSLLHOOKSTRUCT) };
    if data.flags & LLMHF_INJECTED != 0 {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
    }
    if !shared.capture_enabled.load(Ordering::Acquire)
        || !shared.enabled.load(Ordering::Acquire)
        || !shared.active.load(Ordering::Acquire)
    {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
    }

    let target = crate::transport::canonical_target(focused_target());
    if push(shared, EventKind::MouseBoundary, 0, 0, target) {
        shared.active.store(false, Ordering::Release);
        shared.active_target.store(0, Ordering::Release);
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) }
}

unsafe extern "system" fn keyboard_proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code < 0 {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
    }
    let Some(shared) = state() else {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
    };
    let data = unsafe { &*(lp as *const KBDLLHOOKSTRUCT) };
    if data.flags & LLKHF_INJECTED != 0 {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
    }
    let message = wp as u32;
    let down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
    let up = message == WM_KEYUP || message == WM_SYSKEYUP;
    if !down && !up {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
    }

    let vk = data.vkCode;
    let mods = update_modifiers(shared, vk, down);

    if up && take_suppressed_up(shared, vk) {
        return 1;
    }
    if modifier(vk) != 0 || !down {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
    }
    if !shared.capture_enabled.load(Ordering::Acquire) || !shared.enabled.load(Ordering::Acquire) {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
    }

    let target = crate::transport::canonical_target(focused_target());
    let active = shared.active.load(Ordering::Acquire);
    let active_target = shared.active_target.load(Ordering::Acquire);
    if active && active_target != target as usize {
        shared.epoch.fetch_add(1, Ordering::AcqRel);
        shared.active.store(false, Ordering::Release);
        shared.active_target.store(0, Ordering::Release);
        unsafe {
            SetEvent(shared.work as HANDLE);
        }
    }

    let active = shared.active.load(Ordering::Acquire);
    if active && vk == VK_SPACE as u32 && mods == INPUT_MOD_SHIFT {
        if push(shared, EventKind::RawBoundary, vk, mods, target) {
            mark_suppressed_up(shared, vk);
            return 1;
        }
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
    }

    if let Some(ch) = printable_char(vk, mods) {
        if active || accepts_char(shared, ch) {
            if !active
                && !crate::transport::may_support_text(
                    target,
                    shared.automation_ready.load(Ordering::Acquire),
                )
            {
                return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
            }
            if push(shared, EventKind::TypeChar, vk, mods, target) {
                shared.active.store(true, Ordering::Release);
                shared
                    .active_target
                    .store(target as usize, Ordering::Release);
                mark_suppressed_up(shared, vk);
                return 1;
            }
            return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
        }
    }

    if !active {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) };
    }

    if vk == VK_BACK as u32 {
        if push(shared, EventKind::Backspace, vk, mods, target) {
            mark_suppressed_up(shared, vk);
            return 1;
        }
    } else if vk == VK_ESCAPE as u32 {
        if push(shared, EventKind::Escape, vk, mods, target) {
            mark_suppressed_up(shared, vk);
            return 1;
        }
    } else if printable_char(vk, mods).is_some() {
        if push(shared, EventKind::FinalizeWithDelimiter, vk, mods, target) {
            mark_suppressed_up(shared, vk);
            return 1;
        }
    } else if natural_boundary_key(vk) {
        if push(shared, EventKind::NaturalBoundary, vk, mods, target) {
            shared.active.store(false, Ordering::Release);
            shared.active_target.store(0, Ordering::Release);
        }
    } else {
        shared.epoch.fetch_add(1, Ordering::AcqRel);
        shared.active.store(false, Ordering::Release);
        shared.active_target.store(0, Ordering::Release);
        unsafe {
            SetEvent(shared.work as HANDLE);
        }
    }

    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wp, lp) }
}

fn apply_settings(
    mut config: EngineConfig,
    settings: &inputkey_windows_settings::Settings,
) -> EngineConfig {
    config.language_id = settings.language.clone();
    config.language.method = settings.method.clone();
    config
        .language
        .toggles
        .insert("auto_restore".into(), settings.auto_restore);
    config
        .language
        .toggles
        .insert("smart_correction".into(), settings.smart_correction);
    config
}

fn runtime_config(shared: &Shared) -> EngineConfig {
    shared
        .engine_config
        .lock()
        .expect("engine config lock")
        .clone()
}

fn update_accepted(shared: &Shared, engine: &dyn inputkey_core_abstractions::TypingEnginePort) {
    let mut words = [0u64; 2];
    for value in 0x20u8..=0x7e {
        let ch = value as char;
        if engine.accepts_key(ch) {
            words[(value / 64) as usize] |= 1u64 << (value % 64);
        }
    }
    for (slot, value) in shared.accepted_ascii.iter().zip(words) {
        slot.store(value, Ordering::Release);
    }
}

fn worker(shared: Arc<Shared>, factory: EngineFactory) {
    let clipboard = ClipboardPaste::new().ok();
    let automation = crate::uia::AutomationText::new().ok();
    shared
        .automation_ready
        .store(automation.is_some(), Ordering::Release);
    let mut engine = factory(runtime_config(&shared));
    update_accepted(&shared, engine.as_ref());
    let mut owned: Option<crate::transport::OwnedTransport> = None;
    let mut seen_epoch = shared.epoch.load(Ordering::Acquire);

    loop {
        unsafe {
            WaitForSingleObject(shared.work as HANDLE, INFINITE);
        }
        if shared.stop.load(Ordering::Acquire) {
            break;
        }

        let current_epoch = shared.epoch.load(Ordering::Acquire);
        if current_epoch != seen_epoch {
            engine = factory(runtime_config(&shared));
            update_accepted(&shared, engine.as_ref());
            owned = None;
            shared.active.store(false, Ordering::Release);
            shared.active_target.store(0, Ordering::Release);
            seen_epoch = current_epoch;
        }

        while let Some(event) = shared.queue.pop() {
            if event.epoch != shared.epoch.load(Ordering::Acquire) {
                continue;
            }
            let target = event.target as HWND;

            match event.kind {
                EventKind::TypeChar => {
                    let Some(ch) = printable_char(event.vk as u32, event.modifiers) else {
                        continue;
                    };

                    if owned.is_none() {
                        match crate::transport::capture(target, automation.as_ref()) {
                            crate::transport::Capture::Ready(transport) => {
                                owned = Some(transport);
                            }
                            crate::transport::Capture::Unsupported
                            | crate::transport::Capture::Denied => {
                                let literal = ch.to_string();
                                let _ = crate::synthetic::replay_literal(target, &literal);
                                engine.reset();
                                shared.active.store(false, Ordering::Release);
                                shared.active_target.store(0, Ordering::Release);
                                continue;
                            }
                        }
                    }

                    if owned
                        .as_ref()
                        .is_some_and(|range| range.target() != target as usize)
                    {
                        engine.reset();
                        owned = None;
                        shared.active.store(false, Ordering::Release);
                        shared.active_target.store(0, Ordering::Release);
                        let literal = ch.to_string();
                        let _ = crate::synthetic::replay_literal(target, &literal);
                        continue;
                    }

                    let text = if engine.accepts_key(ch) {
                        engine.type_key(ch)
                    } else {
                        engine.decision_boundary(ch)
                    };
                    let remains_active = engine.history_active();
                    let ok = owned.as_mut().is_some_and(|range| {
                        range.replace(automation.as_ref(), clipboard.as_ref(), &text)
                    });
                    if !ok {
                        // An owned transport may have mutated the target before its
                        // post-write verification failed. The physical key was already
                        // suppressed by the hook, so replaying it here can duplicate one
                        // keystroke into two. Drop ownership and let the next physical
                        // event start a fresh composition instead.
                        engine.reset();
                        owned = None;
                        shared.active.store(false, Ordering::Release);
                        shared.active_target.store(0, Ordering::Release);
                    } else if remains_active {
                        shared.active.store(true, Ordering::Release);
                    } else {
                        owned = None;
                        shared.active.store(false, Ordering::Release);
                        shared.active_target.store(0, Ordering::Release);
                    }
                }
                EventKind::Backspace | EventKind::Escape => {
                    if let Some(text) = transition::apply(engine.as_mut(), event.kind, None) {
                        let ok = owned.as_mut().is_some_and(|range| {
                            range.replace(automation.as_ref(), clipboard.as_ref(), &text)
                        });
                        if !ok {
                            engine.reset();
                            owned = None;
                            shared.capture_enabled.store(false, Ordering::Release);
                            shared.active.store(false, Ordering::Release);
                            shared.active_target.store(0, Ordering::Release);
                        } else if !engine.history_active() {
                            owned = None;
                            shared.active.store(false, Ordering::Release);
                            shared.active_target.store(0, Ordering::Release);
                        }
                    }
                }
                EventKind::FinalizeWithDelimiter | EventKind::RawBoundary => {
                    let delimiter = (event.kind == EventKind::FinalizeWithDelimiter)
                        .then(|| printable_char(event.vk as u32, event.modifiers))
                        .flatten();
                    let text = transition::apply(engine.as_mut(), event.kind, delimiter)
                        .unwrap_or_default();
                    if let Some(range) = owned.as_mut() {
                        let _ = range.replace(automation.as_ref(), clipboard.as_ref(), &text);
                    }
                    owned = None;
                    shared.active.store(false, Ordering::Release);
                    shared.active_target.store(0, Ordering::Release);
                }
                EventKind::NaturalBoundary | EventKind::MouseBoundary => {
                    let _ = transition::apply(engine.as_mut(), event.kind, None);
                    owned = None;
                    shared.active.store(false, Ordering::Release);
                    shared.active_target.store(0, Ordering::Release);
                }
                EventKind::ResetOnly => {
                    transition::apply(engine.as_mut(), event.kind, None);
                    owned = None;
                    shared.active.store(false, Ordering::Release);
                    shared.active_target.store(0, Ordering::Release);
                }
                EventKind::Pass => {}
            }
            update_accepted(&shared, engine.as_ref());
        }
    }
}

pub struct CompatibilityHandle {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
    hook: Option<JoinHandle<()>>,
}

impl CompatibilityHandle {
    pub fn set_capture_enabled(&self, enabled: bool) {
        self.shared
            .capture_enabled
            .store(enabled, Ordering::Release);
        if !enabled {
            self.shared.epoch.fetch_add(1, Ordering::AcqRel);
            self.shared.active.store(false, Ordering::Release);
            self.shared.active_target.store(0, Ordering::Release);
            unsafe {
                SetEvent(self.shared.work as HANDLE);
            }
        }
    }

    pub fn reload_settings(&self) {
        let settings = inputkey_windows_settings::load();
        self.shared
            .enabled
            .store(settings.enabled, Ordering::Release);
        if let Ok(mut config) = self.shared.engine_config.lock() {
            *config = apply_settings(config.clone(), &settings);
        }
        self.shared.epoch.fetch_add(1, Ordering::AcqRel);
        self.shared.active.store(false, Ordering::Release);
        self.shared.active_target.store(0, Ordering::Release);
        unsafe {
            SetEvent(self.shared.work as HANDLE);
        }
    }
}

impl Drop for CompatibilityHandle {
    fn drop(&mut self) {
        self.shared.capture_enabled.store(false, Ordering::Release);
        self.shared.stop.store(true, Ordering::Release);
        self.shared.epoch.fetch_add(1, Ordering::AcqRel);
        unsafe {
            SetEvent(self.shared.work as HANDLE);
        }

        let hook_thread_id = self.shared.hook_thread_id.load(Ordering::Acquire);
        if hook_thread_id != 0 {
            unsafe {
                PostThreadMessageW(hook_thread_id, WM_QUIT, 0, 0);
            }
        }
        if let Some(hook) = self.hook.take() {
            let _ = hook.join();
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }

        SHARED.store(std::ptr::null_mut(), Ordering::Release);
        unsafe {
            CloseHandle(self.shared.work as HANDLE);
        }
    }
}

pub fn start(config: Config, factory: EngineFactory) -> Result<CompatibilityHandle, String> {
    if !SHARED.load(Ordering::Acquire).is_null() {
        return Err("Windows compatibility input is already running".into());
    }

    let settings = inputkey_windows_settings::load();
    let work = unsafe { CreateEventW(std::ptr::null(), 0, 0, std::ptr::null()) };
    if work.is_null() {
        return Err("CreateEventW(work) failed".into());
    }
    let ready = unsafe { CreateEventW(std::ptr::null(), 0, 0, std::ptr::null()) };
    if ready.is_null() {
        unsafe {
            CloseHandle(work);
        }
        return Err("CreateEventW(ready) failed".into());
    }

    let initial_engine_config = apply_settings(config.engine.clone(), &settings);
    let shared = Arc::new(Shared {
        queue: SpscRing::new(),
        work: work as usize,
        capture_enabled: AtomicBool::new(config.capture_enabled),
        enabled: AtomicBool::new(settings.enabled),
        active: AtomicBool::new(false),
        active_target: AtomicUsize::new(0),
        epoch: AtomicU64::new(0),
        modifiers: AtomicU8::new(0),
        engine_config: Mutex::new(initial_engine_config),
        accepted_ascii: std::array::from_fn(|_| AtomicU64::new(0)),
        suppress_up: std::array::from_fn(|_| AtomicU64::new(0)),
        stop: AtomicBool::new(false),
        hook_thread_id: AtomicU32::new(0),
        installed: AtomicBool::new(false),
        automation_ready: AtomicBool::new(false),
    });

    SHARED.store(Arc::as_ptr(&shared) as *mut Shared, Ordering::Release);

    let worker_shared = Arc::clone(&shared);
    let worker_factory = Arc::clone(&factory);
    let worker = thread::spawn(move || worker(worker_shared, worker_factory));

    let hook_shared = Arc::clone(&shared);
    let ready_handle = ready as usize;
    let hook = thread::spawn(move || {
        let thread_id = unsafe { GetCurrentThreadId() };
        hook_shared
            .hook_thread_id
            .store(thread_id, Ordering::Release);
        let mut bootstrap: MSG = unsafe { std::mem::zeroed() };
        unsafe {
            PeekMessageW(&mut bootstrap, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
        }
        let module = unsafe { GetModuleHandleW(std::ptr::null()) };
        let keyboard_hook =
            unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0) };
        if keyboard_hook.is_null() {
            unsafe {
                SetEvent(ready_handle as HANDLE);
            }
            return;
        }
        let mouse_hook = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0) };
        if mouse_hook.is_null() {
            unsafe {
                UnhookWindowsHookEx(keyboard_hook);
                SetEvent(ready_handle as HANDLE);
            }
            return;
        }
        hook_shared.installed.store(true, Ordering::Release);
        unsafe {
            SetEvent(ready_handle as HANDLE);
        }

        let mut message: MSG = unsafe { std::mem::zeroed() };
        while unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) } > 0 {
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        unsafe {
            UnhookWindowsHookEx(mouse_hook);
            UnhookWindowsHookEx(keyboard_hook);
        }
    });

    unsafe {
        WaitForSingleObject(ready, INFINITE);
        CloseHandle(ready);
    }

    if !shared.installed.load(Ordering::Acquire) {
        shared.stop.store(true, Ordering::Release);
        unsafe {
            SetEvent(work);
        }
        let _ = hook.join();
        let _ = worker.join();
        SHARED.store(std::ptr::null_mut(), Ordering::Release);
        unsafe {
            CloseHandle(work);
        }
        return Err("SetWindowsHookExW failed".into());
    }

    Ok(CompatibilityHandle {
        shared,
        worker: Some(worker),
        hook: Some(hook),
    })
}
