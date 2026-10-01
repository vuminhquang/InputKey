use crate::EventKind;
use inputkey_core_abstractions::TypingEnginePort;

pub fn apply(
    engine: &mut dyn TypingEnginePort,
    kind: EventKind,
    key: Option<char>,
) -> Option<String> {
    match kind {
        EventKind::TypeChar => key.map(|c| engine.type_key(c)),
        EventKind::Backspace => Some(engine.backspace()),
        EventKind::Escape => Some(engine.escape()),
        EventKind::Literalize => Some(engine.literalize_token()),
        EventKind::FinalizeAndReplay => Some(engine.finalize()),
        EventKind::ResetOnly | EventKind::ResetAndReplayKeepDisplayed => {
            engine.reset();
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        value: String,
        calls: usize,
    }
    impl TypingEnginePort for Fake {
        fn type_key(&mut self, key: char) -> String {
            self.value.push(key);
            self.value.clone()
        }
        fn backspace(&mut self) -> String {
            self.value.pop();
            self.value.clone()
        }
        fn escape(&mut self) -> String {
            self.value.clear();
            self.value.clone()
        }
        fn finalize(&mut self) -> String {
            self.value.clone()
        }
        fn literalize_token(&mut self) -> String {
            self.calls += 1;
            self.value = "literal".into();
            self.value.clone()
        }
        fn reset(&mut self) {
            self.value.clear();
        }
        fn rendered(&self) -> String {
            self.value.clone()
        }
        fn raw(&self) -> String {
            self.value.clone()
        }
        fn history_active(&self) -> bool {
            !self.value.is_empty()
        }
    }
    #[test]
    fn literalize_uses_same_transformed_engine() {
        let mut creations = 0;
        let mut factory = || {
            creations += 1;
            Fake {
                value: String::new(),
                calls: 0,
            }
        };
        let mut engine = factory();
        apply(&mut engine, EventKind::TypeChar, Some('a'));
        apply(&mut engine, EventKind::TypeChar, Some('s'));
        assert_eq!(
            apply(&mut engine, EventKind::Literalize, None),
            Some("literal".into())
        );
        assert_eq!(engine.calls, 1);
        assert_eq!(creations, 1);
    }
}
