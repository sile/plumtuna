use crate::contact::service::ContactServiceHandle;
use crate::Result;
use bytecodec::json_codec::{JsonDecoder, JsonEncoder};
use bytecodec::null::{NullDecoder, NullEncoder};
use fibers_rpc::server::{HandleCall, Reply};
use fibers_rpc::{Call, ProcedureId};
use futures::Future;
use plumcast::node::NodeId;

#[derive(Debug)]
pub struct RpcHandler {
    service: ContactServiceHandle,
}
impl RpcHandler {
    pub fn new(service: ContactServiceHandle) -> Self {
        RpcHandler { service }
    }
}

#[derive(Debug)]
pub struct GetContactNodeIdCall;
impl Call for GetContactNodeIdCall {
    const ID: ProcedureId = ProcedureId(0x43a1_0000);
    const NAME: &'static str = "plumtuna.get_contact_node_id";

    type Req = ();
    type Res = Result<NodeId>;

    type ReqEncoder = NullEncoder;
    type ReqDecoder = NullDecoder;

    type ResEncoder = JsonEncoder<Self::Res>;
    type ResDecoder = JsonDecoder<Self::Res>;
}
impl HandleCall<GetContactNodeIdCall> for RpcHandler {
    fn handle_call(&self, _: ()) -> Reply<GetContactNodeIdCall> {
        Reply::future(self.service.get_contact_node_id().then(|result| Ok(result)))
    }
}
