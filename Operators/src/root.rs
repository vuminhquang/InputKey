use inputkey_core_abstractions::{
    EngineState, LanguageMachinePort, RootInput, RootPhase, RootTransition, TypingEnginePort,
};

/// The single root state machine.
///
/// It classifies semantic input, enters the corresponding root state, then
/// synchronously emits that transition to the active language machine.
pub struct Machine {
    language_id: String,
    child: Box<dyn LanguageMachinePort>,
    phase: RootPhase,
}

impl Machine {
    pub fn new(language_id: impl Into<String>, child: Box<dyn LanguageMachinePort>) -> Self {
        Self {
            language_id: language_id.into(),
            child,
            phase: RootPhase::Idle,
        }
    }

    pub fn language_id(&self) -> &str {
        &self.language_id
    }

    pub fn state(&self) -> EngineState {
        let child = self.child.state();
        EngineState {
            language_id: self.language_id.clone(),
            phase: self.phase,
            raw: child.raw,
            rendered: child.rendered,
            child_mode: child.mode,
            child_phase: child.phase,
        }
    }

    pub fn accepts_character(&self, character: char) -> bool {
        self.child.accepts_character(character)
    }

    fn target_phase(input: RootInput) -> RootPhase {
        match input {
            RootInput::Character(_) => RootPhase::Composing,
            RootInput::SpaceBoundary => RootPhase::SpaceBoundary,
            RootInput::PunctuationBoundary(_) => RootPhase::PunctuationBoundary,
            RootInput::CaretMoveBoundary(_) => RootPhase::CaretMoveBoundary,
            RootInput::ShortcutBoundary => RootPhase::ShortcutBoundary,
            RootInput::CompositionControl(_) => RootPhase::CompositionControl,
            RootInput::RawBoundary => RootPhase::RawBoundary,
            RootInput::Lifecycle(_) => RootPhase::Lifecycle,
        }
    }

    fn sync_composition_phase(&mut self) {
        self.phase = if self.child.state().raw.is_empty() {
            RootPhase::Idle
        } else {
            RootPhase::Composing
        };
    }

    pub fn dispatch(&mut self, input: RootInput) -> String {
        let from = self.phase;
        let to = Self::target_phase(input);

        // Enter the root state before the language machine sees the event.
        self.phase = to;
        let transition = RootTransition { from, to, input };
        let mut text = self.child.on_transition(transition).text;

        match input {
            RootInput::Character(_) | RootInput::CompositionControl(_) => {
                self.sync_composition_phase();
            }
            RootInput::SpaceBoundary => {
                text.push(' ');
                self.phase = RootPhase::Idle;
            }
            RootInput::PunctuationBoundary(delimiter) => {
                text.push(delimiter);
                self.phase = RootPhase::Idle;
            }
            RootInput::CaretMoveBoundary(_)
            | RootInput::ShortcutBoundary
            | RootInput::RawBoundary
            | RootInput::Lifecycle(_) => {
                self.phase = RootPhase::Idle;
            }
        }

        text
    }
}

impl TypingEnginePort for Machine {
    fn accepts_character(&self, character: char) -> bool {
        Machine::accepts_character(self, character)
    }

    fn dispatch(&mut self, input: RootInput) -> String {
        Machine::dispatch(self, input)
    }

    fn state(&self) -> EngineState {
        Machine::state(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputkey_core_abstractions::{
        CaretMoveCause, CompositionControl, LanguageState, LanguageTransitionResult,
        LifecycleEvent, RootTransition,
    };
    use std::sync::{Arc, Mutex};

    struct ProbeLanguage {
        raw: String,
        transitions: Arc<Mutex<Vec<RootTransition>>>,
    }

    impl ProbeLanguage {
        fn new(transitions: Arc<Mutex<Vec<RootTransition>>>) -> Self {
            Self {
                raw: String::new(),
                transitions,
            }
        }
    }

    impl LanguageMachinePort for ProbeLanguage {
        fn accepts_character(&self, character: char) -> bool {
            character.is_ascii_alphabetic()
        }

        fn on_transition(&mut self, transition: RootTransition) -> LanguageTransitionResult {
            self.transitions
                .lock()
                .expect("transition log")
                .push(transition);

            let text = match transition.input {
                RootInput::Character(character) => {
                    self.raw.push(character);
                    self.raw.clone()
                }
                RootInput::CompositionControl(CompositionControl::Backspace) => {
                    self.raw.pop();
                    self.raw.clone()
                }
                RootInput::CompositionControl(CompositionControl::Escape) => self.raw.clone(),
                RootInput::SpaceBoundary | RootInput::PunctuationBoundary(_) => {
                    let text = format!("boundary:{}", self.raw);
                    self.raw.clear();
                    text
                }
                RootInput::CaretMoveBoundary(_) | RootInput::ShortcutBoundary => {
                    let text = self.raw.clone();
                    self.raw.clear();
                    text
                }
                RootInput::RawBoundary => {
                    let text = format!("raw:{}", self.raw);
                    self.raw.clear();
                    text
                }
                RootInput::Lifecycle(LifecycleEvent::Reset) => {
                    self.raw.clear();
                    String::new()
                }
                RootInput::Lifecycle(_) => {
                    let text = format!("final:{}", self.raw);
                    self.raw.clear();
                    text
                }
            };
            LanguageTransitionResult { text }
        }

        fn state(&self) -> LanguageState {
            LanguageState {
                raw: self.raw.clone(),
                rendered: self.raw.clone(),
                mode: "probe".into(),
                phase: "probe".into(),
            }
        }
    }

    fn root(transitions: Arc<Mutex<Vec<RootTransition>>>) -> Machine {
        Machine::new("probe", Box::new(ProbeLanguage::new(transitions)))
    }

    #[test]
    fn root_enters_state_before_emitting_transition() {
        let transitions = Arc::new(Mutex::new(Vec::new()));
        let mut root = root(Arc::clone(&transitions));

        assert_eq!(root.dispatch(RootInput::Character('a')), "a");
        assert_eq!(root.state().phase, RootPhase::Composing);

        assert_eq!(root.dispatch(RootInput::SpaceBoundary), "boundary:a ");
        assert_eq!(root.state().phase, RootPhase::Idle);

        let log = transitions.lock().expect("transition log");
        assert_eq!(log[1].from, RootPhase::Composing);
        assert_eq!(log[1].to, RootPhase::SpaceBoundary);
        assert_eq!(log[1].input, RootInput::SpaceBoundary);
    }

    #[test]
    fn space_and_punctuation_are_distinct_states_with_shared_child_policy() {
        let transitions = Arc::new(Mutex::new(Vec::new()));
        let mut root = root(Arc::clone(&transitions));

        root.dispatch(RootInput::Character('a'));
        assert_eq!(root.dispatch(RootInput::SpaceBoundary), "boundary:a ");

        root.dispatch(RootInput::Character('b'));
        assert_eq!(
            root.dispatch(RootInput::PunctuationBoundary('.')),
            "boundary:b."
        );

        let log = transitions.lock().expect("transition log");
        assert_eq!(log[1].to, RootPhase::SpaceBoundary);
        assert_eq!(log[3].to, RootPhase::PunctuationBoundary);
    }

    #[test]
    fn mouse_and_navigation_share_caret_move_state_but_keep_cause() {
        let transitions = Arc::new(Mutex::new(Vec::new()));
        let mut root = root(Arc::clone(&transitions));

        root.dispatch(RootInput::Character('a'));
        assert_eq!(
            root.dispatch(RootInput::CaretMoveBoundary(CaretMoveCause::Mouse)),
            "a"
        );

        root.dispatch(RootInput::Character('b'));
        assert_eq!(
            root.dispatch(RootInput::CaretMoveBoundary(CaretMoveCause::Left)),
            "b"
        );

        let log = transitions.lock().expect("transition log");
        assert_eq!(log[1].to, RootPhase::CaretMoveBoundary);
        assert_eq!(
            log[1].input,
            RootInput::CaretMoveBoundary(CaretMoveCause::Mouse)
        );
        assert_eq!(log[3].to, RootPhase::CaretMoveBoundary);
        assert_eq!(
            log[3].input,
            RootInput::CaretMoveBoundary(CaretMoveCause::Left)
        );
    }

    #[test]
    fn shortcut_is_its_own_root_state() {
        let transitions = Arc::new(Mutex::new(Vec::new()));
        let mut root = root(Arc::clone(&transitions));

        root.dispatch(RootInput::Character('x'));
        assert_eq!(root.dispatch(RootInput::ShortcutBoundary), "x");
        assert_eq!(root.state().phase, RootPhase::Idle);

        let log = transitions.lock().expect("transition log");
        assert_eq!(log[1].to, RootPhase::ShortcutBoundary);
    }

    #[test]
    fn lifecycle_reset_is_an_event_not_a_child_method() {
        let transitions = Arc::new(Mutex::new(Vec::new()));
        let mut root = root(Arc::clone(&transitions));

        root.dispatch(RootInput::Character('x'));
        assert_eq!(
            root.dispatch(RootInput::Lifecycle(LifecycleEvent::Reset)),
            ""
        );
        assert_eq!(root.state().phase, RootPhase::Idle);
        assert!(root.state().raw.is_empty());
    }
}
