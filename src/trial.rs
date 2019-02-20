use crate::study_list::StudyId;
use crate::time::Timestamp;
use crate::{Error, ErrorKind, Result};
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

// TODO: improve
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TrialId2(String);
impl TrialId2 {
    pub fn new(study_id: &crate::study::StudyId) -> Self {
        TrialId2(format!("{}.{}", study_id.as_uuid(), Uuid::new_v4()))
    }

    pub fn get_study_id(&self) -> Result<crate::study::StudyId> {
        let s = track_assert_some!(self.0.split('.').nth(0), ErrorKind::Other);
        let uuid: Uuid = track!(s.parse().map_err(Error::from))?;
        Ok(uuid.into())
    }
}
impl From<String> for TrialId2 {
    fn from(f: String) -> Self {
        Self(f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trial2 {
    pub trial_id: TrialId2,
    state: TrialState,
    pub value: Option<f64>,
    pub intermediate_values: BTreeMap<u32, f64>,
    pub params: HashMap<String, TrialParamValue>,
    pub user_attrs: HashMap<String, JsonValue>,
    pub system_attrs: HashMap<String, JsonValue>,
    pub datetime_start: Option<Seconds>,
    pub datetime_end: Option<Seconds>,
}
impl Trial2 {
    pub fn new(trial_id: TrialId2) -> Self {
        Self {
            trial_id,
            state: TrialState::Running,
            value: None,
            intermediate_values: BTreeMap::new(),
            params: HashMap::new(),
            user_attrs: HashMap::new(),
            system_attrs: HashMap::new(),
            datetime_start: None,
            datetime_end: None,
        }
    }

    pub fn set_state(&mut self, state: TrialState, timestamp: Timestamp) {
        self.state = state;
        if state != TrialState::Running {
            self.datetime_end = Some(timestamp.to_seconds());
        }
    }
}

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

// TODO
pub type Seconds = crate::time::Seconds;
