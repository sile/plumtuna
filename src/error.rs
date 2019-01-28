use plumcast;
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

#[derive(Debug, Clone)]
pub enum ErrorKind {
    Other,
}
impl TrackableErrorKind for ErrorKind {}
