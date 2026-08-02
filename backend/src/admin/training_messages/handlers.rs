use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Deserialize;

use crate::admin::ErrorResponse;
use crate::admin::training_messages::{
    AskTrainingMessageRequest, TrainingMessageAdminPort, TrainingMessageError,
    TrainingMessageResponse,
};
use crate::audit::AuditLogPort;
use crate::audit::record_best_effort;
use crate::auth::extractor::OperatorSession;

#[derive(Clone)]
pub struct TrainingMessageState {
    pub training_messages: Arc<dyn TrainingMessageAdminPort>,
    pub audit: Arc<dyn AuditLogPort>,
}

#[derive(Deserialize)]
pub struct AskRequest {
    pub question: String,
    pub expected_answer: Option<String>,
    /// A manually supplied answer. When present, the session records this
    /// Q&A pair as-is instead of invoking the live bot (used to backfill
    /// historical exchanges into a session).
    pub answer: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateExpectedAnswerRequest {
    pub expected_answer: Option<String>,
}

fn map_message_error(e: TrainingMessageError) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        TrainingMessageError::SessionNotFound(id) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("training session {id} not found"),
            }),
        ),
        TrainingMessageError::MessageNotFound(id) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("training message {id} not found"),
            }),
        ),
        TrainingMessageError::DbError(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: msg }),
        ),
        TrainingMessageError::Rag(msg) => {
            (StatusCode::BAD_GATEWAY, Json(ErrorResponse { error: msg }))
        }
        TrainingMessageError::Serialization(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: msg }),
        ),
    }
}

pub async fn create_message(
    State(state): State<TrainingMessageState>,
    session: OperatorSession,
    Path(session_id): Path<i64>,
    Json(req): Json<AskRequest>,
) -> Result<(StatusCode, Json<TrainingMessageResponse>), (StatusCode, Json<ErrorResponse>)> {
    let response = state
        .training_messages
        .ask(
            session_id,
            AskTrainingMessageRequest {
                question: req.question,
                expected_answer: req.expected_answer,
                answer: req.answer,
            },
        )
        .await
        .map_err(map_message_error)?;
    record_best_effort(
        state.audit.as_ref(),
        &session.actor,
        "create_message",
        &format!("training_message:{}", response.id),
        &serde_json::to_value(&response).unwrap_or_default(),
    )
    .await;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list_messages(
    State(state): State<TrainingMessageState>,
    _session: OperatorSession,
    Path(session_id): Path<i64>,
) -> Result<Json<Vec<TrainingMessageResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let messages = state
        .training_messages
        .list_messages(session_id)
        .await
        .map_err(map_message_error)?;
    Ok(Json(messages))
}

pub async fn update_expected_answer(
    State(state): State<TrainingMessageState>,
    session: OperatorSession,
    Path(message_id): Path<i64>,
    Json(req): Json<UpdateExpectedAnswerRequest>,
) -> Result<Json<TrainingMessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    let response = state
        .training_messages
        .update_expected_answer(message_id, req.expected_answer)
        .await
        .map_err(map_message_error)?;
    record_best_effort(
        state.audit.as_ref(),
        &session.actor,
        "update_expected_answer",
        &format!("training_message:{message_id}"),
        &serde_json::to_value(&response).unwrap_or_default(),
    )
    .await;
    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin::training_messages::TrainingMessageSource;
    use crate::audit::AuditError;

    fn test_state() -> TrainingMessageState {
        TrainingMessageState {
            training_messages: Arc::new(MockTrainingMessageAdmin),
            audit: Arc::new(NoopAudit),
        }
    }

    fn session() -> OperatorSession {
        OperatorSession {
            actor: "operator".into(),
        }
    }

    struct NoopAudit;

    #[async_trait::async_trait]
    impl AuditLogPort for NoopAudit {
        async fn record(
            &self,
            _actor: &str,
            _action: &str,
            _target: &str,
            _payload: &serde_json::Value,
        ) -> Result<(), AuditError> {
            Ok(())
        }
    }

    struct MockTrainingMessageAdmin;

    #[async_trait::async_trait]
    impl TrainingMessageAdminPort for MockTrainingMessageAdmin {
        async fn ask(
            &self,
            session_id: i64,
            req: AskTrainingMessageRequest,
        ) -> Result<TrainingMessageResponse, TrainingMessageError> {
            if session_id == 999 {
                return Err(TrainingMessageError::SessionNotFound(session_id));
            }
            if session_id == 500 {
                return Err(TrainingMessageError::DbError("connection refused".into()));
            }
            if session_id == 502 {
                return Err(TrainingMessageError::Rag(
                    "generation service error: timeout".into(),
                ));
            }
            if session_id == 501 {
                return Err(TrainingMessageError::Serialization(
                    "unexpected end of input".into(),
                ));
            }
            Ok(TrainingMessageResponse {
                id: 1,
                session_id,
                question: req.question,
                answer: "Lo sportello apre alle 9:00.".into(),
                sources: vec![TrainingMessageSource {
                    document_id: 7,
                    source_ref: "orari.md".into(),
                    source_url: None,
                }],
                fell_back: false,
                created_at: "2026-07-24T00:00:00Z".into(),
                expected_answer: req.expected_answer,
                execution_time_ms: Some(42),
                source: "chat".into(),
            })
        }

        async fn list_messages(
            &self,
            session_id: i64,
        ) -> Result<Vec<TrainingMessageResponse>, TrainingMessageError> {
            Ok(vec![TrainingMessageResponse {
                id: 1,
                session_id,
                question: "A che ora apre l'anagrafe?".into(),
                answer: "Lo sportello apre alle 9:00.".into(),
                sources: vec![],
                fell_back: false,
                created_at: "2026-07-24T00:00:00Z".into(),
                expected_answer: None,
                execution_time_ms: Some(42),
                source: "chat".into(),
            }])
        }

        async fn update_expected_answer(
            &self,
            message_id: i64,
            expected_answer: Option<String>,
        ) -> Result<TrainingMessageResponse, TrainingMessageError> {
            if message_id == 999 {
                return Err(TrainingMessageError::MessageNotFound(message_id));
            }
            Ok(TrainingMessageResponse {
                id: message_id,
                session_id: 1,
                question: "A che ora apre l'anagrafe?".into(),
                answer: "Lo sportello apre alle 9:00.".into(),
                sources: vec![],
                fell_back: false,
                created_at: "2026-07-24T00:00:00Z".into(),
                expected_answer,
                execution_time_ms: Some(42),
                source: "chat".into(),
            })
        }
    }

    #[tokio::test]
    async fn should_create_message_and_return_created() {
        let state = test_state();
        let req = AskRequest {
            question: "A che ora apre l'anagrafe?".into(),
            expected_answer: None,
            answer: None,
        };
        let result = create_message(State(state), session(), Path(1), Json(req)).await;
        assert!(result.is_ok());
        let (status, Json(response)) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(response.session_id, 1);
        assert_eq!(response.sources.len(), 1);
    }

    #[tokio::test]
    async fn should_return_404_for_unknown_session_on_create() {
        let state = test_state();
        let req = AskRequest {
            question: "domanda".into(),
            expected_answer: None,
            answer: None,
        };
        let result = create_message(State(state), session(), Path(999), Json(req)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn should_return_500_when_ask_hits_a_db_error() {
        let state = test_state();
        let req = AskRequest {
            question: "domanda".into(),
            expected_answer: None,
            answer: None,
        };
        let result = create_message(State(state), session(), Path(500), Json(req)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn should_return_502_when_ask_hits_a_rag_error() {
        let state = test_state();
        let req = AskRequest {
            question: "domanda".into(),
            expected_answer: None,
            answer: None,
        };
        let result = create_message(State(state), session(), Path(502), Json(req)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn should_return_500_when_ask_hits_a_serialization_error() {
        let state = test_state();
        let req = AskRequest {
            question: "domanda".into(),
            expected_answer: None,
            answer: None,
        };
        let result = create_message(State(state), session(), Path(501), Json(req)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn should_list_messages_for_known_session() {
        let state = test_state();
        let result = list_messages(State(state), session(), Path(1)).await;
        assert!(result.is_ok());
        let Json(messages) = result.unwrap();
        assert_eq!(messages.len(), 1);
    }

    #[tokio::test]
    async fn should_update_expected_answer_and_return_ok() {
        let state = test_state();
        let req = UpdateExpectedAnswerRequest {
            expected_answer: Some("Dalle 9:00 alle 12:30".into()),
        };
        let result = update_expected_answer(State(state), session(), Path(1), Json(req)).await;
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.id, 1);
        assert_eq!(
            response.expected_answer.as_deref(),
            Some("Dalle 9:00 alle 12:30")
        );
    }

    #[tokio::test]
    async fn should_return_404_for_unknown_message_on_update() {
        let state = test_state();
        let req = UpdateExpectedAnswerRequest {
            expected_answer: Some("x".into()),
        };
        let result = update_expected_answer(State(state), session(), Path(999), Json(req)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
