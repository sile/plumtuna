use super::studies::{Studies, Study};
use crate::study::StudyDirection;
use crate::study_list::message::Message;
use crate::trial::{Trial, TrialId, TrialParamValue, TrialState};
use crate::{Error, ErrorKind, Result};
use atomic_immut::AtomicImmut;
use fibers::sync::{mpsc, oneshot};
use futures::{Async, Future, Poll, Stream};
use plumcast::message::MessageId;
use plumcast::node::{Node as PlumcastNode, NodeId};
use rand;
use serde_json::Value as JsonValue;
use slog::Logger;
use std;
use std::collections::HashMap;
use std::sync::atomic::{self, AtomicUsize};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StudyId(u64);
impl StudyId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StudyIdPrefix(u32);
impl StudyIdPrefix {
    pub fn random() -> Self {
        Self(rand::random::<u32>() & 0xFFFF)
    }
}

pub use crate::study::StudyName;

#[derive(Debug)]
enum Command {
    CreateStudy {
        name: StudyName,
        reply_tx: oneshot::Monitored<Option<StudyId>, Error>,
    },
    GetTrials {
        study_id: StudyId,
        reply_tx: oneshot::Monitored<Vec<Trial>, Error>,
    },
    GetTrial {
        trial_id: TrialId,
        reply_tx: oneshot::Monitored<Trial, Error>,
    },
    SetStudyDirection {
        id: StudyId,
        direction: StudyDirection,
    },
    CreateTrial {
        study_id: StudyId,
        trial_id: TrialId,
    },
    Broadcast {
        message: Message,
    },
    SetTrialSystemAttr {
        trial_id: TrialId,
        key: String,
        value: JsonValue,
        reply_tx: oneshot::Monitored<(), Error>,
    },
}

#[derive(Debug, Clone)]
pub struct StudyListNodeHandle {
    command_tx: mpsc::Sender<Command>,
    studies: Arc<AtomicImmut<HashMap<StudyId, Study>>>,
    study_id_prefix: StudyIdPrefix,
    next_trial_id: Arc<AtomicUsize>,
}
impl StudyListNodeHandle {
    pub fn create_study(
        &self,
        name: StudyName,
    ) -> impl Future<Item = Option<StudyId>, Error = Error> {
        let (reply_tx, reply_rx) = oneshot::monitor();
        let command = Command::CreateStudy { name, reply_tx };
        let _ = self.command_tx.send(command);
        track_err!(reply_rx.map_err(Error::from))
    }

    pub fn get_trials(&self, study_id: StudyId) -> impl Future<Item = Vec<Trial>, Error = Error> {
        let (reply_tx, reply_rx) = oneshot::monitor();
        let command = Command::GetTrials { study_id, reply_tx };
        let _ = self.command_tx.send(command);
        track_err!(reply_rx.map_err(Error::from))
    }

    pub fn get_trial(&self, trial_id: TrialId) -> impl Future<Item = Trial, Error = Error> {
        let (reply_tx, reply_rx) = oneshot::monitor();
        let command = Command::GetTrial { trial_id, reply_tx };
        let _ = self.command_tx.send(command);
        track_err!(reply_rx.map_err(Error::from))
    }

    pub fn create_trial(&self, study_id: StudyId) -> Result<TrialId> {
        let suffix = self.next_trial_id.fetch_add(1, atomic::Ordering::SeqCst);
        let trial_id = (u64::from(self.study_id_prefix.0) << 32) | u64::from(suffix as u32);
        let trial_id = TrialId::new(trial_id);

        let command = Command::CreateTrial { study_id, trial_id };
        let _ = self.command_tx.send(command);

        Ok(trial_id)
    }

    pub fn studies(&self) -> Arc<HashMap<StudyId, Study>> {
        self.studies.load()
    }

    pub fn fetch_study(&self, study_id: StudyId) -> Result<Study> {
        let study =
            track_assert_some!(self.studies().get(&study_id).cloned(), ErrorKind::Other; study_id);
        Ok(study)
    }

    pub fn fetch_study_by_name(&self, study_name: &StudyName) -> Result<Study> {
        let study = self
            .studies()
            .values()
            .find(|s| s.name == *study_name)
            .cloned();
        Ok(track_assert_some!(study, ErrorKind::Other; study_name))
    }

    pub fn set_study_direction(&self, study_id: StudyId, direction: StudyDirection) -> Result<()> {
        track_assert_some!(self.studies().get(&study_id), ErrorKind::Other; study_id);

        let command = Command::SetStudyDirection {
            id: study_id,
            direction,
        };
        let _ = self.command_tx.send(command);
        Ok(())
    }

    pub fn set_trial_state(&self, trial_id: TrialId, state: TrialState) -> Result<()> {
        let message = Message::SetTrialState { trial_id, state };
        let command = Command::Broadcast { message };
        let _ = self.command_tx.send(command);
        Ok(())
    }

    pub fn set_trial_value(&self, trial_id: TrialId, value: f64) -> Result<()> {
        let message = Message::SetTrialValue { trial_id, value };
        let command = Command::Broadcast { message };
        let _ = self.command_tx.send(command);
        Ok(())
    }

    pub fn set_trial_intermediate_value(
        &self,
        trial_id: TrialId,
        step: u32,
        value: f64,
    ) -> Result<()> {
        let message = Message::SetTrialIntermediateValue {
            trial_id,
            step,
            value,
        };
        let command = Command::Broadcast { message };
        let _ = self.command_tx.send(command);
        Ok(())
    }

    pub fn set_trial_param(
        &self,
        trial_id: TrialId,
        key: String,
        value: TrialParamValue,
    ) -> Result<()> {
        let message = Message::SetTrialParamValue {
            trial_id,
            key,
            value,
        };
        let command = Command::Broadcast { message };
        let _ = self.command_tx.send(command);
        Ok(())
    }

    pub fn set_trial_user_attr(
        &self,
        trial_id: TrialId,
        key: String,
        value: JsonValue,
    ) -> Result<()> {
        let message = Message::SetTrialUserAttr {
            trial_id,
            key,
            value,
        };
        let command = Command::Broadcast { message };
        let _ = self.command_tx.send(command);
        Ok(())
    }

    pub fn set_trial_system_attr(
        &self,
        trial_id: TrialId,
        key: String,
        value: JsonValue,
    ) -> impl Future<Item = (), Error = Error> {
        let (reply_tx, reply_rx) = oneshot::monitor();
        let command = Command::SetTrialSystemAttr {
            trial_id,
            key,
            value,
            reply_tx,
        };
        let _ = self.command_tx.send(command);
        track_err!(reply_rx.map_err(Error::from))
    }
}

#[derive(Debug)]
pub struct StudyListNode {
    logger: Logger,
    study_id_prefix: StudyIdPrefix,
    plumcast_node: PlumcastNode<Message>,
    seqno: u32,
    studies: Studies,
    trials: HashMap<TrialId, Trial>,
    pendings: HashMap<TrialId, Vec<(MessageId, Message)>>,
    deliverables: Vec<(MessageId, Message)>,
    next_trial_id: Arc<AtomicUsize>,
    command_tx: mpsc::Sender<Command>,
    command_rx: mpsc::Receiver<Command>,
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

        let (command_tx, command_rx) = mpsc::channel();
        Self {
            logger,
            study_id_prefix,
            plumcast_node,
            seqno: rand::random(),
            studies: Studies::default(),
            trials: HashMap::new(),
            pendings: HashMap::new(),
            deliverables: Vec::new(),
            next_trial_id: Default::default(),
            command_tx,
            command_rx,
        }
    }

    pub fn handle(&self) -> StudyListNodeHandle {
        StudyListNodeHandle {
            command_tx: self.command_tx.clone(),
            studies: self.studies.handle(),
            study_id_prefix: self.study_id_prefix,
            next_trial_id: Arc::clone(&self.next_trial_id),
        }
    }

    fn id(&self) -> NodeId {
        self.plumcast_node.id()
    }

    fn next_study_id(&mut self) -> StudyId {
        let study_id = StudyId(u64::from(self.study_id_prefix.0) << 32 | u64::from(self.seqno));
        self.seqno += 1;
        study_id
    }

    fn handle_command(&mut self, cmd: Command) {
        match cmd {
            Command::GetTrials { study_id, reply_tx } => {
                if self.studies.get_by_id(study_id).is_none() {
                    reply_tx.exit(Err(track!(Error::new(format!(
                        "No such study: {:?}",
                        study_id
                    )))));
                    return;
                }
                let trials = self
                    .trials
                    .values()
                    .filter(|t| t.study_id == study_id)
                    .cloned()
                    .collect();
                reply_tx.exit(Ok(trials));
            }
            Command::GetTrial { trial_id, reply_tx } => {
                if let Some(t) = self.trials.get(&trial_id).cloned() {
                    reply_tx.exit(Ok(t));
                } else {
                    reply_tx.exit(Err(Error::new(format!("No such trial: {:?}", trial_id))));
                }
            }
            Command::SetTrialSystemAttr {
                trial_id,
                key,
                value,
                reply_tx,
            } => {
                if let Some(_) = self.trials.get_mut(&trial_id) {
                    let m = Message::SetTrialSystemAttr {
                        trial_id,
                        key,
                        value,
                    };
                    self.plumcast_node.broadcast(m);
                    reply_tx.exit(Ok(()));
                } else {
                    reply_tx.exit(Err(Error::new(format!("No such trial: {:?}", trial_id))));
                }
            }
            Command::CreateStudy { name, reply_tx } => {
                if self.studies.contains(&name) {
                    warn!(self.logger, "Duplicate study {:?}", name);
                    reply_tx.exit(Ok(None));
                    return;
                }

                let study_id = self.next_study_id();
                info!(
                    self.logger,
                    "Creates new study {:?}: id={:?}", name, study_id
                );
                let m = Message::CreateStudy {
                    study_name: name.clone(),
                    study_id,
                };
                let mid = self.plumcast_node.broadcast(m);
                self.studies.insert(name, study_id, mid);
                reply_tx.exit(Ok(Some(study_id)));
            }
            Command::SetStudyDirection { id, direction } => {
                let m = Message::SetStudyDirection {
                    study_id: id,
                    direction,
                };
                self.plumcast_node.broadcast(m);
            }
            Command::CreateTrial { study_id, trial_id } => {
                let m = Message::CreateTrial { study_id, trial_id };
                self.plumcast_node.broadcast(m);
            }
            Command::Broadcast { message } => {
                self.plumcast_node.broadcast(message);
            }
        }
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
            Message::CreateStudy {
                study_name,
                study_id,
            } => {
                if mid.node() == self.id() {
                    return;
                }

                info!(
                    self.logger,
                    "New study created: name={:?}, id={:?}", study_name, study_id
                );

                if let Some(s) = self.studies.get_by_id(study_id) {
                    warn!(
                        self.logger,
                        "Study ID conflicts: name={:?}, id={:?}", s.name, study_id
                    );
                    if s.create_mid < mid {
                        return;
                    } else {
                        self.studies.remove_by_id(study_id);
                    }
                }
                if let Some(s) = self.studies.get(&study_name) {
                    warn!(
                        self.logger,
                        "Study ID conflicts: name={:?}, id={:?}", s.name, study_id
                    );
                    if s.create_mid < mid {
                        return;
                    } else {
                        self.studies.remove_by_id(study_id);
                    }
                }
                self.studies.insert(study_name, study_id, mid);
            }
            Message::JoinStudy { study_id, node } => {
                if mid.node() == self.id() {
                    return;
                }

                info!(
                    self.logger,
                    "New study node joined: id={:?}, node={:?}", study_id, node
                );
                // TODO
                // if let Some(local) = self.studies.get_local_node(study_id) {
                //     local.invite(node);
                // }
            }
            Message::SetStudyDirection {
                study_id,
                direction,
            } => {
                debug!(
                    self.logger,
                    "Set study direction: id={:?}, direction={:?}", study_id, direction
                );
                self.studies
                    .update_study(study_id, |s| s.direction = direction);
            }
            Message::CreateTrial { study_id, trial_id } => {
                debug!(
                    self.logger,
                    "New trial was created: study={:?}, trial={:?}", study_id, trial_id
                );
                assert!(!self.trials.contains_key(&trial_id));
                self.trials.insert(trial_id, Trial::new(trial_id, study_id));
                if let Some(messages) = self.pendings.remove(&trial_id) {
                    self.deliverables.extend(messages);
                }
            }
            Message::SetTrialState { trial_id, state } => {
                if let Some(t) = self.trials.get_mut(&trial_id) {
                    debug!(
                        self.logger,
                        "Set trial state: trial={:?}, state={:?}", trial_id, state
                    );
                    t.set_state(state);
                } else {
                    debug!(self.logger, "No such trial: {:?}", trial_id);
                    self.pendings.entry(trial_id).or_default().push((mid, m));
                }
            }
            Message::SetTrialValue { trial_id, value } => {
                if let Some(t) = self.trials.get_mut(&trial_id) {
                    t.value = Some(value);
                } else {
                    debug!(self.logger, "No such trial: {:?}", trial_id);
                    self.pendings.entry(trial_id).or_default().push((mid, m));
                }
            }
            Message::SetTrialIntermediateValue {
                trial_id,
                step,
                value,
            } => {
                if let Some(t) = self.trials.get_mut(&trial_id) {
                    t.intermediate_values.insert(step, value);
                } else {
                    debug!(self.logger, "No such trial: {:?}", trial_id);
                    self.pendings.entry(trial_id).or_default().push((mid, m));
                }
            }
            Message::SetTrialParamValue {
                trial_id,
                key,
                value,
            } => {
                if let Some(t) = self.trials.get_mut(&trial_id) {
                    t.params.insert(key, value);
                } else {
                    debug!(self.logger, "No such trial: {:?}", trial_id);
                    let m = Message::SetTrialParamValue {
                        trial_id,
                        key,
                        value,
                    };
                    self.pendings.entry(trial_id).or_default().push((mid, m));
                }
            }
            Message::SetTrialSystemAttr {
                trial_id,
                key,
                value,
            } => {
                if let Some(t) = self.trials.get_mut(&trial_id) {
                    t.system_attrs.insert(key, value);
                } else {
                    debug!(self.logger, "No such trial: {:?}", trial_id);
                    let m = Message::SetTrialSystemAttr {
                        trial_id,
                        key,
                        value,
                    };
                    self.pendings.entry(trial_id).or_default().push((mid, m));
                }
            }
            Message::SetTrialUserAttr {
                trial_id,
                key,
                value,
            } => {
                if let Some(t) = self.trials.get_mut(&trial_id) {
                    t.user_attrs.insert(key, value);
                } else {
                    debug!(self.logger, "No such trial: {:?}", trial_id);
                    let m = Message::SetTrialUserAttr {
                        trial_id,
                        key,
                        value,
                    };
                    self.pendings.entry(trial_id).or_default().push((mid, m));
                }
            }
        }
    }
}
impl Future for StudyListNode {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut did_something = true;
        while did_something {
            did_something = false;

            while let Async::Ready(Some(m)) = track!(self.plumcast_node.poll())? {
                let mid = m.id().clone();
                self.handle_message(mid, m.into_payload());
                did_something = true;
            }
            while let Async::Ready(Some(c)) = self.command_rx.poll().expect("never fails") {
                self.handle_command(c);
                did_something = true;
            }
            for (mid, m) in std::mem::replace(&mut self.deliverables, Vec::new()) {
                self.handle_message(mid, m);
                did_something = true;
            }
        }
        Ok(Async::NotReady)
    }
}
