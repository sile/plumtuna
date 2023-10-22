use crate::study::operation::{Operation, OperationKey};
use crate::study::subscriber::{SubscribeId, Subscriber};
use crate::study::{
    Message, Seconds, StudyDirection, StudyId, StudyName, StudyNameAndId, StudySummary,
};
use crate::time::Timestamp;
use crate::trial::{Trial, TrialId, TrialParamValue, TrialState};
use crate::{Error, PlumcastNode};
use fibers::sync::{mpsc, oneshot};
use futures::{Async, Future, Poll, Stream};
use plumcast::message::MessageId;
use plumcast::node::NodeId;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::time::Duration;

const TIMEOUT_SEC: u64 = 60 * 60; // TODO:

#[derive(Debug)]
pub struct StudyNode {
    study_name: StudyName,
    study_id: StudyId,
    direction: StudyDirection,
    user_attrs: HashMap<String, JsonValue>,
    system_attrs: HashMap<String, JsonValue>,
    trials: HashMap<TrialId, Trial>,
    datetime_start: Seconds,
    inner: PlumcastNode,
    command_tx: mpsc::Sender<Command>,
    command_rx: mpsc::Receiver<Command>,
    operations: HashMap<OperationKey, Operation>,
    expiry_time: Duration,
    next_subscribe_id: SubscribeId,
    subscribers: HashMap<SubscribeId, Subscriber>,
}
impl StudyNode {
    pub fn new(study: StudyNameAndId, inner: PlumcastNode) -> Self {
        let expiry_time = inner.clock().now().as_duration() + Duration::from_secs(TIMEOUT_SEC);
        let (command_tx, command_rx) = mpsc::channel();
        StudyNode {
            study_name: study.study_name,
            study_id: study.study_id,
            direction: StudyDirection::NotSet,
            user_attrs: HashMap::new(),
            system_attrs: HashMap::new(),
            trials: HashMap::new(),
            datetime_start: Seconds::now(),
            inner,
            command_tx,
            command_rx,
            operations: HashMap::new(),
            expiry_time,
            next_subscribe_id: SubscribeId::new(),
            subscribers: HashMap::new(),
        }
    }

    pub fn handle(&self) -> StudyNodeHandle {
        StudyNodeHandle {
            command_tx: self.command_tx.clone(),
            node_id: self.inner.id(),
        }
    }

    fn now(&self) -> Duration {
        self.inner.clock().now().as_duration()
    }

    fn handle_message(&mut self, mid: MessageId, message: Message) {
        if !self.check_message(mid, &message) {
            return;
        }

        for s in self.subscribers.values_mut() {
            s.push_message(message.clone());
        }

        match message {
            Message::SetStudyDirection { direction, .. } => {
                log::debug!("Set study direction: {:?}", direction);
                self.direction = direction;
            }
            Message::SetStudyUserAttr { key, value, .. } => {
                self.user_attrs.insert(key, value);
            }
            Message::SetStudySystemAttr { key, value, .. } => {
                self.system_attrs.insert(key, value);
            }
            Message::CreateTrial {
                trial_id,
                timestamp,
            } => {
                self.get_trial_mut(trial_id).datetime_start = Some(timestamp.to_seconds());
            }
            Message::SetTrialState {
                trial_id,
                state,
                timestamp,
            } => {
                self.get_trial_mut(trial_id).set_state(state, timestamp);
            }
            Message::SetTrialParam {
                trial_id,
                key,
                value,
                ..
            } => {
                self.get_trial_mut(trial_id).params.insert(key, value);
            }
            Message::SetTrialValue {
                trial_id, value, ..
            } => {
                self.get_trial_mut(trial_id).value = Some(value);
            }
            Message::SetTrialIntermediateValue {
                trial_id,
                step,
                value,
                ..
            } => {
                self.get_trial_mut(trial_id)
                    .intermediate_values
                    .insert(step, value);
            }
            Message::SetTrialUserAttr {
                trial_id,
                key,
                value,
                ..
            } => {
                self.get_trial_mut(trial_id).user_attrs.insert(key, value);
            }
            Message::SetTrialSystemAttr {
                trial_id,
                key,
                value,
                ..
            } => {
                self.get_trial_mut(trial_id).system_attrs.insert(key, value);
            }
        }
    }

    fn get_trial_mut(&mut self, trial_id: TrialId) -> &mut Trial {
        self.trials
            .entry(trial_id.clone())
            .or_insert_with(|| Trial::new(trial_id))
    }

    fn check_message(&mut self, mid: MessageId, message: &Message) -> bool {
        let key = OperationKey::from_message(message);
        let op = Operation::new(mid, message);
        if let Some(existing) = self.operations.remove(&key) {
            if existing < op {
                if existing.mid != op.mid {
                    self.inner.forget_message(&existing.mid);
                }
                self.operations.insert(key, op);
                true
            } else {
                if existing.mid != op.mid {
                    self.inner.forget_message(&op.mid);
                }
                self.operations.insert(key, existing);
                false
            }
        } else {
            self.operations.insert(key, op);
            true
        }
    }

    fn handle_command(&mut self, command: Command) {
        self.expiry_time =
            self.inner.clock().now().as_duration() + Duration::from_secs(TIMEOUT_SEC);
        match command {
            Command::GetSummary { reply_tx } => {
                // TODO: regard direction
                let best_trial = self
                    .trials
                    .values()
                    .filter(|t| t.is_complete())
                    .filter(|t| t.value.map_or(false, |v| !v.is_nan()))
                    .min_by(|a, b| a.value.partial_cmp(&b.value).expect("never fails"))
                    .cloned();
                let summary = StudySummary {
                    study_id: self.study_id.clone(),
                    study_name: self.study_name.clone(),
                    direction: self.direction,
                    user_attrs: self.user_attrs.clone(),
                    system_attrs: self.system_attrs.clone(),
                    best_trial,
                    n_trials: self.trials.len() as u32,
                    datetime_start: self.datetime_start,
                };
                reply_tx.exit(Ok(summary));
            }
            Command::GetTrial { trial_id, reply_tx } => {
                if let Some(trial) = self.trials.get(&trial_id).and_then(|t| t.adjust()) {
                    reply_tx.exit(Ok(trial));
                } else {
                    reply_tx.exit(Err(track!(Error::not_found())))
                }
            }
            Command::GetTrials { reply_tx } => {
                let trials = self.trials.values().filter_map(|t| t.adjust()).collect();
                reply_tx.exit(Ok(trials));
            }
            Command::Subscribe { reply_tx } => {
                let subscribe_id = self.next_subscribe_id.next();
                let mut s = Subscriber::new(self.now());
                for m in self.inner.plumtree_node().messages() {
                    s.push_message(m.1.clone().into_study_message().expect("never fails"));
                }
                self.subscribers.insert(subscribe_id, s);
                reply_tx.exit(Ok(subscribe_id));
            }
            Command::PollEvents {
                subscribe_id,
                reply_tx,
            } => {
                let now = self.now();
                match self.subscribers.get_mut(&subscribe_id) {
                    None => {
                        reply_tx.exit(Err(track!(Error::not_found())));
                    }
                    Some(s) => {
                        s.heartbeat(now);
                        let messages = s.pop_messages();
                        reply_tx.exit(Ok(messages));
                    }
                }
            }
            Command::Broadcast { message } => {
                self.inner.broadcast(message.into());
            }
        }
    }
}
impl Future for StudyNode {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut did_something = true;
        while did_something {
            did_something = false;

            while let Async::Ready(Some(message)) = track!(self.inner.poll())? {
                did_something = true;
                let id = message.id().clone();
                let payload = track!(message.into_payload().into_study_message())?;
                self.handle_message(id, payload);
            }
            while let Async::Ready(Some(command)) = self.command_rx.poll().expect("never fails") {
                did_something = true;
                self.handle_command(command);
            }

            if self.expiry_time < self.inner.clock().now().as_duration() {
                log::info!("Study timeout");
                return Ok(Async::Ready(()));
            }

            if !self.subscribers.is_empty() {
                let mut expired = Vec::new();
                for (k, v) in self.subscribers.iter() {
                    if v.has_expired(self.now()) {
                        log::info!("Subscriber {:?} has expired", k);
                        expired.push(*k);
                    }
                }
                for k in expired {
                    self.subscribers.remove(&k);
                }
            }
        }
        Ok(Async::NotReady)
    }
}

#[derive(Debug, Clone)]
pub struct StudyNodeHandle {
    command_tx: mpsc::Sender<Command>,
    node_id: NodeId,
}
impl StudyNodeHandle {
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub fn get_summary(&self) -> impl Future<Item = StudySummary, Error = Error> {
        let (reply_tx, reply_rx) = oneshot::monitor();
        let command = Command::GetSummary { reply_tx };
        let _ = self.command_tx.send(command);
        track_err!(reply_rx.map_err(Error::from))
    }

    pub fn get_trial(&self, trial_id: TrialId) -> impl Future<Item = Trial, Error = Error> {
        let (reply_tx, reply_rx) = oneshot::monitor();
        let command = Command::GetTrial { trial_id, reply_tx };
        let _ = self.command_tx.send(command);
        track_err!(reply_rx.map_err(Error::from))
    }

    pub fn get_trials(&self) -> impl Future<Item = Vec<Trial>, Error = Error> {
        let (reply_tx, reply_rx) = oneshot::monitor();
        let command = Command::GetTrials { reply_tx };
        let _ = self.command_tx.send(command);
        track_err!(reply_rx.map_err(Error::from))
    }

    pub fn set_study_direction(&self, direction: StudyDirection) {
        let message = Message::SetStudyDirection {
            direction,
            timestamp: Timestamp::now(),
        };
        let command = Command::Broadcast { message };
        let _ = self.command_tx.send(command);
    }

    pub fn set_study_user_attr(&self, key: String, value: JsonValue) {
        let message = Message::SetStudyUserAttr {
            key,
            value,
            timestamp: Timestamp::now(),
        };
        let command = Command::Broadcast { message };
        let _ = self.command_tx.send(command);
    }

    pub fn set_study_system_attr(&self, key: String, value: JsonValue) {
        let message = Message::SetStudySystemAttr {
            key,
            value,
            timestamp: Timestamp::now(),
        };
        let command = Command::Broadcast { message };
        let _ = self.command_tx.send(command);
    }

    pub fn create_trial(&self, trial_id: TrialId) {
        let message = Message::CreateTrial {
            trial_id,
            timestamp: Timestamp::now(),
        };
        let command = Command::Broadcast { message };
        let _ = self.command_tx.send(command);
    }

    pub fn set_trial_state(&self, trial_id: TrialId, state: TrialState) {
        let message = Message::SetTrialState {
            trial_id,
            state,
            timestamp: Timestamp::now(),
        };
        let command = Command::Broadcast { message };
        let _ = self.command_tx.send(command);
    }

    pub fn set_trial_param(&self, trial_id: TrialId, key: String, value: TrialParamValue) {
        let message = Message::SetTrialParam {
            trial_id,
            key,
            value,
            timestamp: Timestamp::now(),
        };
        let command = Command::Broadcast { message };
        let _ = self.command_tx.send(command);
    }

    pub fn set_trial_value(&self, trial_id: TrialId, value: f64) {
        let message = Message::SetTrialValue {
            trial_id,
            value,
            timestamp: Timestamp::now(),
        };
        let command = Command::Broadcast { message };
        let _ = self.command_tx.send(command);
    }

    pub fn set_trial_intermediate_value(&self, trial_id: TrialId, step: u32, value: f64) {
        let message = Message::SetTrialIntermediateValue {
            trial_id,
            step,
            value,
            timestamp: Timestamp::now(),
        };
        let command = Command::Broadcast { message };
        let _ = self.command_tx.send(command);
    }

    pub fn set_trial_user_attr(&self, trial_id: TrialId, key: String, value: JsonValue) {
        let message = Message::SetTrialUserAttr {
            trial_id,
            key,
            value,
            timestamp: Timestamp::now(),
        };
        let command = Command::Broadcast { message };
        let _ = self.command_tx.send(command);
    }

    pub fn set_trial_system_attr(&self, trial_id: TrialId, key: String, value: JsonValue) {
        let message = Message::SetTrialSystemAttr {
            trial_id,
            key,
            value,
            timestamp: Timestamp::now(),
        };
        let command = Command::Broadcast { message };
        let _ = self.command_tx.send(command);
    }

    pub fn subscribe(&self) -> impl Future<Item = SubscribeId, Error = Error> {
        let (reply_tx, reply_rx) = oneshot::monitor();
        let command = Command::Subscribe { reply_tx };
        let _ = self.command_tx.send(command);
        track_err!(reply_rx.map_err(Error::from))
    }

    pub fn poll_events(
        &self,
        subscribe_id: SubscribeId,
    ) -> impl Future<Item = Vec<Message>, Error = Error> {
        let (reply_tx, reply_rx) = oneshot::monitor();
        let command = Command::PollEvents {
            subscribe_id,
            reply_tx,
        };
        let _ = self.command_tx.send(command);
        track_err!(reply_rx.map_err(Error::from))
    }
}

// TODO: Heartbeat

#[derive(Debug)]
enum Command {
    GetSummary {
        reply_tx: oneshot::Monitored<StudySummary, Error>,
    },
    GetTrial {
        trial_id: TrialId,
        reply_tx: oneshot::Monitored<Trial, Error>,
    },
    GetTrials {
        reply_tx: oneshot::Monitored<Vec<Trial>, Error>,
    },
    Subscribe {
        reply_tx: oneshot::Monitored<SubscribeId, Error>,
    },
    PollEvents {
        subscribe_id: SubscribeId,
        reply_tx: oneshot::Monitored<Vec<Message>, Error>,
    },
    Broadcast {
        message: Message,
    },
}
