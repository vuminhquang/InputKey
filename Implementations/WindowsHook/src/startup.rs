use std::path::Path;

pub fn startup_command(exe: &Path) -> Result<String, &'static str> {
    if !exe.is_absolute() {
        return Err("executable path must be absolute");
    }
    let path = exe.to_str().ok_or("executable path must be Unicode")?;
    if path.contains(['\0', '"']) {
        return Err("invalid executable path");
    }
    Ok(format!("\"{path}\""))
}

#[cfg(windows)]
mod registry {
    use super::startup_command;
    use windows_sys::Win32::System::Registry::*;
    const PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    pub fn is_enabled() -> bool {
        let mut key: HKEY = std::ptr::null_mut();
        if unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                wide(PATH).as_ptr(),
                0,
                KEY_QUERY_VALUE,
                &mut key,
            )
        } != 0
        {
            return false;
        }
        let mut kind = 0;
        let mut size = 0;
        let result = unsafe {
            RegQueryValueExW(
                key,
                wide("InputKey").as_ptr(),
                std::ptr::null_mut(),
                &mut kind,
                std::ptr::null_mut(),
                &mut size,
            ) == 0
                && kind == REG_SZ
        };
        unsafe {
            RegCloseKey(key);
        }
        result
    }
    pub fn set_enabled(enabled: bool) {
        let mut key: HKEY = std::ptr::null_mut();
        let access = if enabled {
            KEY_SET_VALUE
        } else {
            KEY_SET_VALUE | KEY_QUERY_VALUE
        };
        let status = if enabled {
            unsafe {
                RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    wide(PATH).as_ptr(),
                    0,
                    std::ptr::null(),
                    0,
                    access,
                    std::ptr::null(),
                    &mut key,
                    std::ptr::null_mut(),
                )
            }
        } else {
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, wide(PATH).as_ptr(), 0, access, &mut key) }
        };
        if status != 0 {
            return;
        }
        unsafe {
            if enabled {
                if let Ok(exe) = std::env::current_exe() {
                    if let Ok(command) = startup_command(&exe) {
                        let chars = wide(&command);
                        RegSetValueExW(
                            key,
                            wide("InputKey").as_ptr(),
                            0,
                            REG_SZ,
                            chars.as_ptr().cast(),
                            (chars.len() * 2) as u32,
                        );
                    }
                }
            } else {
                RegDeleteValueW(key, wide("InputKey").as_ptr());
            }
            RegCloseKey(key);
        }
    }
}
#[cfg(windows)]
pub use registry::{is_enabled, set_enabled};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quotes_and_validates_executable() {
        let absolute = if cfg!(windows) {
            "C:\\Program Files\\InputKey\\InputKey.exe"
        } else {
            "/opt/Input Key/InputKey"
        };
        assert_eq!(
            startup_command(Path::new(absolute)).unwrap(),
            format!("\"{absolute}\"")
        );
        assert!(startup_command(Path::new("relative.exe")).is_err());
        let invalid = if cfg!(windows) {
            "C:\\bad\"name.exe"
        } else {
            "/bad\"name"
        };
        assert!(startup_command(Path::new(invalid)).is_err());
        let nul = if cfg!(windows) {
            "C:\\bad\0name.exe"
        } else {
            "/bad\0name"
        };
        assert!(startup_command(Path::new(nul)).is_err());
    }
}
