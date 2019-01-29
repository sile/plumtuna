#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate trackable;

pub use self::error::{Error, ErrorKind};

pub mod http;
pub mod study_list;

mod error;

pub type Result<T> = std::result::Result<T, Error>;
