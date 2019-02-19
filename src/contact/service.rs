use crate::contact::rpc::RpcHandler;
use crate::Error;
use fibers::sync::{mpsc, oneshot};
use fibers_rpc::server::ServerBuilder as RpcServerBulider;
use futures::{Async, Future, Poll, Stream};
use plumcast::node::NodeId;

#[derive(Debug)]
pub struct ContactService {
    contact_node_id: Option<NodeId>,
    command_tx: mpsc::Sender<Command>,
    command_rx: mpsc::Receiver<Command>,
    contact_node_waitings: Vec<oneshot::Monitored<NodeId, Error>>,
}
impl ContactService {
    pub fn new(rpc: &mut RpcServerBulider) -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let this = Self {
            contact_node_id: None,
            command_tx,
            command_rx,
            contact_node_waitings: Vec::new(),
        };

        rpc.add_call_handler(RpcHandler::new(this.handle()));
        this
    }

    pub fn handle(&self) -> ContactServiceHandle {
        ContactServiceHandle {
            command_tx: self.command_tx.clone(),
        }
    }

    fn handle_command(&mut self, command: Command) {
        match command {
            Command::GetContactNodeId { reply_tx } => {
                if let Some(id) = self.contact_node_id {
                    reply_tx.exit(Ok(id));
                } else {
                    self.contact_node_waitings.push(reply_tx);
                }
            }
            Command::SetContactNodeId { node_id } => {
                self.contact_node_id = Some(node_id);
                for tx in self.contact_node_waitings.drain(..) {
                    tx.exit(Ok(node_id));
                }
            }
        }
    }
}
impl Future for ContactService {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready(Some(command)) = self.command_rx.poll().expect("never fails") {
            self.handle_command(command);
        }
        Ok(Async::NotReady)
    }
}

#[derive(Debug, Clone)]
pub struct ContactServiceHandle {
    command_tx: mpsc::Sender<Command>,
}
impl ContactServiceHandle {
    pub fn get_contact_node_id(&self) -> impl Future<Item = NodeId, Error = Error> {
        let (reply_tx, reply_rx) = oneshot::monitor();
        let command = Command::GetContactNodeId { reply_tx };
        let _ = self.command_tx.send(command);
        track_err!(reply_rx.map_err(Error::from))
    }

    pub fn set_contact_node_id(&self, node_id: NodeId) {
        let command = Command::SetContactNodeId { node_id };
        let _ = self.command_tx.send(command);
    }
}

#[derive(Debug)]
enum Command {
    GetContactNodeId {
        reply_tx: oneshot::Monitored<NodeId, Error>,
    },
    SetContactNodeId {
        node_id: NodeId,
    },
}
