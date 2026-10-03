mod config;

pub use config::{EngineConfig, EngineFactory};

pub const CLSID_INPUTKEY_VALUE: u128 = 0x5f4a4c92_85b3_4f69_a6bc_b427d51d5e50;
pub const CLSID_INPUTKEY_STR: &str = "{5F4A4C92-85B3-4F69-A6BC-B427D51D5E50}";
pub const GUID_INPUTKEY_PROFILE_VALUE: u128 = 0x612a5f0f_30cd_49ad_9e95_40dc3a27d4f7;
pub const GUID_INPUTKEY_PROFILE_STR: &str = "{612A5F0F-30CD-49AD-9E95-40DC3A27D4F7}";

#[cfg(windows)]
pub const CLSID_INPUTKEY_TEXT_SERVICE: windows_core::GUID =
    windows_core::GUID::from_u128(CLSID_INPUTKEY_VALUE);
#[cfg(windows)]
pub const GUID_INPUTKEY_PROFILE: windows_core::GUID =
    windows_core::GUID::from_u128(GUID_INPUTKEY_PROFILE_VALUE);
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
    text_service_active, text_service_available, text_service_bound, text_service_registered,
    unregister_text_service,
};
#[cfg(windows)]
pub use service::{can_unload_now, class_factory};

#[cfg(not(windows))]
pub fn text_service_available() -> bool {
    false
}

#[cfg(not(windows))]
pub fn text_service_active() -> bool {
    false
}

#[cfg(not(windows))]
pub fn activate_text_service() -> Result<(), &'static str> {
    Err("Windows TSF is unavailable on this platform")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_com_identity() {
        assert_eq!(CLSID_INPUTKEY_VALUE, 0x5f4a4c92_85b3_4f69_a6bc_b427d51d5e50);
        assert_eq!(
            GUID_INPUTKEY_PROFILE_VALUE,
            0x612a5f0f_30cd_49ad_9e95_40dc3a27d4f7
        );
        assert_eq!(INPUTKEY_LANGID, 0x0409);
    }
}
