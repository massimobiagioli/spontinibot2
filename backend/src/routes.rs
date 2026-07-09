use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct ChatResponse {
    answer: &'static str,
    sources: Vec<&'static str>,
}

pub async fn health() -> impl IntoResponse {
    Json(HealthResponse { status: "ok" })
}

pub async fn home() -> impl IntoResponse {
    StatusCode::OK
}

pub async fn chat() -> impl IntoResponse {
    Json(ChatResponse {
        answer: "(walking skeleton)",
        sources: Vec::new(),
    })
}
