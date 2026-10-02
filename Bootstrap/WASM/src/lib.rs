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
pub unsafe extern "C" fn inputkey_accepts_key_utf8(h: u32, p: *const u8, n: usize) -> u32 {
    let key = unsafe { read_utf8(p, n) }.chars().next();
    let Some(key) = key else {
        return 0;
    };
    SESSIONS.with(|all| {
        all.borrow()
            .1
            .get(&h)
            .map(|session| u32::from(session.machine().accepts_key(key)))
            .unwrap_or(0)
    })
}

/// # Safety
/// p points to n UTF-8 bytes when n > 0.
#[no_mangle]
pub unsafe extern "C" fn inputkey_key_utf8(h: u32, p: *const u8, n: usize) {
    let key = unsafe { read_utf8(p, n) };
    action(h, |s| s.handle(event(InputType::Key, key)).rendered);
}

/// # Safety
/// p points to n UTF-8 bytes containing one delimiter.
#[no_mangle]
pub unsafe extern "C" fn inputkey_decision_boundary_utf8(h: u32, p: *const u8, n: usize) {
    let delimiter = unsafe { read_utf8(p, n) };
    action(h, |s| {
        s.handle(event(InputType::DecisionBoundary, delimiter))
            .commit
    });
}

fn cmd(h: u32, kind: InputType, commit: bool) {
    action(h, |s| {
        let out = s.handle(event(kind, String::new()));
        if commit {
            out.commit
        } else {
            out.rendered
        }
    });
}

#[no_mangle]
pub extern "C" fn inputkey_backspace(h: u32) {
    cmd(h, InputType::Backspace, false)
}

#[no_mangle]
pub extern "C" fn inputkey_escape(h: u32) {
    cmd(h, InputType::Escape, false)
}

#[no_mangle]
pub extern "C" fn inputkey_finalize(h: u32) {
    cmd(h, InputType::Finalize, true)
}

#[no_mangle]
pub extern "C" fn inputkey_commit_raw_boundary(h: u32) {
    cmd(h, InputType::CommitRawBoundary, true)
}

#[no_mangle]
pub extern "C" fn inputkey_commit_displayed(h: u32) {
    cmd(h, InputType::CommitDisplayed, true)
}

#[no_mangle]
pub extern "C" fn inputkey_reset(h: u32) {
    cmd(h, InputType::Reset, false)
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
