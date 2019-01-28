use crate::study_list::message::Message;
use crate::Error;
use futures::{Async, Future, Poll, Stream};
use plumcast::node::Node as PlumcastNode;
use rand;
use slog::Logger;
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId(Uuid);
impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}
impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "NodeId({})", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StudyId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StudyIdPrefix(u32);
impl StudyIdPrefix {
    pub fn random() -> Self {
        Self(rand::random())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StudyName(String);

#[derive(Debug)]
pub struct StudyListNode {
    logger: Logger,
    id: NodeId,
    study_id_prefix: StudyIdPrefix,
    plumcast_node: PlumcastNode<Message>,
}
impl StudyListNode {
    pub fn new(logger: Logger, mut plumcast_node: PlumcastNode<Message>) -> Self {
        let id = NodeId::new();
        let logger = logger.new(o!("id" => id.0.to_string()));
        let study_id_prefix = StudyIdPrefix::random();
        info!(
            logger,
            "Starts StudyListNode: study_id_prefix={:?}", study_id_prefix
        );
        let m = Message::ReservePrefix {
            node_id: id.clone(),
            study_id_prefix,
        };
        plumcast_node.broadcast(m);
        Self {
            logger,
            id,
            study_id_prefix,
            plumcast_node,
        }
    }

    fn handle_message(&mut self, m: Message) {
        match m {
            Message::ReservePrefix {
                node_id,
                study_id_prefix,
            } => {
                if node_id == self.id {
                    return;
                }
                info!(
                    self.logger,
                    "New StudyListNode {}: study_id_prefix={:?}", node_id, study_id_prefix
                );
                if self.study_id_prefix == study_id_prefix && self.id < node_id {
                    self.study_id_prefix = StudyIdPrefix::random();
                    warn!(
                        self.logger,
                        "`study_id_prefix` conflict: new_prefix={:?}", self.study_id_prefix
                    );

                    // TODO: forget old `ReservePrefix` message
                    let m = Message::ReservePrefix {
                        node_id: self.id.clone(),
                        study_id_prefix: self.study_id_prefix,
                    };
                    self.plumcast_node.broadcast(m);
                }
            }
            Message::CreateNewStudyId { .. } => panic!(),
        }
    }
}
impl Future for StudyListNode {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready(Some(m)) = track!(self.plumcast_node.poll())? {
            self.handle_message(m.into_payload());
        }
        Ok(Async::NotReady)
    }
}
