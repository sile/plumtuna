use crate::time::{Seconds, Timestamp};
use crate::{Error, ErrorKind, Result};
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

// TODO: improve
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TrialId(String);
impl TrialId {
    pub fn new(study_id: &crate::study::StudyId) -> Self {
        TrialId(format!("{}.{}", study_id.as_uuid(), Uuid::new_v4()))
    }

    pub fn get_study_id(&self) -> Result<crate::study::StudyId> {
        let s = track_assert_some!(self.0.split('.').nth(0), ErrorKind::Other);
        let uuid: Uuid = track!(s.parse().map_err(Error::from))?;
        Ok(uuid.into())
    }
}
impl From<String> for TrialId {
    fn from(f: String) -> Self {
        Self(f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trial {
    pub trial_id: TrialId,
    state: TrialState,
    pub value: Option<f64>,
    pub intermediate_values: BTreeMap<u32, f64>,
    pub params: HashMap<String, TrialParamValue>,
    pub user_attrs: HashMap<String, JsonValue>,
    pub system_attrs: HashMap<String, JsonValue>,
    pub datetime_start: Option<Seconds>,
    pub datetime_end: Option<Seconds>,
}
impl Trial {
    pub fn new(trial_id: TrialId) -> Self {
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

    pub fn adjust(&self) -> Option<Self> {
        if self.datetime_start.is_none() {
            None
        } else {
            let mut trial = self.clone();
            if trial.state == TrialState::Complete && trial.value.is_none() {
                trial.state = TrialState::Running;
                trial.datetime_end = None;
            }
            Some(trial)
        }
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
