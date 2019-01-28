use super::node::{StudyId, StudyName};
use plumcast::message::MessageId;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Studies {
    name_to_id: HashMap<StudyName, StudyId>,
    studies: HashMap<StudyId, Study>,
}

#[derive(Debug)]
struct Study {
    name: StudyName,
    create_mid: MessageId,
    // TODO: nodeid
}
