use inputkey_core_abstractions::{
    CaretMoveCause, CompositionControl, LanguageConfig, LanguageMachinePort, LanguageMetadata,
    LanguagePackPort, LifecycleEvent, RootInput, RootPhase, RootTransition,
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

struct SessionEntry {
    machine: Box<dyn LanguageMachinePort>,
    last_result: String,
}

pub struct PackHost {
    pack: Arc<dyn LanguagePackPort>,
    next: AtomicU64,
    sessions: Mutex<HashMap<u64, SessionEntry>>,
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
            sessions.insert(
                handle,
                SessionEntry {
                    machine,
                    last_result: String::new(),
                },
            );
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

    pub fn accepts_character(&self, handle: u64, character: char) -> bool {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| {
                sessions
                    .get(&handle)
                    .map(|entry| entry.machine.accepts_character(character))
            })
            .unwrap_or(false)
    }

    pub fn transition_json(&self, handle: u64, text: &str) -> bool {
        let Some(transition) = parse_transition(text) else {
            return false;
        };
        let Ok(mut sessions) = self.sessions.lock() else {
            return false;
        };
        let Some(entry) = sessions.get_mut(&handle) else {
            return false;
        };
        entry.last_result = entry.machine.on_transition(transition).text;
        true
    }

    pub fn result(&self, handle: u64) -> String {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(&handle).map(|entry| entry.last_result.clone()))
            .unwrap_or_default()
    }

    pub fn state_json(&self, handle: u64) -> String {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| {
                sessions.get(&handle).map(|entry| {
                    let state = entry.machine.state();
                    json!({
                        "raw": state.raw,
                        "rendered": state.rendered,
                        "mode": state.mode,
                        "phase": state.phase
                    })
                    .to_string()
                })
            })
            .unwrap_or_default()
    }
}

fn parse_phase(value: &str) -> Option<RootPhase> {
    Some(match value {
        "idle" => RootPhase::Idle,
        "composing" => RootPhase::Composing,
        "space_boundary" => RootPhase::SpaceBoundary,
        "punctuation_boundary" => RootPhase::PunctuationBoundary,
        "caret_move_boundary" => RootPhase::CaretMoveBoundary,
        "shortcut_boundary" => RootPhase::ShortcutBoundary,
        "composition_control" => RootPhase::CompositionControl,
        "raw_boundary" => RootPhase::RawBoundary,
        "lifecycle" => RootPhase::Lifecycle,
        _ => return None,
    })
}

fn parse_caret_cause(value: &str) -> CaretMoveCause {
    match value {
        "mouse" => CaretMoveCause::Mouse,
        "enter" => CaretMoveCause::Enter,
        "tab" => CaretMoveCause::Tab,
        "left" => CaretMoveCause::Left,
        "right" => CaretMoveCause::Right,
        "up" => CaretMoveCause::Up,
        "down" => CaretMoveCause::Down,
        "home" => CaretMoveCause::Home,
        "end" => CaretMoveCause::End,
        "page_up" => CaretMoveCause::PageUp,
        "page_down" => CaretMoveCause::PageDown,
        "delete" => CaretMoveCause::Delete,
        "insert" => CaretMoveCause::Insert,
        _ => CaretMoveCause::Other,
    }
}

fn parse_input(kind: &str, value: &str) -> Option<RootInput> {
    Some(match kind {
        "character" => RootInput::Character(value.chars().next()?),
        "space_boundary" => RootInput::SpaceBoundary,
        "punctuation_boundary" => RootInput::PunctuationBoundary(value.chars().next()?),
        "caret_move_boundary" => RootInput::CaretMoveBoundary(parse_caret_cause(value)),
        "shortcut_boundary" => RootInput::ShortcutBoundary,
        "composition_control" => RootInput::CompositionControl(match value {
            "backspace" => CompositionControl::Backspace,
            "escape" => CompositionControl::Escape,
            _ => return None,
        }),
        "raw_boundary" => RootInput::RawBoundary,
        "lifecycle" => RootInput::Lifecycle(match value {
            "reset" => LifecycleEvent::Reset,
            "focus_lost" => LifecycleEvent::FocusLost,
            "context_destroyed" => LifecycleEvent::ContextDestroyed,
            "disabled" => LifecycleEvent::Disabled,
            "language_changed" => LifecycleEvent::LanguageChanged,
            _ => LifecycleEvent::Finalize,
        }),
        _ => return None,
    })
}

fn parse_transition(text: &str) -> Option<RootTransition> {
    let value: Value = serde_json::from_str(text).ok()?;
    let from = parse_phase(value.get("from")?.as_str()?)?;
    let to = parse_phase(value.get("to")?.as_str()?)?;
    let kind = value.get("kind")?.as_str()?;
    let detail = value
        .get("value")
        .and_then(Value::as_str)
        .unwrap_or_default();
    Some(RootTransition {
        from,
        to,
        input: parse_input(kind, detail)?,
    })
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
