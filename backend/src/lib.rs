use std::sync::Arc;

pub use axum::Router;
use axum::extract::Extension;
use axum::routing::{delete, get, patch, post, put};

use crate::admin::ingest_config::adapter::KbStoreIngestConfigAdapter;
use crate::admin::ingest_config::handlers::IngestConfigState;
use crate::admin::ingest_manual::adapter::PipelineIngestManualAdapter;
use crate::admin::ingest_manual::composite_adapter::CuratingIngestManualAdapter;
use crate::admin::ingest_manual::halley::curation::HalleyCurationAdapter;
use crate::admin::ingest_manual::handlers::IngestManualState;
use crate::admin::ingest_run::adapter::KbStoreIngestRunAdapter;
use crate::admin::ingest_run::handlers::IngestRunState;
use crate::admin::scraper_options::adapter::KbStoreScraperOptionsAdapter;
use crate::admin::scraper_options::handlers::ScraperOptionsState;
use crate::admin::training_feedback::adapter::KbStoreTrainingFeedbackAdapter;
use crate::admin::training_feedback::handlers::TrainingFeedbackState;
use crate::admin::training_messages::adapter::RagTrainingMessageAdapter;
use crate::admin::training_messages::handlers::TrainingMessageState;
use crate::admin::training_sessions::adapter::KbStoreTrainingSessionAdapter;
use crate::admin::training_sessions::handlers::TrainingSessionState;
use crate::admin::upload::adapter::IngestCoreUploadAdapter;
use crate::admin::upload::ports::UploadPort;
use crate::admin::upload::preview_store::PreviewStore;
use crate::audit::AuditLogPort;
use crate::audit::adapter::KbStoreAuditLogAdapter;
use crate::auth::handlers::AuthState;
use crate::auth::session_store::SessionStore;
use crate::config::Config;
use crate::rag_engine::embedding::EmbeddingAdapter;
use crate::rag_engine::engine::RagEngine;
use crate::rag_engine::generation::GenerationAdapter;
use crate::rag_engine::persona::PersonaAdapter;
use crate::rag_engine::persona_admin::PersonaAdminAdapter;
use crate::rag_engine::ports::PersonaAdminPort;
use crate::rag_engine::retrieval::RetrievalAdapter;
use crate::rag_engine::training_notes::TrainingNotesAdapter;

pub mod admin;
pub mod audit;
pub mod auth;
pub mod config;
pub mod rag_engine;
mod routes;

#[derive(Clone)]
pub struct AppState {
    pub rag_engine: Arc<RagEngine>,
}

/// Bundles the per-endpoint-group admin states so `router_with` takes one
/// parameter instead of growing an argument per admin feature added.
#[derive(Clone)]
pub struct AdminRouterState {
    pub upload: admin::upload::handlers::UploadState,
    pub ingest_config: IngestConfigState,
    pub ingest_manual: IngestManualState,
    pub ingest_run: IngestRunState,
    pub scraper_options: ScraperOptionsState,
    pub training_sessions: TrainingSessionState,
    pub training_messages: TrainingMessageState,
    pub training_feedback: TrainingFeedbackState,
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
    let upload_port: Arc<dyn UploadPort> =
        Arc::new(IngestCoreUploadAdapter::new(ingest_pipeline.clone()));
    let preview_store = Arc::new(PreviewStore::new(15));
    let audit_port: Arc<dyn AuditLogPort> = Arc::new(KbStoreAuditLogAdapter::new(store.clone()));
    let upload_state = admin::upload::handlers::UploadState {
        upload: upload_port.clone(),
        preview_store: preview_store.clone(),
        config: config.clone(),
        audit: audit_port.clone(),
    };

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

    let training_notes: Arc<dyn crate::rag_engine::ports::TrainingNotesPort> =
        Arc::new(TrainingNotesAdapter::new(config.training_notes_dir.clone()));
    let rag_engine = Arc::new(
        RagEngine::new(
            embedding,
            retrieval,
            persona,
            generation,
            config.top_k,
            config.min_score,
        )
        .with_training_notes(training_notes),
    );

    let ingest_config_port: Arc<dyn crate::admin::ingest_config::IngestConfigAdminPort> =
        Arc::new(KbStoreIngestConfigAdapter::new(store.clone()));
    let ingest_config_state = IngestConfigState {
        ingest_config: ingest_config_port,
        audit: audit_port.clone(),
    };

    let ingest_run_port: Arc<dyn crate::admin::ingest_run::IngestRunAdminPort> =
        Arc::new(KbStoreIngestRunAdapter::new(store.clone()));
    let ingest_run_state = IngestRunState {
        ingest_run: ingest_run_port,
        audit: audit_port.clone(),
    };

    // Same shared `ingest_pipeline` instance the upload port uses above — this
    // is the "one shared service" both the admin-ui trigger and the `bin/ingest`
    // CLI call (Plan 0029), not a second, divergent ingestion code path.
    let scrape_manual_port: Arc<dyn crate::admin::ingest_manual::IngestManualAdminPort> =
        Arc::new(PipelineIngestManualAdapter::new(ingest_pipeline));
    // The Halley curation path (Plan 0030) reuses the same `store` and the
    // same `upload_port` the human-reviewed upload flow already uses. No
    // domain is hardcoded anywhere in this path — `config.curation_allowed_hosts`
    // (empty unless explicitly set, see Config::from_env) is the only source
    // of which hosts this dispatches to curation instead of the scrape path.
    let curation_manual_port: Arc<dyn crate::admin::ingest_manual::IngestManualAdminPort> =
        Arc::new(HalleyCurationAdapter::new(store.clone(), upload_port));
    let ingest_manual_port: Arc<dyn crate::admin::ingest_manual::IngestManualAdminPort> =
        Arc::new(CuratingIngestManualAdapter::new(
            scrape_manual_port,
            curation_manual_port,
            config.curation_allowed_hosts.clone(),
        ));
    let ingest_manual_state = IngestManualState {
        ingest_manual: ingest_manual_port,
        audit: audit_port.clone(),
    };

    let scraper_options_port: Arc<dyn crate::admin::scraper_options::ScraperOptionsAdminPort> =
        Arc::new(KbStoreScraperOptionsAdapter::new(store.clone()));
    let scraper_options_state = ScraperOptionsState {
        scraper_options: scraper_options_port,
        audit: audit_port.clone(),
    };

    let training_session_port: Arc<dyn crate::admin::training_sessions::TrainingSessionAdminPort> =
        Arc::new(KbStoreTrainingSessionAdapter::new(store.clone()));
    let training_session_state = TrainingSessionState {
        training_sessions: training_session_port,
        audit: audit_port.clone(),
    };

    let training_message_port: Arc<dyn crate::admin::training_messages::TrainingMessageAdminPort> =
        Arc::new(RagTrainingMessageAdapter::new(
            store.clone(),
            rag_engine.clone(),
        ));
    let training_message_state = TrainingMessageState {
        training_messages: training_message_port,
        audit: audit_port.clone(),
    };

    let training_feedback_port: Arc<
        dyn crate::admin::training_feedback::TrainingFeedbackAdminPort,
    > = Arc::new(KbStoreTrainingFeedbackAdapter::new(store.clone()));
    let training_feedback_state = TrainingFeedbackState {
        training_feedback: training_feedback_port,
        audit: audit_port.clone(),
    };

    let session_store = Arc::new(SessionStore::new(config.session_ttl_secs));

    router_with(
        AppState { rag_engine },
        persona_admin,
        config,
        session_store,
        audit_port,
        AdminRouterState {
            upload: upload_state,
            ingest_config: ingest_config_state,
            ingest_manual: ingest_manual_state,
            ingest_run: ingest_run_state,
            scraper_options: scraper_options_state,
            training_sessions: training_session_state,
            training_messages: training_message_state,
            training_feedback: training_feedback_state,
        },
    )
}

pub fn router_with(
    state: AppState,
    persona_admin: Arc<dyn PersonaAdminPort>,
    config: Config,
    session_store: Arc<SessionStore>,
    audit: Arc<dyn AuditLogPort>,
    admin_router_state: AdminRouterState,
) -> Router {
    let AdminRouterState {
        upload: upload_state,
        ingest_config: ingest_config_state,
        ingest_manual: ingest_manual_state,
        ingest_run: ingest_run_state,
        scraper_options: scraper_options_state,
        training_sessions: training_session_state,
        training_messages: training_message_state,
        training_feedback: training_feedback_state,
    } = admin_router_state;

    let admin_state = admin::AdminState {
        persona_admin,
        audit,
    };
    let auth_state = AuthState {
        config,
        session_store: session_store.clone(),
    };

    Router::new()
        .route("/health", get(routes::health))
        .route("/", get(routes::home))
        .route("/chat", post(routes::chat).with_state(state))
        .route(
            "/admin/api/auth/login",
            post(crate::auth::handlers::login).with_state(auth_state.clone()),
        )
        .route(
            "/admin/api/auth/logout",
            post(crate::auth::handlers::logout).with_state(auth_state),
        )
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
            post(admin::activate_persona).with_state(admin_state.clone()),
        )
        .route(
            "/admin/api/persona/:id",
            delete(admin::delete_persona).with_state(admin_state),
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
            "/admin/api/ingest/config/sections/:id/documents",
            get(admin::ingest_config::handlers::list_section_documents)
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
        .route(
            "/admin/api/ingest/run",
            post(admin::ingest_run::handlers::trigger_run).with_state(ingest_run_state.clone()),
        )
        .route(
            "/admin/api/ingest/run/:id",
            get(admin::ingest_run::handlers::get_run).with_state(ingest_run_state),
        )
        .route(
            "/admin/api/ingest/manual",
            post(admin::ingest_manual::handlers::ingest_manual).with_state(ingest_manual_state),
        )
        .route(
            "/admin/api/scraper/robots-bypass-hosts",
            get(admin::scraper_options::handlers::list_robots_bypass_hosts)
                .put(admin::scraper_options::handlers::replace_robots_bypass_hosts)
                .with_state(scraper_options_state),
        )
        .route(
            "/admin/api/training/sessions",
            post(admin::training_sessions::handlers::create_session)
                .get(admin::training_sessions::handlers::list_sessions)
                .with_state(training_session_state.clone()),
        )
        .route(
            "/admin/api/training/sessions/:id",
            get(admin::training_sessions::handlers::get_session)
                .delete(admin::training_sessions::handlers::delete_session)
                .with_state(training_session_state.clone()),
        )
        .route(
            "/admin/api/training/sessions/:id/close",
            post(admin::training_sessions::handlers::close_session)
                .with_state(training_session_state),
        )
        .route(
            "/admin/api/training/sessions/:id/messages",
            post(admin::training_messages::handlers::create_message)
                .get(admin::training_messages::handlers::list_messages)
                .with_state(training_message_state.clone()),
        )
        .route(
            "/admin/api/training/messages/:id",
            patch(admin::training_messages::handlers::update_expected_answer)
                .with_state(training_message_state),
        )
        .route(
            "/admin/api/training/feedback",
            post(admin::training_feedback::handlers::create_feedback)
                .with_state(training_feedback_state.clone()),
        )
        .route(
            "/admin/api/training/messages/:id/feedback",
            get(admin::training_feedback::handlers::list_feedback)
                .with_state(training_feedback_state),
        )
        .layer(Extension(session_store))
}
