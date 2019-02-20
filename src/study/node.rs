use crate::study::operation::{Operation, OperationKey};
use crate::study::{
    Message, Seconds, StudyDirection, StudyId, StudyName, StudyNameAndId, StudySummary,
};
use crate::time::Timestamp;
use crate::trial::{Trial, TrialId, TrialParamValue, TrialState};
use crate::{Error, PlumcastNode};
use fibers::sync::{mpsc, oneshot};
use futures::{Async, Future, Poll, Stream};
use plumcast::message::MessageId;
use serde_json::Value as JsonValue;
use slog::Logger;
use std::collections::HashMap;
use std::time::Duration;

const TIMEOUT_SEC: u64 = 60 * 60; // TODO:

#[derive(Debug)]
pub struct StudyNode {
    logger: Logger,
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
}
impl StudyNode {
    pub fn new(logger: Logger, study: StudyNameAndId, inner: PlumcastNode) -> Self {
        let logger = logger.new(o!("study" => study.study_name.as_str().to_owned()));
        let expiry_time = inner.clock().now().as_duration() + Duration::from_secs(TIMEOUT_SEC);
        let (command_tx, command_rx) = mpsc::channel();
        StudyNode {
            logger,
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
        }
    }

    pub fn handle(&self) -> StudyNodeHandle {
        StudyNodeHandle {
            command_tx: self.command_tx.clone(),
        }
    }

    fn handle_message(&mut self, mid: MessageId, message: Message) {
        if !self.check_message(mid, &message) {
            return;
        }

        match message {
            Message::SetStudyDirection { direction, .. } => {
                debug!(self.logger, "Set study direction: {:?}", direction);
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
        self.create_trial_if_absent(&trial_id);
        self.trials.get_mut(&trial_id).expect("never fails")
    }

    fn create_trial_if_absent(&mut self, trial_id: &TrialId) {
        if self.trials.contains_key(trial_id) {
            self.trials
                .insert(trial_id.clone(), Trial::new(trial_id.clone()));
        }
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
                false
            } else {
                if existing.mid != op.mid {
                    self.inner.forget_message(&op.mid);
                }
                self.operations.insert(key, existing);
                true
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
                info!(self.logger, "Study timeout");
                return Ok(Async::Ready(()));
            }
        }
        Ok(Async::NotReady)
    }
}

#[derive(Debug, Clone)]
pub struct StudyNodeHandle {
    command_tx: mpsc::Sender<Command>,
}
impl StudyNodeHandle {
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
    Broadcast {
        message: Message,
    },
}
