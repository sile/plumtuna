use crate::study::{StudyId, StudyName, StudyNameAndId};
use crate::Error;
use crate::PlumcastNode;
use futures::{Async, Future, Poll, Stream};
use slog::Logger;

#[derive(Debug)]
pub struct StudyNode {
    logger: Logger,
    study_name: StudyName,
    study_id: StudyId,
    inner: PlumcastNode,
}
impl StudyNode {
    pub fn new(logger: Logger, study: StudyNameAndId, inner: PlumcastNode) -> Self {
        let logger = logger.new(o!("study" => study.study_name.as_str().to_owned()));
        StudyNode {
            logger,
            study_name: study.study_name,
            study_id: study.study_id,
            inner,
        }
    }

    pub fn handle(&self) -> StudyNodeHandle {
        StudyNodeHandle
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

            // TODO: expiration
        }
        Ok(Async::NotReady)
    }
}

#[derive(Debug, Clone)]
pub struct StudyNodeHandle;
