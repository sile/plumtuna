use super::studies::Studies;
use crate::study_list::message::Message;
use crate::Error;
use futures::{Async, Future, Poll, Stream};
use plumcast::message::MessageId;
use plumcast::node::{Node as PlumcastNode, NodeId};
use rand;
use slog::Logger;

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
    study_id_prefix: StudyIdPrefix,
    plumcast_node: PlumcastNode<Message>,
    seqno: u32,
    studies: Studies,
}
impl StudyListNode {
    pub fn new(logger: Logger, mut plumcast_node: PlumcastNode<Message>) -> Self {
        let logger = logger.new(o!("id" => plumcast_node.id().to_string()));
        let study_id_prefix = StudyIdPrefix::random();
        info!(
            logger,
            "Starts StudyListNode: study_id_prefix={:?}", study_id_prefix
        );
        let m = Message::ReservePrefix { study_id_prefix };
        plumcast_node.broadcast(m);
        Self {
            logger,
            study_id_prefix,
            plumcast_node,
            seqno: rand::random(),
            studies: Studies::default(),
        }
    }

    fn id(&self) -> NodeId {
        self.plumcast_node.id()
    }

    fn handle_message(&mut self, mid: MessageId, m: Message) {
        match m {
            Message::ReservePrefix { study_id_prefix } => {
                if mid.node() == self.id() {
                    return;
                }
                info!(
                    self.logger,
                    "New StudyListNode {}: study_id_prefix={:?}",
                    mid.node(),
                    study_id_prefix
                );
                if self.study_id_prefix == study_id_prefix && self.id() < mid.node() {
                    self.study_id_prefix = StudyIdPrefix::random();
                    warn!(
                        self.logger,
                        "`study_id_prefix` conflict: new_prefix={:?}", self.study_id_prefix
                    );

                    // TODO: forget old `ReservePrefix` message
                    let m = Message::ReservePrefix {
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
            let mid = m.id().clone();
            self.handle_message(mid, m.into_payload());
        }
        Ok(Async::NotReady)
    }
}
