use crate::study::operation::{Operation, OperationKey};
use crate::study::{
    Message, Seconds, StudyDirection, StudyId, StudyName, StudyNameAndId, StudySummary,
};
use crate::time::Timestamp;
use crate::{Error, PlumcastNode};
use fibers::sync::{mpsc, oneshot};
use futures::{Async, Future, Poll, Stream};
use plumcast::message::MessageId;
use serde_json::Value as JsonValue;
use slog::Logger;
use std::collections::HashMap;

#[derive(Debug)]
pub struct StudyNode {
    logger: Logger,
    study_name: StudyName,
    study_id: StudyId,
    direction: StudyDirection,
    user_attrs: HashMap<String, JsonValue>,
    system_attrs: HashMap<String, JsonValue>,
    datetime_start: Seconds,
    inner: PlumcastNode,
    command_tx: mpsc::Sender<Command>,
    command_rx: mpsc::Receiver<Command>,
    operations: HashMap<OperationKey, Operation>,
}
impl StudyNode {
    pub fn new(logger: Logger, study: StudyNameAndId, inner: PlumcastNode) -> Self {
        let logger = logger.new(o!("study" => study.study_name.as_str().to_owned()));
        let (command_tx, command_rx) = mpsc::channel();
        StudyNode {
            logger,
            study_name: study.study_name,
            study_id: study.study_id,
            direction: StudyDirection::NotSet,
            user_attrs: HashMap::new(),
            system_attrs: HashMap::new(),
            datetime_start: Seconds::now(),
            inner,
            command_tx,
            command_rx,
            operations: HashMap::new(),
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
        }
    }

    fn check_message(&mut self, mid: MessageId, message: &Message) -> bool {
        let key = OperationKey::from_message(message);
        let op = Operation::new(mid, message);
        if let Some(existing) = self.operations.remove(&key) {
            if existing < op {
                self.operations.insert(key, op);
                self.inner.forget_message(&existing.mid);
                false
            } else {
                self.operations.insert(key, existing);
                self.inner.forget_message(&op.mid);
                true
            }
        } else {
            self.operations.insert(key, op);
            true
        }
    }

    fn handle_command(&mut self, command: Command) {
        match command {
            Command::GetSummary { reply_tx } => {
                let summary = StudySummary {
                    study_id: self.study_id.clone(),
                    study_name: self.study_name.clone(),
                    direction: self.direction,
                    user_attrs: self.user_attrs.clone(),
                    system_attrs: self.system_attrs.clone(),
                    datetime_start: self.datetime_start,
                };
                reply_tx.exit(Ok(summary));
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

            // TODO: expiration
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
}

#[derive(Debug)]
enum Command {
    GetSummary {
        reply_tx: oneshot::Monitored<StudySummary, Error>,
    },
    Broadcast {
        message: Message,
    },
}
