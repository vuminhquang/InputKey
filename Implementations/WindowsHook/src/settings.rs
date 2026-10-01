use crate::Modifiers;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub enabled: bool,
    pub vk: u32,
    pub modifiers: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            vk: 0x20,
            modifiers: Modifiers::CTRL,
        }
    }
}

impl Settings {
    pub fn normalized(enabled: u32, vk: u32, modifiers: u32) -> Self {
        let valid = (1..=0xfe).contains(&vk)
            && modifiers != 0
            && modifiers
                & !(Modifiers::CTRL | Modifiers::ALT | Modifiers::SHIFT | Modifiers::WIN) as u32
                == 0;
        Self {
            enabled: enabled != 0,
            ..if valid {
                Self {
                    enabled: true,
                    vk,
                    modifiers: modifiers as u8,
                }
            } else {
                Self::default()
            }
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
    unsafe fn read(key: HKEY, name: &str) -> Option<u32> {
        let mut value = 0u32;
        let mut size = 4u32;
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
    unsafe fn write(key: HKEY, name: &str, value: u32) {
        unsafe {
            RegSetValueExW(
                key,
                wide(name).as_ptr(),
                0,
                REG_DWORD,
                (&value as *const u32).cast(),
                4,
            );
        }
    }
    pub fn load() -> Settings {
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
            return Settings::default();
        }
        let d = Settings::default();
        let settings = unsafe {
            Settings::normalized(
                read(key, "LiteralizeEnabled").unwrap_or(d.enabled as u32),
                read(key, "LiteralizeVK").unwrap_or(d.vk),
                read(key, "LiteralizeModifiers").unwrap_or(d.modifiers as u32),
            )
        };
        unsafe {
            RegCloseKey(key);
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
            write(key, "LiteralizeEnabled", settings.enabled as u32);
            write(key, "LiteralizeVK", settings.vk);
            write(key, "LiteralizeModifiers", settings.modifiers as u32);
            RegCloseKey(key);
        }
    }
}
#[cfg(windows)]
pub use registry::{load, save};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_and_normalization() {
        assert_eq!(Settings::default(), Settings::normalized(1, 0x20, 1));
        assert_eq!(
            Settings::normalized(0, 0, 0),
            Settings {
                enabled: false,
                ..Settings::default()
            }
        );
        assert_eq!(Settings::normalized(1, 0x41, 3).modifiers, 3);
    }
}
