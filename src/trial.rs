#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TrialId(u64);
impl TrialId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}
