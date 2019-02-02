use crate::study::StudyDirection;
use crate::study_list::node::{StudyId, StudyIdPrefix, StudyName};
use crate::trial::{TrialId, TrialState};
use bytecodec::bincode_codec::{BincodeDecoder, BincodeEncoder};
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
    SetTrialSystemAttr {
        trial_id: TrialId,
        key: String,
        value: JsonValue,
    },
}
impl MessagePayload for Message {
    type Encoder = BincodeEncoder<Message>;
    type Decoder = BincodeDecoder<Message>;
}
