pub use self::client::ContactServiceClient;
pub use self::service::{ContactService, ContactServiceHandle};

mod client;
mod rpc;
mod service;
