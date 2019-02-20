use crate::study::{StudyId, StudyName};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    CreateStudy { name: StudyName, id: StudyId },
    JoinStudy { name: StudyName },
}
