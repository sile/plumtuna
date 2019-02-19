use crate::global;
use crate::{ErrorKind, Result};
use bytecodec::json_codec::{JsonDecoder, JsonEncoder};
use plumcast::message::MessagePayload;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnionMessage {
    Global(global::Message),
    Study,
}
impl UnionMessage {
    pub fn into_global_message(self) -> Result<global::Message> {
        if let UnionMessage::Global(m) = self {
            Ok(m)
        } else {
            track_panic!(ErrorKind::Other);
        }
    }
}
impl MessagePayload for UnionMessage {
    type Encoder = JsonEncoder<Self>;
    type Decoder = JsonDecoder<Self>;
}
