use crate::{global, study};
use crate::{ErrorKind, Result};
use bytecodec::json_codec::{JsonDecoder, JsonEncoder};
use plumcast::message::MessagePayload;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnionMessage {
    Global(global::Message),
    Study(study::Message),
}
impl UnionMessage {
    pub fn into_global_message(self) -> Result<global::Message> {
        if let UnionMessage::Global(m) = self {
            Ok(m)
        } else {
            track_panic!(ErrorKind::Other);
        }
    }

    pub fn into_study_message(self) -> Result<study::Message> {
        if let UnionMessage::Study(m) = self {
            Ok(m)
        } else {
            track_panic!(ErrorKind::Other);
        }
    }
}
impl From<global::Message> for UnionMessage {
    fn from(f: global::Message) -> Self {
        UnionMessage::Global(f)
    }
}
impl From<study::Message> for UnionMessage {
    fn from(f: study::Message) -> Self {
        UnionMessage::Study(f)
    }
}
impl MessagePayload for UnionMessage {
    type Encoder = JsonEncoder<Self>;
    type Decoder = JsonDecoder<Self>;
}
