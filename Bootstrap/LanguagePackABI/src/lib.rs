use inputkey_core_abstractions::{
    LanguageConfig, LanguageMachinePort, LanguageMetadata, LanguagePackPort,
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

pub struct PackHost {
    pack: Arc<dyn LanguagePackPort>,
    next: AtomicU64,
    sessions: Mutex<HashMap<u64, Box<dyn LanguageMachinePort>>>,
}

impl PackHost {
    pub fn new(pack: Arc<dyn LanguagePackPort>) -> Self {
        Self {
            pack,
            next: AtomicU64::new(1),
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn metadata_json(&self) -> String {
        metadata_json(self.pack.metadata())
    }

    pub fn create(&self, config_json: &str) -> u64 {
        let config = parse_config(config_json, &self.pack.metadata());
        let Ok(machine) = self.pack.create(config) else {
            return 0;
        };
        let handle = self.next.fetch_add(1, Ordering::Relaxed).max(1);
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.insert(handle, machine);
            handle
        } else {
            0
        }
    }

    pub fn destroy(&self, handle: u64) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(&handle);
        }
    }

    pub fn accepts_key(&self, handle: u64, key: char) -> bool {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(&handle).map(|m| m.accepts_key(key)))
            .unwrap_or(false)
    }

    pub fn type_key(&self, handle: u64, key: char) -> String {
        self.with_mut(handle, |machine| machine.type_key(key))
    }

    pub fn backspace(&self, handle: u64) -> String {
        self.with_mut(handle, |machine| machine.backspace())
    }

    pub fn escape(&self, handle: u64) -> String {
        self.with_mut(handle, |machine| machine.escape())
    }

    pub fn finalize(&self, handle: u64) -> String {
        self.with_mut(handle, |machine| machine.finalize())
    }

    pub fn reset(&self, handle: u64) {
        let _ = self.with_mut(handle, |machine| {
            machine.reset();
            String::new()
        });
    }

    pub fn rendered(&self, handle: u64) -> String {
        self.with_ref(handle, |machine| machine.rendered())
    }

    pub fn raw(&self, handle: u64) -> String {
        self.with_ref(handle, |machine| machine.raw())
    }

    pub fn has_history(&self, handle: u64) -> bool {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(&handle).map(|m| m.history_active()))
            .unwrap_or(false)
    }

    pub fn state_json(&self, handle: u64) -> String {
        self.with_ref(handle, |machine| {
            let state = machine.state();
            json!({
                "raw": state.raw,
                "rendered": state.rendered,
                "mode": state.mode,
                "phase": state.phase
            })
            .to_string()
        })
    }

    fn with_mut(
        &self,
        handle: u64,
        action: impl FnOnce(&mut dyn LanguageMachinePort) -> String,
    ) -> String {
        self.sessions
            .lock()
            .ok()
            .and_then(|mut sessions| {
                sessions
                    .get_mut(&handle)
                    .map(|machine| action(machine.as_mut()))
            })
            .unwrap_or_default()
    }

    fn with_ref(
        &self,
        handle: u64,
        action: impl FnOnce(&dyn LanguageMachinePort) -> String,
    ) -> String {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| {
                sessions
                    .get(&handle)
                    .map(|machine| action(machine.as_ref()))
            })
            .unwrap_or_default()
    }
}

/// Copies UTF-8 into caller-provided storage and returns the full byte length.
///
/// # Safety
/// When `out` is non-null, it must point to at least `cap` writable bytes.
pub unsafe fn copy_utf8(text: &str, out: *mut u8, cap: usize) -> usize {
    let bytes = text.as_bytes();
    if !out.is_null() && cap > 0 {
        let count = bytes.len().min(cap - 1);
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, count);
            *out.add(count) = 0;
        }
    }
    bytes.len()
}

/// Reads a borrowed UTF-8 view from a caller-provided buffer.
///
/// # Safety
/// When `len` is non-zero, `ptr` must point to at least `len` readable bytes
/// that remain valid for the returned lifetime.
pub unsafe fn read_utf8<'a>(ptr: *const u8, len: usize) -> &'a str {
    if ptr.is_null() || len == 0 {
        return "";
    }
    std::str::from_utf8(unsafe { std::slice::from_raw_parts(ptr, len) }).unwrap_or("")
}

fn parse_config(text: &str, metadata: &LanguageMetadata) -> LanguageConfig {
    let mut config = LanguageConfig::new(metadata.default_method.clone());
    for option in &metadata.options {
        config
            .toggles
            .insert(option.id.clone(), option.default_enabled);
    }
    let Ok(Value::Object(root)) = serde_json::from_str::<Value>(text) else {
        return config;
    };
    if let Some(method) = root.get("method").and_then(Value::as_str) {
        config.method = method.to_owned();
    }
    if let Some(Value::Object(toggles)) = root.get("toggles") {
        for (key, value) in toggles {
            if let Some(enabled) = value.as_bool() {
                config.toggles.insert(key.clone(), enabled);
            }
        }
    }
    if let Some(Value::Object(values)) = root.get("values") {
        for (key, value) in values {
            if let Some(value) = value.as_str() {
                config.values.insert(key.clone(), value.to_owned());
            }
        }
    }
    config
}

fn metadata_json(metadata: LanguageMetadata) -> String {
    json!({
        "id": metadata.id,
        "displayName": metadata.display_name,
        "nativeName": metadata.native_name,
        "defaultMethod": metadata.default_method,
        "methods": metadata.methods.into_iter().map(|method| json!({
            "id": method.id,
            "label": method.label
        })).collect::<Vec<_>>(),
        "options": metadata.options.into_iter().map(|option| json!({
            "id": option.id,
            "label": option.label,
            "defaultEnabled": option.default_enabled
        })).collect::<Vec<_>>()
    })
    .to_string()
}
