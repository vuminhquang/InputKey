use inputkey_core_abstractions::{
    CaretMoveCause, CompositionControl, LanguageConfig, LanguageMachinePort, LanguageMetadata,
    LanguageMethodMetadata, LanguageOptionMetadata, LanguagePackPort, LanguageState,
    LanguageTransitionResult, LifecycleEvent, RootInput, RootPhase, RootTransition,
};
use inputkey_language_pack_native_abi::{
    LanguagePackApiV3, LANGUAGE_PACK_ABI_VERSION, LANGUAGE_PACK_ENTRYPOINT,
};
use libloading::{Library, Symbol};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

type Entry = unsafe extern "C" fn() -> *const LanguagePackApiV3;

fn read_call(call: impl Fn(*mut u8, usize) -> usize) -> String {
    let n = call(std::ptr::null_mut(), 0);
    if n == 0 {
        return String::new();
    }
    let mut buffer = vec![0u8; n + 1];
    let actual = call(buffer.as_mut_ptr(), buffer.len()).min(n);
    String::from_utf8_lossy(&buffer[..actual]).into_owned()
}

fn metadata_from_json(text: &str) -> Result<LanguageMetadata, String> {
    let value: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let s = |name: &str| {
        value
            .get(name)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| format!("metadata missing {name}"))
    };
    let methods = value
        .get("methods")
        .and_then(Value::as_array)
        .ok_or("metadata missing methods")?
        .iter()
        .map(|m| {
            Ok(LanguageMethodMetadata {
                id: m
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or("method id")?
                    .to_owned(),
                label: m
                    .get("label")
                    .and_then(Value::as_str)
                    .ok_or("method label")?
                    .to_owned(),
            })
        })
        .collect::<Result<Vec<_>, &str>>()
        .map_err(str::to_owned)?;
    let options = value
        .get("options")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|o| {
                    Ok(LanguageOptionMetadata {
                        id: o
                            .get("id")
                            .and_then(Value::as_str)
                            .ok_or("option id")?
                            .to_owned(),
                        label: o
                            .get("label")
                            .and_then(Value::as_str)
                            .ok_or("option label")?
                            .to_owned(),
                        default_enabled: o
                            .get("defaultEnabled")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    })
                })
                .collect::<Result<Vec<_>, &str>>()
        })
        .transpose()
        .map_err(str::to_owned)?
        .unwrap_or_default();
    Ok(LanguageMetadata {
        id: s("id")?,
        display_name: s("displayName")?,
        native_name: s("nativeName")?,
        default_method: s("defaultMethod")?,
        methods,
        options,
    })
}

fn config_json(config: &LanguageConfig) -> String {
    json!({"method":config.method,"toggles":config.toggles,"values":config.values}).to_string()
}

fn phase_id(phase: RootPhase) -> &'static str {
    match phase {
        RootPhase::Idle => "idle",
        RootPhase::Composing => "composing",
        RootPhase::SpaceBoundary => "space_boundary",
        RootPhase::PunctuationBoundary => "punctuation_boundary",
        RootPhase::CaretMoveBoundary => "caret_move_boundary",
        RootPhase::ShortcutBoundary => "shortcut_boundary",
        RootPhase::CompositionControl => "composition_control",
        RootPhase::RawBoundary => "raw_boundary",
        RootPhase::Lifecycle => "lifecycle",
    }
}

fn caret_id(cause: CaretMoveCause) -> &'static str {
    match cause {
        CaretMoveCause::Mouse => "mouse",
        CaretMoveCause::Enter => "enter",
        CaretMoveCause::Tab => "tab",
        CaretMoveCause::Left => "left",
        CaretMoveCause::Right => "right",
        CaretMoveCause::Up => "up",
        CaretMoveCause::Down => "down",
        CaretMoveCause::Home => "home",
        CaretMoveCause::End => "end",
        CaretMoveCause::PageUp => "page_up",
        CaretMoveCause::PageDown => "page_down",
        CaretMoveCause::Delete => "delete",
        CaretMoveCause::Insert => "insert",
        CaretMoveCause::Other => "other",
    }
}

fn transition_json(transition: RootTransition) -> String {
    let (kind, value) = match transition.input {
        RootInput::Character(character) => ("character", character.to_string()),
        RootInput::SpaceBoundary => ("space_boundary", String::new()),
        RootInput::PunctuationBoundary(delimiter) => {
            ("punctuation_boundary", delimiter.to_string())
        }
        RootInput::CaretMoveBoundary(cause) => ("caret_move_boundary", caret_id(cause).to_owned()),
        RootInput::ShortcutBoundary => ("shortcut_boundary", String::new()),
        RootInput::CompositionControl(CompositionControl::Backspace) => {
            ("composition_control", "backspace".into())
        }
        RootInput::CompositionControl(CompositionControl::Escape) => {
            ("composition_control", "escape".into())
        }
        RootInput::RawBoundary => ("raw_boundary", String::new()),
        RootInput::Lifecycle(event) => (
            "lifecycle",
            match event {
                LifecycleEvent::Finalize => "finalize",
                LifecycleEvent::Reset => "reset",
                LifecycleEvent::FocusLost => "focus_lost",
                LifecycleEvent::ContextDestroyed => "context_destroyed",
                LifecycleEvent::Disabled => "disabled",
                LifecycleEvent::LanguageChanged => "language_changed",
            }
            .into(),
        ),
    };
    json!({"from":phase_id(transition.from),"to":phase_id(transition.to),"kind":kind,"value":value})
        .to_string()
}

pub struct DynamicPack {
    library: Arc<Library>,
    api: LanguagePackApiV3,
    metadata: LanguageMetadata,
}

impl DynamicPack {
    /// Loads executable code from a native language-pack library.
    ///
    /// # Safety
    /// The caller must trust the library at `path` to execute in the current process.
    pub unsafe fn load(path: &Path) -> Result<Self, String> {
        let library = Arc::new(unsafe { Library::new(path) }.map_err(|e| e.to_string())?);
        let mut symbol = LANGUAGE_PACK_ENTRYPOINT.as_bytes().to_vec();
        symbol.push(0);
        let entry: Symbol<Entry> = unsafe { library.get(&symbol) }.map_err(|e| e.to_string())?;
        let ptr = unsafe { entry() };
        if ptr.is_null() {
            return Err("language pack returned null API".into());
        }
        let api = unsafe { *ptr };
        if api.abi_version != LANGUAGE_PACK_ABI_VERSION {
            return Err(format!(
                "language-pack ABI {} != {}",
                api.abi_version, LANGUAGE_PACK_ABI_VERSION
            ));
        }
        if api.struct_size < std::mem::size_of::<LanguagePackApiV3>() {
            return Err("language-pack API table is too small".into());
        }
        let metadata = metadata_from_json(&read_call(|out, cap| unsafe {
            (api.metadata_json)(out, cap)
        }))?;
        Ok(Self {
            library,
            api,
            metadata,
        })
    }
}

impl LanguagePackPort for DynamicPack {
    fn metadata(&self) -> LanguageMetadata {
        self.metadata.clone()
    }

    fn create(&self, config: LanguageConfig) -> Result<Box<dyn LanguageMachinePort>, String> {
        let json = config_json(&config);
        let handle = unsafe { (self.api.create)(json.as_ptr(), json.len()) };
        if handle == 0 {
            return Err("language pack create failed".into());
        }
        Ok(Box::new(DynamicMachine {
            library: Arc::clone(&self.library),
            api: self.api,
            handle,
        }))
    }
}

struct DynamicMachine {
    #[allow(dead_code)]
    library: Arc<Library>,
    api: LanguagePackApiV3,
    handle: u64,
}

impl DynamicMachine {
    fn query(&self, f: unsafe extern "C" fn(u64, *mut u8, usize) -> usize) -> String {
        read_call(|out, cap| unsafe { f(self.handle, out, cap) })
    }
}

impl Drop for DynamicMachine {
    fn drop(&mut self) {
        unsafe { (self.api.destroy)(self.handle) }
    }
}

impl LanguageMachinePort for DynamicMachine {
    fn accepts_character(&self, character: char) -> bool {
        let mut buffer = [0u8; 4];
        let text = character.encode_utf8(&mut buffer);
        unsafe { (self.api.accepts_character)(self.handle, text.as_ptr(), text.len()) != 0 }
    }

    fn on_transition(&mut self, transition: RootTransition) -> LanguageTransitionResult {
        let json = transition_json(transition);
        let ok = unsafe { (self.api.transition_json)(self.handle, json.as_ptr(), json.len()) != 0 };
        LanguageTransitionResult {
            text: if ok {
                self.query(self.api.result)
            } else {
                String::new()
            },
        }
    }

    fn state(&self) -> LanguageState {
        let text = self.query(self.api.state_json);
        let value: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        let get = |name: &str| {
            value
                .get(name)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        LanguageState {
            raw: get("raw"),
            rendered: get("rendered"),
            mode: get("mode"),
            phase: get("phase"),
        }
    }
}

fn candidate(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if cfg!(target_os = "windows") {
        name.starts_with("InputKeyLanguage") && name.ends_with(".dll")
    } else if cfg!(target_os = "macos") {
        name.starts_with("libInputKeyLanguage") && name.ends_with(".dylib")
    } else {
        name.starts_with("libInputKeyLanguage") && name.ends_with(".so")
    }
}

#[cfg(windows)]
fn module_dir() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::LibraryLoader::{
        GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
        GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    };

    let mut module = std::ptr::null_mut();
    let address = module_dir as *const () as *const u16;
    let flags =
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
    if unsafe { GetModuleHandleExW(flags, address, &mut module) } == 0 || module.is_null() {
        return None;
    }
    let mut buffer = vec![0u16; 32768];
    let len = unsafe { GetModuleFileNameW(module, buffer.as_mut_ptr(), buffer.len() as u32) };
    if len == 0 {
        return None;
    }
    let path = PathBuf::from(std::ffi::OsString::from_wide(&buffer[..len as usize]));
    path.parent().map(Path::to_path_buf)
}

#[cfg(unix)]
fn module_dir() -> Option<PathBuf> {
    use std::ffi::{c_void, CStr};
    let mut info: libc::Dl_info = unsafe { std::mem::zeroed() };
    let address = module_dir as *const () as *const c_void;
    if unsafe { libc::dladdr(address, &mut info) } == 0 || info.dli_fname.is_null() {
        return None;
    }
    let path = PathBuf::from(
        unsafe { CStr::from_ptr(info.dli_fname) }
            .to_string_lossy()
            .into_owned(),
    );
    path.parent().map(Path::to_path_buf)
}

pub fn default_search_dirs() -> Vec<PathBuf> {
    if let Some(raw) = std::env::var_os("INPUTKEY_LANGUAGE_PACK_DIR") {
        let mut dirs: Vec<PathBuf> = std::env::split_paths(&raw).collect();
        dirs.sort();
        dirs.dedup();
        return dirs;
    }

    let mut dirs = Vec::new();
    if let Some(module) = module_dir() {
        dirs.push(module.join("languages"));
        dirs.push(module.join("LanguagePacks"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.join("languages"));
            dirs.push(parent.join("LanguagePacks"));
        }
    }
    dirs.sort();
    dirs.dedup();
    dirs
}

pub fn discover() -> Vec<Arc<dyn LanguagePackPort>> {
    let mut out: Vec<Arc<dyn LanguagePackPort>> = Vec::new();
    for dir in default_search_dirs() {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() || !candidate(&path) {
                continue;
            }
            if let Ok(pack) = unsafe { DynamicPack::load(&path) } {
                out.push(Arc::new(pack));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_json_preserves_root_state_and_cause() {
        let transition = RootTransition {
            from: RootPhase::Composing,
            to: RootPhase::CaretMoveBoundary,
            input: RootInput::CaretMoveBoundary(CaretMoveCause::Left),
        };
        let value: Value = serde_json::from_str(&transition_json(transition)).unwrap();
        assert_eq!(value["from"], "composing");
        assert_eq!(value["to"], "caret_move_boundary");
        assert_eq!(value["kind"], "caret_move_boundary");
        assert_eq!(value["value"], "left");
    }
}
