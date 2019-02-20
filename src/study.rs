use crate::time::Seconds;
use uuid::Uuid;

pub use self::node::{StudyNode, StudyNodeHandle};

mod node;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StudyName(String);
impl StudyName {
    pub fn new(name: String) -> Self {
        Self(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StudyId(Uuid);
impl StudyId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}
impl From<Uuid> for StudyId {
    fn from(f: Uuid) -> Self {
        Self(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum StudyDirection {
    NotSet,
    Minimize,
    Maximize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StudyNameAndId {
    pub study_name: StudyName,
    pub study_id: StudyId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StudySummary {
    pub study_id: StudyId,
    pub study_name: StudyName,
    pub direction: StudyDirection,
    // TODO: best_trial, user_attrs, system_attrs, n_trials,
    pub datetime_start: Seconds,
}
