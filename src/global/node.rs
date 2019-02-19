use crate::global::Message;
use crate::study::{StudyId, StudyName};
use crate::time::Timestamp;
use crate::{Error, PlumcastNode, Result};
use fibers::sync::{mpsc, oneshot};
use fibers::time::timer::{self, Timeout};
use futures::{Async, Future, Poll, Stream};
use plumcast::message::MessageId;
use slog::Logger;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::Duration;

#[derive(Debug)]
struct Creating {
    study_id: StudyId,
    timeout: Timeout,
    timestamp: Timestamp,
    reply_tx: oneshot::Monitored<(), Error>,
}

#[derive(Debug)]
struct LocalStudy {
    created_at: Timestamp,
}

#[derive(Debug)]
pub struct GlobalNode {
    logger: Logger,
    inner: PlumcastNode,
    command_tx: mpsc::Sender<Command>,
    command_rx: mpsc::Receiver<Command>,
    creatings: HashMap<StudyName, Creating>,
    study_names: HashMap<StudyName, StudyId>,
    studies: HashMap<StudyId, LocalStudy>,
    forget_queue: VecDeque<(Duration, MessageId)>,
}
impl GlobalNode {
    pub fn new(logger: Logger, inner: PlumcastNode) -> Self {
        info!(logger, "Starts global node: {:?}", inner.id());
        let (command_tx, command_rx) = mpsc::channel();
        Self {
            logger,
            inner,
            command_tx,
            command_rx,
            creatings: HashMap::new(),
            study_names: HashMap::new(),
            studies: HashMap::new(),
            forget_queue: VecDeque::new(),
        }
    }

    pub fn handle(&self) -> GlobalNodeHandle {
        GlobalNodeHandle {
            command_tx: self.command_tx.clone(),
        }
    }

    fn forget_time(&self) -> Duration {
        // TODO: parameterize
        self.inner.clock().now().as_duration() + Duration::from_secs(60)
    }

    fn handle_message(&mut self, mid: MessageId, m: Message) -> Result<()> {
        self.forget_queue.push_back((self.forget_time(), mid));
        match m {
            Message::CreateStudy {
                name,
                id,
                timestamp,
            } => {
                if let Some(c) = self.creatings.remove(&name) {
                    if c.study_id == id {
                        self.creatings.insert(name.clone(), c);
                    } else if c.timestamp < timestamp {
                        self.creatings.insert(name.clone(), c);
                        panic!() // TODO: reply (RPC)
                    } else {
                        assert_ne!(c.timestamp, timestamp); // TODO
                        warn!(
                            self.logger,
                            "Study {:?} is superseded by the other same name study", name
                        );
                        c.reply_tx.exit(Err(track!(Error::already_exists())));
                    }
                } else if let Some(self_id) = self.study_names.get(&name).cloned() {
                    if self_id != id {
                        warn!(
                            self.logger,
                            "Conflicted study {:?}: self={:?}, peer={:?}", name, self_id, id
                        );
                        // TODO: reply (RPC)
                        panic!()
                    }
                } else {
                    assert!(!self.studies.contains_key(&id), "Study ID conflicts");
                }
            }
            Message::OpenStudy { .. } => panic!(),
            Message::OpenStudyById { .. } => panic!(),
        }
        Ok(())
    }

    fn handle_command(&mut self, command: Command) {
        match command {
            Command::CreateStudy {
                name,
                id,
                wait_time,
                reply_tx,
            } => {
                info!(self.logger, "Try creating new study: {:?}, {:?}", name, id);
                if self.creatings.contains_key(&name) {
                    warn!(self.logger, "Study {:?} is already creating", name);
                    reply_tx.exit(Err(track!(Error::already_exists())));
                    return;
                }

                let timestamp = Timestamp::new();
                let m = Message::CreateStudy {
                    name: name.clone(),
                    id: id.clone(),
                    timestamp,
                };
                self.inner.broadcast(m.into());
                let creating = Creating {
                    study_id: id,
                    timeout: timer::timeout(wait_time),
                    timestamp,
                    reply_tx,
                };
                self.creatings.insert(name, creating);
            }
        }
    }

    fn handle_creatings(&mut self) -> Result<bool> {
        let mut did_something = false;
        let mut timeouts = Vec::new();
        for (name, c) in self.creatings.iter_mut() {
            if track!(c.timeout.poll().map_err(Error::from))?.is_ready() {
                did_something = true;
                timeouts.push(name.clone());
            }
        }
        for name in timeouts {
            info!(self.logger, "New study is created: {:?}", name);

            // TODO: spawn study node
            let c = self.creatings.remove(&name).expect("never fails");
            let study = LocalStudy {
                created_at: c.timestamp,
            };
            self.study_names.insert(name, c.study_id.clone());
            self.studies
                .insert(c.study_id, study)
                .expect("ID conflicts");
        }
        Ok(did_something)
    }

    fn handle_forget(&mut self) {
        while self
            .forget_queue
            .front()
            .map_or(false, |x| x.0 < self.inner.clock().now().as_duration())
        {
            let mid = self.forget_queue.pop_front().expect("never fails").1;
            self.inner.forget_message(&mid);
        }
    }
}
impl Future for GlobalNode {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut did_something = true;
        while did_something {
            did_something = false;

            while let Async::Ready(Some(message)) = track!(self.inner.poll())? {
                did_something = true;
                let id = message.id().clone();
                let payload = track!(message.into_payload().into_global_message())?;
                track!(self.handle_message(id, payload))?;
            }
            while let Async::Ready(Some(command)) = self.command_rx.poll().expect("never fails") {
                did_something = true;
                self.handle_command(command);
            }
            if !self.creatings.is_empty() {
                if track!(self.handle_creatings())? {
                    did_something = true;
                }
            }
            self.handle_forget();
        }
        Ok(Async::NotReady)
    }
}

#[derive(Debug, Clone)]
pub struct GlobalNodeHandle {
    command_tx: mpsc::Sender<Command>,
}
impl GlobalNodeHandle {
    pub fn create_study(
        &self,
        name: StudyName,
        wait_time: Duration,
    ) -> impl Future<Item = StudyId, Error = Error> {
        let id = StudyId::new();
        let (reply_tx, reply_rx) = oneshot::monitor();
        let command = Command::CreateStudy {
            name,
            id: id.clone(),
            wait_time,
            reply_tx,
        };
        let _ = self.command_tx.send(command);
        track_err!(reply_rx.map(move |()| id).map_err(Error::from))
    }
}

#[derive(Debug)]
enum Command {
    CreateStudy {
        name: StudyName,
        id: StudyId,
        wait_time: Duration,
        reply_tx: oneshot::Monitored<(), Error>,
    },
}
