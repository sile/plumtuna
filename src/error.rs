use fibers;
use plumcast;
use std;
use trackable::error::TrackableError;
use trackable::error::{ErrorKind as TrackableErrorKind, ErrorKindExt};

#[derive(Debug, Clone, TrackableError)]
pub struct Error(TrackableError<ErrorKind>);
impl Error {
    pub fn new(message: String) -> Self {
        ErrorKind::Other.cause(message).into()
    }
}
impl From<plumcast::Error> for Error {
    fn from(f: plumcast::Error) -> Self {
        ErrorKind::Other.takes_over(f).into()
    }
}
impl From<std::num::ParseIntError> for Error {
    fn from(f: std::num::ParseIntError) -> Self {
        ErrorKind::Other.cause(f).into()
    }
}
impl From<fibers::sync::oneshot::MonitorError<Error>> for Error {
    fn from(f: fibers::sync::oneshot::MonitorError<Error>) -> Self {
        f.unwrap_or_else(|| {
            ErrorKind::Other
                .cause("Monitor channel disconnected")
                .into()
        })
    }
}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    Other,
}
impl TrackableErrorKind for ErrorKind {}
