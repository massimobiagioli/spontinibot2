use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::rag_engine::types::RagError;

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
}

#[derive(Deserialize)]
pub struct ChatRequest {
    pub question: String,
}

#[derive(Serialize)]
pub struct ChatSource {
    pub document_id: i64,
    pub source_ref: String,
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub answer: String,
    pub sources: Vec<ChatSource>,
    pub fell_back: bool,
}

pub async fn health() -> impl IntoResponse {
    Json(HealthResponse { status: "ok" })
}

pub async fn home() -> impl IntoResponse {
    StatusCode::OK
}

pub async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, Json<serde_json::Value>)> {
    let answer = state
        .rag_engine
        .answer(&req.question)
        .await
        .map_err(|e| match e {
            RagError::NoActivePersona => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "no active persona configured"})),
            ),
            e => {
                tracing::error!(?e, "upstream error");
                (
                    StatusCode::BAD_GATEWAY,
                    Json(serde_json::json!({"error": "upstream service unavailable"})),
                )
            }
        })?;

    let sources = answer
        .sources
        .iter()
        .map(|s| ChatSource {
            document_id: s.document_id,
            source_ref: s.source_ref.clone(),
        })
        .collect();

    Ok(Json(ChatResponse {
        answer: answer.text,
        sources,
        fell_back: answer.fell_back,
    }))
}
