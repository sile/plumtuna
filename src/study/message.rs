use crate::study::StudyDirection;
use crate::time::Timestamp;
use crate::trial::{TrialId, TrialParamValue, TrialState};
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
    CreateTrial {
        trial_id: TrialId,
        timestamp: Timestamp,
    },
    SetTrialState {
        trial_id: TrialId,
        state: TrialState,
        timestamp: Timestamp,
    },
    SetTrialParam {
        trial_id: TrialId,
        key: String,
        value: TrialParamValue,
        timestamp: Timestamp,
    },
    SetTrialValue {
        trial_id: TrialId,
        value: f64,
        timestamp: Timestamp,
    },
    SetTrialIntermediateValue {
        trial_id: TrialId,
        step: u32,
        value: f64,
        timestamp: Timestamp,
    },
    SetTrialUserAttr {
        trial_id: TrialId,
        key: String,
        value: JsonValue,
        timestamp: Timestamp,
    },
    SetTrialSystemAttr {
        trial_id: TrialId,
        key: String,
        value: JsonValue,
        timestamp: Timestamp,
    },
}
impl Message {
    pub fn timestamp(&self) -> Timestamp {
        match self {
            Message::SetStudyDirection { timestamp, .. }
            | Message::CreateTrial { timestamp, .. }
            | Message::SetTrialUserAttr { timestamp, .. }
            | Message::SetTrialSystemAttr { timestamp, .. }
            | Message::SetTrialParam { timestamp, .. }
            | Message::SetTrialIntermediateValue { timestamp, .. }
            | Message::SetTrialValue { timestamp, .. }
            | Message::SetTrialState { timestamp, .. }
            | Message::SetStudyUserAttr { timestamp, .. }
            | Message::SetStudySystemAttr { timestamp, .. } => *timestamp,
        }
    }
}
