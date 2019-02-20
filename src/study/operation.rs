use crate::study::Message;
use crate::time::Timestamp;
use plumcast::message::MessageId;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum OperationKey {
    SetStudyDirection,
    SetStudyUserAttr { key: String },
    SetStudySystemAttr { key: String },
}
impl OperationKey {
    pub fn from_message(m: &Message) -> Self {
        match m {
            Message::SetStudyDirection { .. } => OperationKey::SetStudyDirection,
            Message::SetStudyUserAttr { key, .. } => {
                OperationKey::SetStudyUserAttr { key: key.clone() }
            }
            Message::SetStudySystemAttr { key, .. } => {
                OperationKey::SetStudySystemAttr { key: key.clone() }
            }
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
