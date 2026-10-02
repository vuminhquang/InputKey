#![allow(non_snake_case)]

use inputkey_language_pack_abi::{copy_utf8, read_utf8, PackHost};
use inputkey_language_pack_native_abi::{LanguagePackApiV2, LANGUAGE_PACK_ABI_VERSION};
use std::sync::{Arc, OnceLock};

fn host() -> &'static PackHost {
    static HOST: OnceLock<PackHost> = OnceLock::new();
    HOST.get_or_init(|| PackHost::new(Arc::new(inputkey_language_french::FrenchPack)))
}

unsafe extern "C" fn metadata_json(out: *mut u8, cap: usize) -> usize {
    copy_utf8(&host().metadata_json(), out, cap)
}

unsafe extern "C" fn create(config: *const u8, len: usize) -> u64 {
    host().create(unsafe { read_utf8(config, len) })
}

unsafe extern "C" fn destroy(handle: u64) {
    host().destroy(handle)
}

unsafe extern "C" fn accepts_key(handle: u64, key: *const u8, len: usize) -> i32 {
    unsafe { read_utf8(key, len) }
        .chars()
        .next()
        .is_some_and(|key| host().accepts_key(handle, key)) as i32
}

unsafe extern "C" fn key_utf8(
    handle: u64,
    key: *const u8,
    len: usize,
    out: *mut u8,
    cap: usize,
) -> usize {
    let value = unsafe { read_utf8(key, len) }
        .chars()
        .next()
        .map(|key| host().type_key(handle, key))
        .unwrap_or_default();
    copy_utf8(&value, out, cap)
}

unsafe extern "C" fn backspace(handle: u64, out: *mut u8, cap: usize) -> usize {
    copy_utf8(&host().backspace(handle), out, cap)
}

unsafe extern "C" fn escape(handle: u64, out: *mut u8, cap: usize) -> usize {
    copy_utf8(&host().escape(handle), out, cap)
}

unsafe extern "C" fn finalize(handle: u64, out: *mut u8, cap: usize) -> usize {
    copy_utf8(&host().finalize(handle), out, cap)
}

unsafe extern "C" fn correct_boundary(handle: u64, out: *mut u8, cap: usize) -> usize {
    copy_utf8(&host().correct_boundary(handle), out, cap)
}

unsafe extern "C" fn state_json(handle: u64, out: *mut u8, cap: usize) -> usize {
    copy_utf8(&host().state_json(handle), out, cap)
}

unsafe extern "C" fn rendered(handle: u64, out: *mut u8, cap: usize) -> usize {
    copy_utf8(&host().rendered(handle), out, cap)
}

unsafe extern "C" fn raw(handle: u64, out: *mut u8, cap: usize) -> usize {
    copy_utf8(&host().raw(handle), out, cap)
}

unsafe extern "C" fn reset(handle: u64) {
    host().reset(handle)
}

unsafe extern "C" fn has_history(handle: u64) -> i32 {
    host().has_history(handle) as i32
}

static API: LanguagePackApiV2 = LanguagePackApiV2 {
    abi_version: LANGUAGE_PACK_ABI_VERSION,
    struct_size: std::mem::size_of::<LanguagePackApiV2>(),
    metadata_json,
    create,
    destroy,
    accepts_key,
    key_utf8,
    backspace,
    escape,
    finalize,
    correct_boundary,
    state_json,
    rendered,
    raw,
    reset,
    has_history,
};

#[no_mangle]
pub extern "C" fn inputkey_language_pack_v2() -> *const LanguagePackApiV2 {
    &API
}
