use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::admin::ErrorResponse;
use crate::admin::training_sessions::{
    TrainingSessionAdminPort, TrainingSessionError, TrainingSessionResponse,
};
use crate::audit::AuditLogPort;
use crate::audit::record_best_effort;
use crate::auth::extractor::OperatorSession;

#[derive(Clone)]
pub struct TrainingSessionState {
    pub training_sessions: Arc<dyn TrainingSessionAdminPort>,
    pub audit: Arc<dyn AuditLogPort>,
}

#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub title: String,
    pub created_by: Option<String>,
}

#[derive(Serialize)]
pub struct ClosedResponse {
    pub closed: bool,
}

fn map_session_error(e: TrainingSessionError) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        TrainingSessionError::DbError(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: msg }),
        ),
    }
}

pub async fn create_session(
    State(state): State<TrainingSessionState>,
    session: OperatorSession,
    Json(req): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<TrainingSessionResponse>), (StatusCode, Json<ErrorResponse>)> {
    let new_session = kb_store::NewTrainingSession {
        title: req.title,
        created_by: req.created_by,
    };
    let response = state
        .training_sessions
        .create_session(new_session)
        .await
        .map_err(map_session_error)?;
    record_best_effort(
        state.audit.as_ref(),
        &session.actor,
        "create_session",
        &format!("training_session:{}", response.id),
        &serde_json::to_value(&response).unwrap_or_default(),
    )
    .await;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list_sessions(
    State(state): State<TrainingSessionState>,
    _session: OperatorSession,
) -> Result<Json<Vec<TrainingSessionResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let sessions = state
        .training_sessions
        .list_sessions()
        .await
        .map_err(map_session_error)?;
    Ok(Json(sessions))
}

pub async fn get_session(
    State(state): State<TrainingSessionState>,
    _session: OperatorSession,
    Path(id): Path<i64>,
) -> Result<Json<TrainingSessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let session = state
        .training_sessions
        .get_session(id)
        .await
        .map_err(map_session_error)?;
    session.map(Json).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("training session {id} not found"),
            }),
        )
    })
}

pub async fn close_session(
    State(state): State<TrainingSessionState>,
    session: OperatorSession,
    Path(id): Path<i64>,
) -> Result<Json<ClosedResponse>, (StatusCode, Json<ErrorResponse>)> {
    let closed = state
        .training_sessions
        .close_session(id)
        .await
        .map_err(map_session_error)?;
    record_best_effort(
        state.audit.as_ref(),
        &session.actor,
        "close_session",
        &format!("training_session:{id}"),
        &serde_json::json!({"closed": closed}),
    )
    .await;
    Ok(Json(ClosedResponse { closed }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditError;

    fn test_state() -> TrainingSessionState {
        TrainingSessionState {
            training_sessions: Arc::new(MockTrainingSessionAdmin),
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

    struct MockTrainingSessionAdmin;

    #[async_trait::async_trait]
    impl TrainingSessionAdminPort for MockTrainingSessionAdmin {
        async fn create_session(
            &self,
            req: kb_store::NewTrainingSession,
        ) -> Result<TrainingSessionResponse, TrainingSessionError> {
            Ok(TrainingSessionResponse {
                id: 1,
                title: req.title,
                created_at: "2026-07-24T00:00:00Z".into(),
                created_by: req.created_by,
                closed_at: None,
            })
        }

        async fn list_sessions(
            &self,
        ) -> Result<Vec<TrainingSessionResponse>, TrainingSessionError> {
            Ok(vec![TrainingSessionResponse {
                id: 1,
                title: "Sessione".into(),
                created_at: "2026-07-24T00:00:00Z".into(),
                created_by: None,
                closed_at: None,
            }])
        }

        async fn get_session(
            &self,
            id: i64,
        ) -> Result<Option<TrainingSessionResponse>, TrainingSessionError> {
            if id == 1 {
                Ok(Some(TrainingSessionResponse {
                    id: 1,
                    title: "Sessione".into(),
                    created_at: "2026-07-24T00:00:00Z".into(),
                    created_by: None,
                    closed_at: None,
                }))
            } else {
                Ok(None)
            }
        }

        async fn close_session(&self, id: i64) -> Result<bool, TrainingSessionError> {
            Ok(id == 1)
        }
    }

    #[tokio::test]
    async fn should_create_session_and_return_created() {
        let state = test_state();
        let req = CreateSessionRequest {
            title: "Sessione".into(),
            created_by: Some("operator1".into()),
        };
        let result = create_session(State(state), session(), Json(req)).await;
        assert!(result.is_ok());
        let (status, Json(response)) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(response.title, "Sessione");
    }

    #[tokio::test]
    async fn should_list_sessions() {
        let state = test_state();
        let result = list_sessions(State(state), session()).await;
        assert!(result.is_ok());
        let Json(sessions) = result.unwrap();
        assert_eq!(sessions.len(), 1);
    }

    #[tokio::test]
    async fn should_return_session_for_known_id() {
        let state = test_state();
        let result = get_session(State(state), session(), Path(1)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn should_return_404_for_unknown_session_id() {
        let state = test_state();
        let result = get_session(State(state), session(), Path(999)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn should_close_known_session() {
        let state = test_state();
        let result = close_session(State(state), session(), Path(1)).await;
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert!(response.closed);
    }

    #[tokio::test]
    async fn should_return_false_when_closing_unknown_session() {
        let state = test_state();
        let result = close_session(State(state), session(), Path(999)).await;
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert!(!response.closed);
    }
}
