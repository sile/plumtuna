use crate::study_list::node::{StudyId, StudyIdPrefix, StudyName};
use bytecodec::bincode_codec::{BincodeDecoder, BincodeEncoder};
use plumcast::message::MessagePayload;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    ReservePrefix {
        study_id_prefix: StudyIdPrefix,
    },
    CreateStudy {
        study_name: StudyName,
        study_id: StudyId,
    },
}
impl MessagePayload for Message {
    type Encoder = BincodeEncoder<Message>;
    type Decoder = BincodeDecoder<Message>;
}
