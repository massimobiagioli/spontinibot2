pub mod adapter;
pub mod handlers;

use std::fmt;

use async_trait::async_trait;
use serde::Serialize;

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct TrainingFeedbackResponse {
    pub id: i64,
    pub message_id: i64,
    pub chunk_id: Option<i64>,
    pub answer_span: String,
    pub sentiment: String,
    pub comment: Option<String>,
    pub created_at: String,
}

impl From<kb_store::TrainingFeedback> for TrainingFeedbackResponse {
    fn from(f: kb_store::TrainingFeedback) -> Self {
        Self {
            id: f.id,
            message_id: f.message_id,
            chunk_id: f.chunk_id,
            answer_span: f.answer_span,
            sentiment: f.sentiment.to_string(),
            comment: f.comment,
            created_at: f.created_at,
        }
    }
}

#[derive(Debug)]
pub enum TrainingFeedbackError {
    MessageNotFound(i64),
    DbError(String),
}

impl fmt::Display for TrainingFeedbackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrainingFeedbackError::MessageNotFound(id) => {
                write!(f, "training message {id} not found")
            }
            TrainingFeedbackError::DbError(msg) => write!(f, "database error: {msg}"),
        }
    }
}

impl std::error::Error for TrainingFeedbackError {}

impl From<kb_store::KbStoreError> for TrainingFeedbackError {
    fn from(e: kb_store::KbStoreError) -> Self {
        TrainingFeedbackError::DbError(e.to_string())
    }
}

#[async_trait]
pub trait TrainingFeedbackAdminPort: Send + Sync {
    async fn create_feedback(
        &self,
        req: kb_store::NewTrainingFeedback,
    ) -> Result<TrainingFeedbackResponse, TrainingFeedbackError>;

    async fn list_feedback(
        &self,
        message_id: i64,
    ) -> Result<Vec<TrainingFeedbackResponse>, TrainingFeedbackError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use kb_store::Sentiment;

    #[test]
    fn should_format_message_not_found_error_display() {
        let err = TrainingFeedbackError::MessageNotFound(42);
        assert_eq!(err.to_string(), "training message 42 not found");
    }

    #[test]
    fn should_format_db_error_display() {
        let err = TrainingFeedbackError::DbError("connection refused".into());
        assert_eq!(err.to_string(), "database error: connection refused");
    }

    #[test]
    fn should_convert_training_feedback_with_chunk_and_comment() {
        let feedback = kb_store::TrainingFeedback {
            id: 1,
            message_id: 2,
            chunk_id: Some(7),
            answer_span: "alle 9:00".into(),
            sentiment: Sentiment::Negative,
            comment: Some("orario sbagliato".into()),
            created_at: "2026-07-24T00:00:00Z".into(),
        };

        let response = TrainingFeedbackResponse::from(feedback);
        assert_eq!(response.chunk_id, Some(7));
        assert_eq!(response.sentiment, "negative");
        assert_eq!(response.comment.as_deref(), Some("orario sbagliato"));
    }

    #[test]
    fn should_convert_training_feedback_without_chunk_or_comment() {
        let feedback = kb_store::TrainingFeedback {
            id: 1,
            message_id: 2,
            chunk_id: None,
            answer_span: "alle 9:00".into(),
            sentiment: Sentiment::Positive,
            comment: None,
            created_at: "2026-07-24T00:00:00Z".into(),
        };

        let response = TrainingFeedbackResponse::from(feedback);
        assert_eq!(response.chunk_id, None);
        assert_eq!(response.sentiment, "positive");
        assert_eq!(response.comment, None);
    }

    #[test]
    fn should_serialize_training_feedback_response() {
        let response = TrainingFeedbackResponse {
            id: 1,
            message_id: 2,
            chunk_id: Some(7),
            answer_span: "alle 9:00".into(),
            sentiment: "negative".into(),
            comment: None,
            created_at: "2026-07-24T00:00:00Z".into(),
        };

        let json = serde_json::to_value(&response).expect("serialization failed");
        assert_eq!(json["sentiment"], "negative");
        assert_eq!(json["chunk_id"], 7);
    }
}
