#[cfg(windows)]
fn main() -> windows::core::Result<()> {
    use inputkey_windows_tsf::{
        activate_text_service, disable_text_service, register_text_service, text_service_active,
        text_service_available, unregister_text_service,
    };
    use std::path::PathBuf;
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

    let args: Vec<String> = std::env::args().collect();
    let unregister = args.iter().any(|arg| arg == "--unregister");
    let register_only = args.iter().any(|arg| arg == "--register-only");
    let activate_only = args.iter().any(|arg| arg == "--activate-only");
    let status = args.iter().any(|arg| arg == "--status");
    let disable = args.iter().any(|arg| arg == "--disable");

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
    }

    if status {
        println!(
            "available={} active={}",
            text_service_available(),
            text_service_active()
        );
        unsafe {
            CoUninitialize();
        }
        return Ok(());
    }

    let result = if unregister {
        unregister_text_service()
    } else if disable {
        disable_text_service()
    } else if activate_only {
        activate_text_service()
    } else {
        let dll = args
            .iter()
            .position(|arg| arg == "--dll")
            .and_then(|index| args.get(index + 1))
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::current_exe()
                    .expect("registrar path")
                    .parent()
                    .expect("registrar directory")
                    .join("InputKeyTSF.dll")
            });
        if register_only {
            register_text_service(&dll)
        } else {
            register_text_service(&dll).and_then(|_| activate_text_service())
        }
    };

    unsafe {
        CoUninitialize();
    }
    result
}

#[cfg(not(windows))]
fn main() {
    eprintln!("InputKeyTSFRegister only runs on Windows");
}
