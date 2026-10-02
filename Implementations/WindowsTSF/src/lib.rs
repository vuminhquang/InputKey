mod config;

pub use config::{EngineConfig, EngineFactory};

pub const CLSID_INPUTKEY_TEXT_SERVICE: windows_core::GUID =
    windows_core::GUID::from_u128(0x5f4a4c92_85b3_4f69_a6bc_b427d51d5e50);
pub const CLSID_INPUTKEY_STR: &str = "{5F4A4C92-85B3-4F69-A6BC-B427D51D5E50}";
pub const GUID_INPUTKEY_PROFILE: windows_core::GUID =
    windows_core::GUID::from_u128(0x612a5f0f_30cd_49ad_9e95_40dc3a27d4f7);
// Register against the user's actual US keyboard language; InputKey supplies the selected
// language transformation while preserving the physical keyboard layout.
pub const INPUTKEY_LANGID: u16 = 0x0409;

#[cfg(windows)]
mod registration;
#[cfg(windows)]
mod service;

#[cfg(windows)]
pub use registration::{
    activate_text_service, bind_text_service_dll, disable_text_service, register_text_service,
    text_service_active, text_service_available, unregister_text_service,
};
#[cfg(windows)]
pub use service::{can_unload_now, class_factory};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_com_identity() {
        assert_eq!(
            CLSID_INPUTKEY_TEXT_SERVICE,
            windows_core::GUID::from_u128(0x5f4a4c92_85b3_4f69_a6bc_b427d51d5e50)
        );
        assert_eq!(INPUTKEY_LANGID, 0x0409);
    }
}
