//! Compatibility input capture for Windows targets that cannot use the TSF path.

pub use inputkey_core_abstractions::EngineConfig;
use inputkey_core_abstractions::TypingEnginePort;
use std::sync::Arc;

#[cfg(windows)]
mod native;
mod ring;
#[cfg(windows)]
mod synthetic;
#[cfg(any(windows, test))]
mod transition;
#[cfg(windows)]
mod transport;
#[cfg(windows)]
mod uia;

pub use ring::{Event, EventKind, SpscRing};

#[derive(Clone, Debug)]
pub struct Config {
    pub capture_enabled: bool,
    pub engine: EngineConfig,
}

pub type EngineFactory =
    Arc<dyn Fn(EngineConfig) -> Box<dyn TypingEnginePort> + Send + Sync + 'static>;

#[cfg(windows)]
mod win32;

#[cfg(windows)]
pub use win32::CompatibilityHandle;

#[cfg(windows)]
pub fn start(config: Config, factory: EngineFactory) -> Result<CompatibilityHandle, String> {
    win32::start(config, factory)
}

#[cfg(not(windows))]
pub struct CompatibilityHandle;

#[cfg(not(windows))]
impl CompatibilityHandle {
    pub fn set_capture_enabled(&self, _enabled: bool) {}
    pub fn reload_settings(&self) {}
}

#[cfg(not(windows))]
pub fn start(_config: Config, _factory: EngineFactory) -> Result<CompatibilityHandle, String> {
    Err("Windows compatibility input is only available on Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_config_defaults_match_windows_ui() {
        let config = EngineConfig::default();
        assert_eq!(config.language_id, "vi");
        assert_eq!(config.language.method, "telex");
        assert!(config.language.enabled("auto_restore", true));
        assert!(config.language.enabled("smart_correction", true));
    }

    #[test]
    fn queue_full_is_nonblocking_and_fifo() {
        let queue = SpscRing::<u32, 2>::new();
        assert!(queue.push(10));
        assert!(queue.push(20));
        assert!(!queue.push(30));
        assert_eq!(queue.pop(), Some(10));
        assert_eq!(queue.pop(), Some(20));
        assert_eq!(queue.pop(), None);
    }
}
