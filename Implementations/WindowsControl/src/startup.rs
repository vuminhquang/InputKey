use std::path::{Path, PathBuf};
use windows_sys::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
    System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
        RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ,
    },
};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "InputKey";
const LEGACY_VALUE_NAME: &str = "VietnameseKeyboard";

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn startup_command(executable: &Path) -> Option<String> {
    if !executable.is_absolute() || !executable.is_file() {
        return None;
    }
    Some(format!("\"{}\"", executable.to_string_lossy()))
}

struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}

fn open_run(access: u32) -> Option<Key> {
    let path = wide(RUN_KEY);
    let mut key: HKEY = std::ptr::null_mut();
    let status = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, access, &mut key) };
    (status == ERROR_SUCCESS).then_some(Key(key))
}

fn create_run() -> Option<Key> {
    let path = wide(RUN_KEY);
    let mut key: HKEY = std::ptr::null_mut();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            path.as_ptr(),
            0,
            std::ptr::null(),
            0,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        )
    };
    (status == ERROR_SUCCESS).then_some(Key(key))
}

fn read_value(name: &str) -> Option<String> {
    let key = open_run(KEY_QUERY_VALUE)?;
    let name = wide(name);
    let mut kind = 0u32;
    let mut bytes = 0u32;
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            name.as_ptr(),
            std::ptr::null(),
            &mut kind,
            std::ptr::null_mut(),
            &mut bytes,
        )
    };
    if status != ERROR_SUCCESS || kind != REG_SZ || bytes < 2 {
        return None;
    }

    let mut data = vec![0u16; (bytes as usize).div_ceil(2)];
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            name.as_ptr(),
            std::ptr::null(),
            &mut kind,
            data.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    if status != ERROR_SUCCESS || kind != REG_SZ {
        return None;
    }
    let end = data
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(data.len());
    Some(String::from_utf16_lossy(&data[..end]))
}

fn write_value(name: &str, value: &str) -> bool {
    let Some(key) = create_run() else {
        return false;
    };
    let name = wide(name);
    let data = wide(value);
    unsafe {
        RegSetValueExW(
            key.0,
            name.as_ptr(),
            0,
            REG_SZ,
            data.as_ptr().cast(),
            (data.len() * std::mem::size_of::<u16>()) as u32,
        ) == ERROR_SUCCESS
    }
}

fn delete_value(name: &str) -> bool {
    let Some(key) = open_run(KEY_SET_VALUE) else {
        return true;
    };
    let name = wide(name);
    let status = unsafe { RegDeleteValueW(key.0, name.as_ptr()) };
    status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND
}

fn stale_shortcut() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(PathBuf::from).map(|path| {
        path.join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Startup")
            .join("InputKey.lnk")
    })
}

fn remove_stale_shortcut() -> bool {
    let Some(path) = stale_shortcut() else {
        return true;
    };
    match std::fs::remove_file(path) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Err(_) => false,
    }
}

pub fn is_enabled() -> bool {
    let Ok(executable) = std::env::current_exe() else {
        return false;
    };
    let Some(expected) = startup_command(&executable) else {
        return false;
    };
    read_value(VALUE_NAME).is_some_and(|actual| actual.eq_ignore_ascii_case(&expected))
}

pub fn set_enabled(enabled: bool) -> bool {
    if enabled {
        let Ok(executable) = std::env::current_exe() else {
            return false;
        };
        let Some(command) = startup_command(&executable) else {
            return false;
        };
        if !write_value(VALUE_NAME, &command) {
            return false;
        }
        let legacy_removed = delete_value(LEGACY_VALUE_NAME);
        let shortcut_removed = remove_stale_shortcut();
        legacy_removed && shortcut_removed && is_enabled()
    } else {
        let current_removed = delete_value(VALUE_NAME);
        let legacy_removed = delete_value(LEGACY_VALUE_NAME);
        let shortcut_removed = remove_stale_shortcut();
        current_removed && legacy_removed && shortcut_removed && !is_enabled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_command_quotes_the_current_executable() {
        let executable = std::env::current_exe().expect("current executable");
        let command = startup_command(&executable).expect("absolute executable");
        assert!(command.starts_with('"'));
        assert!(command.ends_with('"'));
        assert!(command.contains(&executable.to_string_lossy().to_string()));
    }
}
