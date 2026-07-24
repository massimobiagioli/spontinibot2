use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::rag_engine::ports::PersonaAdminPort;
use crate::rag_engine::types::{AdminPersonaSnapshot, NewPersonaRequest, RagError};

pub mod ingest_config;
pub mod ingest_run;
pub mod training_feedback;
pub mod training_messages;
pub mod training_sessions;
pub mod upload;

#[derive(Deserialize)]
pub struct ListVersionsQuery {
    pub name: String,
}

#[derive(Deserialize)]
pub struct CreatePersonaRequest {
    pub name: String,
    pub system_prompt: String,
    pub tone: Option<String>,
    pub fallback_message: Option<String>,
    pub activate: bool,
}

#[derive(Serialize)]
pub struct PersonaResponse {
    pub id: i64,
    pub version: i32,
    pub name: String,
    pub system_prompt: String,
    pub tone: Option<String>,
    pub fallback_message: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub created_by: Option<String>,
}

impl From<AdminPersonaSnapshot> for PersonaResponse {
    fn from(p: AdminPersonaSnapshot) -> Self {
        Self {
            id: p.id,
            version: p.version,
            name: p.name,
            system_prompt: p.system_prompt,
            tone: p.tone,
            fallback_message: p.fallback_message,
            is_active: p.is_active,
            created_at: p.created_at,
            created_by: p.created_by,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn check_admin_key(
    headers: &HeaderMap,
    config: &Config,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let header_val = headers
        .get("x-admin-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if header_val != config.admin_api_key {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "invalid or missing X-Admin-Key header".into(),
            }),
        ));
    }
    Ok(())
}

fn map_rag_error(e: RagError) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        RagError::PersonaNotFound => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        ),
    }
}

#[derive(Clone)]
pub struct AdminState {
    pub persona_admin: Arc<dyn PersonaAdminPort>,
    pub config: Config,
}

pub async fn list_persona_versions(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Query(query): Query<ListVersionsQuery>,
) -> Result<Json<Vec<PersonaResponse>>, (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;
    let versions = state
        .persona_admin
        .list_versions(&query.name)
        .await
        .map_err(map_rag_error)?;
    Ok(Json(
        versions.into_iter().map(PersonaResponse::from).collect(),
    ))
}

pub async fn create_persona(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Json(req): Json<CreatePersonaRequest>,
) -> Result<(StatusCode, Json<PersonaResponse>), (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;
    let domain_req = NewPersonaRequest {
        name: req.name,
        system_prompt: req.system_prompt,
        tone: req.tone,
        fallback_message: req.fallback_message,
        activate: req.activate,
        // TODO(0027): use authenticated admin identity
        created_by: Some("admin".into()),
    };
    let persona = state
        .persona_admin
        .insert_persona(domain_req)
        .await
        .map_err(map_rag_error)?;
    Ok((StatusCode::CREATED, Json(PersonaResponse::from(persona))))
}

pub async fn activate_persona(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;
    state
        .persona_admin
        .activate_persona(id)
        .await
        .map_err(map_rag_error)?;
    Ok(Json(serde_json::json!({"status": "activated"})))
}

pub async fn reload_persona(
    State(state): State<AdminState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;
    state
        .persona_admin
        .reload_persona()
        .await
        .map_err(map_rag_error)?;
    Ok(Json(serde_json::json!({"status": "reloaded"})))
}
