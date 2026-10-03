use crate::ControlActions;
use inputkey_core_abstractions::LanguageMetadata;
use inputkey_windows_settings as settings;
use std::sync::OnceLock;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::{GetStockObject, DEFAULT_GUI_FONT, HBRUSH};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const CLASS_NAME: &str = "InputKeySettingsWindow";
pub const SETTINGS_CHANGED_MESSAGE: u32 = WM_APP + 12;

const ID_ENABLED: i32 = 4101;
const ID_LANGUAGE: i32 = 4102;
const ID_METHOD: i32 = 4103;
const ID_SIMPLE_TELEX: i32 = 4104;
const ID_AUTO_RESTORE: i32 = 4105;
const ID_SMART_CORRECTION: i32 = 4106;
const ID_CLOSE: i32 = 4107;

const INPUTKEY_BST_UNCHECKED: u32 = 0;
const INPUTKEY_BST_CHECKED: u32 = 1;
const INPUTKEY_COLOR_WINDOW: usize = 5;

static ACTIONS: OnceLock<ControlActions> = OnceLock::new();
static mut WINDOW: HWND = std::ptr::null_mut();
static mut OWNER: HWND = std::ptr::null_mut();

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn languages() -> Vec<LanguageMetadata> {
    ACTIONS
        .get()
        .map(|actions| (actions.languages)())
        .unwrap_or_default()
}

fn active_language<'a>(
    all: &'a [LanguageMetadata],
    current: &settings::Settings,
) -> Option<&'a LanguageMetadata> {
    all.iter()
        .find(|language| language.id == current.language)
        .or_else(|| all.first())
}

unsafe fn set_text(hwnd: HWND, text: &str) {
    unsafe {
        SetWindowTextW(hwnd, wide(text).as_ptr());
    }
}

unsafe fn child(parent: HWND, id: i32) -> HWND {
    unsafe { GetDlgItem(parent, id) }
}

unsafe fn set_check(parent: HWND, id: i32, checked: bool) {
    let value = if checked {
        INPUTKEY_BST_CHECKED
    } else {
        INPUTKEY_BST_UNCHECKED
    };
    unsafe {
        SendMessageW(child(parent, id), BM_SETCHECK, value as usize, 0);
    }
}

unsafe fn checked(parent: HWND, id: i32) -> bool {
    unsafe { SendMessageW(child(parent, id), BM_GETCHECK, 0, 0) as u32 == INPUTKEY_BST_CHECKED }
}

unsafe fn combo_clear(combo: HWND) {
    unsafe {
        SendMessageW(combo, CB_RESETCONTENT, 0, 0);
    }
}

unsafe fn combo_add(combo: HWND, label: &str) {
    let label = wide(label);
    unsafe {
        SendMessageW(combo, CB_ADDSTRING, 0, label.as_ptr() as isize);
    }
}

unsafe fn combo_select(combo: HWND, index: usize) {
    unsafe {
        SendMessageW(combo, CB_SETCURSEL, index, 0);
    }
}

unsafe fn combo_index(combo: HWND) -> Option<usize> {
    let index = unsafe { SendMessageW(combo, CB_GETCURSEL, 0, 0) };
    (index >= 0).then_some(index as usize)
}

unsafe fn option_visible(parent: HWND, id: i32, visible: bool) {
    unsafe {
        ShowWindow(child(parent, id), if visible { SW_SHOW } else { SW_HIDE });
    }
}

unsafe fn refresh_methods(parent: HWND, current: &settings::Settings, language: &LanguageMetadata) {
    let combo = unsafe { child(parent, ID_METHOD) };
    unsafe {
        combo_clear(combo);
    }
    let mut selected = 0usize;
    for (index, method) in language.methods.iter().enumerate() {
        unsafe {
            combo_add(combo, &method.label);
        }
        if method.id == current.method {
            selected = index;
        }
    }
    if !language.methods.is_empty() {
        unsafe {
            combo_select(combo, selected);
        }
    }
}

unsafe fn refresh_options(parent: HWND, current: &settings::Settings, language: &LanguageMetadata) {
    let has = |id: &str| language.options.iter().any(|option| option.id == id);

    unsafe {
        option_visible(parent, ID_SIMPLE_TELEX, has("simple_telex"));
        option_visible(parent, ID_AUTO_RESTORE, has("auto_restore"));
        option_visible(parent, ID_SMART_CORRECTION, has("smart_correction"));
        set_check(parent, ID_SIMPLE_TELEX, current.simple_telex);
        set_check(parent, ID_AUTO_RESTORE, current.auto_restore);
        set_check(parent, ID_SMART_CORRECTION, current.smart_correction);
    }
}

unsafe fn refresh(parent: HWND) {
    let current = settings::load();
    let all = languages();
    let combo = unsafe { child(parent, ID_LANGUAGE) };
    unsafe {
        combo_clear(combo);
    }

    let mut selected = 0usize;
    for (index, language) in all.iter().enumerate() {
        unsafe {
            combo_add(combo, &language.native_name);
        }
        if language.id == current.language {
            selected = index;
        }
    }
    if !all.is_empty() {
        unsafe {
            combo_select(combo, selected);
        }
    }

    unsafe {
        set_check(parent, ID_ENABLED, current.enabled);
    }

    if let Some(language) = active_language(&all, &current) {
        unsafe {
            refresh_methods(parent, &current, language);
            refresh_options(parent, &current, language);
        }
    }

    unsafe {
        set_text(
            child(parent, 4190),
            &format!("InputKey {}", env!("CARGO_PKG_VERSION")),
        );
    }
}

fn apply_option_default(current: &mut settings::Settings, language: &LanguageMetadata, id: &str) {
    let Some(option) = language.options.iter().find(|option| option.id == id) else {
        return;
    };
    match id {
        "simple_telex" => current.simple_telex = option.default_enabled,
        "auto_restore" => current.auto_restore = option.default_enabled,
        "smart_correction" => current.smart_correction = option.default_enabled,
        _ => {}
    }
}

unsafe fn save_and_signal(current: settings::Settings) {
    settings::save(current);
    if !unsafe { OWNER }.is_null() {
        unsafe {
            PostMessageW(OWNER, SETTINGS_CHANGED_MESSAGE, 0, 0);
        }
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn create_control(
    parent: HWND,
    class_name: &str,
    text: &str,
    style: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    id: i32,
) -> HWND {
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            wide(class_name).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            x,
            y,
            width,
            height,
            parent,
            id as usize as HMENU,
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        )
    };
    if !hwnd.is_null() {
        let font = unsafe { GetStockObject(DEFAULT_GUI_FONT) };
        unsafe {
            SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
        }
    }
    hwnd
}

unsafe fn build_controls(parent: HWND) {
    unsafe {
        create_control(parent, "STATIC", "InputKey", 0, 24, 20, 230, 28, 4189);
        create_control(parent, "STATIC", "", 0, 24, 48, 230, 20, 4190);
        create_control(
            parent,
            "BUTTON",
            "Enable InputKey",
            BS_AUTOCHECKBOX as u32,
            24,
            82,
            220,
            24,
            ID_ENABLED,
        );

        create_control(parent, "STATIC", "Language", 0, 24, 124, 120, 20, 4191);
        create_control(
            parent,
            "COMBOBOX",
            "",
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            150,
            120,
            250,
            220,
            ID_LANGUAGE,
        );

        create_control(parent, "STATIC", "Input method", 0, 24, 164, 120, 20, 4192);
        create_control(
            parent,
            "COMBOBOX",
            "",
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            150,
            160,
            250,
            220,
            ID_METHOD,
        );

        create_control(
            parent,
            "BUTTON",
            "Simple Telex",
            BS_AUTOCHECKBOX as u32,
            24,
            208,
            260,
            24,
            ID_SIMPLE_TELEX,
        );
        create_control(
            parent,
            "BUTTON",
            "Auto Restore",
            BS_AUTOCHECKBOX as u32,
            24,
            240,
            260,
            24,
            ID_AUTO_RESTORE,
        );
        create_control(
            parent,
            "BUTTON",
            "Smart correction",
            BS_AUTOCHECKBOX as u32,
            24,
            272,
            260,
            24,
            ID_SMART_CORRECTION,
        );

        create_control(
            parent,
            "STATIC",
            "Space commits the current token. Shift+Space keeps the physical key sequence and ends composition without inserting a space.",
            0,
            24,
            314,
            376,
            44,
            4193,
        );

        create_control(
            parent,
            "BUTTON",
            "Close",
            BS_PUSHBUTTON as u32,
            310,
            370,
            90,
            28,
            ID_CLOSE,
        );

        refresh(parent);
    }
}

unsafe fn on_language_change(parent: HWND) {
    let all = languages();
    let Some(index) = (unsafe { combo_index(child(parent, ID_LANGUAGE)) }) else {
        return;
    };
    let Some(language) = all.get(index) else {
        return;
    };

    let mut current = settings::load();
    current.language = language.id.clone();
    current.method = language.default_method.clone();
    apply_option_default(&mut current, language, "simple_telex");
    apply_option_default(&mut current, language, "auto_restore");
    apply_option_default(&mut current, language, "smart_correction");
    unsafe {
        save_and_signal(current);
        refresh(parent);
    }
}

unsafe fn on_method_change(parent: HWND) {
    let all = languages();
    let mut current = settings::load();
    let Some(language) = active_language(&all, &current) else {
        return;
    };
    let Some(index) = (unsafe { combo_index(child(parent, ID_METHOD)) }) else {
        return;
    };
    let Some(method) = language.methods.get(index) else {
        return;
    };
    current.method = method.id.clone();
    unsafe {
        save_and_signal(current);
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CREATE => {
            unsafe {
                build_controls(hwnd);
            }
            0
        }
        WM_COMMAND => {
            let id = (wparam & 0xffff) as i32;
            let notification = ((wparam >> 16) & 0xffff) as u32;
            match id {
                ID_ENABLED if notification == BN_CLICKED => {
                    let mut current = settings::load();
                    current.enabled = unsafe { checked(hwnd, ID_ENABLED) };
                    unsafe {
                        save_and_signal(current);
                    }
                }
                ID_LANGUAGE if notification == CBN_SELCHANGE => unsafe {
                    on_language_change(hwnd);
                },
                ID_METHOD if notification == CBN_SELCHANGE => unsafe {
                    on_method_change(hwnd);
                },
                ID_SIMPLE_TELEX if notification == BN_CLICKED => {
                    let mut current = settings::load();
                    current.simple_telex = unsafe { checked(hwnd, ID_SIMPLE_TELEX) };
                    unsafe {
                        save_and_signal(current);
                    }
                }
                ID_AUTO_RESTORE if notification == BN_CLICKED => {
                    let mut current = settings::load();
                    current.auto_restore = unsafe { checked(hwnd, ID_AUTO_RESTORE) };
                    unsafe {
                        save_and_signal(current);
                    }
                }
                ID_SMART_CORRECTION if notification == BN_CLICKED => {
                    let mut current = settings::load();
                    current.smart_correction = unsafe { checked(hwnd, ID_SMART_CORRECTION) };
                    unsafe {
                        save_and_signal(current);
                    }
                }
                ID_CLOSE if notification == BN_CLICKED => unsafe {
                    ShowWindow(hwnd, SW_HIDE);
                },
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            unsafe {
                ShowWindow(hwnd, SW_HIDE);
            }
            0
        }
        WM_DESTROY => {
            unsafe {
                if WINDOW == hwnd {
                    WINDOW = std::ptr::null_mut();
                }
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

pub unsafe fn show(owner: HWND, actions: &ControlActions) {
    let _ = ACTIONS.set(actions.clone());
    unsafe {
        OWNER = owner;
    }

    let existing = unsafe { WINDOW };
    if !existing.is_null() {
        unsafe {
            refresh(existing);
            ShowWindow(existing, SW_SHOW);
            SetForegroundWindow(existing);
        }
        return;
    }

    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    let class = wide(CLASS_NAME);
    let window_class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        lpszClassName: class.as_ptr(),
        hCursor: unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW) },
        hbrBackground: (INPUTKEY_COLOR_WINDOW + 1) as HBRUSH,
        ..unsafe { std::mem::zeroed() }
    };
    unsafe {
        RegisterClassW(&window_class);
    }

    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            class.as_ptr(),
            wide("InputKey Settings").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            450,
            455,
            owner,
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        return;
    }

    unsafe {
        WINDOW = hwnd;
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
    }
}
