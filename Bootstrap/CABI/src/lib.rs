use inputkey_boundary::{InputEvent, InputType};
use inputkey_core_abstractions::LanguageCatalogPort;
use inputkey_operators::Session;
use inputkey_runtime::{catalog_json, config_from_json, create_machine, Catalog};
use std::collections::HashMap;
use std::ffi::{c_char, CStr};
use std::sync::{Mutex, OnceLock};

static SESSIONS: OnceLock<Mutex<HashMap<u64, Session>>> = OnceLock::new();
static NEXT: OnceLock<Mutex<u64>> = OnceLock::new();
static CATALOG: OnceLock<Catalog> = OnceLock::new();

fn sessions() -> &'static Mutex<HashMap<u64, Session>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn catalog() -> &'static Catalog {
    CATALOG.get_or_init(Catalog::installed)
}

fn event(kind: InputType, key: String) -> InputEvent {
    InputEvent {
        kind,
        key,
        timestamp: 0,
    }
}

fn copy(text: &str, out: *mut u8, capacity: usize) -> usize {
    let bytes = text.as_bytes();
    if !out.is_null() && capacity > 0 {
        let count = bytes.len().min(capacity - 1);
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, count);
            *out.add(count) = 0;
        }
    }
    bytes.len()
}

fn cstr(value: *const c_char, fallback: &str) -> &str {
    if value.is_null() {
        return fallback;
    }
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .unwrap_or(fallback)
}

fn result(
    handle: u64,
    action: impl FnOnce(&mut Session) -> String,
    out: *mut u8,
    cap: usize,
) -> usize {
    let text = sessions()
        .lock()
        .ok()
        .and_then(|mut all| all.get_mut(&handle).map(action))
        .unwrap_or_default();
    copy(&text, out, cap)
}

fn insert_session(language: &str, method: Option<&str>, options_json: Option<&str>) -> u64 {
    let catalog = catalog();
    let config = config_from_json(catalog, language, method, options_json);
    let Ok(machine) = create_machine(catalog, language, config) else {
        return 0;
    };
    let session = Session::new(machine, None);
    let mut next = NEXT
        .get_or_init(|| Mutex::new(1))
        .lock()
        .expect("handle lock");
    let handle = *next;
    *next += 1;
    sessions()
        .lock()
        .expect("session lock")
        .insert(handle, session);
    handle
}

/// Creates a language-neutral InputKey session.
///
/// # Safety
/// Non-null string pointers must be valid NUL-terminated UTF-8 for this call.
#[no_mangle]
pub unsafe extern "C" fn inputkey_create_ex(
    language: *const c_char,
    method: *const c_char,
    options_json: *const c_char,
) -> u64 {
    let language = cstr(language, "vi");
    let method = (!method.is_null()).then(|| cstr(method, ""));
    let options_json = (!options_json.is_null()).then(|| cstr(options_json, ""));
    insert_session(language, method, options_json)
}

/// Legacy Vietnamese constructor retained for existing adapters.
///
/// # Safety
/// If method is non-null it must point to a valid NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn inputkey_create(
    method: *const c_char,
    simple_telex: i32,
    auto_restore: i32,
) -> u64 {
    let method = cstr(method, "telex");
    let options = format!(
        "{{\"simple_telex\":{},\"auto_restore\":{},\"smart_correction\":true}}",
        simple_telex != 0,
        auto_restore != 0
    );
    insert_session("vi", Some(method), Some(&options))
}

#[no_mangle]
pub extern "C" fn inputkey_destroy(handle: u64) {
    if let Ok(mut all) = sessions().lock() {
        all.remove(&handle);
    }
}

#[no_mangle]
pub extern "C" fn inputkey_catalog_json(out: *mut u8, cap: usize) -> usize {
    copy(&catalog_json(catalog()), out, cap)
}

/// # Safety
/// When len > 0, bytes must point to len readable bytes.
#[no_mangle]
pub unsafe extern "C" fn inputkey_accepts_key_utf8(
    handle: u64,
    bytes: *const u8,
    len: usize,
) -> i32 {
    if bytes.is_null() || len == 0 {
        return 0;
    }
    let text = String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(bytes, len) });
    let Some(key) = text.chars().next() else {
        return 0;
    };
    sessions()
        .lock()
        .ok()
        .and_then(|all| all.get(&handle).map(|s| s.machine().accepts_key(key)))
        .map(i32::from)
        .unwrap_or(0)
}

/// # Safety
/// When len > 0, bytes must point to len readable bytes. out/cap must describe
/// writable storage when supplied.
#[no_mangle]
pub unsafe extern "C" fn inputkey_key_utf8(
    handle: u64,
    bytes: *const u8,
    len: usize,
    out: *mut u8,
    cap: usize,
) -> usize {
    if bytes.is_null() && len != 0 {
        return copy("", out, cap);
    }
    let key = String::from_utf8_lossy(if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(bytes, len) }
    })
    .into_owned();
    result(
        handle,
        |s| s.handle(event(InputType::Key, key)).rendered,
        out,
        cap,
    )
}

/// # Safety
/// delimiter must point to a valid UTF-8 character when len > 0.
#[no_mangle]
pub unsafe extern "C" fn inputkey_decision_boundary_utf8(
    handle: u64,
    delimiter: *const u8,
    len: usize,
    out: *mut u8,
    cap: usize,
) -> usize {
    if delimiter.is_null() || len == 0 {
        return copy("", out, cap);
    }
    let delimiter =
        String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(delimiter, len) }).into_owned();
    result(
        handle,
        |s| {
            s.handle(event(InputType::DecisionBoundary, delimiter))
                .commit
        },
        out,
        cap,
    )
}

fn command(handle: u64, kind: InputType, commit: bool, out: *mut u8, cap: usize) -> usize {
    result(
        handle,
        |s| {
            let o = s.handle(event(kind, String::new()));
            if commit {
                o.commit
            } else {
                o.rendered
            }
        },
        out,
        cap,
    )
}

#[no_mangle]
pub extern "C" fn inputkey_backspace(h: u64, o: *mut u8, c: usize) -> usize {
    command(h, InputType::Backspace, false, o, c)
}

#[no_mangle]
pub extern "C" fn inputkey_escape(h: u64, o: *mut u8, c: usize) -> usize {
    command(h, InputType::Escape, false, o, c)
}

#[no_mangle]
pub extern "C" fn inputkey_finalize(h: u64, o: *mut u8, c: usize) -> usize {
    command(h, InputType::Finalize, true, o, c)
}

#[no_mangle]
pub extern "C" fn inputkey_commit_raw_boundary(h: u64, o: *mut u8, c: usize) -> usize {
    command(h, InputType::CommitRawBoundary, true, o, c)
}

#[no_mangle]
pub extern "C" fn inputkey_commit_displayed(h: u64, o: *mut u8, c: usize) -> usize {
    command(h, InputType::CommitDisplayed, true, o, c)
}

#[no_mangle]
pub extern "C" fn inputkey_reset(h: u64) {
    let _ = result(
        h,
        |s| {
            s.handle(event(InputType::Reset, String::new()));
            String::new()
        },
        std::ptr::null_mut(),
        0,
    );
}

#[no_mangle]
pub extern "C" fn inputkey_rendered(h: u64, o: *mut u8, c: usize) -> usize {
    result(h, |s| s.state().rendered, o, c)
}

#[no_mangle]
pub extern "C" fn inputkey_raw(h: u64, o: *mut u8, c: usize) -> usize {
    result(h, |s| s.state().raw, o, c)
}

#[no_mangle]
pub extern "C" fn inputkey_has_history(h: u64) -> i32 {
    sessions()
        .lock()
        .ok()
        .and_then(|s| s.get(&h).map(|s| i32::from(s.machine().has_history())))
        .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn inputkey_language_count() -> usize {
    catalog().languages().len()
}

fn language_at(index: usize) -> Option<inputkey_core_abstractions::LanguageMetadata> {
    catalog().languages().into_iter().nth(index)
}

fn language_named(id: &str) -> Option<inputkey_core_abstractions::LanguageMetadata> {
    catalog()
        .languages()
        .into_iter()
        .find(|language| language.id == id)
}

#[no_mangle]
pub extern "C" fn inputkey_language_id(index: usize, out: *mut u8, cap: usize) -> usize {
    copy(
        &language_at(index)
            .map(|language| language.id)
            .unwrap_or_default(),
        out,
        cap,
    )
}

#[no_mangle]
pub extern "C" fn inputkey_language_name(index: usize, out: *mut u8, cap: usize) -> usize {
    copy(
        &language_at(index)
            .map(|language| language.native_name)
            .unwrap_or_default(),
        out,
        cap,
    )
}

#[no_mangle]
pub extern "C" fn inputkey_language_default_method(
    index: usize,
    out: *mut u8,
    cap: usize,
) -> usize {
    copy(
        &language_at(index)
            .map(|language| language.default_method)
            .unwrap_or_default(),
        out,
        cap,
    )
}

/// Returns the number of methods exposed by a language pack.
///
/// # Safety
/// `language` must be null or point to a valid NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn inputkey_method_count(language: *const c_char) -> usize {
    language_named(cstr(language, ""))
        .map(|language| language.methods.len())
        .unwrap_or(0)
}

/// Copies a method id from the selected language metadata.
///
/// # Safety
/// `language` must be null or valid NUL-terminated UTF-8. When `out` is non-null,
/// it must point to at least `cap` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn inputkey_method_id(
    language: *const c_char,
    index: usize,
    out: *mut u8,
    cap: usize,
) -> usize {
    let value = language_named(cstr(language, ""))
        .and_then(|language| language.methods.into_iter().nth(index))
        .map(|method| method.id)
        .unwrap_or_default();
    copy(&value, out, cap)
}

/// Copies a method label from the selected language metadata.
///
/// # Safety
/// `language` must be null or valid NUL-terminated UTF-8. When `out` is non-null,
/// it must point to at least `cap` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn inputkey_method_label(
    language: *const c_char,
    index: usize,
    out: *mut u8,
    cap: usize,
) -> usize {
    let value = language_named(cstr(language, ""))
        .and_then(|language| language.methods.into_iter().nth(index))
        .map(|method| method.label)
        .unwrap_or_default();
    copy(&value, out, cap)
}

/// Returns the number of boolean options exposed by a language pack.
///
/// # Safety
/// `language` must be null or point to a valid NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn inputkey_option_count(language: *const c_char) -> usize {
    language_named(cstr(language, ""))
        .map(|language| language.options.len())
        .unwrap_or(0)
}

/// Copies an option id from the selected language metadata.
///
/// # Safety
/// `language` must be null or valid NUL-terminated UTF-8. When `out` is non-null,
/// it must point to at least `cap` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn inputkey_option_id(
    language: *const c_char,
    index: usize,
    out: *mut u8,
    cap: usize,
) -> usize {
    let value = language_named(cstr(language, ""))
        .and_then(|language| language.options.into_iter().nth(index))
        .map(|option| option.id)
        .unwrap_or_default();
    copy(&value, out, cap)
}

/// Copies an option label from the selected language metadata.
///
/// # Safety
/// `language` must be null or valid NUL-terminated UTF-8. When `out` is non-null,
/// it must point to at least `cap` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn inputkey_option_label(
    language: *const c_char,
    index: usize,
    out: *mut u8,
    cap: usize,
) -> usize {
    let value = language_named(cstr(language, ""))
        .and_then(|language| language.options.into_iter().nth(index))
        .map(|option| option.label)
        .unwrap_or_default();
    copy(&value, out, cap)
}

/// Returns whether a language option is enabled by default.
///
/// # Safety
/// `language` must be null or point to a valid NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn inputkey_option_default_enabled(
    language: *const c_char,
    index: usize,
) -> i32 {
    language_named(cstr(language, ""))
        .and_then(|language| language.options.into_iter().nth(index))
        .map(|option| i32::from(option.default_enabled))
        .unwrap_or(0)
}
