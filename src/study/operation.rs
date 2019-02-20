use crate::study::Message;
use crate::time::Timestamp;
use plumcast::message::MessageId;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum OperationKey {
    SetStudyDirection,
}
impl OperationKey {
    pub fn from_message(m: &Message) -> Self {
        match m {
            Message::SetStudyDirection { .. } => OperationKey::SetStudyDirection,
        }
    }
}

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct Operation {
    pub timestamp: Timestamp,
    pub mid: MessageId,
}
impl Operation {
    pub fn new(mid: MessageId, m: &Message) -> Self {
        Operation {
            mid,
            timestamp: m.timestamp(),
        }
    }
}
