//! Native Windows transport boundary. Engine policy remains in the host.
use inputkey_core_abstractions::TypingEnginePort;

mod ring;
#[cfg(any(windows, test))]
mod settings;
mod shortcut;
#[cfg(any(windows, test))]
mod startup;
#[cfg(any(windows, test))]
mod transition;

pub use ring::{Event, EventKind, SpscRing};
pub use shortcut::{Modifiers, Shortcut, ShortcutRecorder};

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub enabled: bool,
}

pub type EngineFactory = Box<dyn FnOnce() -> Box<dyn TypingEnginePort> + Send>;

#[cfg(windows)]
mod win32;

#[cfg(windows)]
pub fn run(config: Config, factory: EngineFactory) {
    win32::run(config, factory);
}

#[cfg(not(windows))]
pub fn run(_config: Config, _factory: EngineFactory) {
    eprintln!("InputKey WindowsHook can only run on Windows");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_full_is_nonblocking_and_fifo() {
        let q = SpscRing::<u32, 2>::new();
        assert!(q.push(10));
        assert!(q.push(20));
        assert!(!q.push(30));
        assert_eq!(q.pop(), Some(10));
        assert_eq!(q.pop(), Some(20));
        assert_eq!(q.pop(), None);
    }
}
