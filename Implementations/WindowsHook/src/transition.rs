use inputkey_core_abstractions::{RootInput, TypingEnginePort};

pub fn apply(engine: &mut dyn TypingEnginePort, input: RootInput) -> String {
    engine.dispatch(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputkey_core_abstractions::{CaretMoveCause, CompositionControl, EngineState, RootPhase};

    struct Fake {
        value: String,
    }

    impl TypingEnginePort for Fake {
        fn accepts_character(&self, character: char) -> bool {
            character.is_ascii_alphabetic()
        }

        fn dispatch(&mut self, input: RootInput) -> String {
            match input {
                RootInput::Character(character) => {
                    self.value.push(character);
                    self.value.clone()
                }
                RootInput::CompositionControl(CompositionControl::Backspace) => {
                    self.value.pop();
                    self.value.clone()
                }
                RootInput::CompositionControl(CompositionControl::Escape) => {
                    self.value.clear();
                    String::new()
                }
                RootInput::SpaceBoundary => {
                    let mut value = std::mem::take(&mut self.value);
                    value.push(' ');
                    value
                }
                RootInput::PunctuationBoundary(delimiter) => {
                    let mut value = std::mem::take(&mut self.value);
                    value.push(delimiter);
                    value
                }
                RootInput::CaretMoveBoundary(_)
                | RootInput::ShortcutBoundary
                | RootInput::RawBoundary => std::mem::take(&mut self.value),
                RootInput::Lifecycle(_) => {
                    self.value.clear();
                    String::new()
                }
            }
        }

        fn state(&self) -> EngineState {
            EngineState {
                language_id: "fake".into(),
                phase: if self.value.is_empty() {
                    RootPhase::Idle
                } else {
                    RootPhase::Composing
                },
                raw: self.value.clone(),
                rendered: self.value.clone(),
                child_mode: "fake".into(),
                child_phase: "fake".into(),
            }
        }
    }

    #[test]
    fn caret_move_ends_the_current_engine_session() {
        let mut engine = Fake {
            value: String::new(),
        };
        apply(&mut engine, RootInput::Character('g'));
        apply(&mut engine, RootInput::Character('o'));
        assert_eq!(
            apply(
                &mut engine,
                RootInput::CaretMoveBoundary(CaretMoveCause::Left)
            ),
            "go"
        );
        assert!(!engine.history_active());
    }

    #[test]
    fn raw_boundary_ends_the_current_engine_session() {
        let mut engine = Fake {
            value: String::new(),
        };
        apply(&mut engine, RootInput::Character('a'));
        apply(&mut engine, RootInput::Character('s'));
        assert_eq!(apply(&mut engine, RootInput::RawBoundary), "as");
        assert!(!engine.history_active());
    }
}
