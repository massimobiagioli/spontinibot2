mod routes;

use axum::Router;
use axum::routing::{get, post};

pub fn router() -> Router {
    Router::new()
        .route("/health", get(routes::health))
        .route("/", get(routes::home))
        .route("/chat", post(routes::chat))
}
