use inputkey_boundary::{InputEvent, InputType, OutputDto};
use inputkey_core_abstractions::{
    CaretMoveCause, CompositionControl, CoreEvent, CoreEventType, EngineState, EventPublisher,
    LifecycleEvent, RootInput, RootPhase,
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
            RootPhase::SpaceBoundary => "space_boundary",
            RootPhase::PunctuationBoundary => "punctuation_boundary",
            RootPhase::CaretMoveBoundary => "caret_move_boundary",
            RootPhase::ShortcutBoundary => "shortcut_boundary",
            RootPhase::CompositionControl => "composition_control",
            RootPhase::RawBoundary => "raw_boundary",
            RootPhase::Lifecycle => "lifecycle",
        }
        .to_owned(),
        phase: state.child_phase,
        changed: false,
    }
}

fn caret_cause(value: &str) -> CaretMoveCause {
    match value {
        "mouse" => CaretMoveCause::Mouse,
        "enter" => CaretMoveCause::Enter,
        "tab" => CaretMoveCause::Tab,
        "left" => CaretMoveCause::Left,
        "right" => CaretMoveCause::Right,
        "up" => CaretMoveCause::Up,
        "down" => CaretMoveCause::Down,
        "home" => CaretMoveCause::Home,
        "end" => CaretMoveCause::End,
        "page_up" => CaretMoveCause::PageUp,
        "page_down" => CaretMoveCause::PageDown,
        "delete" => CaretMoveCause::Delete,
        "insert" => CaretMoveCause::Insert,
        _ => CaretMoveCause::Other,
    }
}

fn control(value: &str) -> Option<CompositionControl> {
    match value {
        "backspace" => Some(CompositionControl::Backspace),
        "escape" => Some(CompositionControl::Escape),
        _ => None,
    }
}

fn lifecycle(value: &str) -> LifecycleEvent {
    match value {
        "reset" => LifecycleEvent::Reset,
        "focus_lost" => LifecycleEvent::FocusLost,
        "context_destroyed" => LifecycleEvent::ContextDestroyed,
        "disabled" => LifecycleEvent::Disabled,
        "language_changed" => LifecycleEvent::LanguageChanged,
        _ => LifecycleEvent::Finalize,
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
        let before = self.machine.state().rendered;
        let semantic = match input.kind {
            InputType::Character => input.key.chars().next().map(RootInput::Character),
            InputType::SpaceBoundary => Some(RootInput::SpaceBoundary),
            InputType::PunctuationBoundary => {
                input.key.chars().next().map(RootInput::PunctuationBoundary)
            }
            InputType::CaretMoveBoundary => {
                Some(RootInput::CaretMoveBoundary(caret_cause(&input.key)))
            }
            InputType::ShortcutBoundary => Some(RootInput::ShortcutBoundary),
            InputType::CompositionControl => control(&input.key).map(RootInput::CompositionControl),
            InputType::RawBoundary => Some(RootInput::RawBoundary),
            InputType::Lifecycle => Some(RootInput::Lifecycle(lifecycle(&input.key))),
        };

        let result = semantic
            .map(|event| self.machine.dispatch(event))
            .unwrap_or_default();

        let commits = matches!(
            input.kind,
            InputType::SpaceBoundary
                | InputType::PunctuationBoundary
                | InputType::CaretMoveBoundary
                | InputType::ShortcutBoundary
                | InputType::RawBoundary
                | InputType::Lifecycle
        ) && !(input.kind == InputType::Lifecycle && input.key == "reset");
        let commit = if commits { result } else { String::new() };

        let mut out = project(&self.machine, &commit);
        out.changed = before != out.rendered;
        self.last = out.clone();

        if input.kind == InputType::Lifecycle && input.key == "reset" {
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
