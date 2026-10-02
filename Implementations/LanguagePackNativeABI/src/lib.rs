//! Native function-table contract for dynamically loaded InputKey language packs.

pub const LANGUAGE_PACK_ABI_VERSION: u32 = 2;
pub const LANGUAGE_PACK_ENTRYPOINT: &str = "inputkey_language_pack_v2";

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LanguagePackApiV2 {
    pub abi_version: u32,
    pub struct_size: usize,
    pub metadata_json: unsafe extern "C" fn(out: *mut u8, cap: usize) -> usize,
    pub create: unsafe extern "C" fn(config_json: *const u8, len: usize) -> u64,
    pub destroy: unsafe extern "C" fn(handle: u64),
    pub accepts_key: unsafe extern "C" fn(handle: u64, key_utf8: *const u8, len: usize) -> i32,
    // Mutating functions execute exactly once per call, even when the output buffer is null.
    // Callers must not use the usual two-pass size/read pattern on them; mutate once,
    // then query rendered/state through the non-mutating functions below.
    pub key_utf8: unsafe extern "C" fn(
        handle: u64,
        key_utf8: *const u8,
        len: usize,
        out: *mut u8,
        cap: usize,
    ) -> usize,
    pub backspace: unsafe extern "C" fn(handle: u64, out: *mut u8, cap: usize) -> usize,
    pub escape: unsafe extern "C" fn(handle: u64, out: *mut u8, cap: usize) -> usize,
    // `finalize` performs ordinary language finalization only. The root calls
    // `correct_boundary` separately, and only for its Space decision boundary.
    pub finalize: unsafe extern "C" fn(handle: u64, out: *mut u8, cap: usize) -> usize,
    pub correct_boundary: unsafe extern "C" fn(handle: u64, out: *mut u8, cap: usize) -> usize,
    pub state_json: unsafe extern "C" fn(handle: u64, out: *mut u8, cap: usize) -> usize,
    pub rendered: unsafe extern "C" fn(handle: u64, out: *mut u8, cap: usize) -> usize,
    pub raw: unsafe extern "C" fn(handle: u64, out: *mut u8, cap: usize) -> usize,
    pub reset: unsafe extern "C" fn(handle: u64),
    pub has_history: unsafe extern "C" fn(handle: u64) -> i32,
}
