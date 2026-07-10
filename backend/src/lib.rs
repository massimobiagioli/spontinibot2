use std::sync::Arc;

pub use axum::Router;
use axum::routing::{delete, get, post, put};

use crate::admin::ingest_config::adapter::KbStoreIngestConfigAdapter;
use crate::admin::ingest_config::handlers::IngestConfigState;
use crate::admin::upload::adapter::IngestCoreUploadAdapter;
use crate::admin::upload::ports::UploadPort;
use crate::admin::upload::preview_store::PreviewStore;
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

    // Create ingest pipeline for manual upload processing
    let kb_for_ingest = kb_store::KbStore::open(&config.kb_path)
        .await
        .expect("failed to open kb.db for ingest pipeline");
    let ingest_pipeline = Arc::new(
        ingest_core::pipeline::IngestPipeline::new(
            "spontini-backend".into(),
            config.embed_url.clone(),
            512,
            64,
            kb_for_ingest,
        )
        .expect("failed to create ingest pipeline"),
    );
    let upload_port: Arc<dyn UploadPort> = Arc::new(IngestCoreUploadAdapter::new(ingest_pipeline));
    let preview_store = Arc::new(PreviewStore::new(15));

    // Background eviction task for expired preview tokens
    {
        let preview_store = preview_store.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                preview_store.evict_expired();
            }
        });
    }

    let rag_engine = Arc::new(RagEngine::new(
        embedding,
        retrieval,
        persona,
        generation,
        config.top_k,
        config.min_score,
    ));

    let ingest_config_port: Arc<dyn crate::admin::ingest_config::IngestConfigAdminPort> =
        Arc::new(KbStoreIngestConfigAdapter::new(store.clone()));
    let ingest_config_state = IngestConfigState {
        ingest_config: ingest_config_port,
        config: config.clone(),
    };

    router_with(
        AppState { rag_engine },
        persona_admin,
        config,
        upload_port,
        preview_store,
        ingest_config_state,
    )
}

pub fn router_with(
    state: AppState,
    persona_admin: Arc<dyn PersonaAdminPort>,
    config: Config,
    upload: Arc<dyn UploadPort>,
    preview_store: Arc<PreviewStore>,
    ingest_config_state: IngestConfigState,
) -> Router {
    let admin_state = admin::AdminState {
        persona_admin,
        // Clone config for upload state below
        config: config.clone(),
    };

    let upload_state = admin::upload::handlers::UploadState {
        upload,
        preview_store,
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
        .route(
            "/admin/api/upload",
            post(admin::upload::handlers::upload_document).with_state(upload_state.clone()),
        )
        .route(
            "/admin/api/upload/preview/:token",
            get(admin::upload::handlers::get_preview).with_state(upload_state.clone()),
        )
        .route(
            "/admin/api/upload/confirm/:token",
            post(admin::upload::handlers::confirm_upload).with_state(upload_state),
        )
        .route(
            "/admin/api/ingest/config",
            get(admin::ingest_config::handlers::get_config).with_state(ingest_config_state.clone()),
        )
        .route(
            "/admin/api/ingest/config/schedule",
            put(admin::ingest_config::handlers::upsert_schedule)
                .with_state(ingest_config_state.clone()),
        )
        .route(
            "/admin/api/ingest/config/sections",
            post(admin::ingest_config::handlers::create_section)
                .with_state(ingest_config_state.clone()),
        )
        .route(
            "/admin/api/ingest/config/sections/:id",
            delete(admin::ingest_config::handlers::delete_section)
                .with_state(ingest_config_state.clone()),
        )
        .route(
            "/admin/api/ingest/config/sources",
            post(admin::ingest_config::handlers::create_source)
                .with_state(ingest_config_state.clone()),
        )
        .route(
            "/admin/api/ingest/config/sources/:id",
            delete(admin::ingest_config::handlers::delete_source).with_state(ingest_config_state),
        )
}
