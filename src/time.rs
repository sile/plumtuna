use std::time::{Duration, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(Duration);
impl Timestamp {
    pub fn now() -> Self {
        Self(UNIX_EPOCH.elapsed().expect("never fails"))
    }

    pub fn to_seconds(&self) -> Seconds {
        let d = self.0;
        let s = (d.as_secs() as f64) + ((d.subsec_micros() as f64) / 1_000_000.0);
        Seconds(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Seconds(f64);
impl Seconds {
    pub fn now() -> Self {
        let d = UNIX_EPOCH.elapsed().expect("never fails");
        let s = (d.as_secs() as f64) + ((d.subsec_micros() as f64) / 1_000_000.0);
        Self(s)
    }
}
