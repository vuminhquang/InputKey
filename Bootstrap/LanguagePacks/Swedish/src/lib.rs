#![allow(non_snake_case)]

use inputkey_language_pack_abi::{copy_utf8, read_utf8, PackHost};
use inputkey_language_pack_native_abi::{LanguagePackApiV3, LANGUAGE_PACK_ABI_VERSION};
use std::sync::{Arc, OnceLock};

fn host() -> &'static PackHost {
    static HOST: OnceLock<PackHost> = OnceLock::new();
    HOST.get_or_init(|| PackHost::new(Arc::new(inputkey_language_germanic::SwedishPack)))
}

unsafe extern "C" fn metadata_json(out: *mut u8, cap: usize) -> usize {
    unsafe { copy_utf8(&host().metadata_json(), out, cap) }
}

unsafe extern "C" fn create(config: *const u8, len: usize) -> u64 {
    host().create(unsafe { read_utf8(config, len) })
}

unsafe extern "C" fn destroy(handle: u64) {
    host().destroy(handle)
}

unsafe extern "C" fn accepts_character(handle: u64, character: *const u8, len: usize) -> i32 {
    unsafe { read_utf8(character, len) }
        .chars()
        .next()
        .is_some_and(|character| host().accepts_character(handle, character)) as i32
}

unsafe extern "C" fn transition_json(handle: u64, transition: *const u8, len: usize) -> i32 {
    host().transition_json(handle, unsafe { read_utf8(transition, len) }) as i32
}

unsafe extern "C" fn result(handle: u64, out: *mut u8, cap: usize) -> usize {
    unsafe { copy_utf8(&host().result(handle), out, cap) }
}

unsafe extern "C" fn state_json(handle: u64, out: *mut u8, cap: usize) -> usize {
    unsafe { copy_utf8(&host().state_json(handle), out, cap) }
}

static API: LanguagePackApiV3 = LanguagePackApiV3 {
    abi_version: LANGUAGE_PACK_ABI_VERSION,
    struct_size: std::mem::size_of::<LanguagePackApiV3>(),
    metadata_json,
    create,
    destroy,
    accepts_character,
    transition_json,
    result,
    state_json,
};

#[no_mangle]
pub extern "C" fn inputkey_language_pack_v3() -> *const LanguagePackApiV3 {
    &API
}
