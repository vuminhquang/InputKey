#![cfg_attr(windows, windows_subsystem = "windows")]

use inputkey_core_abstractions::TypingEnginePort;
use inputkey_runtime::{create_machine, Catalog};
use inputkey_windows_control::ControlActions;
use inputkey_windows_hook::{Config, EngineConfig, EngineFactory};
use std::sync::{Arc, Mutex};

fn engine_factory(catalog: Arc<Catalog>) -> EngineFactory {
    Arc::new(move |config: EngineConfig| {
        let machine = create_machine(
            catalog.as_ref(),
            &config.language_id,
            config.language.clone(),
        )
        .or_else(|_| {
            let fallback = EngineConfig::default();
            create_machine(catalog.as_ref(), &fallback.language_id, fallback.language)
        })
        .expect("bundled Vietnamese language pack");
        Box::new(machine) as Box<dyn TypingEnginePort>
    })
}

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
fn register_text_service_elevated() -> bool {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE},
        UI::{
            Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW},
            WindowsAndMessaging::SW_HIDE,
        },
    };

    let Ok(executable) = std::env::current_exe() else {
        return false;
    };
    let Some(directory) = executable.parent() else {
        return false;
    };
    let registrar = directory.join("InputKeyTSFRegister.exe");
    let dll = directory.join("InputKeyTSF.dll");
    if !registrar.is_file() || !dll.is_file() {
        return false;
    }

    let verb = wide("runas");
    let file = wide(&registrar.to_string_lossy());
    let parameters = wide(&format!(
        "--register-only --dll \"{}\"",
        dll.to_string_lossy()
    ));
    let working_directory = wide(&directory.to_string_lossy());
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS;
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpParameters = parameters.as_ptr();
    info.lpDirectory = working_directory.as_ptr();
    info.nShow = SW_HIDE;

    if unsafe { ShellExecuteExW(&mut info) } == 0 || info.hProcess.is_null() {
        return false;
    }

    unsafe {
        WaitForSingleObject(info.hProcess, INFINITE);
    }
    let mut exit_code = u32::MAX;
    let ok = unsafe { GetExitCodeProcess(info.hProcess, &mut exit_code) } != 0 && exit_code == 0;
    unsafe {
        CloseHandle(info.hProcess);
    }
    ok
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
                "InputKey Text Service is installed and active.\n\nApplications that were already open before installation may need to be restarted once before InputKey is available there. Choosing Turn off InputKey and exit disables InputKey before the tray closes.",
            )
        } else {
            wide(
                "InputKey Text Service is active.\n\nIf an application was already open before the Text Service was installed, restart only that application once.",
            )
        }
    } else {
        wide("InputKey could not activate the Text Service. Compatibility mode remains available for supported edit controls.")
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
fn bind_packaged_text_service() {
    if !inputkey_windows_tsf::text_service_available() {
        return;
    }
    let Ok(executable) = std::env::current_exe() else {
        return;
    };
    let Some(directory) = executable.parent() else {
        return;
    };
    let dll = directory.join("InputKeyTSF.dll");
    let languages = directory.join("languages");
    if !dll.is_file() || !languages.is_dir() {
        return;
    }
    let _ = inputkey_windows_tsf::bind_text_service_dll(&dll);
}

#[cfg(windows)]
fn install_text_service() {
    std::thread::spawn(|| {
        let apartment = ComApartment::sta();
        if !apartment.is_initialized() {
            show_text_service_result(false, false);
            return;
        }

        let already_available = inputkey_windows_tsf::text_service_available();
        let registered = already_available || register_text_service_elevated();
        let active = registered
            && inputkey_windows_tsf::activate_text_service().is_ok()
            && inputkey_windows_tsf::text_service_active();
        show_text_service_result(active, registered && !already_available);
    });
}

fn main() {
    #[cfg(windows)]
    let com_apartment = ComApartment::sta();
    #[cfg(windows)]
    let tsf_com_ready = com_apartment.is_initialized();
    #[cfg(not(windows))]
    let tsf_com_ready = false;

    #[cfg(windows)]
    if tsf_com_ready {
        bind_packaged_text_service();
    }

    let catalog = Arc::new(Catalog::installed());
    let factory = engine_factory(Arc::clone(&catalog));
    let compatibility = inputkey_windows_hook::start(
        Config {
            capture_enabled: true,
            engine: EngineConfig::default(),
        },
        factory,
    )
    .ok()
    .map(|handle| Arc::new(Mutex::new(handle)));

    if tsf_com_ready
        && inputkey_windows_tsf::text_service_available()
        && !inputkey_windows_tsf::text_service_active()
    {
        let _ = inputkey_windows_tsf::activate_text_service();
    }

    let settings_hook = compatibility.clone();
    let language_catalog = Arc::clone(&catalog);
    let actions = ControlActions {
        settings_changed: Arc::new(move || {
            if let Some(handle) = &settings_hook {
                if let Ok(handle) = handle.lock() {
                    handle.reload_settings();
                }
            }
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
                let _ = inputkey_windows_tsf::disable_text_service();
                inputkey_windows_tsf::unregister_text_service().is_ok()
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
}
