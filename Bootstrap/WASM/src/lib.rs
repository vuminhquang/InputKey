use inputkey_boundary::{InputEvent, InputType, Options};
use inputkey_english_bloom::EnglishBloom;
use inputkey_operators::Session;
use std::cell::RefCell;
use std::collections::HashMap;
thread_local! { static SESSIONS: RefCell<(u32,HashMap<u32,Session>)> = RefCell::new((1,HashMap::new())); static OUTPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) }; }
fn event(k: InputType, key: String) -> InputEvent {
    InputEvent {
        kind: k,
        key,
        timestamp: 0,
    }
}
fn output(s: String) {
    OUTPUT.with(|o| {
        let mut o = o.borrow_mut();
        o.clear();
        o.extend_from_slice(s.as_bytes());
    });
}
fn action(h: u32, f: impl FnOnce(&mut Session) -> String) {
    let s = SESSIONS.with(|all| {
        let mut all = all.borrow_mut();
        all.1.get_mut(&h).map(f).unwrap_or_default()
    });
    output(s);
}
#[no_mangle]
pub extern "C" fn inputkey_alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len);
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}
/// Releases a buffer previously returned by `inputkey_alloc`.
///
/// # Safety
/// `p` and `len` must be the exact pointer and capacity returned/used by `inputkey_alloc`, and the allocation must not have been freed already.
#[no_mangle]
pub unsafe extern "C" fn inputkey_free(p: *mut u8, len: usize) {
    if !p.is_null() {
        // SAFETY: pointer/length must be from inputkey_alloc with same length.
        unsafe {
            drop(Vec::from_raw_parts(p, 0, len));
        }
    }
}
#[no_mangle]
pub extern "C" fn inputkey_create(method: u32, simple: u32, restore: u32) -> u32 {
    SESSIONS.with(|a| {
        let mut a = a.borrow_mut();
        let h = a.0;
        a.0 = a.0.wrapping_add(1).max(1);
        let o = Options {
            method: if method == 1 { "vni" } else { "telex" }.into(),
            simple_telex: simple != 0,
            auto_restore: restore != 0,
            double_cancel: true,
            cancel_preference: "explicit".into(),
        };
        a.1.insert(
            h,
            Session::new(o, Some(Box::new(EnglishBloom::new())), None),
        );
        h
    })
}
#[no_mangle]
pub extern "C" fn inputkey_destroy(h: u32) {
    SESSIONS.with(|a| {
        a.borrow_mut().1.remove(&h);
    });
}
/// Feeds UTF-8 bytes from WASM linear memory into a session.
///
/// # Safety
/// When `n > 0`, `p` must point to at least `n` readable bytes in this module's linear memory for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn inputkey_key_utf8(h: u32, p: *const u8, n: usize) {
    let k = if p.is_null() {
        String::new()
    } else {
        // SAFETY: caller provides n readable bytes.
        String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(p, n) }).into_owned()
    };
    action(h, |s| s.handle(event(InputType::Key, k)).rendered);
}
fn cmd(h: u32, k: InputType, commit: bool) {
    action(h, |s| {
        let o = s.handle(event(k, String::new()));
        if commit {
            o.commit
        } else {
            o.rendered
        }
    })
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
pub extern "C" fn inputkey_literalize_token(h: u32) {
    cmd(h, InputType::LiteralizeToken, false)
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
