use inputkey_core_abstractions::{EngineState, LanguageMachinePort, RootPhase, TypingEnginePort};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RootEvent {
    Key(char),
    Backspace,
    Escape,
    Finalize,
    DecisionBoundary(char),
    CommitBoundary,
    NaturalBoundary,
    MouseBoundary,
    CommitRawBoundary,
    Reset,
}

/// The single root composition state machine.
///
/// It owns language-independent composition boundaries and delegates only
/// language interpretation to the active child machine.
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

    fn sync_phase(&mut self) {
        self.phase = if self.child.history_active() {
            RootPhase::Composing
        } else {
            RootPhase::Idle
        };
    }

    fn enter_boundary(&mut self, phase: RootPhase) {
        self.phase = phase;
    }

    fn commit_displayed_and_reset(&mut self) -> String {
        let committed = self.child.rendered();
        self.child.reset();
        self.phase = RootPhase::Idle;
        committed
    }

    fn resolve_boundary(&mut self) -> String {
        if matches!(
            self.phase,
            RootPhase::NaturalBoundary | RootPhase::MouseBoundary
        ) {
            return self.commit_displayed_and_reset();
        }
        let committed = match self.phase {
            RootPhase::CorrectionBoundary => self.child.correct_boundary(),
            RootPhase::RawBoundary => self.child.raw(),
            RootPhase::FinalizeBoundary => self.child.finalize(),
            RootPhase::Idle | RootPhase::Composing => return String::new(),
            RootPhase::NaturalBoundary | RootPhase::MouseBoundary => unreachable!(),
        };
        self.child.reset();
        self.phase = RootPhase::Idle;
        committed
    }

    fn transition(&mut self, event: RootEvent) -> String {
        match event {
            RootEvent::Key(key) => {
                let rendered = self.child.type_key(key);
                self.sync_phase();
                rendered
            }
            RootEvent::Backspace => {
                let rendered = self.child.backspace();
                self.sync_phase();
                rendered
            }
            RootEvent::Escape => {
                let rendered = self.child.escape();
                self.sync_phase();
                rendered
            }
            RootEvent::Finalize => {
                let rendered = self.child.finalize();
                self.sync_phase();
                rendered
            }
            RootEvent::DecisionBoundary(delimiter) => {
                if !self.child.history_active() {
                    self.phase = RootPhase::Idle;
                    return delimiter.to_string();
                }
                self.enter_boundary(if delimiter == ' ' {
                    RootPhase::CorrectionBoundary
                } else {
                    RootPhase::FinalizeBoundary
                });
                let mut committed = self.resolve_boundary();
                committed.push(delimiter);
                committed
            }
            RootEvent::CommitBoundary => {
                self.enter_boundary(RootPhase::FinalizeBoundary);
                self.resolve_boundary()
            }
            RootEvent::NaturalBoundary => {
                self.enter_boundary(RootPhase::NaturalBoundary);
                self.resolve_boundary()
            }
            RootEvent::MouseBoundary => {
                self.enter_boundary(RootPhase::MouseBoundary);
                self.resolve_boundary()
            }
            RootEvent::CommitRawBoundary => {
                self.enter_boundary(RootPhase::RawBoundary);
                self.resolve_boundary()
            }
            RootEvent::Reset => {
                self.child.reset();
                self.phase = RootPhase::Idle;
                String::new()
            }
        }
    }

    pub fn accepts_key(&self, key: char) -> bool {
        self.child.accepts_key(key)
    }

    pub fn type_key(&mut self, key: char) -> String {
        self.transition(RootEvent::Key(key))
    }

    pub fn backspace(&mut self) -> String {
        self.transition(RootEvent::Backspace)
    }

    pub fn escape(&mut self) -> String {
        self.transition(RootEvent::Escape)
    }

    pub fn finalize(&mut self) -> String {
        self.transition(RootEvent::Finalize)
    }

    pub fn decision_boundary(&mut self, delimiter: char) -> String {
        self.transition(RootEvent::DecisionBoundary(delimiter))
    }

    pub fn commit_boundary(&mut self) -> String {
        self.transition(RootEvent::CommitBoundary)
    }

    pub fn natural_boundary(&mut self) -> String {
        self.transition(RootEvent::NaturalBoundary)
    }

    pub fn mouse_boundary(&mut self) -> String {
        self.transition(RootEvent::MouseBoundary)
    }

    pub fn commit_raw_boundary(&mut self) -> String {
        self.transition(RootEvent::CommitRawBoundary)
    }

    pub fn reset(&mut self) {
        let _ = self.transition(RootEvent::Reset);
    }

    pub fn rendered_text(&self) -> String {
        self.child.rendered()
    }

    pub fn raw_text(&self) -> String {
        self.child.raw()
    }

    pub fn has_history(&self) -> bool {
        self.child.history_active()
    }
}

impl TypingEnginePort for Machine {
    fn accepts_key(&self, key: char) -> bool {
        Machine::accepts_key(self, key)
    }

    fn type_key(&mut self, key: char) -> String {
        Machine::type_key(self, key)
    }

    fn backspace(&mut self) -> String {
        Machine::backspace(self)
    }

    fn escape(&mut self) -> String {
        Machine::escape(self)
    }

    fn finalize(&mut self) -> String {
        Machine::finalize(self)
    }

    fn decision_boundary(&mut self, delimiter: char) -> String {
        Machine::decision_boundary(self, delimiter)
    }

    fn commit_boundary(&mut self) -> String {
        Machine::commit_boundary(self)
    }

    fn natural_boundary(&mut self) -> String {
        Machine::natural_boundary(self)
    }

    fn mouse_boundary(&mut self) -> String {
        Machine::mouse_boundary(self)
    }

    fn commit_raw_boundary(&mut self) -> String {
        Machine::commit_raw_boundary(self)
    }

    fn reset(&mut self) {
        Machine::reset(self)
    }

    fn rendered(&self) -> String {
        Machine::rendered_text(self)
    }

    fn raw(&self) -> String {
        Machine::raw_text(self)
    }

    fn history_active(&self) -> bool {
        Machine::has_history(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputkey_core_abstractions::LanguageState;

    struct ProbeLanguage {
        raw: String,
    }

    impl ProbeLanguage {
        fn new() -> Self {
            Self { raw: String::new() }
        }
    }

    impl LanguageMachinePort for ProbeLanguage {
        fn accepts_key(&self, key: char) -> bool {
            key.is_ascii_alphabetic()
        }

        fn type_key(&mut self, key: char) -> String {
            self.raw.push(key);
            self.raw.clone()
        }

        fn backspace(&mut self) -> String {
            self.raw.pop();
            self.raw.clone()
        }

        fn escape(&mut self) -> String {
            self.raw.clone()
        }

        fn finalize(&mut self) -> String {
            format!("final:{}", self.raw)
        }

        fn correct_boundary(&mut self) -> String {
            format!("correct:{}", self.raw)
        }

        fn reset(&mut self) {
            self.raw.clear();
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

    #[test]
    fn natural_and_mouse_are_distinct_states_with_the_same_commit_policy() {
        let mut root = Machine::new("probe", Box::new(ProbeLanguage::new()));
        root.type_key('a');
        root.enter_boundary(RootPhase::NaturalBoundary);
        assert_eq!(root.state().phase, RootPhase::NaturalBoundary);
        assert_eq!(root.resolve_boundary(), "a");
        assert_eq!(root.state().phase, RootPhase::Idle);

        root.type_key('b');
        root.enter_boundary(RootPhase::MouseBoundary);
        assert_eq!(root.state().phase, RootPhase::MouseBoundary);
        assert_eq!(root.resolve_boundary(), "b");
        assert_eq!(root.state().phase, RootPhase::Idle);
    }

    #[test]
    fn public_boundary_events_end_composition() {
        let mut root = Machine::new("probe", Box::new(ProbeLanguage::new()));
        root.type_key('x');
        assert_eq!(root.natural_boundary(), "x");
        assert_eq!(root.state().phase, RootPhase::Idle);

        root.type_key('y');
        assert_eq!(root.mouse_boundary(), "y");
        assert_eq!(root.state().phase, RootPhase::Idle);
    }

    #[test]
    fn only_space_decision_boundary_invokes_language_correction() {
        let mut root = Machine::new("probe", Box::new(ProbeLanguage::new()));

        root.type_key('a');
        assert_eq!(root.decision_boundary(' '), "correct:a ");

        root.type_key('b');
        assert_eq!(root.decision_boundary('.'), "final:b.");

        root.type_key('c');
        assert_eq!(root.commit_boundary(), "final:c");

        root.type_key('d');
        assert_eq!(root.finalize(), "final:d");
    }
}
