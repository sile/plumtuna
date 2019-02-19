use crate::global::Message;
use crate::study::StudyServiceHandle;
use crate::{Error, PlumcastNode, Result};
use futures::{Async, Future, Poll, Stream};
use plumcast::message::MessageId;
use slog::Logger;

#[derive(Debug)]
pub struct GlobalNode {
    logger: Logger,
    inner: PlumcastNode,
}
impl GlobalNode {
    pub fn new(logger: Logger, inner: PlumcastNode) -> Self {
        info!(logger, "Starts global node: {:?}", inner.id());
        Self { logger, inner }
    }

    fn handle_message(&mut self, mid: MessageId, m: Message) -> Result<()> {
        match m {
            Message::CreateStudy { .. } => {}
            Message::OpenStudy { .. } => {}
            Message::OpenStudyById { .. } => {}
        }
        Ok(())
    }
}
impl Future for GlobalNode {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready(Some(message)) = track!(self.inner.poll())? {
            let id = message.id().clone();
            let payload = track!(message.into_payload().into_global_message())?;
            track!(self.handle_message(id, payload))?;
        }
        Ok(Async::NotReady)
    }
}
