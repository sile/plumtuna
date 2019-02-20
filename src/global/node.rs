use crate::global::rpc;
use crate::global::Message;
use crate::study::{StudyId, StudyName, StudyNameAndId};
use crate::{Error, PlumcastNode, Result};
use fibers::sync::{mpsc, oneshot};
use fibers::time::timer::{self, Timeout};
use fibers_rpc::client::ClientServiceHandle as RpcClientServiceHandle;
use fibers_rpc::server::ServerBuilder as RpcServerBuilder;
use fibers_rpc::Cast;
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
    reply_tx: oneshot::Monitored<(), Error>,
    waitings: Vec<MessageId>,
}

#[derive(Debug)]
struct Joining {
    study_name: StudyName,
    timeout: Timeout,
    reply_tx: oneshot::Monitored<StudyId, Error>,
}

#[derive(Debug)]
struct LocalStudy {}

#[derive(Debug)]
pub struct GlobalNodeBuilder {
    command_tx: mpsc::Sender<Command>,
    command_rx: mpsc::Receiver<Command>,
}
impl GlobalNodeBuilder {
    pub fn new(rpc: &mut RpcServerBuilder) -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let handle = GlobalNodeHandle {
            command_tx: command_tx.clone(),
        };
        rpc.add_cast_handler(rpc::RpcHandler::new(handle));
        Self {
            command_tx,
            command_rx,
        }
    }

    pub fn finish(
        self,
        logger: Logger,
        inner: PlumcastNode,
        rpc: RpcClientServiceHandle,
    ) -> GlobalNode {
        info!(logger, "Starts global node: {:?}", inner.id());
        GlobalNode {
            logger,
            inner,
            command_tx: self.command_tx,
            command_rx: self.command_rx,
            creatings: HashMap::new(),
            joinings: Vec::new(),
            study_names: HashMap::new(),
            studies: HashMap::new(),
            forget_queue: VecDeque::new(),
            rpc,
        }
    }
}

#[derive(Debug)]
pub struct GlobalNode {
    logger: Logger,
    inner: PlumcastNode,
    command_tx: mpsc::Sender<Command>,
    command_rx: mpsc::Receiver<Command>,
    creatings: HashMap<StudyName, Creating>,
    joinings: Vec<Joining>,
    study_names: HashMap<StudyName, StudyId>,
    studies: HashMap<StudyId, LocalStudy>,
    forget_queue: VecDeque<(Duration, MessageId)>,
    rpc: RpcClientServiceHandle,
}
impl GlobalNode {
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
            Message::CreateStudy { name, id } => {
                if let Some(c) = self.creatings.remove(&name) {
                    if c.study_id == id {
                        self.creatings.insert(name.clone(), c);
                    } else if c.study_id < id {
                        self.notify_study(mid, name.clone(), c.study_id.clone(), false);
                        self.creatings.insert(name.clone(), c);
                    } else {
                        warn!(
                            self.logger,
                            "Study {:?} is superseded by the other same name study", name
                        );
                        c.reply_tx.exit(Err(track!(Error::already_exists())));
                    }
                }
                if let Some(self_id) = self.study_names.get(&name).cloned() {
                    if self_id != id {
                        warn!(
                            self.logger,
                            "Conflicted study {:?}: self={:?}, peer={:?}", name, self_id, id
                        );
                        self.notify_study(mid, name.clone(), self_id.clone(), true);
                    }
                } else {
                    assert!(!self.studies.contains_key(&id), "Study ID conflicts");
                }
            }
            Message::JoinStudy { name } => {
                if let Some(id) = self.study_names.get(&name).cloned() {
                    self.notify_study(mid, name.clone(), id.clone(), true);
                } else if let Some(c) = self.creatings.get_mut(&name) {
                    info!(self.logger, "Add to waitings: {:?}, {:?}", name, mid);
                    c.waitings.push(mid);
                }
            }
        }
        Ok(())
    }

    fn notify_study(&self, to: MessageId, study_name: StudyName, study_id: StudyId, created: bool) {
        let mut client = rpc::StudyCast::client(&self.rpc);
        client.options_mut().force_wakeup = true;
        let study = StudyNameAndId {
            study_name,
            study_id,
        };
        let _ = client.cast(to.node().address(), (study, created));
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

                let m = Message::CreateStudy {
                    name: name.clone(),
                    id: id.clone(),
                };
                self.inner.broadcast(m.into());
                let creating = Creating {
                    study_id: id,
                    timeout: timer::timeout(wait_time),
                    reply_tx,
                    waitings: Vec::new(),
                };
                self.creatings.insert(name, creating);
            }
            Command::JoinStudy {
                name,
                wait_time,
                reply_tx,
            } => {
                if let Some(id) = self.study_names.get(&name).cloned() {
                    reply_tx.exit(Ok(id));
                    return;
                }

                info!(self.logger, "Starts finding the study: {:?}", name);
                let m = Message::JoinStudy { name: name.clone() };
                self.inner.broadcast(m.into());
                let joining = Joining {
                    study_name: name,
                    timeout: timer::timeout(wait_time),
                    reply_tx,
                };
                self.joinings.push(joining);
            }
            Command::NotifyStudy { study, created } => {
                if let Some(c) = self.creatings.remove(&study.study_name) {
                    info!(self.logger, "Study already exists: {:?}", study);
                    c.reply_tx.exit(Err(track!(Error::already_exists())));
                }
                if created {
                    let mut i = 0;
                    while i < self.joinings.len() {
                        if self.joinings[i].study_name == study.study_name {
                            let j = self.joinings.swap_remove(i);
                            if !self.study_names.contains_key(&j.study_name) {
                                self.spawn_study_node(study.clone());
                            }
                            j.reply_tx.exit(Ok(study.study_id.clone()));
                        } else {
                            i += 1;
                        }
                    }
                }
            }
        }
    }

    fn spawn_study_node(&mut self, study: StudyNameAndId) {
        panic!()
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

            let c = self.creatings.remove(&name).expect("never fails");
            self.spawn_study_node(StudyNameAndId {
                study_name: name.clone(),
                study_id: c.study_id.clone(),
            });
            for w in c.waitings {
                self.notify_study(w, name.clone(), c.study_id.clone(), true);
            }

            let study = LocalStudy {};
            self.study_names.insert(name, c.study_id.clone());
            if self.studies.insert(c.study_id, study).is_some() {
                panic!("ID conflicts");
            }
            c.reply_tx.exit(Ok(()));
        }
        Ok(did_something)
    }

    fn handle_joinings(&mut self) -> Result<bool> {
        let mut did_something = false;
        let mut i = 0;
        while i < self.joinings.len() {
            if track!(self.joinings[i].timeout.poll().map_err(Error::from))?.is_ready() {
                did_something = true;
                let joining = self.joinings.swap_remove(i);
                joining.reply_tx.exit(Err(track!(Error::not_found())));
            } else {
                i += 1;
            }
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
            if !self.joinings.is_empty() {
                if track!(self.handle_joinings())? {
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

    pub fn join_study(
        &self,
        name: StudyName,
        wait_time: Duration,
    ) -> impl Future<Item = StudyId, Error = Error> {
        let (reply_tx, reply_rx) = oneshot::monitor();
        let command = Command::JoinStudy {
            name,
            wait_time,
            reply_tx,
        };
        let _ = self.command_tx.send(command);
        track_err!(reply_rx.map_err(Error::from))
    }

    pub fn notify_study(&self, study: StudyNameAndId, created: bool) {
        let command = Command::NotifyStudy { study, created };
        let _ = self.command_tx.send(command);
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
    JoinStudy {
        name: StudyName,
        wait_time: Duration,
        reply_tx: oneshot::Monitored<StudyId, Error>,
    },
    NotifyStudy {
        study: StudyNameAndId,
        created: bool,
    },
}
