use crate::study_list::StudyId;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TrialId(u64);
impl TrialId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TrialState {
    Running,
    Complete,
    Pruned,
    Fail,
}

#[derive(Debug, Clone)]
pub struct Trial {
    pub id: TrialId,
    pub state: TrialState,
    pub study_id: StudyId,
    pub system_attrs: HashMap<String, JsonValue>,
}
impl Trial {
    pub fn new(id: TrialId, study_id: StudyId) -> Self {
        Self {
            id,
            state: TrialState::Running,
            study_id,
            system_attrs: HashMap::new(),
        }
    }
}
