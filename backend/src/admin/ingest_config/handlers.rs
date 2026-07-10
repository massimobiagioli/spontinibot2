use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};

use crate::admin::ErrorResponse;
use crate::admin::check_admin_key;
use crate::admin::ingest_config::{
    IngestConfigAdminPort, IngestConfigError, IngestConfigResponse, IngestScheduleResponse,
    IngestSectionResponse, IngestSourceResponse,
};
use crate::config::Config;

#[derive(Clone)]
pub struct IngestConfigState {
    pub ingest_config: Arc<dyn IngestConfigAdminPort>,
    pub config: Config,
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
    headers: HeaderMap,
) -> Result<Json<IngestConfigResponse>, (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;

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
    headers: HeaderMap,
    Json(req): Json<UpsertScheduleRequest>,
) -> Result<Json<IngestScheduleResponse>, (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;

    let schedule = kb_store::NewIngestSchedule {
        cron_expr: req.cron_expr,
        enabled: req.enabled,
    };
    let response = state
        .ingest_config
        .upsert_schedule(schedule)
        .await
        .map_err(map_config_error)?;
    Ok(Json(response))
}

pub async fn create_section(
    State(state): State<IngestConfigState>,
    headers: HeaderMap,
    Json(req): Json<CreateSectionRequest>,
) -> Result<(StatusCode, Json<IngestSectionResponse>), (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;

    let section = kb_store::NewIngestSection {
        name: req.name,
        ordering: req.ordering,
    };
    let response = state
        .ingest_config
        .create_section(section)
        .await
        .map_err(map_config_error)?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn delete_section(
    State(state): State<IngestConfigState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<DeletedResponse>, (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;

    let deleted = state
        .ingest_config
        .delete_section(id)
        .await
        .map_err(map_config_error)?;
    Ok(Json(DeletedResponse { deleted }))
}

pub async fn create_source(
    State(state): State<IngestConfigState>,
    headers: HeaderMap,
    Query(query): Query<SectionIdQuery>,
    Json(req): Json<CreateSourceRequest>,
) -> Result<(StatusCode, Json<IngestSourceResponse>), (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;

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
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn delete_source(
    State(state): State<IngestConfigState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<DeletedResponse>, (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;

    let deleted = state
        .ingest_config
        .delete_source(id)
        .await
        .map_err(map_config_error)?;
    Ok(Json(DeletedResponse { deleted }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin::ingest_config::IngestScheduleResponse;

    fn test_state() -> IngestConfigState {
        let config = Config {
            embed_url: "http://localhost:8080".into(),
            generate_url: "http://localhost:8081".into(),
            kb_path: "/tmp/test.db".into(),
            top_k: 5,
            min_score: 0.35,
            admin_api_key: "test-key".into(),
            upload_max_bytes: 10_485_760,
        };
        IngestConfigState {
            ingest_config: Arc::new(MockIngestConfigAdmin),
            config,
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
    async fn should_reject_request_without_admin_key() {
        let state = test_state();
        let result = get_config(State(state), no_auth_headers()).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn should_return_config_with_schedule() {
        let state = test_state();
        let result = get_config(State(state), auth_headers()).await;
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
        let result = upsert_schedule(State(state), auth_headers(), Json(req)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn should_create_section() {
        let state = test_state();
        let req = CreateSectionRequest {
            name: "sport".into(),
            ordering: 10,
        };
        let result = create_section(State(state), auth_headers(), Json(req)).await;
        assert!(result.is_ok());
        let (status, _) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
    }

    #[tokio::test]
    async fn should_delete_section() {
        let state = test_state();
        let result = delete_section(State(state), auth_headers(), Path(1)).await;
        assert!(result.is_ok());
        let Json(resp) = result.unwrap();
        assert!(resp.deleted);
    }

    #[tokio::test]
    async fn should_delete_source() {
        let state = test_state();
        let result = delete_source(State(state), auth_headers(), Path(1)).await;
        assert!(result.is_ok());
        let Json(resp) = result.unwrap();
        assert!(resp.deleted);
    }
}
