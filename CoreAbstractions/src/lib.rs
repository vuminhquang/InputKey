//! Semantic declarations and ports. No executable policy lives here.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    None,
    Acute,
    Grave,
    Hook,
    Tilde,
    Dot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Start,
    ViCandidate,
    RawLocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Start,
    Onset,
    Nucleus,
    Coda,
    Complete,
    PendingShape,
    Dead,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineState {
    pub mode: Mode,
    pub phase: Phase,
    pub raw: String,
    pub fallback: String,
    pub rendered: String,
    pub tone: Tone,
    pub ambiguous: bool,
    pub transformed: bool,
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

pub trait LexiconPort: Send + Sync {
    fn has_word(&self, word: &str) -> bool;
}

pub trait EventPublisher: Send + Sync {
    fn publish(&self, event: CoreEvent);
}

pub trait EventSubscriber: Send + Sync {
    fn subscribe(&self, handler: Box<dyn Fn(CoreEvent) + Send + Sync>) -> Box<dyn FnOnce()>;
}

/// Port consumed by platform transports. Concrete typing policy is implemented by Operators.
pub trait TypingEnginePort: Send {
    fn type_key(&mut self, key: char) -> String;
    fn backspace(&mut self) -> String;
    fn escape(&mut self) -> String;
    fn finalize(&mut self) -> String;
    fn literalize_token(&mut self) -> String;
    fn reset(&mut self);
    fn rendered(&self) -> String;
    fn raw(&self) -> String;
    fn history_active(&self) -> bool;
}
