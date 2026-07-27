use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;

use crate::admin::ErrorResponse;
use crate::admin::ingest_manual::{
    IngestManualAdminPort, IngestManualError, IngestManualRequest, IngestManualResponse,
    RecencyWindow,
};
use crate::audit::AuditLogPort;
use crate::audit::record_best_effort;
use crate::auth::extractor::OperatorSession;

#[derive(Clone)]
pub struct IngestManualState {
    pub ingest_manual: Arc<dyn IngestManualAdminPort>,
    pub audit: Arc<dyn AuditLogPort>,
}

fn map_manual_error(e: IngestManualError) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        IngestManualError::InvalidWindow(err) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: err.to_string(),
            }),
        ),
        IngestManualError::RobotsTxt(msg) => (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: format!("robots.txt: {msg}"),
            }),
        ),
        IngestManualError::Ingest(msg) => {
            (StatusCode::BAD_GATEWAY, Json(ErrorResponse { error: msg }))
        }
    }
}

pub async fn ingest_manual(
    State(state): State<IngestManualState>,
    session: OperatorSession,
    Json(req): Json<IngestManualRequest>,
) -> Result<(StatusCode, Json<IngestManualResponse>), (StatusCode, Json<ErrorResponse>)> {
    let window = RecencyWindow::parse(&req.window)
        .map_err(|e| map_manual_error(IngestManualError::InvalidWindow(e)))?;

    let response = state
        .ingest_manual
        .ingest(&req.section, &req.src, window)
        .await
        .map_err(map_manual_error)?;

    record_best_effort(
        state.audit.as_ref(),
        &session.actor,
        "ingest_manual",
        &format!("{}:{}", response.section, response.src),
        &serde_json::to_value(&response).unwrap_or_default(),
    )
    .await;

    Ok((StatusCode::OK, Json(response)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditError;
    use async_trait::async_trait;

    fn session() -> OperatorSession {
        OperatorSession {
            actor: "operator".into(),
        }
    }

    struct NoopAudit;

    #[async_trait]
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

    struct StubIngestManualAdmin {
        fail_robots: bool,
    }

    #[async_trait]
    impl IngestManualAdminPort for StubIngestManualAdmin {
        async fn ingest(
            &self,
            section: &str,
            src: &str,
            window: RecencyWindow,
        ) -> Result<IngestManualResponse, IngestManualError> {
            if self.fail_robots {
                return Err(IngestManualError::RobotsTxt(
                    "disallowed for testing".into(),
                ));
            }
            Ok(IngestManualResponse {
                section: section.to_string(),
                src: src.to_string(),
                window: window.to_string(),
                status: "ingested".to_string(),
            })
        }
    }

    fn test_state(fail_robots: bool) -> IngestManualState {
        IngestManualState {
            ingest_manual: Arc::new(StubIngestManualAdmin { fail_robots }),
            audit: Arc::new(NoopAudit),
        }
    }

    #[tokio::test]
    async fn should_ingest_and_return_200_for_valid_request() {
        let state = test_state(false);
        let req = IngestManualRequest {
            section: "storia".into(),
            src: "https://it.wikipedia.org/wiki/Maiolati_Spontini".into(),
            window: "30d".into(),
        };
        let result = ingest_manual(State(state), session(), Json(req)).await;
        assert!(result.is_ok());
        let (status, Json(response)) = result.unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.status, "ingested");
    }

    #[tokio::test]
    async fn should_return_400_for_invalid_window() {
        let state = test_state(false);
        let req = IngestManualRequest {
            section: "storia".into(),
            src: "https://it.wikipedia.org/wiki/Maiolati_Spontini".into(),
            window: "banana".into(),
        };
        let result = ingest_manual(State(state), session(), Json(req)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn should_return_403_for_robots_disallowed_source() {
        let state = test_state(true);
        let req = IngestManualRequest {
            section: "delibere".into(),
            src: "https://www.halleyweb.com/c042023/zf/index.php/atti-amministrativi/delibere"
                .into(),
            window: "30d".into(),
        };
        let result = ingest_manual(State(state), session(), Json(req)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::FORBIDDEN);
    }
}
