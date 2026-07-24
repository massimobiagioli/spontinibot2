pub mod adapter;
pub mod handlers;

use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use kb_store::IngestRunRequest;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct IngestRunResponse {
    pub id: i64,
    pub status: String,
    pub requested_at: String,
}

impl From<IngestRunRequest> for IngestRunResponse {
    fn from(r: IngestRunRequest) -> Self {
        Self {
            id: r.id,
            status: r.status.to_string(),
            requested_at: r.requested_at,
        }
    }
}

#[derive(Debug)]
pub enum IngestRunError {
    DbError(String),
}

impl fmt::Display for IngestRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IngestRunError::DbError(msg) => write!(f, "database error: {msg}"),
        }
    }
}

impl std::error::Error for IngestRunError {}

impl From<kb_store::KbStoreError> for IngestRunError {
    fn from(e: kb_store::KbStoreError) -> Self {
        IngestRunError::DbError(e.to_string())
    }
}

#[async_trait]
pub trait IngestRunAdminPort: Send + Sync {
    async fn trigger_run(&self) -> Result<IngestRunResponse, IngestRunError>;
    async fn get_run(&self, id: i64) -> Result<Option<IngestRunResponse>, IngestRunError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use kb_store::RunRequestStatus;

    #[test]
    fn should_format_ingest_run_error_display() {
        let db_error = IngestRunError::DbError("connection refused".into());
        assert_eq!(db_error.to_string(), "database error: connection refused");
    }

    #[test]
    fn should_convert_run_request_to_response() {
        let request = IngestRunRequest {
            id: 7,
            requested_at: "2026-07-24T00:00:00Z".into(),
            status: RunRequestStatus::Pending,
        };
        let response = IngestRunResponse::from(request);
        assert_eq!(response.id, 7);
        assert_eq!(response.status, "pending");
        assert_eq!(response.requested_at, "2026-07-24T00:00:00Z");
    }
}
