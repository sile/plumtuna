use std::time::{Duration, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(Duration);
impl Timestamp {
    pub fn new() -> Self {
        Self(UNIX_EPOCH.elapsed().expect("never fails"))
    }
}
