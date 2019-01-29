use super::node::{StudyId, StudyName};
use atomic_immut::AtomicImmut;
use plumcast::message::MessageId;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct Studies {
    name_to_id: HashMap<StudyName, StudyId>,
    studies: Arc<AtomicImmut<HashMap<StudyId, Study>>>,
}
impl Studies {
    pub fn contains(&self, name: &StudyName) -> bool {
        self.name_to_id.contains_key(name)
    }

    pub fn handle(&self) -> Arc<AtomicImmut<HashMap<StudyId, Study>>> {
        Arc::clone(&self.studies)
    }

    pub fn insert(&mut self, name: StudyName, id: StudyId, mid: MessageId) {
        let s = Study {
            id,
            name: name.clone(),
            create_mid: mid,
        };
        self.name_to_id.insert(name, id);
        self.studies.update(|studies| {
            let mut studies = studies.clone();
            studies.insert(id, s.clone());
            studies
        });
    }

    pub fn get_by_id(&self, id: StudyId) -> Option<Study> {
        self.studies.load().get(&id).cloned()
    }

    pub fn get(&self, name: &StudyName) -> Option<Study> {
        self.name_to_id
            .get(name)
            .and_then(|id| self.studies.load().get(id).cloned())
    }

    pub fn remove_by_id(&mut self, id: StudyId) {
        if let Some(s) = self.get_by_id(id) {
            self.name_to_id.remove(&s.name);
        }
        self.studies.update(|studies| {
            studies
                .iter()
                .filter(|x| *x.0 != id)
                .map(|(&id, s)| (id, s.clone()))
                .collect()
        });
    }
}

#[derive(Debug, Clone)]
pub struct Study {
    pub id: StudyId,
    pub name: StudyName,
    pub create_mid: MessageId,
}
