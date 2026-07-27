use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::admin::ErrorResponse;
use crate::admin::ingest_config::{
    IngestConfigAdminPort, IngestConfigError, IngestConfigResponse, IngestScheduleResponse,
    IngestSectionResponse, IngestSourceResponse, IngestedDocumentResponse,
};
use crate::audit::AuditLogPort;
use crate::audit::record_best_effort;
use crate::auth::extractor::OperatorSession;

#[derive(Clone)]
pub struct IngestConfigState {
    pub ingest_config: Arc<dyn IngestConfigAdminPort>,
    pub audit: Arc<dyn AuditLogPort>,
}

#[derive(Deserialize)]
pub struct UpsertScheduleRequest {
    pub cron_expr: String,
    pub enabled: bool,
}

#[derive(Deserialize)]
pub struct CreateSectionRequest {
    pub name: String,
    pub ordering: i32,
}

#[derive(Deserialize)]
pub struct CreateSourceRequest {
    pub source_type: String,
    pub url: String,
    pub enabled: bool,
}

#[derive(Deserialize)]
pub struct SectionIdQuery {
    pub section_id: i64,
}

#[derive(Serialize)]
pub struct DeletedResponse {
    pub deleted: bool,
}

fn map_config_error(e: IngestConfigError) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        IngestConfigError::NotFound(msg) => {
            (StatusCode::NOT_FOUND, Json(ErrorResponse { error: msg }))
        }
        IngestConfigError::DbError(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: msg }),
        ),
    }
}

pub async fn get_config(
    State(state): State<IngestConfigState>,
    _session: OperatorSession,
) -> Result<Json<IngestConfigResponse>, (StatusCode, Json<ErrorResponse>)> {
    let schedule = state
        .ingest_config
        .get_schedule()
        .await
        .map_err(map_config_error)?;
    let sections = state
        .ingest_config
        .list_sections()
        .await
        .map_err(map_config_error)?;

    let mut sections_with_sources = Vec::new();
    for section in sections {
        let sources = state
            .ingest_config
            .list_sources(section.id)
            .await
            .map_err(map_config_error)?;
        sections_with_sources
            .push(crate::admin::ingest_config::IngestSectionWithSources { section, sources });
    }

    Ok(Json(IngestConfigResponse {
        schedule,
        sections: sections_with_sources,
    }))
}

pub async fn upsert_schedule(
    State(state): State<IngestConfigState>,
    session: OperatorSession,
    Json(req): Json<UpsertScheduleRequest>,
) -> Result<Json<IngestScheduleResponse>, (StatusCode, Json<ErrorResponse>)> {
    let schedule = kb_store::NewIngestSchedule {
        cron_expr: req.cron_expr,
        enabled: req.enabled,
    };
    let response = state
        .ingest_config
        .upsert_schedule(schedule)
        .await
        .map_err(map_config_error)?;
    record_best_effort(
        state.audit.as_ref(),
        &session.actor,
        "upsert_schedule",
        "ingest_schedule",
        &serde_json::to_value(&response).unwrap_or_default(),
    )
    .await;
    Ok(Json(response))
}

pub async fn create_section(
    State(state): State<IngestConfigState>,
    session: OperatorSession,
    Json(req): Json<CreateSectionRequest>,
) -> Result<(StatusCode, Json<IngestSectionResponse>), (StatusCode, Json<ErrorResponse>)> {
    let section = kb_store::NewIngestSection {
        name: req.name,
        ordering: req.ordering,
    };
    let response = state
        .ingest_config
        .create_section(section)
        .await
        .map_err(map_config_error)?;
    record_best_effort(
        state.audit.as_ref(),
        &session.actor,
        "create_section",
        &format!("ingest_section:{}", response.id),
        &serde_json::to_value(&response).unwrap_or_default(),
    )
    .await;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn delete_section(
    State(state): State<IngestConfigState>,
    session: OperatorSession,
    Path(id): Path<i64>,
) -> Result<Json<DeletedResponse>, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state
        .ingest_config
        .delete_section(id)
        .await
        .map_err(map_config_error)?;
    record_best_effort(
        state.audit.as_ref(),
        &session.actor,
        "delete_section",
        &format!("ingest_section:{id}"),
        &serde_json::json!({"deleted": deleted}),
    )
    .await;
    Ok(Json(DeletedResponse { deleted }))
}

pub async fn create_source(
    State(state): State<IngestConfigState>,
    session: OperatorSession,
    Query(query): Query<SectionIdQuery>,
    Json(req): Json<CreateSourceRequest>,
) -> Result<(StatusCode, Json<IngestSourceResponse>), (StatusCode, Json<ErrorResponse>)> {
    let source_type = req.source_type.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("invalid source_type: {}", req.source_type),
            }),
        )
    })?;

    let source = kb_store::NewIngestSource {
        section_id: query.section_id,
        source_type,
        url: req.url,
        enabled: req.enabled,
    };
    let response = state
        .ingest_config
        .create_source(query.section_id, source)
        .await
        .map_err(map_config_error)?;
    record_best_effort(
        state.audit.as_ref(),
        &session.actor,
        "create_source",
        &format!("ingest_source:{}", response.id),
        &serde_json::to_value(&response).unwrap_or_default(),
    )
    .await;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn delete_source(
    State(state): State<IngestConfigState>,
    session: OperatorSession,
    Path(id): Path<i64>,
) -> Result<Json<DeletedResponse>, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state
        .ingest_config
        .delete_source(id)
        .await
        .map_err(map_config_error)?;
    record_best_effort(
        state.audit.as_ref(),
        &session.actor,
        "delete_source",
        &format!("ingest_source:{id}"),
        &serde_json::json!({"deleted": deleted}),
    )
    .await;
    Ok(Json(DeletedResponse { deleted }))
}

pub async fn list_section_documents(
    State(state): State<IngestConfigState>,
    _session: OperatorSession,
    Path(id): Path<i64>,
) -> Result<Json<Vec<IngestedDocumentResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let documents = state
        .ingest_config
        .list_section_documents(id)
        .await
        .map_err(map_config_error)?;
    Ok(Json(documents))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin::ingest_config::IngestScheduleResponse;
    use crate::audit::AuditError;

    fn test_state() -> IngestConfigState {
        IngestConfigState {
            ingest_config: Arc::new(MockIngestConfigAdmin),
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

    struct MockIngestConfigAdmin;

    #[async_trait::async_trait]
    impl IngestConfigAdminPort for MockIngestConfigAdmin {
        async fn get_schedule(&self) -> Result<Option<IngestScheduleResponse>, IngestConfigError> {
            Ok(Some(IngestScheduleResponse {
                cron_expr: "0 */4 * * *".into(),
                enabled: true,
                updated_at: "2026-07-10T00:00:00Z".into(),
            }))
        }
        async fn upsert_schedule(
            &self,
            _s: kb_store::NewIngestSchedule,
        ) -> Result<IngestScheduleResponse, IngestConfigError> {
            Ok(IngestScheduleResponse {
                cron_expr: "0 */4 * * *".into(),
                enabled: true,
                updated_at: "2026-07-10T00:00:00Z".into(),
            })
        }
        async fn list_sections(&self) -> Result<Vec<IngestSectionResponse>, IngestConfigError> {
            Ok(vec![])
        }
        async fn create_section(
            &self,
            _s: kb_store::NewIngestSection,
        ) -> Result<IngestSectionResponse, IngestConfigError> {
            Ok(IngestSectionResponse {
                id: 1,
                name: "sport".into(),
                ordering: 10,
                created_at: "2026-07-10T00:00:00Z".into(),
            })
        }
        async fn delete_section(&self, _id: i64) -> Result<bool, IngestConfigError> {
            Ok(true)
        }
        async fn list_sources(
            &self,
            _section_id: i64,
        ) -> Result<Vec<IngestSourceResponse>, IngestConfigError> {
            Ok(vec![])
        }
        async fn create_source(
            &self,
            _section_id: i64,
            _s: kb_store::NewIngestSource,
        ) -> Result<IngestSourceResponse, IngestConfigError> {
            Ok(IngestSourceResponse {
                id: 1,
                section_id: 10,
                source_type: "scrape".into(),
                url: "https://example.com".into(),
                enabled: true,
                created_at: "2026-07-10T00:00:00Z".into(),
                coming_soon: false,
            })
        }
        async fn delete_source(&self, _id: i64) -> Result<bool, IngestConfigError> {
            Ok(true)
        }
        async fn list_section_documents(
            &self,
            section_id: i64,
        ) -> Result<Vec<IngestedDocumentResponse>, IngestConfigError> {
            if section_id == 999 {
                return Err(IngestConfigError::NotFound(format!("section {section_id}")));
            }
            Ok(vec![IngestedDocumentResponse {
                source_ref: "https://example.com/news/1".into(),
                source: "scrape".into(),
                chunk_count: 2,
                created_at: "2026-07-24 00:00:00".into(),
            }])
        }
    }

    #[tokio::test]
    async fn should_return_config_with_schedule() {
        let state = test_state();
        let result = get_config(State(state), session()).await;
        assert!(result.is_ok());
        let Json(config) = result.unwrap();
        assert!(config.schedule.is_some());
        assert!(config.sections.is_empty());
    }

    #[tokio::test]
    async fn should_upsert_schedule() {
        let state = test_state();
        let req = UpsertScheduleRequest {
            cron_expr: "0 */4 * * *".into(),
            enabled: true,
        };
        let result = upsert_schedule(State(state), session(), Json(req)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn should_create_section() {
        let state = test_state();
        let req = CreateSectionRequest {
            name: "sport".into(),
            ordering: 10,
        };
        let result = create_section(State(state), session(), Json(req)).await;
        assert!(result.is_ok());
        let (status, _) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
    }

    #[tokio::test]
    async fn should_delete_section() {
        let state = test_state();
        let result = delete_section(State(state), session(), Path(1)).await;
        assert!(result.is_ok());
        let Json(resp) = result.unwrap();
        assert!(resp.deleted);
    }

    #[tokio::test]
    async fn should_delete_source() {
        let state = test_state();
        let result = delete_source(State(state), session(), Path(1)).await;
        assert!(result.is_ok());
        let Json(resp) = result.unwrap();
        assert!(resp.deleted);
    }

    #[tokio::test]
    async fn should_list_section_documents() {
        let state = test_state();
        let result = list_section_documents(State(state), session(), Path(1)).await;
        assert!(result.is_ok());
        let Json(documents) = result.unwrap();
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].source_ref, "https://example.com/news/1");
    }

    #[tokio::test]
    async fn should_return_404_for_unknown_section_when_listing_documents() {
        let state = test_state();
        let result = list_section_documents(State(state), session(), Path(999)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
