//! Low level keyboard capture and tray shell. The callback only classifies keys;
//! all engine calls and injected input are owned by the worker.
use crate::{
    settings, startup, transition, Config, EngineFactory, Event, EventKind, Modifiers, SpscRing,
};
use inputkey_core_abstractions::TypingEnginePort;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::thread;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const EXTRA: usize = 0x494b_4559;
const WAKE: u32 = WM_APP + 7;
const TRAY: u32 = WM_APP + 8;
const SHORTCUT_SAVED: u32 = WM_APP + 9;
const ID_TOGGLE: usize = 101;
const ID_STARTUP: usize = 104;
const ID_RECORD: usize = 102;
const ID_ENABLE_LITERAL: usize = 105;
const ID_RESET: usize = 103;
const ID_EXIT: usize = 100;
static mut QUEUE: *const SpscRing<Event, 256> = std::ptr::null();
static mut EPOCH: *const AtomicU64 = std::ptr::null();
static mut ENABLED: *const AtomicBool = std::ptr::null();
static mut ACTIVE: *const AtomicBool = std::ptr::null();
static mut MODS: *const AtomicU8 = std::ptr::null();
static mut RECORD: *const AtomicBool = std::ptr::null();
static mut LIT_ENABLED: *const AtomicBool = std::ptr::null();
static mut LIT_VK: *const AtomicU32 = std::ptr::null();
static mut LIT_MODS: *const AtomicU8 = std::ptr::null();
static mut SUPPRESS_UP: *const [AtomicU64; 4] = std::ptr::null();
static mut WORK: HANDLE = 0 as HANDLE;
static mut HOOK_ID: u32 = 0;
static mut TRAY_WINDOW: HWND = std::ptr::null_mut();
static mut ICON_V: HICON = std::ptr::null_mut();
static mut ICON_E: HICON = std::ptr::null_mut();

fn modifier(vk: u32) -> u8 {
    match vk {
        0x10 | 0xa0 | 0xa1 => Modifiers::SHIFT,
        0x11 | 0xa2 | 0xa3 => Modifiers::CTRL,
        0x12 | 0xa4 | 0xa5 => Modifiers::ALT,
        0x5b | 0x5c => Modifiers::WIN,
        _ => 0,
    }
}
fn update_modifiers(vk: u32, down: bool) -> u8 {
    let bit = modifier(vk);
    if bit == 0 {
        return unsafe { (&*MODS).load(Ordering::Acquire) };
    };
    let m = unsafe { &*MODS };
    if down {
        m.fetch_or(bit, Ordering::AcqRel);
    } else {
        m.fetch_and(!bit, Ordering::AcqRel);
    }
    m.load(Ordering::Acquire)
}
fn mark_suppressed_up(vk: u32) {
    if vk < 256 {
        unsafe {
            (&*SUPPRESS_UP)[(vk / 64) as usize].fetch_or(1u64 << (vk % 64), Ordering::AcqRel);
        }
    }
}
fn take_suppressed_up(vk: u32) -> bool {
    vk < 256
        && unsafe {
            (&*SUPPRESS_UP)[(vk / 64) as usize].fetch_and(!(1u64 << (vk % 64)), Ordering::AcqRel)
                & (1u64 << (vk % 64))
                != 0
        }
}
fn push(kind: EventKind, data: &KBDLLHOOKSTRUCT, mods: u8, down: bool) -> bool {
    unsafe {
        let q = &*QUEUE;
        let e = &*EPOCH;
        let event = Event {
            kind,
            vk: data.vkCode as u16,
            modifiers: mods,
            target: GetForegroundWindow() as usize,
            epoch: e.load(Ordering::Acquire),
            down,
            scan_code: data.scanCode as u16,
            extended: data.flags & LLKHF_EXTENDED != 0,
        };
        if !q.push(event) {
            e.fetch_add(1, Ordering::AcqRel);
            SetEvent(WORK);
            false
        } else {
            SetEvent(WORK);
            true
        }
    }
}
unsafe extern "system" fn keyboard_proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code < 0 {
        return unsafe { CallNextHookEx(0 as HHOOK, code, wp, lp) };
    };
    let data = unsafe { &*(lp as *const KBDLLHOOKSTRUCT) };
    if data.dwExtraInfo == EXTRA {
        return unsafe { CallNextHookEx(0 as HHOOK, code, wp, lp) };
    };
    let msg = wp as u32;
    let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
    if !down && !up {
        return unsafe { CallNextHookEx(0 as HHOOK, code, wp, lp) };
    };
    let mods = update_modifiers(data.vkCode, down);
    let vk = data.vkCode;
    if up && take_suppressed_up(vk) {
        return 1;
    }
    if !unsafe { (&*ENABLED).load(Ordering::Acquire) } {
        return unsafe { CallNextHookEx(0 as HHOOK, code, wp, lp) };
    };
    if unsafe { (&*RECORD).load(Ordering::Acquire) } && down {
        if modifier(vk) != 0 {
            return unsafe { CallNextHookEx(0 as HHOOK, code, wp, lp) };
        };
        if vk == 0x1b {
            unsafe {
                (&*RECORD).store(false, Ordering::Release);
                mark_suppressed_up(vk);
            }
            return 1;
        }
        if mods != 0 {
            return if push(EventKind::RecordShortcut, data, mods, true) {
                mark_suppressed_up(vk);
                1
            } else {
                unsafe { CallNextHookEx(0 as HHOOK, code, wp, lp) }
            };
        }
        return unsafe { CallNextHookEx(0 as HHOOK, code, wp, lp) };
    }
    if modifier(vk) != 0 {
        return unsafe { CallNextHookEx(0 as HHOOK, code, wp, lp) };
    };
    if down
        && unsafe { (&*LIT_ENABLED).load(Ordering::Acquire) }
        && vk == unsafe { (&*LIT_VK).load(Ordering::Acquire) }
        && mods == unsafe { (&*LIT_MODS).load(Ordering::Acquire) }
    {
        return if push(EventKind::Literalize, data, mods, true) {
            mark_suppressed_up(vk);
            1
        } else {
            unsafe { CallNextHookEx(0 as HHOOK, code, wp, lp) }
        };
    }
    let active = unsafe { (&*ACTIVE).load(Ordering::Acquire) };
    let plain = mods & (Modifiers::CTRL | Modifiers::ALT | Modifiers::WIN) == 0;
    if down && active && !plain {
        push(EventKind::ResetOnly, data, mods, true);
        return unsafe { CallNextHookEx(0 as HHOOK, code, wp, lp) };
    }
    let token = (0x41..=0x5a).contains(&vk)
        || (mods & Modifiers::SHIFT == 0
            && ((0x30..=0x39).contains(&vk) || vk == 0xdb || vk == 0xdd));
    let delimiter = matches!(vk, 0xba..=0xc0 | 0xdb..=0xde | 0x20 | 0x0d | 0x09);
    let kind = if plain && token {
        Some(EventKind::TypeChar)
    } else if active && vk == VK_BACK as u32 {
        Some(EventKind::Backspace)
    } else if active && vk == VK_ESCAPE as u32 {
        Some(EventKind::Escape)
    } else if active
        && plain
        && (delimiter
            || (mods & Modifiers::SHIFT != 0
                && ((0x30..=0x39).contains(&vk) || vk == 0xdb || vk == 0xdd)))
    {
        Some(EventKind::FinalizeAndReplay)
    } else if active
        && (vk == VK_DELETE as u32
            || vk == VK_HOME as u32
            || vk == VK_END as u32
            || (0x21..=0x28).contains(&vk))
    {
        Some(EventKind::ResetAndReplayKeepDisplayed)
    } else {
        None
    };
    if let Some(k) = kind {
        if down {
            if push(k, data, mods, true) {
                mark_suppressed_up(vk);
                return 1;
            }
            return unsafe { CallNextHookEx(0 as HHOOK, code, wp, lp) };
        }
        // A key-up is consumed only if its key-down was successfully queued.
    }
    unsafe { CallNextHookEx(0 as HHOOK, code, wp, lp) }
}

fn inject(text: &str, erase: usize) {
    let mut v = Vec::new();
    for _ in 0..erase {
        for f in [0, KEYEVENTF_KEYUP as u16] {
            v.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_BACK,
                        wScan: 0,
                        dwFlags: f as u32,
                        time: 0,
                        dwExtraInfo: EXTRA,
                    },
                },
            });
        }
    }
    for c in text.encode_utf16() {
        for f in [KEYEVENTF_UNICODE, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP] {
            v.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: 0,
                        wScan: c,
                        dwFlags: f,
                        time: 0,
                        dwExtraInfo: EXTRA,
                    },
                },
            });
        }
    }
    if !v.is_empty() {
        unsafe {
            SendInput(
                v.len() as u32,
                v.as_ptr(),
                std::mem::size_of::<INPUT>() as i32,
            );
        }
    }
}
fn key_char(v: u16, mods: u8) -> Option<char> {
    match v as u32 {
        0x41..=0x5a => Some(if mods & Modifiers::SHIFT != 0 {
            v as u8 as char
        } else {
            (v as u8 + 32) as char
        }),
        0x30..=0x39 => Some(v as u8 as char),
        0xdb => Some('['),
        0xdd => Some(']'),
        _ => None,
    }
}
fn worker(
    mut engine: Box<dyn TypingEnginePort>,
    q: Arc<SpscRing<Event, 256>>,
    epoch: Arc<AtomicU64>,
    active: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    work: HANDLE,
) {
    let mut rendered = String::new();
    let mut seen_epoch = epoch.load(Ordering::Acquire);
    loop {
        unsafe {
            WaitForMultipleObjects(1, &work, 0, INFINITE);
        }
        if stop.load(Ordering::Acquire) {
            break;
        }
        let current_epoch = epoch.load(Ordering::Acquire);
        if current_epoch != seen_epoch {
            engine.reset();
            rendered.clear();
            active.store(false, Ordering::Release);
            seen_epoch = current_epoch;
        }
        while let Some(e) = q.pop() {
            if e.epoch != epoch.load(Ordering::Acquire) {
                engine.reset();
                rendered.clear();
                active.store(false, Ordering::Release);
                seen_epoch = epoch.load(Ordering::Acquire);
                continue;
            }
            if !e.down {
                continue;
            }
            if matches!(
                e.kind,
                EventKind::TypeChar
                    | EventKind::Backspace
                    | EventKind::Escape
                    | EventKind::Literalize
                    | EventKind::FinalizeAndReplay
                    | EventKind::ResetAndReplayKeepDisplayed
            ) && unsafe { GetForegroundWindow() as usize } != e.target
            {
                engine.reset();
                rendered.clear();
                active.store(false, Ordering::Release);
                continue;
            }
            let old = rendered.chars().count();
            match e.kind {
                EventKind::TypeChar
                | EventKind::Backspace
                | EventKind::Escape
                | EventKind::Literalize => {
                    if let Some(out) =
                        transition::apply(engine.as_mut(), e.kind, key_char(e.vk, e.modifiers))
                    {
                        rendered = out;
                        inject(&rendered, old);
                        active.store(engine.history_active(), Ordering::Release);
                    }
                }
                EventKind::FinalizeAndReplay => {
                    let out = transition::apply(engine.as_mut(), e.kind, None).unwrap_or_default();
                    inject(&out, old);
                    rendered.clear();
                    active.store(false, Ordering::Release);
                    replay(&e)
                }
                EventKind::ResetAndReplayKeepDisplayed => {
                    transition::apply(engine.as_mut(), e.kind, None);
                    rendered.clear();
                    active.store(false, Ordering::Release);
                    replay(&e)
                }
                EventKind::ResetOnly => {
                    transition::apply(engine.as_mut(), e.kind, None);
                    rendered.clear();
                    active.store(false, Ordering::Release);
                }
                EventKind::RecordShortcut => unsafe {
                    (&*LIT_VK).store(e.vk as u32, Ordering::Release);
                    (&*LIT_MODS).store(e.modifiers, Ordering::Release);
                    (&*RECORD).store(false, Ordering::Release);
                    PostMessageW(TRAY_WINDOW, SHORTCUT_SAVED, 0, 0);
                },
                _ => {}
            }
        }
    }
}
fn replay(e: &Event) {
    if modifier(e.vk as u32) != 0 {
        return;
    }
    unsafe {
        let mut inputs = [INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: e.vk,
                    wScan: e.scan_code,
                    dwFlags: if e.extended { KEYEVENTF_EXTENDEDKEY } else { 0 },
                    time: 0,
                    dwExtraInfo: EXTRA,
                },
            },
        }; 2];
        inputs[1].Anonymous.ki.dwFlags |= KEYEVENTF_KEYUP;
        SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
unsafe fn save_shortcut() {
    unsafe {
        settings::save(settings::Settings {
            enabled: (&*LIT_ENABLED).load(Ordering::Acquire),
            vk: (&*LIT_VK).load(Ordering::Acquire),
            modifiers: (&*LIT_MODS).load(Ordering::Acquire),
        });
    }
}
unsafe fn update_tray(hwnd: HWND) {
    let enabled = unsafe { (&*ENABLED).load(Ordering::Acquire) };
    let mut ni: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    ni.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    ni.hWnd = hwnd;
    ni.uID = 1;
    ni.uFlags = NIF_ICON | NIF_TIP;
    ni.hIcon = unsafe {
        if enabled {
            ICON_V
        } else {
            ICON_E
        }
    };
    let tip = if enabled {
        "InputKey v5.0.0 — V"
    } else {
        "InputKey v5.0.0 — E"
    };
    for (slot, c) in ni.szTip.iter_mut().zip(wide(tip)) {
        *slot = c;
    }
    unsafe {
        Shell_NotifyIconW(NIM_MODIFY, &ni);
    }
}
fn shortcut_label(vk: u32, mods: u8) -> String {
    let mut s = String::from("Literalize shortcut: ");
    for (bit, name) in [
        (Modifiers::CTRL, "Ctrl+"),
        (Modifiers::ALT, "Alt+"),
        (Modifiers::SHIFT, "Shift+"),
        (Modifiers::WIN, "Win+"),
    ] {
        if mods & bit != 0 {
            s.push_str(name);
        }
    }
    match vk {
        0x20 => s.push_str("Space"),
        0x41..=0x5a => s.push(char::from_u32(vk).unwrap_or('?')),
        _ => s.push_str(&format!("VK {vk:02X}")),
    }
    s
}
unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WAKE => 0,
        SHORTCUT_SAVED => {
            unsafe {
                save_shortcut();
            }
            0
        }
        TRAY if lp as u32 == WM_RBUTTONUP || lp as u32 == WM_CONTEXTMENU => {
            let m = unsafe { CreatePopupMenu() };
            unsafe {
                AppendMenuW(
                    m,
                    MF_STRING | MF_GRAYED,
                    0,
                    wide("InputKey v5.0.0").as_ptr(),
                );
                AppendMenuW(m, MF_SEPARATOR, 0, std::ptr::null());
                let checked = |state: bool| if state { MF_CHECKED } else { MF_UNCHECKED };
                AppendMenuW(
                    m,
                    MF_STRING | checked((&*ENABLED).load(Ordering::Acquire)),
                    ID_TOGGLE,
                    wide("Vietnamese enabled").as_ptr(),
                );
                AppendMenuW(
                    m,
                    MF_STRING | checked(startup::is_enabled()),
                    ID_STARTUP,
                    wide("Start with Windows").as_ptr(),
                );
                AppendMenuW(
                    m,
                    MF_STRING,
                    ID_RECORD,
                    wide(&shortcut_label(
                        (&*LIT_VK).load(Ordering::Acquire),
                        (&*LIT_MODS).load(Ordering::Acquire),
                    ))
                    .as_ptr(),
                );
                AppendMenuW(
                    m,
                    MF_STRING | checked((&*LIT_ENABLED).load(Ordering::Acquire)),
                    ID_ENABLE_LITERAL,
                    wide("Enable literalize").as_ptr(),
                );
                AppendMenuW(
                    m,
                    MF_STRING,
                    ID_RESET,
                    wide("Reset shortcut to Ctrl+Space").as_ptr(),
                );
                AppendMenuW(m, MF_STRING, ID_EXIT, wide("Exit").as_ptr());
                let mut p = POINT { x: 0, y: 0 };
                GetCursorPos(&mut p);
                SetForegroundWindow(hwnd);
                TrackPopupMenu(m, TPM_RIGHTBUTTON, p.x, p.y, 0, hwnd, std::ptr::null());
                DestroyMenu(m);
            }
            0
        }
        WM_COMMAND => {
            match wp {
                ID_TOGGLE => unsafe {
                    (&*ENABLED).fetch_xor(true, Ordering::AcqRel);
                    (&*EPOCH).fetch_add(1, Ordering::AcqRel);
                    SetEvent(WORK);
                    update_tray(hwnd);
                },
                ID_STARTUP => startup::set_enabled(!startup::is_enabled()),
                ID_RECORD => unsafe {
                    (&*RECORD).store(true, Ordering::Release);
                },
                ID_ENABLE_LITERAL => unsafe {
                    (&*LIT_ENABLED).fetch_xor(true, Ordering::AcqRel);
                    save_shortcut();
                },
                ID_RESET => unsafe {
                    (&*LIT_VK).store(0x20, Ordering::Release);
                    (&*LIT_MODS).store(Modifiers::CTRL, Ordering::Release);
                    save_shortcut();
                },
                ID_EXIT => unsafe {
                    PostQuitMessage(0);
                },
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            unsafe {
                PostQuitMessage(0);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}
pub fn run(config: Config, factory: EngineFactory) {
    let q = Arc::new(SpscRing::<Event, 256>::new());
    let epoch = Arc::new(AtomicU64::new(0));
    let enabled = Arc::new(AtomicBool::new(config.enabled));
    let active = Arc::new(AtomicBool::new(false));
    let stop = Arc::new(AtomicBool::new(false));
    let mods = Arc::new(AtomicU8::new(0));
    let record = Arc::new(AtomicBool::new(false));
    let saved = settings::load();
    let lit_enabled = Arc::new(AtomicBool::new(saved.enabled));
    let lit_vk = Arc::new(AtomicU32::new(saved.vk));
    let lit_mods = Arc::new(AtomicU8::new(saved.modifiers));
    let suppress_up = Arc::new(std::array::from_fn(|_| AtomicU64::new(0)));
    let work = unsafe { CreateEventW(std::ptr::null(), 0, 0, std::ptr::null()) };
    let ready = unsafe { CreateEventW(std::ptr::null(), 0, 0, std::ptr::null()) };
    let installed = Arc::new(AtomicBool::new(false));
    unsafe {
        WORK = work;
        QUEUE = Arc::as_ptr(&q);
        EPOCH = Arc::as_ptr(&epoch);
        ENABLED = Arc::as_ptr(&enabled);
        ACTIVE = Arc::as_ptr(&active);
        MODS = Arc::as_ptr(&mods);
        RECORD = Arc::as_ptr(&record);
        LIT_ENABLED = Arc::as_ptr(&lit_enabled);
        LIT_VK = Arc::as_ptr(&lit_vk);
        LIT_MODS = Arc::as_ptr(&lit_mods);
        SUPPRESS_UP = Arc::as_ptr(&suppress_up);
    }
    let wq = q.clone();
    let we = epoch.clone();
    let wa = active.clone();
    let ws = stop.clone();
    let work_handle = work as usize;
    let worker = thread::spawn(move || worker(factory(), wq, we, wa, ws, work_handle as HANDLE));
    let inst = unsafe { GetModuleHandleW(std::ptr::null()) };
    let cn = wide("InputKeyTrayWindow");
    let wc = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: inst,
        lpszClassName: cn.as_ptr(),
        ..unsafe { std::mem::zeroed() }
    };
    unsafe {
        RegisterClassW(&wc);
    }
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            cn.as_ptr(),
            wide("InputKey").as_ptr(),
            0,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            0 as HMENU,
            inst,
            std::ptr::null(),
        )
    };
    unsafe {
        TRAY_WINDOW = hwnd;
    }
    let icon_path = |name: &str| {
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join(name)))
    };
    unsafe {
        if let Some(path) = icon_path("inputkey-v.ico") {
            ICON_V = LoadImageW(
                std::ptr::null_mut(),
                wide(&path.to_string_lossy()).as_ptr(),
                IMAGE_ICON,
                0,
                0,
                LR_LOADFROMFILE,
            ) as HICON;
        }
        if let Some(path) = icon_path("inputkey-e.ico") {
            ICON_E = LoadImageW(
                std::ptr::null_mut(),
                wide(&path.to_string_lossy()).as_ptr(),
                IMAGE_ICON,
                0,
                0,
                LR_LOADFROMFILE,
            ) as HICON;
        }
    }
    let mut ni: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    ni.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    ni.hWnd = hwnd;
    ni.uID = 1;
    ni.uFlags = NIF_MESSAGE | NIF_TIP | NIF_ICON;
    ni.uCallbackMessage = TRAY;
    ni.hIcon = unsafe {
        if config.enabled {
            ICON_V
        } else {
            ICON_E
        }
    };
    for (i, c) in wide(if config.enabled {
        "InputKey v5.0.0 — V"
    } else {
        "InputKey v5.0.0 — E"
    })
    .iter()
    .enumerate()
    .take(ni.szTip.len() - 1)
    {
        ni.szTip[i] = *c;
    }
    unsafe {
        Shell_NotifyIconW(NIM_ADD, &ni);
    }
    let instance_handle = inst as usize;
    let ready_handle = ready as usize;
    let hook_installed = installed.clone();
    let hook = thread::spawn(move || {
        let id = unsafe { GetCurrentThreadId() };
        unsafe {
            HOOK_ID = id;
        }
        let mut initial: MSG = unsafe { std::mem::zeroed() };
        unsafe {
            PeekMessageW(&mut initial, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
        }
        let h = unsafe {
            SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), instance_handle as _, 0)
        };
        if h.is_null() {
            unsafe {
                SetEvent(ready_handle as HANDLE);
            }
            return;
        }
        hook_installed.store(true, Ordering::Release);
        unsafe {
            SetEvent(ready_handle as HANDLE);
        }
        let mut m: MSG = unsafe { std::mem::zeroed() };
        while unsafe { GetMessageW(&mut m, 0 as HWND, 0, 0) } > 0 {
            unsafe {
                TranslateMessage(&m);
                DispatchMessageW(&m);
            }
        }
        unsafe {
            UnhookWindowsHookEx(h);
        }
    });
    unsafe {
        WaitForSingleObject(ready, INFINITE);
    }
    if !installed.load(Ordering::Acquire) {
        unsafe {
            PostMessageW(hwnd, WM_CLOSE, 0, 0);
        }
    }
    let mut m: MSG = unsafe { std::mem::zeroed() };
    while unsafe { GetMessageW(&mut m, 0 as HWND, 0, 0) } > 0 {
        unsafe {
            TranslateMessage(&m);
            DispatchMessageW(&m);
        }
    }
    enabled.store(false, Ordering::Release);
    epoch.fetch_add(1, Ordering::AcqRel);
    unsafe {
        SetEvent(work);
        Shell_NotifyIconW(NIM_DELETE, &ni);
        if installed.load(Ordering::Acquire) {
            PostThreadMessageW(HOOK_ID, WM_QUIT, 0, 0);
        }
    }
    let _ = hook.join();
    stop.store(true, Ordering::Release);
    unsafe {
        SetEvent(work);
    }
    let _ = worker.join();
    unsafe {
        DestroyWindow(hwnd);
        if !ICON_V.is_null() {
            DestroyIcon(ICON_V);
        }
        if !ICON_E.is_null() {
            DestroyIcon(ICON_E);
        }
        CloseHandle(work);
        CloseHandle(ready);
        QUEUE = std::ptr::null();
        EPOCH = std::ptr::null();
        ENABLED = std::ptr::null();
        ACTIVE = std::ptr::null();
        MODS = std::ptr::null();
        RECORD = std::ptr::null();
        LIT_ENABLED = std::ptr::null();
        LIT_VK = std::ptr::null();
        LIT_MODS = std::ptr::null();
        SUPPRESS_UP = std::ptr::null();
        WORK = std::ptr::null_mut();
        HOOK_ID = 0;
        TRAY_WINDOW = std::ptr::null_mut();
        ICON_V = std::ptr::null_mut();
        ICON_E = std::ptr::null_mut();
    }
}
