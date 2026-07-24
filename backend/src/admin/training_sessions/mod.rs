pub mod adapter;
pub mod handlers;

use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use kb_store::{NewTrainingSession, TrainingSession};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TrainingSessionResponse {
    pub id: i64,
    pub title: String,
    pub created_at: String,
    pub created_by: Option<String>,
    pub closed_at: Option<String>,
}

impl From<TrainingSession> for TrainingSessionResponse {
    fn from(s: TrainingSession) -> Self {
        Self {
            id: s.id,
            title: s.title,
            created_at: s.created_at,
            created_by: s.created_by,
            closed_at: s.closed_at,
        }
    }
}

#[derive(Debug)]
pub enum TrainingSessionError {
    DbError(String),
}

impl fmt::Display for TrainingSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrainingSessionError::DbError(msg) => write!(f, "database error: {msg}"),
        }
    }
}

impl std::error::Error for TrainingSessionError {}

impl From<kb_store::KbStoreError> for TrainingSessionError {
    fn from(e: kb_store::KbStoreError) -> Self {
        TrainingSessionError::DbError(e.to_string())
    }
}

#[async_trait]
pub trait TrainingSessionAdminPort: Send + Sync {
    async fn create_session(
        &self,
        req: NewTrainingSession,
    ) -> Result<TrainingSessionResponse, TrainingSessionError>;
    async fn list_sessions(&self) -> Result<Vec<TrainingSessionResponse>, TrainingSessionError>;
    async fn get_session(
        &self,
        id: i64,
    ) -> Result<Option<TrainingSessionResponse>, TrainingSessionError>;
    async fn close_session(&self, id: i64) -> Result<bool, TrainingSessionError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_format_training_session_error_display() {
        let db_error = TrainingSessionError::DbError("connection refused".into());
        assert_eq!(db_error.to_string(), "database error: connection refused");
    }

    #[test]
    fn should_convert_training_session_to_response_with_all_fields() {
        let session = TrainingSession {
            id: 3,
            title: "Sessione".into(),
            created_at: "2026-07-24T00:00:00Z".into(),
            created_by: Some("operator1".into()),
            closed_at: Some("2026-07-24T01:00:00Z".into()),
        };
        let response = TrainingSessionResponse::from(session);
        assert_eq!(response.id, 3);
        assert_eq!(response.title, "Sessione");
        assert_eq!(response.created_by.as_deref(), Some("operator1"));
        assert_eq!(response.closed_at.as_deref(), Some("2026-07-24T01:00:00Z"));
    }

    #[test]
    fn should_convert_open_training_session_to_response_with_none_fields() {
        let session = TrainingSession {
            id: 4,
            title: "Sessione aperta".into(),
            created_at: "2026-07-24T00:00:00Z".into(),
            created_by: None,
            closed_at: None,
        };
        let response = TrainingSessionResponse::from(session);
        assert!(response.created_by.is_none());
        assert!(response.closed_at.is_none());
    }
}
