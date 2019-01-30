use crate::study_list::node::{StudyId, StudyIdPrefix, StudyName};
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
}
impl MessagePayload for Message {
    type Encoder = BincodeEncoder<Message>;
    type Decoder = BincodeDecoder<Message>;
}
