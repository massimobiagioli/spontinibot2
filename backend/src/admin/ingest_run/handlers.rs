use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::admin::ErrorResponse;
use crate::admin::ingest_run::{IngestRunAdminPort, IngestRunError, IngestRunResponse};
use crate::audit::AuditLogPort;
use crate::audit::record_best_effort;
use crate::auth::extractor::OperatorSession;

#[derive(Clone)]
pub struct IngestRunState {
    pub ingest_run: Arc<dyn IngestRunAdminPort>,
    pub audit: Arc<dyn AuditLogPort>,
}

fn map_run_error(e: IngestRunError) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        IngestRunError::DbError(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: msg }),
        ),
    }
}

pub async fn trigger_run(
    State(state): State<IngestRunState>,
    session: OperatorSession,
) -> Result<(StatusCode, Json<IngestRunResponse>), (StatusCode, Json<ErrorResponse>)> {
    let response = state
        .ingest_run
        .trigger_run()
        .await
        .map_err(map_run_error)?;
    record_best_effort(
        state.audit.as_ref(),
        &session.actor,
        "trigger_run",
        &format!("ingest_run:{}", response.id),
        &serde_json::to_value(&response).unwrap_or_default(),
    )
    .await;
    Ok((StatusCode::ACCEPTED, Json(response)))
}

pub async fn get_run(
    State(state): State<IngestRunState>,
    _session: OperatorSession,
    Path(id): Path<i64>,
) -> Result<Json<IngestRunResponse>, (StatusCode, Json<ErrorResponse>)> {
    let response = state.ingest_run.get_run(id).await.map_err(map_run_error)?;
    response.map(Json).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("run request {id} not found"),
            }),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditError;

    fn test_state() -> IngestRunState {
        IngestRunState {
            ingest_run: Arc::new(MockIngestRunAdmin),
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

    struct MockIngestRunAdmin;

    #[async_trait::async_trait]
    impl IngestRunAdminPort for MockIngestRunAdmin {
        async fn trigger_run(&self) -> Result<IngestRunResponse, IngestRunError> {
            Ok(IngestRunResponse {
                id: 1,
                status: "pending".into(),
                requested_at: "2026-07-24T00:00:00Z".into(),
            })
        }

        async fn get_run(&self, id: i64) -> Result<Option<IngestRunResponse>, IngestRunError> {
            if id == 1 {
                Ok(Some(IngestRunResponse {
                    id: 1,
                    status: "done".into(),
                    requested_at: "2026-07-24T00:00:00Z".into(),
                }))
            } else {
                Ok(None)
            }
        }
    }

    #[tokio::test]
    async fn should_trigger_run_and_return_accepted() {
        let state = test_state();
        let result = trigger_run(State(state), session()).await;
        assert!(result.is_ok());
        let (status, Json(response)) = result.unwrap();
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(response.status, "pending");
    }

    #[tokio::test]
    async fn should_return_run_for_known_id() {
        let state = test_state();
        let result = get_run(State(state), session(), Path(1)).await;
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.status, "done");
    }

    #[tokio::test]
    async fn should_return_404_for_unknown_id() {
        let state = test_state();
        let result = get_run(State(state), session(), Path(999)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
