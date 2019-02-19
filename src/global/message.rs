use crate::study::{StudyId, StudyName};
use crate::time::Timestamp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    CreateStudy {
        name: StudyName,
        id: StudyId,
        timestamp: Timestamp,
    },
    OpenStudy {
        name: StudyName,
    },
    OpenStudyById {
        id: StudyId,
    },
}
