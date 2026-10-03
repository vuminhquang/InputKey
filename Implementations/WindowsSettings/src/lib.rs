#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub enabled: bool,
    pub language: String,
    pub method: String,
    pub simple_telex: bool,
    pub auto_restore: bool,
    pub smart_correction: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            language: "vi".into(),
            method: "telex".into(),
            simple_telex: false,
            auto_restore: true,
            smart_correction: true,
        }
    }
}

#[cfg(windows)]
mod registry {
    use super::Settings;
    use windows_sys::Win32::System::Registry::*;

    const PATH: &str = "Software\\InputKey\\Settings";

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    unsafe fn read_dword(key: HKEY, name: &str) -> Option<u32> {
        let mut value = 0u32;
        let mut size = std::mem::size_of::<u32>() as u32;
        let mut kind = 0u32;
        let status = unsafe {
            RegQueryValueExW(
                key,
                wide(name).as_ptr(),
                std::ptr::null_mut(),
                &mut kind,
                (&mut value as *mut u32).cast(),
                &mut size,
            )
        };
        (status == 0 && kind == REG_DWORD && size == 4).then_some(value)
    }

    unsafe fn read_string(key: HKEY, name: &str) -> Option<String> {
        let mut kind = 0u32;
        let mut size = 0u32;
        let name = wide(name);
        if unsafe {
            RegQueryValueExW(
                key,
                name.as_ptr(),
                std::ptr::null_mut(),
                &mut kind,
                std::ptr::null_mut(),
                &mut size,
            )
        } != 0
            || !matches!(kind, REG_SZ | REG_EXPAND_SZ)
            || size < 2
        {
            return None;
        }
        let mut buffer = vec![0u16; size as usize / 2];
        if unsafe {
            RegQueryValueExW(
                key,
                name.as_ptr(),
                std::ptr::null_mut(),
                &mut kind,
                buffer.as_mut_ptr().cast(),
                &mut size,
            )
        } != 0
        {
            return None;
        }
        if buffer.last().copied() == Some(0) {
            buffer.pop();
        }
        String::from_utf16(&buffer).ok()
    }

    unsafe fn write_dword(key: HKEY, name: &str, value: u32) {
        unsafe {
            let _ = RegSetValueExW(
                key,
                wide(name).as_ptr(),
                0,
                REG_DWORD,
                (&value as *const u32).cast(),
                std::mem::size_of::<u32>() as u32,
            );
        }
    }

    unsafe fn write_string(key: HKEY, name: &str, value: &str) {
        let value = wide(value);
        unsafe {
            let _ = RegSetValueExW(
                key,
                wide(name).as_ptr(),
                0,
                REG_SZ,
                value.as_ptr().cast(),
                (value.len() * std::mem::size_of::<u16>()) as u32,
            );
        }
    }

    pub fn load() -> Settings {
        let defaults = Settings::default();
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
            return defaults;
        }

        let settings = unsafe {
            let language = read_string(key, "Language").unwrap_or(defaults.language);
            let method =
                read_string(key, "MethodName").unwrap_or_else(|| match read_dword(key, "Method") {
                    Some(1) => "vni".into(),
                    _ => defaults.method,
                });
            Settings {
                enabled: read_dword(key, "Enabled").unwrap_or(defaults.enabled as u32) != 0,
                language,
                method,
                simple_telex: read_dword(key, "SimpleTelex")
                    .unwrap_or(defaults.simple_telex as u32)
                    != 0,
                auto_restore: read_dword(key, "AutoRestore")
                    .unwrap_or(defaults.auto_restore as u32)
                    != 0,
                smart_correction: read_dword(key, "SmartCorrection")
                    .unwrap_or(defaults.smart_correction as u32)
                    != 0,
            }
        };

        unsafe {
            let _ = RegCloseKey(key);
        }
        settings
    }

    pub fn save(settings: Settings) {
        let mut key: HKEY = std::ptr::null_mut();
        if unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                wide(PATH).as_ptr(),
                0,
                std::ptr::null(),
                0,
                KEY_SET_VALUE,
                std::ptr::null(),
                &mut key,
                std::ptr::null_mut(),
            )
        } != 0
        {
            return;
        }

        unsafe {
            write_dword(key, "Enabled", settings.enabled as u32);
            write_string(key, "Language", &settings.language);
            write_string(key, "MethodName", &settings.method);
            if settings.language == "vi" {
                write_dword(
                    key,
                    "Method",
                    u32::from(settings.method.eq_ignore_ascii_case("vni")),
                );
            }
            write_dword(key, "SimpleTelex", settings.simple_telex as u32);
            write_dword(key, "AutoRestore", settings.auto_restore as u32);
            write_dword(key, "SmartCorrection", settings.smart_correction as u32);
            let _ = RegCloseKey(key);
        }
    }
}

#[cfg(windows)]
pub use registry::{load, save};

#[cfg(not(windows))]
pub fn load() -> Settings {
    Settings::default()
}

#[cfg(not(windows))]
pub fn save(_settings: Settings) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_vietnamese_telex_with_smart_correction() {
        let settings = Settings::default();
        assert_eq!(settings.language, "vi");
        assert_eq!(settings.method, "telex");
        assert!(!settings.simple_telex);
        assert!(settings.smart_correction);
    }
}
