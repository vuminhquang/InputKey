use crate::{
    CLSID_INPUTKEY_STR, CLSID_INPUTKEY_TEXT_SERVICE, GUID_INPUTKEY_PROFILE,
    GUID_INPUTKEY_PROFILE_STR, INPUTKEY_LANGID,
};
use std::path::Path;
use windows::{
    core::{Error, Result, HRESULT, PCWSTR},
    Win32::{
        Foundation::ERROR_SUCCESS,
        System::{
            Com::{CoCreateInstance, CLSCTX_INPROC_SERVER},
            Registry::{
                RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegOpenKeyExW, RegSetValueExW, HKEY,
                HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SZ,
            },
        },
        UI::{
            Input::KeyboardAndMouse::HKL,
            TextServices::{
                CLSID_TF_CategoryMgr, CLSID_TF_InputProcessorProfiles, ITfCategoryMgr,
                ITfInputProcessorProfileMgr, ITfInputProcessorProfiles,
                GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT, GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
                GUID_TFCAT_TIPCAP_TSF3, GUID_TFCAT_TIP_KEYBOARD, TF_IPPMF_ENABLEPROFILE,
                TF_IPPMF_FORSESSION, TF_PROFILETYPE_INPUTPROCESSOR,
            },
        },
    },
};

fn wide_nul(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn win32_result(code: windows::Win32::Foundation::WIN32_ERROR) -> Result<()> {
    if code == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(Error::from_hresult(HRESULT::from_win32(code.0)))
    }
}

fn create_key(path: &str) -> Result<HKEY> {
    let path = wide_nul(path);
    let mut key = HKEY::default();
    win32_result(unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(path.as_ptr()),
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut key,
            None,
        )
    })?;
    Ok(key)
}

fn key_exists(path: &str) -> bool {
    let path = wide_nul(path);
    let mut key = HKEY::default();
    let code = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(path.as_ptr()),
            None,
            KEY_READ,
            &mut key,
        )
    };
    if code != ERROR_SUCCESS {
        return false;
    }
    unsafe {
        let _ = RegCloseKey(key);
    }
    true
}

fn set_default_sz(key: HKEY, value: &str) -> Result<()> {
    let value = wide_nul(value);
    let bytes = unsafe {
        std::slice::from_raw_parts(value.as_ptr().cast::<u8>(), value.len() * size_of::<u16>())
    };
    win32_result(unsafe { RegSetValueExW(key, PCWSTR::null(), None, REG_SZ, Some(bytes)) })
}

fn set_named_sz(key: HKEY, name: &str, value: &str) -> Result<()> {
    let name = wide_nul(name);
    let value = wide_nul(value);
    let bytes = unsafe {
        std::slice::from_raw_parts(value.as_ptr().cast::<u8>(), value.len() * size_of::<u16>())
    };
    win32_result(unsafe { RegSetValueExW(key, PCWSTR(name.as_ptr()), None, REG_SZ, Some(bytes)) })
}

fn set_named_dword(key: HKEY, name: &str, value: u32) -> Result<()> {
    let name = wide_nul(name);
    let bytes = value.to_ne_bytes();
    win32_result(unsafe {
        RegSetValueExW(key, PCWSTR(name.as_ptr()), None, REG_DWORD, Some(&bytes))
    })
}

fn user_profile_path() -> String {
    format!(
        r"Software\Microsoft\CTF\TIP\{CLSID_INPUTKEY_STR}\LanguageProfile\0x{INPUTKEY_LANGID:08X}\{GUID_INPUTKEY_PROFILE_STR}"
    )
}

fn register_user_tip_state() -> Result<()> {
    let key = create_key(&user_profile_path())?;
    let result = set_named_dword(key, "Enable", 1);
    unsafe {
        let _ = RegCloseKey(key);
    }
    result
}

fn register_com_class(dll_path: &Path) -> Result<()> {
    let root = format!(r"Software\Classes\CLSID\{CLSID_INPUTKEY_STR}");
    let key = create_key(&root)?;
    let result = set_default_sz(key, "InputKey Text Service");
    unsafe {
        let _ = RegCloseKey(key);
    }
    result?;

    let inproc_path = format!(r"{root}\InprocServer32");
    let key = create_key(&inproc_path)?;
    let dll = dll_path.to_string_lossy();
    let result =
        set_default_sz(key, &dll).and_then(|_| set_named_sz(key, "ThreadingModel", "Apartment"));
    unsafe {
        let _ = RegCloseKey(key);
    }
    result
}

fn unregister_com_class() -> Result<()> {
    let path = wide_nul(&format!(r"Software\Classes\CLSID\{CLSID_INPUTKEY_STR}"));
    let code = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(path.as_ptr())) };
    if code == ERROR_SUCCESS || code.0 == 2 {
        Ok(())
    } else {
        win32_result(code)
    }
}

fn unregister_user_tip_state() -> Result<()> {
    let path = wide_nul(&format!(r"Software\Microsoft\CTF\TIP\{CLSID_INPUTKEY_STR}"));
    let code = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(path.as_ptr())) };
    if code == ERROR_SUCCESS || code.0 == 2 {
        Ok(())
    } else {
        win32_result(code)
    }
}

pub fn text_service_registered() -> bool {
    key_exists(&user_profile_path())
}

pub fn text_service_bound() -> bool {
    key_exists(&format!(
        r"Software\Classes\CLSID\{CLSID_INPUTKEY_STR}\InprocServer32"
    ))
}

pub fn bind_text_service_dll(dll_path: &Path) -> Result<()> {
    register_com_class(dll_path)
}

unsafe fn profiles() -> Result<ITfInputProcessorProfiles> {
    unsafe { CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER) }
}

unsafe fn categories() -> Result<ITfCategoryMgr> {
    unsafe { CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER) }
}

unsafe fn profile_manager() -> Result<ITfInputProcessorProfileMgr> {
    unsafe { CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER) }
}

pub fn register_text_service(dll_path: &Path) -> Result<()> {
    register_com_class(dll_path)?;
    register_user_tip_state()?;

    if let Ok(categories) = unsafe { categories() } {
        for category in [
            GUID_TFCAT_TIP_KEYBOARD,
            GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
            GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
            GUID_TFCAT_TIPCAP_TSF3,
        ] {
            unsafe {
                let _ = categories.RegisterCategory(
                    &CLSID_INPUTKEY_TEXT_SERVICE,
                    &category,
                    &CLSID_INPUTKEY_TEXT_SERVICE,
                );
            }
        }
    }

    if let Ok(profiles) = unsafe { profiles() } {
        unsafe {
            let _ = profiles.EnableLanguageProfile(
                &CLSID_INPUTKEY_TEXT_SERVICE,
                INPUTKEY_LANGID,
                &GUID_INPUTKEY_PROFILE,
                true,
            );
        }
    }

    if text_service_registered() && text_service_bound() {
        Ok(())
    } else {
        Err(Error::new(
            HRESULT(0x80004005u32 as i32),
            "TSF current-user registration did not materialize",
        ))
    }
}

pub fn text_service_available() -> bool {
    text_service_registered() && text_service_bound()
}

pub fn text_service_active() -> bool {
    let Ok(profiles) = (unsafe { profiles() }) else {
        return false;
    };
    let mut langid = 0u16;
    let mut profile = windows::core::GUID::zeroed();
    unsafe {
        profiles
            .GetActiveLanguageProfile(&CLSID_INPUTKEY_TEXT_SERVICE, &mut langid, &mut profile)
            .is_ok()
            && langid == INPUTKEY_LANGID
            && profile == GUID_INPUTKEY_PROFILE
    }
}

pub fn activate_text_service() -> Result<()> {
    let manager = unsafe { profile_manager()? };
    unsafe {
        manager.ActivateProfile(
            TF_PROFILETYPE_INPUTPROCESSOR,
            INPUTKEY_LANGID,
            &CLSID_INPUTKEY_TEXT_SERVICE,
            &GUID_INPUTKEY_PROFILE,
            HKL::default(),
            TF_IPPMF_FORSESSION | TF_IPPMF_ENABLEPROFILE,
        )
    }
}

pub fn disable_text_service() -> Result<()> {
    if !text_service_registered() {
        return Ok(());
    }

    if let Ok(manager) = unsafe { profile_manager() } {
        unsafe {
            let _ = manager.DeactivateProfile(
                TF_PROFILETYPE_INPUTPROCESSOR,
                INPUTKEY_LANGID,
                &CLSID_INPUTKEY_TEXT_SERVICE,
                &GUID_INPUTKEY_PROFILE,
                HKL::default(),
                0,
            );
        }
    }

    let profiles = unsafe { profiles()? };
    unsafe {
        profiles.EnableLanguageProfile(
            &CLSID_INPUTKEY_TEXT_SERVICE,
            INPUTKEY_LANGID,
            &GUID_INPUTKEY_PROFILE,
            false,
        )
    }
}

pub fn unregister_text_service() -> Result<()> {
    if let Ok(manager) = unsafe { profile_manager() } {
        unsafe {
            let _ = manager.UnregisterProfile(
                &CLSID_INPUTKEY_TEXT_SERVICE,
                INPUTKEY_LANGID,
                &GUID_INPUTKEY_PROFILE,
                0,
            );
        }
    }

    if let Ok(categories) = unsafe { categories() } {
        for category in [
            GUID_TFCAT_TIP_KEYBOARD,
            GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
            GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
            GUID_TFCAT_TIPCAP_TSF3,
        ] {
            unsafe {
                let _ = categories.UnregisterCategory(
                    &CLSID_INPUTKEY_TEXT_SERVICE,
                    &category,
                    &CLSID_INPUTKEY_TEXT_SERVICE,
                );
            }
        }
    }

    if let Ok(profiles) = unsafe { profiles() } {
        unsafe {
            let _ = profiles.RemoveLanguageProfile(
                &CLSID_INPUTKEY_TEXT_SERVICE,
                INPUTKEY_LANGID,
                &GUID_INPUTKEY_PROFILE,
            );
            let _ = profiles.Unregister(&CLSID_INPUTKEY_TEXT_SERVICE);
        }
    }

    unregister_user_tip_state()?;
    unregister_com_class()
}
