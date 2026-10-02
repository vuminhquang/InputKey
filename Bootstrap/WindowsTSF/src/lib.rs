#![allow(non_snake_case)]
#![cfg_attr(not(windows), allow(dead_code))]

#[cfg(windows)]
mod windows_dll {
    use inputkey_core_abstractions::TypingEnginePort;
    use inputkey_runtime::{create_machine, Catalog};
    use inputkey_windows_tsf::{
        can_unload_now, class_factory, EngineConfig, EngineFactory, CLSID_INPUTKEY_TEXT_SERVICE,
    };
    use std::{ffi::c_void, sync::Arc};
    use windows::{
        core::{Interface, GUID, HRESULT},
        Win32::Foundation::{CLASS_E_CLASSNOTAVAILABLE, E_POINTER},
    };

    fn engine_factory() -> EngineFactory {
        let catalog = Arc::new(Catalog::installed());
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

    #[no_mangle]
    pub unsafe extern "system" fn DllGetClassObject(
        rclsid: *const GUID,
        riid: *const GUID,
        ppv: *mut *mut c_void,
    ) -> HRESULT {
        if rclsid.is_null() || riid.is_null() || ppv.is_null() {
            return E_POINTER;
        }
        unsafe {
            *ppv = std::ptr::null_mut();
            if *rclsid != CLSID_INPUTKEY_TEXT_SERVICE {
                return CLASS_E_CLASSNOTAVAILABLE;
            }
        }

        let factory = class_factory(engine_factory(), EngineConfig::default());
        unsafe { factory.query(riid, ppv) }
    }

    #[no_mangle]
    pub extern "system" fn DllCanUnloadNow() -> HRESULT {
        can_unload_now()
    }
}

#[cfg(not(windows))]
pub fn non_windows_marker() {}
