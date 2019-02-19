#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate trackable;

pub use self::error::{Error, ErrorKind};

pub mod contact;
pub mod global;
pub mod http;
pub mod study;
pub mod study_list;
pub mod time;
pub mod trial;

mod error;
mod message;

pub type Result<T> = std::result::Result<T, Error>;

type PlumcastNode = plumcast::node::Node<message::UnionMessage>;
