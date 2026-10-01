use inputkey_boundary::{InputEvent, InputType, Options, OutputDto};
use inputkey_core_abstractions::{
    CoreEvent, CoreEventType, EngineState, EventPublisher, LexiconPort, Mode, Phase,
};

use crate::Machine;

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Start => "start",
        Mode::ViCandidate => "vi_candidate",
        Mode::RawLocked => "raw_locked",
    }
}

fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Start => "start",
        Phase::Onset => "onset",
        Phase::Nucleus => "nucleus",
        Phase::Coda => "coda",
        Phase::Complete => "complete",
        Phase::PendingShape => "pending_shape",
        Phase::Dead => "dead",
    }
}

fn project(machine: &Machine, commit: &str) -> OutputDto {
    let state = machine.state();
    OutputDto {
        rendered: state.rendered,
        raw: state.raw,
        commit: commit.to_owned(),
        mode: mode_name(state.mode).to_owned(),
        phase: phase_name(state.phase).to_owned(),
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
    pub fn new(
        options: Options,
        lexicon: Option<Box<dyn LexiconPort>>,
        publisher: Option<Box<dyn EventPublisher>>,
    ) -> Self {
        let machine = Machine::new(options, lexicon);
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
        let before = self.machine.rendered_text().to_owned();
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
            InputType::Reset => {
                self.machine.reset();
            }
            InputType::LiteralizeToken => {
                self.machine.literalize_token();
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
