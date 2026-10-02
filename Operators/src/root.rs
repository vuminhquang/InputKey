use inputkey_core_abstractions::{EngineState, LanguageMachinePort, RootPhase, TypingEnginePort};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RootEvent {
    Key(char),
    Backspace,
    Escape,
    Finalize,
    DecisionBoundary(char),
    CommitBoundary,
    CommitDisplayed,
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
}

impl Machine {
    pub fn new(language_id: impl Into<String>, child: Box<dyn LanguageMachinePort>) -> Self {
        Self {
            language_id: language_id.into(),
            child,
        }
    }

    pub fn language_id(&self) -> &str {
        &self.language_id
    }

    pub fn state(&self) -> EngineState {
        let child = self.child.state();
        EngineState {
            language_id: self.language_id.clone(),
            phase: if child.raw.is_empty() {
                RootPhase::Idle
            } else {
                RootPhase::Composing
            },
            raw: child.raw,
            rendered: child.rendered,
            child_mode: child.mode,
            child_phase: child.phase,
        }
    }

    fn transition(&mut self, event: RootEvent) -> String {
        match event {
            RootEvent::Key(key) => self.child.type_key(key),
            RootEvent::Backspace => self.child.backspace(),
            RootEvent::Escape => self.child.escape(),
            RootEvent::Finalize => self.child.finalize(),
            RootEvent::DecisionBoundary(delimiter) => {
                if !self.child.history_active() {
                    return delimiter.to_string();
                }
                let mut committed = self.child.finalize();
                self.child.reset();
                committed.push(delimiter);
                committed
            }
            RootEvent::CommitBoundary => {
                let committed = self.child.finalize();
                self.child.reset();
                committed
            }
            RootEvent::CommitDisplayed => {
                let committed = self.child.rendered();
                self.child.reset();
                committed
            }
            RootEvent::CommitRawBoundary => {
                let committed = self.child.raw();
                self.child.reset();
                committed
            }
            RootEvent::Reset => {
                self.child.reset();
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

    pub fn commit_displayed(&mut self) -> String {
        self.transition(RootEvent::CommitDisplayed)
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

    fn commit_displayed(&mut self) -> String {
        Machine::commit_displayed(self)
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
