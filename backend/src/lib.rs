use std::sync::Arc;

pub use axum::Router;
use axum::routing::{get, post};

use crate::config::Config;
use crate::rag_engine::embedding::EmbeddingAdapter;
use crate::rag_engine::engine::RagEngine;
use crate::rag_engine::generation::GenerationAdapter;
use crate::rag_engine::persona::PersonaAdapter;
use crate::rag_engine::persona_admin::PersonaAdminAdapter;
use crate::rag_engine::ports::PersonaAdminPort;
use crate::rag_engine::retrieval::RetrievalAdapter;

pub mod admin;
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
        Arc::new(EmbeddingAdapter::new(config.embed_url.clone()));
    let retrieval: Arc<dyn crate::rag_engine::ports::RetrievalPort> =
        Arc::new(RetrievalAdapter::new(store.clone()));
    let persona: Arc<dyn crate::rag_engine::ports::PersonaPort> =
        Arc::new(PersonaAdapter::new(store.clone()));
    let persona_admin: Arc<dyn PersonaAdminPort> =
        Arc::new(PersonaAdminAdapter::new(store.clone(), persona.clone()));
    let generation: Arc<dyn crate::rag_engine::ports::GenerationPort> =
        Arc::new(GenerationAdapter::new(config.generate_url.clone()));

    let rag_engine = Arc::new(RagEngine::new(
        embedding,
        retrieval,
        persona,
        generation,
        config.top_k,
        config.min_score,
    ));

    router_with(AppState { rag_engine }, persona_admin, config)
}

pub fn router_with(
    state: AppState,
    persona_admin: Arc<dyn PersonaAdminPort>,
    config: Config,
) -> Router {
    let admin_state = admin::AdminState {
        persona_admin,
        config,
    };

    Router::new()
        .route("/health", get(routes::health))
        .route("/", get(routes::home))
        .route("/chat", post(routes::chat).with_state(state))
        .route(
            "/admin/api/persona",
            get(admin::list_persona_versions)
                .post(admin::create_persona)
                .with_state(admin_state.clone()),
        )
        .route(
            "/admin/api/persona/reload",
            post(admin::reload_persona).with_state(admin_state.clone()),
        )
        .route(
            "/admin/api/persona/:id/activate",
            post(admin::activate_persona).with_state(admin_state),
        )
}
