use crate::study::StudyDirection;
use crate::study_list::node::{StudyId, StudyIdPrefix, StudyName};
use crate::trial::TrialId;
use bytecodec::bincode_codec::{BincodeDecoder, BincodeEncoder};
use plumcast::message::MessagePayload;
use plumcast::node::NodeId;

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
}
impl MessagePayload for Message {
    type Encoder = BincodeEncoder<Message>;
    type Decoder = BincodeDecoder<Message>;
}
