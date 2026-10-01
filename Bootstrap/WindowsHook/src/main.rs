#![cfg_attr(windows, windows_subsystem = "windows")]

use inputkey_boundary::{InputEvent, InputType, Options};
use inputkey_core_abstractions::{LexiconPort, TypingEnginePort};
use inputkey_english_bloom::EnglishBloom;
use inputkey_operators::Session;

struct Adapter(Session);
impl Adapter {
    fn dispatch(&mut self, kind: InputType, key: Option<char>) -> String {
        let out = self.0.handle(InputEvent {
            kind,
            key: key.map_or_else(String::new, |c| c.to_string()),
            timestamp: 0,
        });
        out.rendered
    }
}
impl TypingEnginePort for Adapter {
    fn type_key(&mut self, c: char) -> String {
        self.dispatch(InputType::Key, Some(c))
    }
    fn backspace(&mut self) -> String {
        self.dispatch(InputType::Backspace, None)
    }
    fn escape(&mut self) -> String {
        self.dispatch(InputType::Escape, None)
    }
    fn finalize(&mut self) -> String {
        self.dispatch(InputType::Finalize, None)
    }
    fn literalize_token(&mut self) -> String {
        self.dispatch(InputType::LiteralizeToken, None)
    }
    fn reset(&mut self) {
        self.dispatch(InputType::Reset, None);
    }
    fn rendered(&self) -> String {
        self.0.state().rendered
    }
    fn raw(&self) -> String {
        self.0.state().raw
    }
    fn history_active(&self) -> bool {
        !self.0.state().raw.is_empty()
    }
}
fn main() {
    let options = Options {
        method: "telex".into(),
        simple_telex: false,
        auto_restore: true,
        double_cancel: true,
        cancel_preference: "explicit".into(),
    };
    let factory = Box::new(move || {
        Box::new(Adapter(Session::new(
            options,
            Some(Box::new(EnglishBloom::new()) as Box<dyn LexiconPort>),
            None,
        ))) as Box<dyn TypingEnginePort>
    });
    inputkey_windows_hook::run(inputkey_windows_hook::Config { enabled: true }, factory);
}
