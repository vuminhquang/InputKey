//! Native function-table contract for dynamically loaded InputKey language packs.

pub const LANGUAGE_PACK_ABI_VERSION: u32 = 3;
pub const LANGUAGE_PACK_ENTRYPOINT: &str = "inputkey_language_pack_v3";

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LanguagePackApiV3 {
    pub abi_version: u32,
    pub struct_size: usize,
    pub metadata_json: unsafe extern "C" fn(out: *mut u8, cap: usize) -> usize,
    pub create: unsafe extern "C" fn(config_json: *const u8, len: usize) -> u64,
    pub destroy: unsafe extern "C" fn(handle: u64),
    pub accepts_character:
        unsafe extern "C" fn(handle: u64, character_utf8: *const u8, len: usize) -> i32,
    /// Applies one semantic root transition exactly once.
    pub transition_json:
        unsafe extern "C" fn(handle: u64, transition_json: *const u8, len: usize) -> i32,
    /// Reads the result produced by the most recent transition.
    pub result: unsafe extern "C" fn(handle: u64, out: *mut u8, cap: usize) -> usize,
    pub state_json: unsafe extern "C" fn(handle: u64, out: *mut u8, cap: usize) -> usize,
}
