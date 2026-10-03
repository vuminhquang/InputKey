use crate::{settings_window, ControlActions};
use inputkey_core_abstractions::LanguageMetadata;
use inputkey_windows_settings as settings;
use std::sync::OnceLock;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const CLASS_NAME: &str = "InputKeyControlWindow";
const TSF_CONTROL_CLASS: &str = "InputKeyTSFControlWindow";
const TRAY_MESSAGE: u32 = WM_APP + 11;
const ID_TOGGLE: usize = 101;
const ID_SETTINGS: usize = 102;
const ID_SIMPLE_TELEX: usize = 103;
const ID_AUTO_RESTORE: usize = 104;
const ID_SMART_CORRECTION: usize = 105;
const ID_STARTUP: usize = 106;
const ID_INSTALL_TSF: usize = 107;
const ID_EXIT: usize = 108;
const ID_REMOVE_WINDOWS_INTEGRATION: usize = 109;
const ID_LANGUAGE_BASE: usize = 2000;
const ID_METHOD_BASE: usize = 3000;

static ACTIONS: OnceLock<ControlActions> = OnceLock::new();
static mut ICON_V: HICON = std::ptr::null_mut();
static mut ICON_E: HICON = std::ptr::null_mut();

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn display_name() -> String {
    format!("InputKey v{}", env!("CARGO_PKG_VERSION"))
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

fn notify_tsf_reload() {
    let message = unsafe { RegisterWindowMessageW(wide("InputKey.TSF.ReloadSettings").as_ptr()) };
    let class = wide(TSF_CONTROL_CLASS);
    let mut after: HWND = std::ptr::null_mut();
    loop {
        let hwnd = unsafe { FindWindowExW(HWND_MESSAGE, after, class.as_ptr(), std::ptr::null()) };
        if hwnd.is_null() {
            break;
        }
        unsafe {
            PostMessageW(hwnd, message, 0, 0);
        }
        after = hwnd;
    }
}

fn notify_settings_changed() {
    notify_tsf_reload();
    if let Some(actions) = ACTIONS.get() {
        (actions.settings_changed)();
    }
}

fn icon_path(name: &str) -> Option<std::path::PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(name)))
}

unsafe fn load_icons() {
    if let Some(path) = icon_path("inputkey-v.ico") {
        unsafe {
            ICON_V = LoadImageW(
                std::ptr::null_mut(),
                wide(&path.to_string_lossy()).as_ptr(),
                IMAGE_ICON,
                0,
                0,
                LR_LOADFROMFILE,
            ) as HICON;
        }
    }
    if let Some(path) = icon_path("inputkey-e.ico") {
        unsafe {
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
}

unsafe fn update_tray(hwnd: HWND) {
    let current = settings::load();
    let all = languages();
    let active = active_language(&all, &current);
    let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = 1;
    data.uFlags = NIF_ICON | NIF_TIP;
    data.hIcon = unsafe {
        if current.enabled {
            ICON_V
        } else {
            ICON_E
        }
    };

    let language_name = active
        .map(|language| language.native_name.as_str())
        .unwrap_or(current.language.as_str());
    let method_name = active
        .and_then(|language| {
            language
                .methods
                .iter()
                .find(|method| method.id == current.method)
                .map(|method| method.label.as_str())
        })
        .unwrap_or(current.method.as_str());
    let tip = if current.enabled {
        format!("{} — {language_name} | {method_name}", display_name())
    } else {
        format!("{} — Off", display_name())
    };
    for (slot, value) in data.szTip.iter_mut().zip(wide(&tip)) {
        *slot = value;
    }
    unsafe {
        Shell_NotifyIconW(NIM_MODIFY, &data);
    }
}

fn save_and_notify(value: settings::Settings, hwnd: HWND) {
    settings::save(value);
    notify_settings_changed();
    unsafe {
        update_tray(hwnd);
    }
}

unsafe fn show_menu(hwnd: HWND) {
    let current = settings::load();
    let all = languages();
    let active = active_language(&all, &current);
    let menu = unsafe { CreatePopupMenu() };
    if menu.is_null() {
        return;
    }
    let checked = |value: bool| if value { MF_CHECKED } else { MF_UNCHECKED };

    unsafe {
        AppendMenuW(
            menu,
            MF_STRING | MF_GRAYED,
            0,
            wide(&display_name()).as_ptr(),
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(
            menu,
            MF_STRING | checked(current.enabled),
            ID_TOGGLE,
            wide("InputKey enabled").as_ptr(),
        );

        let language_menu = CreatePopupMenu();
        if !language_menu.is_null() {
            for (index, language) in all.iter().enumerate() {
                AppendMenuW(
                    language_menu,
                    MF_STRING | checked(language.id == current.language),
                    ID_LANGUAGE_BASE + index,
                    wide(&language.native_name).as_ptr(),
                );
            }
            AppendMenuW(
                menu,
                MF_POPUP | MF_STRING,
                language_menu as usize,
                wide("Language").as_ptr(),
            );
        }

        if let Some(language) = active {
            let method_menu = CreatePopupMenu();
            if !method_menu.is_null() {
                for (index, method) in language.methods.iter().enumerate() {
                    AppendMenuW(
                        method_menu,
                        MF_STRING | checked(method.id == current.method),
                        ID_METHOD_BASE + index,
                        wide(&method.label).as_ptr(),
                    );
                }
                AppendMenuW(
                    menu,
                    MF_POPUP | MF_STRING,
                    method_menu as usize,
                    wide("Method").as_ptr(),
                );
            }

            let has_simple_telex = language.options.iter().any(|o| o.id == "simple_telex");
            if has_simple_telex {
                AppendMenuW(
                    menu,
                    MF_STRING | checked(current.simple_telex),
                    ID_SIMPLE_TELEX,
                    wide("Simple Telex").as_ptr(),
                );
            }
            let has_auto_restore = language.options.iter().any(|o| o.id == "auto_restore");
            if has_auto_restore {
                AppendMenuW(
                    menu,
                    MF_STRING | checked(current.auto_restore),
                    ID_AUTO_RESTORE,
                    wide("Restore original keys automatically").as_ptr(),
                );
            }
            let has_smart_correction = language.options.iter().any(|o| o.id == "smart_correction");
            if has_smart_correction {
                AppendMenuW(
                    menu,
                    MF_STRING | checked(current.smart_correction),
                    ID_SMART_CORRECTION,
                    wide("Smart correction").as_ptr(),
                );
            }
        }

        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(
            menu,
            MF_STRING | MF_GRAYED,
            0,
            wide(&format!("Toggle shortcut: {}", current.toggle_shortcut)).as_ptr(),
        );
        AppendMenuW(menu, MF_STRING, ID_SETTINGS, wide("Settings...").as_ptr());
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());

        let text_service_available = ACTIONS
            .get()
            .is_some_and(|actions| (actions.text_service_available)());
        let text_service_active = ACTIONS
            .get()
            .is_some_and(|actions| (actions.text_service_active)());
        if text_service_active {
            AppendMenuW(
                menu,
                MF_STRING | MF_GRAYED,
                0,
                wide("Text Service: active").as_ptr(),
            );
            AppendMenuW(
                menu,
                MF_STRING,
                ID_INSTALL_TSF,
                wide("Repair Text Service binding...").as_ptr(),
            );
        } else if text_service_available {
            AppendMenuW(
                menu,
                MF_STRING,
                ID_INSTALL_TSF,
                wide("Activate Text Service").as_ptr(),
            );
        } else {
            AppendMenuW(
                menu,
                MF_STRING,
                ID_INSTALL_TSF,
                wide("Install Text Service...").as_ptr(),
            );
        }

        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(
            menu,
            MF_STRING,
            ID_REMOVE_WINDOWS_INTEGRATION,
            wide("Remove Windows integration...").as_ptr(),
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        let startup_enabled = crate::startup::is_enabled();
        AppendMenuW(
            menu,
            MF_STRING | checked(startup_enabled),
            ID_STARTUP,
            wide("Start with Windows").as_ptr(),
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(
            menu,
            MF_STRING,
            ID_EXIT,
            wide("Turn off InputKey and exit").as_ptr(),
        );

        let mut point = POINT { x: 0, y: 0 };
        GetCursorPos(&mut point);
        SetForegroundWindow(hwnd);
        TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON,
            point.x,
            point.y,
            0,
            hwnd,
            std::ptr::null(),
        );
        DestroyMenu(menu);
    }
}

fn turn_off_and_notify(hwnd: HWND) {
    let mut current = settings::load();
    if current.enabled {
        current.enabled = false;
        save_and_notify(current, hwnd);
    } else {
        notify_settings_changed();
    }
}

unsafe fn remove_windows_integration(hwnd: HWND) {
    let title = wide("Remove InputKey from Windows");
    let prompt = wide("This turns InputKey off, removes Start with Windows, unregisters the current user's InputKey Text Service profile/COM binding, and exits the tray app. The portable folder itself is not deleted.\n\nAlready-open applications may keep their loaded InputKey runtime until they close. Continue?");
    let answer = unsafe {
        MessageBoxW(
            hwnd,
            prompt.as_ptr(),
            title.as_ptr(),
            MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2,
        )
    };
    if answer != IDYES {
        return;
    }

    turn_off_and_notify(hwnd);

    let startup_removed = crate::startup::set_enabled(false);
    let removed = startup_removed
        && ACTIONS
            .get()
            .is_some_and(|actions| (actions.remove_windows_integration)());
    if removed {
        let message = wide("Windows integration was removed. You can move or delete the portable InputKey folder after closing applications that may still have InputKeyTSF.dll loaded.");
        unsafe {
            MessageBoxW(
                hwnd,
                message.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONINFORMATION,
            );
        }
        unsafe {
            DestroyWindow(hwnd);
        }
    } else {
        let message = wide("InputKey was turned off, but Windows integration could not be fully removed. The portable folder was left untouched.");
        unsafe {
            MessageBoxW(hwnd, message.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR);
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        crate::toggle_shortcut::TOGGLE_MESSAGE => {
            let mut current = settings::load();
            current.enabled = !current.enabled;
            save_and_notify(current, hwnd);
            0
        }
        TRAY_MESSAGE if lparam as u32 == WM_LBUTTONUP => {
            let mut current = settings::load();
            current.enabled = !current.enabled;
            save_and_notify(current, hwnd);
            0
        }
        TRAY_MESSAGE if lparam as u32 == WM_RBUTTONUP || lparam as u32 == WM_CONTEXTMENU => {
            unsafe {
                show_menu(hwnd);
            }
            0
        }
        WM_COMMAND => {
            let id = wparam & 0xffff;
            let mut current = settings::load();
            match id {
                ID_TOGGLE => {
                    current.enabled = !current.enabled;
                    save_and_notify(current, hwnd);
                }
                id if (ID_LANGUAGE_BASE..ID_METHOD_BASE).contains(&id) => {
                    let all = languages();
                    if let Some(language) = all.get(id - ID_LANGUAGE_BASE) {
                        current.language = language.id.clone();
                        current.method = language.default_method.clone();
                        if let Some(option) =
                            language.options.iter().find(|o| o.id == "auto_restore")
                        {
                            current.auto_restore = option.default_enabled;
                        }
                        if let Some(option) =
                            language.options.iter().find(|o| o.id == "smart_correction")
                        {
                            current.smart_correction = option.default_enabled;
                        }
                        save_and_notify(current, hwnd);
                    }
                }
                id if id >= ID_METHOD_BASE => {
                    let all = languages();
                    if let Some(language) = active_language(&all, &current) {
                        if let Some(method) = language.methods.get(id - ID_METHOD_BASE) {
                            current.method = method.id.clone();
                            save_and_notify(current, hwnd);
                        }
                    }
                }
                ID_SETTINGS => unsafe {
                    if let Some(actions) = ACTIONS.get() {
                        settings_window::show(hwnd, actions);
                    }
                },
                ID_SIMPLE_TELEX => {
                    current.simple_telex = !current.simple_telex;
                    save_and_notify(current, hwnd);
                }
                ID_AUTO_RESTORE => {
                    current.auto_restore = !current.auto_restore;
                    save_and_notify(current, hwnd);
                }
                ID_SMART_CORRECTION => {
                    current.smart_correction = !current.smart_correction;
                    save_and_notify(current, hwnd);
                }
                ID_STARTUP => {
                    let enabled = crate::startup::is_enabled();
                    let _ = crate::startup::set_enabled(!enabled);
                }
                ID_INSTALL_TSF => {
                    if let Some(actions) = ACTIONS.get() {
                        (actions.install_text_service)();
                    }
                }
                ID_REMOVE_WINDOWS_INTEGRATION => unsafe {
                    remove_windows_integration(hwnd);
                },
                ID_EXIT => {
                    turn_off_and_notify(hwnd);
                    unsafe {
                        DestroyWindow(hwnd);
                    }
                }
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            turn_off_and_notify(hwnd);
            unsafe {
                DestroyWindow(hwnd);
            }
            0
        }
        settings_window::SETTINGS_CHANGED_MESSAGE => {
            crate::toggle_shortcut::refresh();
            notify_settings_changed();
            unsafe {
                update_tray(hwnd);
            }
            0
        }
        WM_DESTROY => {
            unsafe {
                PostQuitMessage(0);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

pub fn run(actions: ControlActions) {
    let _ = ACTIONS.set(actions);
    let class = wide(CLASS_NAME);
    if !unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) }.is_null() {
        return;
    }

    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    let window_class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        lpszClassName: class.as_ptr(),
        ..unsafe { std::mem::zeroed() }
    };
    unsafe {
        RegisterClassW(&window_class);
    }
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            wide("InputKey").as_ptr(),
            0,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        return;
    }

    unsafe {
        load_icons();
    }
    let mut tray: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    tray.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    tray.hWnd = hwnd;
    tray.uID = 1;
    tray.uFlags = NIF_MESSAGE | NIF_TIP | NIF_ICON;
    tray.uCallbackMessage = TRAY_MESSAGE;
    tray.hIcon = unsafe {
        if settings::load().enabled {
            ICON_V
        } else {
            ICON_E
        }
    };
    for (slot, value) in tray.szTip.iter_mut().zip(wide(&display_name())) {
        *slot = value;
    }
    unsafe {
        Shell_NotifyIconW(NIM_ADD, &tray);
        update_tray(hwnd);
    }
    let _toggle_shortcut_hook = crate::toggle_shortcut::install(hwnd);

    let mut message: MSG = unsafe { std::mem::zeroed() };
    while unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) } > 0 {
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    unsafe {
        Shell_NotifyIconW(NIM_DELETE, &tray);
        if !ICON_V.is_null() {
            DestroyIcon(ICON_V);
            ICON_V = std::ptr::null_mut();
        }
        if !ICON_E.is_null() {
            DestroyIcon(ICON_E);
            ICON_E = std::ptr::null_mut();
        }
    }
}
