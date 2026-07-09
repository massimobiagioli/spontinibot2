use std::sync::Arc;

pub use axum::Router;
use axum::routing::{get, post};

use crate::config::Config;
use crate::rag_engine::embedding::EmbeddingAdapter;
use crate::rag_engine::engine::RagEngine;
use crate::rag_engine::generation::GenerationAdapter;
use crate::rag_engine::persona::PersonaAdapter;
use crate::rag_engine::retrieval::RetrievalAdapter;

pub mod config;
pub mod rag_engine;
mod routes;

#[derive(Clone)]
pub struct AppState {
    pub rag_engine: Arc<RagEngine>,
}

pub async fn router() -> Router {
    let config = Config::from_env();
    let store = Arc::new(
        kb_store::KbStore::open(&config.kb_path)
            .await
            .expect("failed to open kb.db"),
    );

    let embedding: Arc<dyn crate::rag_engine::ports::EmbeddingPort> =
        Arc::new(EmbeddingAdapter::new(config.embed_url));
    let retrieval: Arc<dyn crate::rag_engine::ports::RetrievalPort> =
        Arc::new(RetrievalAdapter::new(store.clone()));
    let persona: Arc<dyn crate::rag_engine::ports::PersonaPort> =
        Arc::new(PersonaAdapter::new(store.clone()));
    let generation: Arc<dyn crate::rag_engine::ports::GenerationPort> =
        Arc::new(GenerationAdapter::new(config.generate_url));

    let rag_engine = Arc::new(RagEngine::new(
        embedding,
        retrieval,
        persona,
        generation,
        config.top_k,
        config.min_score,
    ));

    router_with(AppState { rag_engine })
}

pub fn router_with(state: AppState) -> Router {
    Router::new()
        .route("/health", get(routes::health))
        .route("/", get(routes::home))
        .route("/chat", post(routes::chat).with_state(state))
}
