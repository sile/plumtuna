use crate::contact::rpc;
use crate::Error;
use fibers_rpc::client::ClientServiceHandle as RpcClientServiceHandle;
use fibers_rpc::Call;
use futures::Future;
use plumcast::node::NodeId;
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ContactServiceClient {
    rpc_client_service: RpcClientServiceHandle,
}
impl ContactServiceClient {
    pub fn new(rpc_client_service: RpcClientServiceHandle) -> Self {
        Self { rpc_client_service }
    }

    pub fn get_contact_node_id(
        &self,
        server_addr: SocketAddr,
    ) -> impl Future<Item = NodeId, Error = Error> {
        let mut client = rpc::GetContactNodeIdCall::client(&self.rpc_client_service);
        client.options_mut().force_wakeup = true;
        client.options_mut().timeout = Some(Duration::from_millis(100));
        track_err!(client
            .call(server_addr, ())
            .map_err(Error::from)
            .and_then(|result| result))
    }
}
