use inputkey_boundary::{InputEvent, OutputDto};
use inputkey_core_abstractions::EventPublisher;

use crate::{Machine, Session};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayResult {
    pub outputs: Vec<OutputDto>,
    pub final_output: OutputDto,
}

pub fn replay(
    machine: Machine,
    inputs: &[InputEvent],
    publisher: Option<Box<dyn EventPublisher>>,
) -> ReplayResult {
    let mut session = Session::new(machine, publisher);
    let outputs = inputs
        .iter()
        .cloned()
        .map(|input| session.handle(input))
        .collect();
    ReplayResult {
        outputs,
        final_output: session.output(),
    }
}
