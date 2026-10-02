//! Stable semantic declarations and ports shared by InputKey runtimes.

use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageMethodMetadata {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageOptionMetadata {
    pub id: String,
    pub label: String,
    pub default_enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageMetadata {
    pub id: String,
    pub display_name: String,
    pub native_name: String,
    pub default_method: String,
    pub methods: Vec<LanguageMethodMetadata>,
    pub options: Vec<LanguageOptionMetadata>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageConfig {
    pub method: String,
    pub toggles: BTreeMap<String, bool>,
    pub values: BTreeMap<String, String>,
}

impl LanguageConfig {
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            toggles: BTreeMap::new(),
            values: BTreeMap::new(),
        }
    }

    pub fn toggle(mut self, id: impl Into<String>, enabled: bool) -> Self {
        self.toggles.insert(id.into(), enabled);
        self
    }

    pub fn value(mut self, id: impl Into<String>, value: impl Into<String>) -> Self {
        self.values.insert(id.into(), value.into());
        self
    }

    pub fn enabled(&self, id: &str, fallback: bool) -> bool {
        self.toggles.get(id).copied().unwrap_or(fallback)
    }

    pub fn value_or<'a>(&'a self, id: &str, fallback: &'a str) -> &'a str {
        self.values.get(id).map(String::as_str).unwrap_or(fallback)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineConfig {
    pub language_id: String,
    pub language: LanguageConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            language_id: "vi".into(),
            language: LanguageConfig::new("telex")
                .toggle("simple_telex", false)
                .toggle("auto_restore", true)
                .toggle("smart_correction", true)
                .value("cancel_preference", "explicit"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageState {
    pub raw: String,
    pub rendered: String,
    pub mode: String,
    pub phase: String,
}

pub trait LexiconPort: Send + Sync {
    fn has_word(&self, word: &str) -> bool;
}

pub trait LanguageMachinePort: Send {
    fn accepts_key(&self, key: char) -> bool;
    fn type_key(&mut self, key: char) -> String;
    fn backspace(&mut self) -> String;
    fn escape(&mut self) -> String;
    fn finalize(&mut self) -> String;

    /// Gives the active language one correction opportunity when the root
    /// enters its Space decision boundary. Languages without correction rules
    /// keep normal finalize behavior.
    fn correct_boundary(&mut self) -> String {
        self.finalize()
    }

    fn reset(&mut self);
    fn state(&self) -> LanguageState;

    fn rendered(&self) -> String {
        self.state().rendered
    }

    fn raw(&self) -> String {
        self.state().raw
    }

    fn history_active(&self) -> bool {
        !self.state().raw.is_empty()
    }
}

pub trait LanguagePackPort: Send + Sync {
    fn metadata(&self) -> LanguageMetadata;
    fn create(&self, config: LanguageConfig) -> Result<Box<dyn LanguageMachinePort>, String>;
}

pub trait LanguageCatalogPort: Send + Sync {
    fn languages(&self) -> Vec<LanguageMetadata>;
    fn create(
        &self,
        language_id: &str,
        config: LanguageConfig,
    ) -> Result<Box<dyn LanguageMachinePort>, String>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootPhase {
    Idle,
    Composing,
    CorrectionBoundary,
    RawBoundary,
    NaturalBoundary,
    MouseBoundary,
    FinalizeBoundary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineState {
    pub language_id: String,
    pub phase: RootPhase,
    pub raw: String,
    pub rendered: String,
    pub child_mode: String,
    pub child_phase: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreEventType {
    PreeditChanged,
    TextCommitted,
    StateChanged,
    Reset,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreEvent {
    pub kind: CoreEventType,
    pub sequence: u64,
    pub text: String,
    pub state: EngineState,
}

pub trait EventPublisher: Send + Sync {
    fn publish(&self, event: CoreEvent);
}

pub trait EventSubscriber: Send + Sync {
    fn subscribe(&self, handler: Box<dyn Fn(CoreEvent) + Send + Sync>) -> Box<dyn FnOnce()>;
}

/// Root typing contract consumed by platform transports.
///
/// Language-specific interpretation is delegated to a child machine. Composition
/// boundaries remain root semantics so every language observes the same Space,
/// Shift+Space, navigation, and reset behavior.
pub trait TypingEnginePort: Send {
    fn accepts_key(&self, key: char) -> bool;
    fn type_key(&mut self, key: char) -> String;
    fn backspace(&mut self) -> String;
    fn escape(&mut self) -> String;
    fn finalize(&mut self) -> String;
    fn decision_boundary(&mut self, delimiter: char) -> String;
    fn commit_boundary(&mut self) -> String;
    fn natural_boundary(&mut self) -> String;
    fn mouse_boundary(&mut self) -> String;
    fn commit_raw_boundary(&mut self) -> String;
    fn reset(&mut self);
    fn rendered(&self) -> String;
    fn raw(&self) -> String;
    fn history_active(&self) -> bool;
}
