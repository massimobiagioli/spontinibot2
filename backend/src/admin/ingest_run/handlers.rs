use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};

use crate::admin::ErrorResponse;
use crate::admin::check_admin_key;
use crate::admin::ingest_run::{IngestRunAdminPort, IngestRunError, IngestRunResponse};
use crate::config::Config;

#[derive(Clone)]
pub struct IngestRunState {
    pub ingest_run: Arc<dyn IngestRunAdminPort>,
    pub config: Config,
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
    headers: HeaderMap,
) -> Result<(StatusCode, Json<IngestRunResponse>), (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;

    let response = state
        .ingest_run
        .trigger_run()
        .await
        .map_err(map_run_error)?;
    Ok((StatusCode::ACCEPTED, Json(response)))
}

pub async fn get_run(
    State(state): State<IngestRunState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<IngestRunResponse>, (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;

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

    fn test_state() -> IngestRunState {
        let config = Config {
            embed_url: "http://localhost:8080".into(),
            generate_url: "http://localhost:8081".into(),
            kb_path: "/tmp/test.db".into(),
            top_k: 5,
            min_score: 0.35,
            admin_api_key: "test-key".into(),
            upload_max_bytes: 10_485_760,
        };
        IngestRunState {
            ingest_run: Arc::new(MockIngestRunAdmin),
            config,
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

    fn auth_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-admin-key", "test-key".parse().unwrap());
        headers
    }

    fn no_auth_headers() -> HeaderMap {
        HeaderMap::new()
    }

    #[tokio::test]
    async fn should_reject_trigger_without_admin_key() {
        let state = test_state();
        let result = trigger_run(State(state), no_auth_headers()).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn should_trigger_run_and_return_accepted() {
        let state = test_state();
        let result = trigger_run(State(state), auth_headers()).await;
        assert!(result.is_ok());
        let (status, Json(response)) = result.unwrap();
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(response.status, "pending");
    }

    #[tokio::test]
    async fn should_reject_get_run_without_admin_key() {
        let state = test_state();
        let result = get_run(State(state), no_auth_headers(), Path(1)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn should_return_run_for_known_id() {
        let state = test_state();
        let result = get_run(State(state), auth_headers(), Path(1)).await;
        assert!(result.is_ok());
        let Json(response) = result.unwrap();
        assert_eq!(response.status, "done");
    }

    #[tokio::test]
    async fn should_return_404_for_unknown_id() {
        let state = test_state();
        let result = get_run(State(state), auth_headers(), Path(999)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
