use crate::study::Message;
use crate::time::Timestamp;
use crate::trial::TrialId2;
use plumcast::message::MessageId;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum OperationKey {
    SetStudyDirection,
    SetStudyUserAttr { key: String },
    SetStudySystemAttr { key: String },
    CreateTrial { trial_id: TrialId2 }, // TODO: remove?
    SetTrialState { trial_id: TrialId2 },
    SetTrialParam { trial_id: TrialId2, key: String },
    SetTrialValue { trial_id: TrialId2 },
    SetTrialIntermediateValue { trial_id: TrialId2, step: u32 },
    SetTrialUserAttr { trial_id: TrialId2, key: String },
    SetTrialSystemAttr { trial_id: TrialId2, key: String },
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
            Message::CreateTrial { trial_id, .. } => OperationKey::CreateTrial {
                trial_id: trial_id.clone(),
            },
            Message::SetTrialState { trial_id, .. } => OperationKey::SetTrialState {
                trial_id: trial_id.clone(),
            },
            Message::SetTrialParam { trial_id, key, .. } => OperationKey::SetTrialParam {
                trial_id: trial_id.clone(),
                key: key.clone(),
            },
            Message::SetTrialValue { trial_id, .. } => OperationKey::SetTrialValue {
                trial_id: trial_id.clone(),
            },
            Message::SetTrialIntermediateValue { trial_id, step, .. } => {
                OperationKey::SetTrialIntermediateValue {
                    trial_id: trial_id.clone(),
                    step: *step,
                }
            }
            Message::SetTrialUserAttr { trial_id, key, .. } => OperationKey::SetTrialUserAttr {
                trial_id: trial_id.clone(),
                key: key.clone(),
            },
            Message::SetTrialSystemAttr { trial_id, key, .. } => OperationKey::SetTrialSystemAttr {
                trial_id: trial_id.clone(),
                key: key.clone(),
            },
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
