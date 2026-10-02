use crate::{
    CLSID_INPUTKEY_STR, CLSID_INPUTKEY_TEXT_SERVICE, GUID_INPUTKEY_PROFILE, INPUTKEY_LANGID,
};
use std::path::Path;
use windows::{
    core::{Error, Result, HRESULT, PCWSTR},
    Win32::{
        Foundation::ERROR_SUCCESS,
        System::{
            Com::{CoCreateInstance, CLSCTX_INPROC_SERVER},
            Registry::{
                RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY,
                HKEY_CURRENT_USER, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
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

    let profiles = unsafe { profiles()? };
    unsafe {
        profiles
            .Register(&CLSID_INPUTKEY_TEXT_SERVICE)
            .map_err(|e| Error::new(e.code(), "TSF Register text service"))?;
        let desc: Vec<u16> = "InputKey Vietnamese".encode_utf16().collect();
        let icon: Vec<u16> = Vec::new();
        profiles
            .AddLanguageProfile(
                &CLSID_INPUTKEY_TEXT_SERVICE,
                INPUTKEY_LANGID,
                &GUID_INPUTKEY_PROFILE,
                &desc,
                &icon,
                0,
            )
            .map_err(|e| Error::new(e.code(), "TSF AddLanguageProfile"))?;
        profiles
            .EnableLanguageProfile(
                &CLSID_INPUTKEY_TEXT_SERVICE,
                INPUTKEY_LANGID,
                &GUID_INPUTKEY_PROFILE,
                true,
            )
            .map_err(|e| Error::new(e.code(), "TSF EnableLanguageProfile"))?;
    }

    let categories = unsafe { categories()? };
    for category in [
        GUID_TFCAT_TIP_KEYBOARD,
        GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
        GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
        GUID_TFCAT_TIPCAP_TSF3,
    ] {
        unsafe {
            categories
                .RegisterCategory(
                    &CLSID_INPUTKEY_TEXT_SERVICE,
                    &category,
                    &CLSID_INPUTKEY_TEXT_SERVICE,
                )
                .map_err(|e| Error::new(e.code(), "TSF RegisterCategory"))?;
        }
    }
    Ok(())
}

pub fn text_service_available() -> bool {
    let Ok(profiles) = (unsafe { profiles() }) else {
        return false;
    };
    unsafe {
        profiles
            .IsEnabledLanguageProfile(
                &CLSID_INPUTKEY_TEXT_SERVICE,
                INPUTKEY_LANGID,
                &GUID_INPUTKEY_PROFILE,
            )
            .is_ok_and(|enabled| enabled.as_bool())
    }
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

    unregister_com_class()
}
