use crate::study::StudyDirection;
use crate::study_list::node::{StudyId, StudyIdPrefix, StudyName};
use crate::trial::{TrialId, TrialParamValue, TrialState};
use bytecodec::json_codec::{JsonDecoder, JsonEncoder};
use plumcast::message::MessagePayload;
use plumcast::node::NodeId;
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    ReservePrefix {
        study_id_prefix: StudyIdPrefix,
    },
    CreateStudy {
        study_name: StudyName,
        study_id: StudyId,
    },
    JoinStudy {
        study_id: StudyId,
        node: NodeId,
    },
    SetStudyDirection {
        study_id: StudyId,
        direction: StudyDirection,
    },
    CreateTrial {
        study_id: StudyId,
        trial_id: TrialId,
    },
    SetTrialState {
        trial_id: TrialId,
        state: TrialState,
    },
    SetTrialParamValue {
        trial_id: TrialId,
        key: String,
        value: TrialParamValue,
    },
    SetTrialValue {
        trial_id: TrialId,
        value: f64,
    },
    SetTrialIntermediateValue {
        trial_id: TrialId,
        step: u32,
        value: f64,
    },
    SetTrialSystemAttr {
        trial_id: TrialId,
        key: String,
        value: JsonValue,
    },
    SetTrialUserAttr {
        trial_id: TrialId,
        key: String,
        value: JsonValue,
    },
}
impl MessagePayload for Message {
    type Encoder = JsonEncoder<Message>;
    type Decoder = JsonDecoder<Message>;
}
