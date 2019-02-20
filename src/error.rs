use fibers;
use fibers_rpc;
use plumcast;
use std;
use trackable::error::TrackableError;
use trackable::error::{ErrorKind as TrackableErrorKind, ErrorKindExt};

#[derive(Debug, Clone, TrackableError, Serialize, Deserialize)]
pub struct Error(TrackableError<ErrorKind>);
impl Error {
    pub fn new(message: String) -> Self {
        ErrorKind::Other.cause(message).into()
    }
    pub fn already_exists() -> Self {
        ErrorKind::AlreadyExists.error().into()
    }
    pub fn not_found() -> Self {
        ErrorKind::NotFound.error().into()
    }
}
impl From<plumcast::Error> for Error {
    fn from(f: plumcast::Error) -> Self {
        ErrorKind::Other.takes_over(f).into()
    }
}
impl From<fibers_rpc::Error> for Error {
    fn from(f: fibers_rpc::Error) -> Self {
        ErrorKind::Other.takes_over(f).into()
    }
}
impl From<std::num::ParseIntError> for Error {
    fn from(f: std::num::ParseIntError) -> Self {
        ErrorKind::Other.cause(f).into()
    }
}
impl From<std::sync::mpsc::RecvError> for Error {
    fn from(f: std::sync::mpsc::RecvError) -> Self {
        ErrorKind::Other.cause(f).into()
    }
}
impl From<std::str::Utf8Error> for Error {
    fn from(f: std::str::Utf8Error) -> Self {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorKind {
    AlreadyExists,
    NotFound,
    Other,
}
impl TrackableErrorKind for ErrorKind {}
