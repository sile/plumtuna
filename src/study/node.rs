use crate::study::{Seconds, StudyDirection, StudyId, StudyName, StudyNameAndId, StudySummary};
use crate::Error;
use crate::PlumcastNode;
use fibers::sync::{mpsc, oneshot};
use futures::{Async, Future, Poll, Stream};
use slog::Logger;

#[derive(Debug)]
pub struct StudyNode {
    logger: Logger,
    study_name: StudyName,
    study_id: StudyId,
    direction: StudyDirection,
    datetime_start: Seconds,
    inner: PlumcastNode,
    command_tx: mpsc::Sender<Command>,
    command_rx: mpsc::Receiver<Command>,
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
            datetime_start: Seconds::now(),
            inner,
            command_tx,
            command_rx,
        }
    }

    pub fn handle(&self) -> StudyNodeHandle {
        StudyNodeHandle {
            command_tx: self.command_tx.clone(),
        }
    }

    fn handle_command(&mut self, command: Command) {
        match command {
            Command::GetSummary { reply_tx } => {
                let summary = StudySummary {
                    study_id: self.study_id.clone(),
                    study_name: self.study_name.clone(),
                    direction: self.direction,
                    datetime_start: self.datetime_start,
                };
                reply_tx.exit(Ok(summary));
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
                panic!()
                //let payload = track!(message.into_payload().into_study_message())?;
                //track!(self.handle_message(id, payload))?;
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
}

#[derive(Debug)]
enum Command {
    GetSummary {
        reply_tx: oneshot::Monitored<StudySummary, Error>,
    },
}
