use inputkey_boundary::{InputEvent, InputType, OutputDto};
use inputkey_core_abstractions::{
    CoreEvent, CoreEventType, EngineState, EventPublisher, RootPhase,
};

use crate::Machine;

fn project(machine: &Machine, commit: &str) -> OutputDto {
    let state = machine.state();
    OutputDto {
        rendered: state.rendered,
        raw: state.raw,
        commit: commit.to_owned(),
        mode: match state.phase {
            RootPhase::Idle => "idle",
            RootPhase::Composing => "composing",
        }
        .to_owned(),
        phase: state.child_phase,
        changed: false,
    }
}

pub struct Session {
    machine: Machine,
    publisher: Option<Box<dyn EventPublisher>>,
    sequence: u64,
    last: OutputDto,
}

impl Session {
    pub fn new(machine: Machine, publisher: Option<Box<dyn EventPublisher>>) -> Self {
        let last = project(&machine, "");
        Self {
            machine,
            publisher,
            sequence: 0,
            last,
        }
    }

    pub fn machine(&self) -> &Machine {
        &self.machine
    }

    pub fn machine_mut(&mut self) -> &mut Machine {
        &mut self.machine
    }

    pub fn state(&self) -> EngineState {
        self.machine.state()
    }

    pub fn output(&self) -> OutputDto {
        self.last.clone()
    }

    fn publish(&mut self, kind: CoreEventType, text: String) {
        let Some(publisher) = self.publisher.as_ref() else {
            return;
        };
        self.sequence += 1;
        publisher.publish(CoreEvent {
            kind,
            sequence: self.sequence,
            text,
            state: self.machine.state(),
        });
    }

    pub fn handle(&mut self, input: InputEvent) -> OutputDto {
        let before = self.machine.rendered_text();
        let mut commit = String::new();

        match input.kind {
            InputType::Key => {
                if let Some(key) = input.key.chars().next() {
                    self.machine.type_key(key);
                }
            }
            InputType::Backspace => {
                self.machine.backspace();
            }
            InputType::Escape => {
                self.machine.escape();
            }
            InputType::Finalize => {
                commit = self.machine.finalize();
            }
            InputType::DecisionBoundary => {
                if let Some(delimiter) = input.key.chars().next() {
                    commit = self.machine.decision_boundary(delimiter);
                }
            }
            InputType::CommitBoundary => {
                commit = self.machine.commit_boundary();
            }
            InputType::CommitDisplayed => {
                commit = self.machine.commit_displayed();
            }
            InputType::CommitRawBoundary => {
                commit = self.machine.commit_raw_boundary();
            }
            InputType::Reset => {
                self.machine.reset();
            }
        }

        let mut out = project(&self.machine, &commit);
        out.changed = before != out.rendered;
        self.last = out.clone();

        if input.kind == InputType::Reset {
            self.publish(CoreEventType::Reset, String::new());
        } else {
            if out.changed {
                self.publish(CoreEventType::PreeditChanged, out.rendered.clone());
            }
            if !commit.is_empty() {
                self.publish(CoreEventType::TextCommitted, commit);
            }
            self.publish(CoreEventType::StateChanged, out.rendered.clone());
        }
        out
    }
}
