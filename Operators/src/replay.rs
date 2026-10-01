use inputkey_boundary::{InputEvent, Options, OutputDto};
use inputkey_core_abstractions::{EventPublisher, LexiconPort};

use crate::Session;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayResult {
    pub outputs: Vec<OutputDto>,
    pub final_output: OutputDto,
}

pub fn replay(
    options: Options,
    inputs: &[InputEvent],
    lexicon: Option<Box<dyn LexiconPort>>,
    publisher: Option<Box<dyn EventPublisher>>,
) -> ReplayResult {
    let mut session = Session::new(options, lexicon, publisher);
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
