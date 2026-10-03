#![cfg_attr(windows, windows_subsystem = "windows")]

use inputkey_runtime::Catalog;
use inputkey_windows_control::ControlActions;
use std::sync::Arc;

#[cfg(windows)]
const COMPATIBILITY_RELOAD_EVENT: &str = "Local\\InputKey.Compatibility.Reload";

#[cfg(windows)]
struct ComApartment {
    initialized: bool,
}

#[cfg(windows)]
impl ComApartment {
    fn sta() -> Self {
        use windows_sys::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
        let result = unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };
        Self {
            initialized: result >= 0,
        }
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }
}

#[cfg(windows)]
impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.initialized {
            unsafe {
                windows_sys::Win32::System::Com::CoUninitialize();
            }
        }
    }
}

#[cfg(windows)]
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

#[cfg(windows)]
fn package_directory() -> Option<std::path::PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
}

#[cfg(windows)]
fn runtime_directory() -> Option<std::path::PathBuf> {
    package_directory().map(|directory| directory.join("runtime").join(env!("CARGO_PKG_VERSION")))
}

#[cfg(windows)]
fn runtime_tsf_dll() -> Option<std::path::PathBuf> {
    runtime_directory().map(|directory| directory.join("InputKeyTSF.dll"))
}

#[cfg(windows)]
fn configure_runtime_languages() {
    if let Some(directory) = runtime_directory().map(|directory| directory.join("languages")) {
        if directory.is_dir() {
            std::env::set_var("INPUTKEY_LANGUAGE_PACK_DIR", directory);
        }
    }
}

#[cfg(windows)]
fn registrar_path() -> Option<std::path::PathBuf> {
    package_directory().map(|directory| directory.join("InputKeyTSFRegister.exe"))
}

#[cfg(windows)]
fn run_registrar(arguments: &[String]) -> bool {
    let Some(registrar) = registrar_path().filter(|path| path.is_file()) else {
        return false;
    };
    std::process::Command::new(registrar)
        .args(arguments)
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(windows)]
fn compatibility_path() -> Option<std::path::PathBuf> {
    package_directory().map(|directory| directory.join("InputKeyCompatibility.exe"))
}

#[cfg(windows)]
fn start_compatibility() -> Option<std::process::Child> {
    let helper = compatibility_path().filter(|path| path.is_file())?;
    std::process::Command::new(helper)
        .arg("--parent")
        .arg(std::process::id().to_string())
        .spawn()
        .ok()
}

#[cfg(windows)]
fn notify_compatibility_reload() {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{OpenEventW, SetEvent, EVENT_MODIFY_STATE},
    };
    let event = unsafe {
        OpenEventW(
            EVENT_MODIFY_STATE,
            0,
            wide(COMPATIBILITY_RELOAD_EVENT).as_ptr(),
        )
    };
    if event.is_null() {
        return;
    }
    unsafe {
        SetEvent(event);
        CloseHandle(event);
    }
}

#[cfg(windows)]
fn show_text_service_result(active: bool, installed_now: bool) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK,
    };

    let title = wide("InputKey Text Service");
    let message = if active {
        if installed_now {
            wide(
                "InputKey Text Service is installed for the current user and active.\n\nApplications that were already open may need to be restarted once before they load InputKey.",
            )
        } else {
            wide(
                "InputKey Text Service is repaired and active for the current user.\n\nApplications that were already open may need to be restarted once.",
            )
        }
    } else {
        wide(
            "InputKey could not register or activate the Text Service for the current user. Compatibility mode remains available for supported text controls.",
        )
    };
    let flags = if active {
        MB_OK | MB_ICONINFORMATION
    } else {
        MB_OK | MB_ICONERROR
    };
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            flags,
        );
    }
}

#[cfg(windows)]
fn ensure_packaged_text_service() -> bool {
    let Some(directory) = runtime_directory() else {
        return false;
    };
    let Some(dll) = runtime_tsf_dll() else {
        return false;
    };
    let languages = directory.join("languages");
    if !dll.is_file() || !languages.is_dir() {
        return false;
    }

    let registered = run_registrar(&[
        "--register-only".to_owned(),
        "--dll".to_owned(),
        dll.to_string_lossy().into_owned(),
    ]);
    if !registered {
        return false;
    }
    run_registrar(&["--activate-only".to_owned()])
        && inputkey_windows_tsf::text_service_registered()
        && inputkey_windows_tsf::text_service_bound()
}

#[cfg(windows)]
fn install_text_service() {
    std::thread::spawn(|| {
        let apartment = ComApartment::sta();
        if !apartment.is_initialized() {
            show_text_service_result(false, false);
            return;
        }

        let installed_now = !inputkey_windows_tsf::text_service_registered();
        let active = ensure_packaged_text_service();
        show_text_service_result(active, installed_now && active);
    });
}

#[cfg(windows)]
fn remove_user_text_service_integration() -> bool {
    let _ = run_registrar(&["--disable".to_owned()]);
    run_registrar(&["--unregister".to_owned()])
}

fn main() {
    #[cfg(windows)]
    let com_apartment = ComApartment::sta();
    #[cfg(windows)]
    let tsf_com_ready = com_apartment.is_initialized();
    #[cfg(not(windows))]
    let tsf_com_ready = false;

    #[cfg(windows)]
    configure_runtime_languages();

    #[cfg(windows)]
    if tsf_com_ready {
        let _ = ensure_packaged_text_service();
    }

    let catalog = Arc::new(Catalog::installed());

    #[cfg(windows)]
    let mut compatibility = start_compatibility();

    let language_catalog = Arc::clone(&catalog);
    let actions = ControlActions {
        settings_changed: Arc::new(move || {
            #[cfg(windows)]
            notify_compatibility_reload();
        }),
        languages: Arc::new(move || {
            use inputkey_core_abstractions::LanguageCatalogPort;
            language_catalog.languages()
        }),
        install_text_service: Arc::new(|| {
            #[cfg(windows)]
            install_text_service();
        }),
        remove_windows_integration: Arc::new(move || {
            #[cfg(windows)]
            {
                if !tsf_com_ready {
                    return false;
                }
                remove_user_text_service_integration()
            }
            #[cfg(not(windows))]
            {
                false
            }
        }),
        text_service_available: Arc::new(move || {
            tsf_com_ready && inputkey_windows_tsf::text_service_available()
        }),
        text_service_active: Arc::new(move || {
            tsf_com_ready && inputkey_windows_tsf::text_service_active()
        }),
    };

    inputkey_windows_control::run(actions);

    #[cfg(windows)]
    if let Some(child) = compatibility.as_mut() {
        let _ = child.kill();
        let _ = child.wait();
    }
}
