pub mod adapter;
pub mod handlers;

use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TrainingMessageSource {
    pub document_id: i64,
    pub source_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
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
    pub expected_answer: Option<String>,
    pub execution_time_ms: Option<i64>,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct AskTrainingMessageRequest {
    pub question: String,
    pub expected_answer: Option<String>,
    /// A manually supplied answer. When present, the adapter records the
    /// message as-is instead of invoking the RAG engine (used to backfill
    /// historical Q&A pairs into a session without a live bot call).
    pub answer: Option<String>,
}

#[derive(Debug)]
pub enum TrainingMessageError {
    SessionNotFound(i64),
    MessageNotFound(i64),
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
            TrainingMessageError::MessageNotFound(id) => {
                write!(f, "training message {id} not found")
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
        req: AskTrainingMessageRequest,
    ) -> Result<TrainingMessageResponse, TrainingMessageError>;

    async fn list_messages(
        &self,
        session_id: i64,
    ) -> Result<Vec<TrainingMessageResponse>, TrainingMessageError>;

    /// Sets or clears `expected_answer` on an already-created message.
    /// `expected_answer` can only be supplied at creation time via `ask`
    /// otherwise — this is the retroactive path for a question that was
    /// asked without one (see AGENTS.md §3.8).
    async fn update_expected_answer(
        &self,
        message_id: i64,
        expected_answer: Option<String>,
    ) -> Result<TrainingMessageResponse, TrainingMessageError>;
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
                source_url: None,
            }],
            fell_back: false,
            created_at: "2026-07-24T00:00:00Z".into(),
            expected_answer: None,
            execution_time_ms: Some(120),
            source: "chat".into(),
        };

        let json = serde_json::to_value(&response).expect("serialization failed");
        assert_eq!(
            json["sources"],
            serde_json::json!([{"document_id": 7, "source_ref": "orari.md"}])
        );
    }

    #[test]
    fn should_serialize_source_url_when_present() {
        let source = TrainingMessageSource {
            document_id: 7,
            source_ref: "delibera-di-giunta-74-2026-07-13.pdf".into(),
            source_url: Some("https://www.halleyweb.com/detail/74".into()),
        };

        let json = serde_json::to_value(&source).expect("serialization failed");
        assert_eq!(json["source_url"], "https://www.halleyweb.com/detail/74");
    }

    #[test]
    fn should_deserialize_sources_json_missing_source_url_as_none() {
        // Rows persisted before `source_url` existed store `sources` JSON
        // without that key at all — this must not fail deserialization.
        let source: TrainingMessageSource =
            serde_json::from_str(r#"{"document_id": 7, "source_ref": "orari.md"}"#)
                .expect("deserialization failed");
        assert_eq!(source.source_url, None);
    }
}
