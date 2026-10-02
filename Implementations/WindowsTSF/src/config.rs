pub use inputkey_core_abstractions::EngineConfig;
use inputkey_core_abstractions::TypingEnginePort;
use std::sync::Arc;

pub type EngineFactory =
    Arc<dyn Fn(EngineConfig) -> Box<dyn TypingEnginePort> + Send + Sync + 'static>;
