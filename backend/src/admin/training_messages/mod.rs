pub mod adapter;
pub mod handlers;

use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TrainingMessageSource {
    pub document_id: i64,
    pub source_ref: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TrainingMessageResponse {
    pub id: i64,
    pub session_id: i64,
    pub question: String,
    pub answer: String,
    pub sources: Vec<TrainingMessageSource>,
    pub fell_back: bool,
    pub created_at: String,
}

#[derive(Debug)]
pub enum TrainingMessageError {
    SessionNotFound(i64),
    DbError(String),
    Rag(String),
    Serialization(String),
}

impl fmt::Display for TrainingMessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrainingMessageError::SessionNotFound(id) => {
                write!(f, "training session {id} not found")
            }
            TrainingMessageError::DbError(msg) => write!(f, "database error: {msg}"),
            TrainingMessageError::Rag(msg) => write!(f, "rag engine error: {msg}"),
            TrainingMessageError::Serialization(msg) => {
                write!(f, "sources serialization error: {msg}")
            }
        }
    }
}

impl std::error::Error for TrainingMessageError {}

impl From<kb_store::KbStoreError> for TrainingMessageError {
    fn from(e: kb_store::KbStoreError) -> Self {
        TrainingMessageError::DbError(e.to_string())
    }
}

#[async_trait]
pub trait TrainingMessageAdminPort: Send + Sync {
    async fn ask(
        &self,
        session_id: i64,
        question: String,
    ) -> Result<TrainingMessageResponse, TrainingMessageError>;

    async fn list_messages(
        &self,
        session_id: i64,
    ) -> Result<Vec<TrainingMessageResponse>, TrainingMessageError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_format_session_not_found_error_display() {
        let err = TrainingMessageError::SessionNotFound(42);
        assert_eq!(err.to_string(), "training session 42 not found");
    }

    #[test]
    fn should_format_db_error_display() {
        let err = TrainingMessageError::DbError("connection refused".into());
        assert_eq!(err.to_string(), "database error: connection refused");
    }

    #[test]
    fn should_format_rag_error_display() {
        let err = TrainingMessageError::Rag("no active persona configured".into());
        assert_eq!(
            err.to_string(),
            "rag engine error: no active persona configured"
        );
    }

    #[test]
    fn should_format_serialization_error_display() {
        let err = TrainingMessageError::Serialization("unexpected end of input".into());
        assert_eq!(
            err.to_string(),
            "sources serialization error: unexpected end of input"
        );
    }

    #[test]
    fn should_serialize_training_message_response_sources_as_json_array() {
        let response = TrainingMessageResponse {
            id: 1,
            session_id: 2,
            question: "A che ora apre l'anagrafe?".into(),
            answer: "Lo sportello apre alle 9:00.".into(),
            sources: vec![TrainingMessageSource {
                document_id: 7,
                source_ref: "orari.md".into(),
            }],
            fell_back: false,
            created_at: "2026-07-24T00:00:00Z".into(),
        };

        let json = serde_json::to_value(&response).expect("serialization failed");
        assert_eq!(
            json["sources"],
            serde_json::json!([{"document_id": 7, "source_ref": "orari.md"}])
        );
    }
}
