use crate::study_list::StudyId;
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, HashMap};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TrialId(u64);
impl TrialId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrialParamValue {
    pub value: f64,
    pub distribution: JsonValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TrialState {
    Running,
    Complete,
    Pruned,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trial {
    pub id: TrialId,
    state: TrialState,
    pub study_id: StudyId,
    pub value: Option<f64>,
    pub intermediate_values: BTreeMap<u32, f64>,
    pub params: HashMap<String, TrialParamValue>,
    pub user_attrs: HashMap<String, JsonValue>,
    pub system_attrs: HashMap<String, JsonValue>,
    pub datetime_start: Seconds,
    pub datetime_end: Option<Seconds>,
}
impl Trial {
    pub fn new(id: TrialId, study_id: StudyId) -> Self {
        Self {
            id,
            state: TrialState::Running,
            study_id,
            value: None,
            intermediate_values: BTreeMap::new(),
            params: HashMap::new(),
            user_attrs: HashMap::new(),
            system_attrs: HashMap::new(),
            datetime_start: Seconds::now(),
            datetime_end: None,
        }
    }

    pub fn set_state(&mut self, state: TrialState) {
        self.state = state;
        if state != TrialState::Running {
            self.datetime_end = Some(Seconds::now());
        }
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
