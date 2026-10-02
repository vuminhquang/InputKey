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
        EventKind::RawBoundary => Some(engine.commit_raw_boundary()),
        EventKind::FinalizeWithDelimiter => {
            key.map(|delimiter| engine.decision_boundary(delimiter))
        }
        EventKind::NaturalBoundary => Some(engine.natural_boundary()),
        EventKind::MouseBoundary => Some(engine.mouse_boundary()),
        EventKind::ResetOnly => {
            engine.reset();
            None
        }
        EventKind::Pass => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        value: String,
    }

    impl TypingEnginePort for Fake {
        fn accepts_key(&self, key: char) -> bool {
            key.is_ascii_alphabetic()
        }
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
        fn decision_boundary(&mut self, delimiter: char) -> String {
            let mut value = std::mem::take(&mut self.value);
            value.push(delimiter);
            value
        }
        fn commit_boundary(&mut self) -> String {
            std::mem::take(&mut self.value)
        }
        fn natural_boundary(&mut self) -> String {
            std::mem::take(&mut self.value)
        }
        fn mouse_boundary(&mut self) -> String {
            std::mem::take(&mut self.value)
        }
        fn commit_raw_boundary(&mut self) -> String {
            std::mem::take(&mut self.value)
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
    fn finalize_starts_the_next_token_cleanly() {
        let mut engine = Fake {
            value: String::new(),
        };
        apply(&mut engine, EventKind::TypeChar, Some('g'));
        apply(&mut engine, EventKind::TypeChar, Some('o'));
        assert_eq!(
            apply(&mut engine, EventKind::NaturalBoundary, None),
            Some("go".into())
        );
        assert!(!engine.history_active());
        assert_eq!(
            apply(&mut engine, EventKind::TypeChar, Some('n')),
            Some("n".into())
        );
    }

    #[test]
    fn raw_boundary_ends_the_current_engine_session() {
        let mut engine = Fake {
            value: String::new(),
        };
        apply(&mut engine, EventKind::TypeChar, Some('a'));
        apply(&mut engine, EventKind::TypeChar, Some('s'));
        assert_eq!(
            apply(&mut engine, EventKind::RawBoundary, None),
            Some("as".into())
        );
        assert!(!engine.history_active());
    }
}
