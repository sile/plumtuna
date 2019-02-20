use crate::global::GlobalNodeHandle;
use crate::study::StudyNameAndId;
use bytecodec::json_codec::{JsonDecoder, JsonEncoder};
use fibers_rpc::server::{HandleCast, NoReply};
use fibers_rpc::{Cast, ProcedureId};

#[derive(Debug)]
pub struct RpcHandler {
    node: GlobalNodeHandle,
}
impl RpcHandler {
    pub fn new(node: GlobalNodeHandle) -> Self {
        Self { node }
    }
}

#[derive(Debug)]
pub struct StudyCast;
impl Cast for StudyCast {
    const ID: ProcedureId = ProcedureId(0x43a2_0000);
    const NAME: &'static str = "plumtuna.global.study";

    type Notification = StudyNameAndId;
    type Encoder = JsonEncoder<Self::Notification>;
    type Decoder = JsonDecoder<Self::Notification>;
}
impl HandleCast<StudyCast> for RpcHandler {
    fn handle_cast(&self, study: StudyNameAndId) -> NoReply {
        self.node.notify_study(study);
        NoReply::done()
    }
}
