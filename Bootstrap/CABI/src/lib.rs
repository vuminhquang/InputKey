use inputkey_boundary::{InputEvent, InputType, Options};
use inputkey_english_bloom::EnglishBloom;
use inputkey_operators::Session;
use std::collections::HashMap;
use std::ffi::{c_char, CStr};
use std::sync::{Mutex, OnceLock};

static SESSIONS: OnceLock<Mutex<HashMap<u64, Session>>> = OnceLock::new();
static NEXT: OnceLock<Mutex<u64>> = OnceLock::new();
fn sessions() -> &'static Mutex<HashMap<u64, Session>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
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
        // SAFETY: caller promises writable output storage of `capacity` bytes.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, count);
            *out.add(count) = 0;
        }
    }
    bytes.len()
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
/// Creates an InputKey session.
///
/// # Safety
/// If `method` is non-null, it must point to a valid NUL-terminated C string for the duration of this call.
#[no_mangle]
pub unsafe extern "C" fn inputkey_create(
    method: *const c_char,
    simple_telex: i32,
    auto_restore: i32,
) -> u64 {
    let method = if method.is_null() {
        "telex"
    } else {
        // SAFETY: a non-null method must point to a valid NUL-terminated string for this call.
        unsafe { CStr::from_ptr(method) }
            .to_str()
            .unwrap_or("telex")
    };
    let options = Options {
        method: method.to_owned(),
        simple_telex: simple_telex != 0,
        auto_restore: auto_restore != 0,
        double_cancel: true,
        cancel_preference: "explicit".to_owned(),
    };
    let session = Session::new(options, Some(Box::new(EnglishBloom::new())), None);
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
#[no_mangle]
pub extern "C" fn inputkey_destroy(handle: u64) {
    if let Ok(mut all) = sessions().lock() {
        all.remove(&handle);
    }
}
/// Feeds one UTF-8 key payload into a session.
///
/// # Safety
/// When `len > 0`, `bytes` must point to at least `len` readable bytes. If `out` is non-null and `cap > 0`, it must point to at least `cap` writable bytes.
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
    // SAFETY: caller promises `bytes` points to `len` readable bytes for this call.
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
pub extern "C" fn inputkey_literalize_token(h: u64, o: *mut u8, c: usize) -> usize {
    command(h, InputType::LiteralizeToken, false, o, c)
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
