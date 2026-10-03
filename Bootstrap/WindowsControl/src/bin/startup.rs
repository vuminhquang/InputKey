#![cfg_attr(windows, windows_subsystem = "windows")]

use std::path::PathBuf;

fn exe_arg(args: &[String]) -> Option<PathBuf> {
    args.iter()
        .position(|arg| arg == "--exe")
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
}

#[cfg(windows)]
mod windows_startup {
    use std::path::{Path, PathBuf};
    use windows::core::{Interface, HSTRING};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED, STGM_READ,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    struct Apartment;

    impl Apartment {
        fn init() -> windows::core::Result<Self> {
            unsafe {
                CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
            }
            Ok(Self)
        }
    }

    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }

    fn startup_dir() -> Option<PathBuf> {
        std::env::var_os("APPDATA").map(PathBuf::from).map(|path| {
            path.join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join("Startup")
        })
    }

    fn shortcut_path() -> Option<PathBuf> {
        startup_dir().map(|path| path.join("InputKey.lnk"))
    }

    fn same_path(left: &Path, right: &Path) -> bool {
        let normalize = |path: &Path| {
            std::fs::canonicalize(path)
                .unwrap_or_else(|_| path.to_path_buf())
                .to_string_lossy()
                .replace('/', "\\")
                .trim_end_matches('\\')
                .to_ascii_lowercase()
        };
        normalize(left) == normalize(right)
    }

    fn load_target(shortcut: &Path) -> windows::core::Result<PathBuf> {
        let _apartment = Apartment::init()?;
        let shell: IShellLinkW =
            unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)? };
        let persist: IPersistFile = shell.cast()?;
        unsafe {
            persist.Load(
                &HSTRING::from(shortcut.as_os_str().to_string_lossy().as_ref()),
                STGM_READ,
            )?;
        }
        let mut buffer = vec![0u16; 32768];
        unsafe {
            shell.GetPath(&mut buffer, std::ptr::null_mut(), 0)?;
        }
        let end = buffer
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(buffer.len());
        Ok(PathBuf::from(String::from_utf16_lossy(&buffer[..end])))
    }

    pub fn enabled_for(exe: &Path) -> bool {
        let Some(shortcut) = shortcut_path() else {
            return false;
        };
        shortcut.is_file()
            && load_target(&shortcut)
                .ok()
                .is_some_and(|target| same_path(&target, exe))
    }

    pub fn enable(exe: &Path) -> bool {
        if !exe.is_absolute() || !exe.is_file() {
            return false;
        }
        let Some(startup) = startup_dir() else {
            return false;
        };
        if std::fs::create_dir_all(&startup).is_err() {
            return false;
        }
        let shortcut = startup.join("InputKey.lnk");
        let Some(working_directory) = exe.parent() else {
            return false;
        };

        let result = (|| -> windows::core::Result<()> {
            let _apartment = Apartment::init()?;
            let shell: IShellLinkW =
                unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)? };
            unsafe {
                shell.SetPath(&HSTRING::from(exe.as_os_str().to_string_lossy().as_ref()))?;
                shell.SetWorkingDirectory(&HSTRING::from(
                    working_directory.as_os_str().to_string_lossy().as_ref(),
                ))?;
                shell.SetDescription(&HSTRING::from("InputKey"))?;
                shell.SetIconLocation(
                    &HSTRING::from(exe.as_os_str().to_string_lossy().as_ref()),
                    0,
                )?;
            }
            let persist: IPersistFile = shell.cast()?;
            unsafe {
                persist.Save(
                    &HSTRING::from(shortcut.as_os_str().to_string_lossy().as_ref()),
                    true,
                )?;
            }
            Ok(())
        })();

        result.is_ok() && enabled_for(exe)
    }

    pub fn disable() -> bool {
        let Some(shortcut) = shortcut_path() else {
            return false;
        };
        match std::fs::remove_file(shortcut) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
            Err(_) => false,
        }
    }
}

#[cfg(windows)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ok = if args.iter().any(|arg| arg == "--status") {
        exe_arg(&args)
            .map(|exe| windows_startup::enabled_for(&exe))
            .unwrap_or(false)
    } else if args.iter().any(|arg| arg == "--enable") {
        exe_arg(&args)
            .map(|exe| windows_startup::enable(&exe))
            .unwrap_or(false)
    } else if args.iter().any(|arg| arg == "--disable") {
        windows_startup::disable()
    } else {
        false
    };
    std::process::exit(if ok { 0 } else { 1 });
}

#[cfg(not(windows))]
fn main() {
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_exe_argument() {
        let args = vec![
            "InputKeyStartup.exe".to_owned(),
            "--enable".to_owned(),
            "--exe".to_owned(),
            "C:\\Portable\\InputKey\\InputKey.exe".to_owned(),
        ];
        assert_eq!(
            exe_arg(&args),
            Some(PathBuf::from("C:\\Portable\\InputKey\\InputKey.exe"))
        );
    }
}
