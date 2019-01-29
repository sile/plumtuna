use super::node::{StudyId, StudyName};
use plumcast::message::MessageId;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Studies {
    name_to_id: HashMap<StudyName, StudyId>,
    studies: HashMap<StudyId, Study>,
}
impl Studies {
    pub fn contains(&self, name: &StudyName) -> bool {
        self.name_to_id.contains_key(name)
    }

    pub fn insert(&mut self, name: StudyName, id: StudyId, mid: MessageId) {
        let s = Study {
            id,
            name: name.clone(),
            create_mid: mid,
        };
        self.name_to_id.insert(name, id);
        self.studies.insert(id, s);
    }

    pub fn get_by_id(&self, id: StudyId) -> Option<&Study> {
        self.studies.get(&id)
    }

    pub fn get(&self, name: &StudyName) -> Option<&Study> {
        self.name_to_id
            .get(name)
            .and_then(|id| self.studies.get(id))
    }

    pub fn remove_by_id(&mut self, id: StudyId) {
        if let Some(s) = self.studies.remove(&id) {
            self.name_to_id.remove(&s.name);
        }
    }
}

#[derive(Debug)]
pub struct Study {
    pub id: StudyId,
    pub name: StudyName,
    pub create_mid: MessageId,
    // TODO: nodeid
}
