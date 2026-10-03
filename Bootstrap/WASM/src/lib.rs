use inputkey_boundary::{InputEvent, InputType};
use inputkey_operators::Session;
use inputkey_runtime::{catalog_json, config_from_json, create_machine, Catalog};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static SESSIONS: RefCell<(u32, HashMap<u32, Session>)> = RefCell::new((1, HashMap::new()));
    static OUTPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static CATALOG: Catalog = Catalog::bundled();
}

fn event(kind: InputType, key: String) -> InputEvent {
    InputEvent {
        kind,
        key,
        timestamp: 0,
    }
}

fn output(value: String) {
    OUTPUT.with(|o| {
        let mut o = o.borrow_mut();
        o.clear();
        o.extend_from_slice(value.as_bytes());
    });
}

fn action(handle: u32, f: impl FnOnce(&mut Session) -> String) {
    let value = SESSIONS.with(|all| {
        let mut all = all.borrow_mut();
        all.1.get_mut(&handle).map(f).unwrap_or_default()
    });
    output(value);
}

unsafe fn read_utf8(pointer: *const u8, len: usize) -> String {
    if pointer.is_null() || len == 0 {
        return String::new();
    }
    String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(pointer, len) }).into_owned()
}

fn insert(language: &str, method: Option<&str>, options_json: Option<&str>) -> u32 {
    CATALOG.with(|catalog| {
        let config = config_from_json(catalog, language, method, options_json);
        let Ok(machine) = create_machine(catalog, language, config) else {
            return 0;
        };
        SESSIONS.with(|all| {
            let mut all = all.borrow_mut();
            let handle = all.0;
            all.0 = all.0.wrapping_add(1).max(1);
            all.1.insert(handle, Session::new(machine, None));
            handle
        })
    })
}

#[no_mangle]
pub extern "C" fn inputkey_alloc(len: usize) -> *mut u8 {
    let mut value = Vec::<u8>::with_capacity(len);
    let pointer = value.as_mut_ptr();
    std::mem::forget(value);
    pointer
}

/// # Safety
/// p/len must match a buffer allocated by inputkey_alloc.
#[no_mangle]
pub unsafe extern "C" fn inputkey_free(p: *mut u8, len: usize) {
    if !p.is_null() {
        unsafe {
            drop(Vec::from_raw_parts(p, 0, len));
        }
    }
}

#[no_mangle]
pub extern "C" fn inputkey_create(method: u32, simple: u32, restore: u32) -> u32 {
    let method = if method == 1 { "vni" } else { "telex" };
    let options = format!(
        "{{\"simple_telex\":{},\"auto_restore\":{},\"smart_correction\":true}}",
        simple != 0,
        restore != 0
    );
    insert("vi", Some(method), Some(&options))
}

/// # Safety
/// Every non-null pointer must reference the declared number of UTF-8 bytes.
#[no_mangle]
pub unsafe extern "C" fn inputkey_create_ex(
    language: *const u8,
    language_len: usize,
    method: *const u8,
    method_len: usize,
    options: *const u8,
    options_len: usize,
) -> u32 {
    let language = unsafe { read_utf8(language, language_len) };
    let method = unsafe { read_utf8(method, method_len) };
    let options = unsafe { read_utf8(options, options_len) };
    insert(
        if language.is_empty() { "vi" } else { &language },
        (!method.is_empty()).then_some(method.as_str()),
        (!options.is_empty()).then_some(options.as_str()),
    )
}

#[no_mangle]
pub extern "C" fn inputkey_destroy(h: u32) {
    SESSIONS.with(|all| {
        all.borrow_mut().1.remove(&h);
    });
}

#[no_mangle]
pub extern "C" fn inputkey_catalog_json() {
    CATALOG.with(|catalog| output(catalog_json(catalog)));
}

/// # Safety
/// p points to n UTF-8 bytes when n > 0.
#[no_mangle]
pub unsafe extern "C" fn inputkey_accepts_character_utf8(h: u32, p: *const u8, n: usize) -> u32 {
    let character = unsafe { read_utf8(p, n) }.chars().next();
    let Some(character) = character else {
        return 0;
    };
    SESSIONS.with(|all| {
        all.borrow()
            .1
            .get(&h)
            .map(|session| u32::from(session.machine().accepts_character(character)))
            .unwrap_or(0)
    })
}

fn cmd(h: u32, kind: InputType, key: String, commit: bool) {
    action(h, |session| {
        let out = session.handle(event(kind, key));
        if commit {
            out.commit
        } else {
            out.rendered
        }
    });
}

/// # Safety
/// p points to n UTF-8 bytes containing one character.
#[no_mangle]
pub unsafe extern "C" fn inputkey_character_utf8(h: u32, p: *const u8, n: usize) {
    cmd(h, InputType::Character, unsafe { read_utf8(p, n) }, false);
}

#[no_mangle]
pub extern "C" fn inputkey_space_boundary(h: u32) {
    cmd(h, InputType::SpaceBoundary, String::new(), true);
}

/// # Safety
/// p points to n UTF-8 bytes containing one punctuation character.
#[no_mangle]
pub unsafe extern "C" fn inputkey_punctuation_boundary_utf8(h: u32, p: *const u8, n: usize) {
    cmd(
        h,
        InputType::PunctuationBoundary,
        unsafe { read_utf8(p, n) },
        true,
    );
}

/// # Safety
/// p points to n UTF-8 bytes naming the caret move cause.
#[no_mangle]
pub unsafe extern "C" fn inputkey_caret_move_boundary_utf8(h: u32, p: *const u8, n: usize) {
    cmd(
        h,
        InputType::CaretMoveBoundary,
        unsafe { read_utf8(p, n) },
        true,
    );
}

#[no_mangle]
pub extern "C" fn inputkey_shortcut_boundary(h: u32) {
    cmd(h, InputType::ShortcutBoundary, String::new(), true);
}

/// # Safety
/// p points to n UTF-8 bytes naming the composition control.
#[no_mangle]
pub unsafe extern "C" fn inputkey_composition_control_utf8(h: u32, p: *const u8, n: usize) {
    cmd(
        h,
        InputType::CompositionControl,
        unsafe { read_utf8(p, n) },
        false,
    );
}

#[no_mangle]
pub extern "C" fn inputkey_raw_boundary(h: u32) {
    cmd(h, InputType::RawBoundary, String::new(), true);
}

/// # Safety
/// p points to n UTF-8 bytes naming the lifecycle event.
#[no_mangle]
pub unsafe extern "C" fn inputkey_lifecycle_utf8(h: u32, p: *const u8, n: usize) {
    cmd(h, InputType::Lifecycle, unsafe { read_utf8(p, n) }, true);
}

#[no_mangle]
pub extern "C" fn inputkey_raw(h: u32) {
    action(h, |s| s.state().raw)
}

#[no_mangle]
pub extern "C" fn inputkey_rendered(h: u32) {
    action(h, |s| s.state().rendered)
}

#[no_mangle]
pub extern "C" fn inputkey_output_ptr() -> *const u8 {
    OUTPUT.with(|o| o.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn inputkey_output_len() -> usize {
    OUTPUT.with(|o| o.borrow().len())
}
