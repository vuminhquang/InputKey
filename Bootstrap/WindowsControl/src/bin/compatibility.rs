#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
use inputkey_core_abstractions::TypingEnginePort;
#[cfg(windows)]
use inputkey_runtime::{create_machine, Catalog};
#[cfg(windows)]
use inputkey_windows_hook::{Config, EngineConfig, EngineFactory};
#[cfg(windows)]
use std::sync::Arc;

#[cfg(windows)]
const RELOAD_EVENT: &str = r"Local\InputKey.Compatibility.Reload";

#[cfg(windows)]
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
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

#[cfg(windows)]
fn configure_runtime_languages() {
    let Some(package) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
    else {
        return;
    };
    let languages = package
        .join("runtime")
        .join(env!("CARGO_PKG_VERSION"))
        .join("languages");
    if languages.is_dir() {
        std::env::set_var("INPUTKEY_LANGUAGE_PACK_DIR", languages);
    }
}

#[cfg(windows)]
fn parent_pid(args: &[String]) -> Option<u32> {
    args.iter()
        .position(|arg| arg == "--parent")
        .and_then(|index| args.get(index + 1))
        .and_then(|value| value.parse().ok())
}

#[cfg(windows)]
fn main() {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0},
        System::Threading::{CreateEventW, OpenProcess, WaitForMultipleObjects, INFINITE},
    };

    let args: Vec<String> = std::env::args().collect();
    let Some(parent_pid) = parent_pid(&args) else {
        return;
    };

    const SYNCHRONIZE_ACCESS: u32 = 0x0010_0000;
    let parent = unsafe { OpenProcess(SYNCHRONIZE_ACCESS, 0, parent_pid) };
    if parent.is_null() {
        return;
    }
    let reload = unsafe { CreateEventW(std::ptr::null(), 0, 0, wide(RELOAD_EVENT).as_ptr()) };
    if reload.is_null() {
        unsafe {
            CloseHandle(parent);
        }
        return;
    }

    configure_runtime_languages();
    let catalog = Arc::new(Catalog::installed());
    let factory = engine_factory(Arc::clone(&catalog));
    let Ok(handle) = inputkey_windows_hook::start(
        Config {
            capture_enabled: true,
            engine: EngineConfig::default(),
        },
        factory,
    ) else {
        unsafe {
            CloseHandle(reload);
            CloseHandle(parent);
        }
        return;
    };

    let handles: [HANDLE; 2] = [parent, reload];
    loop {
        let result =
            unsafe { WaitForMultipleObjects(handles.len() as u32, handles.as_ptr(), 0, INFINITE) };
        if result == WAIT_OBJECT_0 {
            break;
        }
        if result == WAIT_OBJECT_0 + 1 {
            handle.reload_settings();
            continue;
        }
        break;
    }

    drop(handle);
    unsafe {
        CloseHandle(reload);
        CloseHandle(parent);
    }
}

#[cfg(not(windows))]
fn main() {}
