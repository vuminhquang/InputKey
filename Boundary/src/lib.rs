//! Boundary protocol declarations and DTOs. No executable behavior lives here.

pub const PROTOCOL_VERSION: u32 = 9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputType {
    Character,
    SpaceBoundary,
    PunctuationBoundary,
    CaretMoveBoundary,
    ShortcutBoundary,
    CompositionControl,
    RawBoundary,
    Lifecycle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputEvent {
    pub kind: InputType,
    pub key: String,
    pub timestamp: i64,
}

/// Compatibility DTO for the legacy Vietnamese constructors. New language packs use
/// `LanguageConfig` from CoreAbstractions through the root composition contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub method: String,
    pub simple_telex: bool,
    pub auto_restore: bool,
    pub smart_correction: bool,
    pub double_cancel: bool,
    pub cancel_preference: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputDto {
    pub rendered: String,
    pub raw: String,
    pub commit: String,
    pub mode: String,
    pub phase: String,
    pub changed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundaryEvent {
    pub name: String,
    pub input: InputEvent,
    pub output: OutputDto,
    pub sequence: u64,
}
