use crate::study::StudyDirection;
use crate::time::Timestamp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    SetStudyDirection {
        direction: StudyDirection,
        timestamp: Timestamp,
    },
}
impl Message {
    pub fn timestamp(&self) -> Timestamp {
        match self {
            Message::SetStudyDirection { timestamp, .. } => *timestamp,
        }
    }
}
