use crate::study::StudyDirection;
use crate::time::Timestamp;
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    SetStudyDirection {
        direction: StudyDirection,
        timestamp: Timestamp,
    },
    SetStudyUserAttr {
        key: String,
        value: JsonValue,
        timestamp: Timestamp,
    },
    SetStudySystemAttr {
        key: String,
        value: JsonValue,
        timestamp: Timestamp,
    },
}
impl Message {
    pub fn timestamp(&self) -> Timestamp {
        match self {
            Message::SetStudyDirection { timestamp, .. }
            | Message::SetStudyUserAttr { timestamp, .. }
            | Message::SetStudySystemAttr { timestamp, .. } => *timestamp,
        }
    }
}
