use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use argon2::PasswordHasher;
use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use cucumber::{World as _, given, then, when};
use tower::ServiceExt;

use backend::AppState;
use backend::admin::ingest_config::handlers::IngestConfigState;
use backend::admin::ingest_config::{
    IngestConfigAdminPort, IngestConfigError, IngestScheduleResponse, IngestSectionResponse,
    IngestSourceResponse,
};
use backend::admin::ingest_manual::handlers::IngestManualState;
use backend::admin::ingest_manual::{
    IngestManualAdminPort, IngestManualError, IngestManualResponse, RecencyWindow,
};
use backend::admin::ingest_run::handlers::IngestRunState;
use backend::admin::ingest_run::{IngestRunAdminPort, IngestRunError, IngestRunResponse};
use backend::admin::scraper_options::handlers::ScraperOptionsState;
use backend::admin::scraper_options::{ScraperOptionsAdminPort, ScraperOptionsError};
use backend::admin::training_feedback::handlers::TrainingFeedbackState;
use backend::admin::training_feedback::{
    TrainingFeedbackAdminPort, TrainingFeedbackError, TrainingFeedbackResponse,
};
use backend::admin::training_messages::handlers::TrainingMessageState;
use backend::admin::training_messages::{
    TrainingMessageAdminPort, TrainingMessageError, TrainingMessageResponse,
};
use backend::admin::training_sessions::handlers::TrainingSessionState;
use backend::admin::training_sessions::{
    TrainingSessionAdminPort, TrainingSessionError, TrainingSessionResponse,
};
use backend::admin::upload::UploadError;
use backend::admin::upload::handlers::UploadState;
use backend::admin::upload::ports::UploadPort;
use backend::admin::upload::preview_store::PreviewStore;
use backend::audit::adapter::KbStoreAuditLogAdapter;
use backend::audit::{AuditError, AuditLogPort};
use backend::auth::session_store::SessionStore;
use backend::config::Config;
use backend::rag_engine::engine::RagEngine;
use backend::rag_engine::ports::{
    EmbeddingPort, GenerationPort, PersonaAdminPort, PersonaPort, RetrievalPort,
};
use backend::rag_engine::types::{
    AdminPersonaSnapshot, NewPersonaRequest, PersonaSnapshot, PromptParts, RagError, RetrievedChunk,
};

struct StubEmbedding;

#[async_trait]
impl EmbeddingPort for StubEmbedding {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>, RagError> {
        Ok(vec![0.1; 768])
    }
}

struct ConfigurableRetrieval {
    chunks: Vec<RetrievedChunk>,
}

#[async_trait]
impl RetrievalPort for ConfigurableRetrieval {
    async fn retrieve(
        &self,
        _qe: &[f32],
        _top_k: i64,
        _min_score: f64,
    ) -> Result<Vec<RetrievedChunk>, RagError> {
        Ok(self.chunks.clone())
    }
}

struct ConfigurablePersona {
    snapshot: Option<PersonaSnapshot>,
}

#[async_trait]
impl PersonaPort for ConfigurablePersona {
    async fn active_persona(&self) -> Result<Option<PersonaSnapshot>, RagError> {
        Ok(self.snapshot.clone())
    }

    async fn reload_persona(&self) -> Result<(), RagError> {
        Ok(())
    }
}

struct StubPersonaAdmin;

#[async_trait]
impl PersonaAdminPort for StubPersonaAdmin {
    async fn list_versions(&self, _name: &str) -> Result<Vec<AdminPersonaSnapshot>, RagError> {
        todo!()
    }
    async fn insert_persona(
        &self,
        _req: NewPersonaRequest,
    ) -> Result<AdminPersonaSnapshot, RagError> {
        todo!()
    }
    async fn activate_persona(&self, _id: i64) -> Result<(), RagError> {
        todo!()
    }
    async fn delete_persona(&self, _id: i64) -> Result<(), RagError> {
        todo!()
    }
    async fn reload_persona(&self) -> Result<(), RagError> {
        todo!()
    }
}

struct StubUploadPort;

#[async_trait]
impl UploadPort for StubUploadPort {
    async fn ingest_uploaded(
        &self,
        _text: &str,
        _section: &str,
        _filename: &str,
        _metadata: &backend::admin::upload::preview_store::UploadMetadata,
    ) -> Result<Vec<i64>, UploadError> {
        Ok(vec![])
    }
}

struct NoopAudit;

#[async_trait]
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

struct StubScraperOptionsAdmin;

#[async_trait]
impl ScraperOptionsAdminPort for StubScraperOptionsAdmin {
    async fn list_robots_bypass_hosts(
        &self,
    ) -> Result<Vec<backend::admin::scraper_options::RobotsBypassHostResponse>, ScraperOptionsError>
    {
        unimplemented!("stub — no BDD scenario in this suite exercises the scraper options API")
    }
    async fn replace_robots_bypass_hosts(
        &self,
        _hosts: Vec<String>,
    ) -> Result<Vec<backend::admin::scraper_options::RobotsBypassHostResponse>, ScraperOptionsError>
    {
        unimplemented!("stub — no BDD scenario in this suite exercises the scraper options API")
    }
}

struct StubIngestConfigAdmin;

#[async_trait]
impl IngestConfigAdminPort for StubIngestConfigAdmin {
    async fn get_schedule(&self) -> Result<Option<IngestScheduleResponse>, IngestConfigError> {
        Ok(None)
    }
    async fn upsert_schedule(
        &self,
        _schedule: kb_store::NewIngestSchedule,
    ) -> Result<IngestScheduleResponse, IngestConfigError> {
        unimplemented!("stub")
    }
    async fn list_sections(&self) -> Result<Vec<IngestSectionResponse>, IngestConfigError> {
        Ok(vec![])
    }
    async fn create_section(
        &self,
        _section: kb_store::NewIngestSection,
    ) -> Result<IngestSectionResponse, IngestConfigError> {
        unimplemented!("stub")
    }
    async fn delete_section(&self, _id: i64) -> Result<bool, IngestConfigError> {
        unimplemented!("stub")
    }
    async fn list_sources(
        &self,
        _section_id: i64,
    ) -> Result<Vec<IngestSourceResponse>, IngestConfigError> {
        Ok(vec![])
    }
    async fn list_curation_sources(
        &self,
        _section_id: i64,
    ) -> Result<Vec<backend::admin::ingest_config::CurationSourceResponse>, IngestConfigError> {
        Ok(vec![])
    }
    async fn create_source(
        &self,
        _section_id: i64,
        _source: kb_store::NewIngestSource,
    ) -> Result<IngestSourceResponse, IngestConfigError> {
        unimplemented!("stub")
    }
    async fn delete_source(&self, _id: i64) -> Result<bool, IngestConfigError> {
        unimplemented!("stub")
    }
    async fn list_section_documents(
        &self,
        _section_id: i64,
    ) -> Result<Vec<backend::admin::ingest_config::IngestedDocumentResponse>, IngestConfigError>
    {
        unimplemented!("stub")
    }
}

struct StubIngestManualAdmin;

#[async_trait]
impl IngestManualAdminPort for StubIngestManualAdmin {
    async fn ingest(
        &self,
        _section: &str,
        _src: &str,
        _window: RecencyWindow,
    ) -> Result<IngestManualResponse, IngestManualError> {
        unimplemented!("stub")
    }
}

struct StubIngestRunAdmin;

#[async_trait]
impl IngestRunAdminPort for StubIngestRunAdmin {
    async fn trigger_run(&self) -> Result<IngestRunResponse, IngestRunError> {
        unimplemented!("stub")
    }
    async fn get_run(&self, _id: i64) -> Result<Option<IngestRunResponse>, IngestRunError> {
        unimplemented!("stub")
    }
}

struct StubTrainingSessionAdmin;

#[async_trait]
impl TrainingSessionAdminPort for StubTrainingSessionAdmin {
    async fn create_session(
        &self,
        _req: kb_store::NewTrainingSession,
    ) -> Result<TrainingSessionResponse, TrainingSessionError> {
        unimplemented!("stub")
    }
    async fn list_sessions(&self) -> Result<Vec<TrainingSessionResponse>, TrainingSessionError> {
        Ok(vec![])
    }
    async fn get_session(
        &self,
        _id: i64,
    ) -> Result<Option<TrainingSessionResponse>, TrainingSessionError> {
        unimplemented!("stub")
    }
    async fn close_session(
        &self,
        _id: i64,
        _notes: Option<String>,
    ) -> Result<bool, TrainingSessionError> {
        unimplemented!("stub")
    }
    async fn delete_session(&self, _id: i64) -> Result<bool, TrainingSessionError> {
        unimplemented!("stub")
    }
}

struct StubTrainingMessageAdmin;

#[async_trait]
impl TrainingMessageAdminPort for StubTrainingMessageAdmin {
    async fn ask(
        &self,
        _session_id: i64,
        _req: backend::admin::training_messages::AskTrainingMessageRequest,
    ) -> Result<TrainingMessageResponse, TrainingMessageError> {
        unimplemented!("stub")
    }
    async fn list_messages(
        &self,
        _session_id: i64,
    ) -> Result<Vec<TrainingMessageResponse>, TrainingMessageError> {
        Ok(vec![])
    }
    async fn update_expected_answer(
        &self,
        _message_id: i64,
        _expected_answer: Option<String>,
    ) -> Result<TrainingMessageResponse, TrainingMessageError> {
        unimplemented!("stub")
    }
}

struct StubTrainingFeedbackAdmin;

#[async_trait]
impl TrainingFeedbackAdminPort for StubTrainingFeedbackAdmin {
    async fn create_feedback(
        &self,
        _req: kb_store::NewTrainingFeedback,
    ) -> Result<TrainingFeedbackResponse, TrainingFeedbackError> {
        unimplemented!("stub")
    }
    async fn list_feedback(
        &self,
        _message_id: i64,
    ) -> Result<Vec<TrainingFeedbackResponse>, TrainingFeedbackError> {
        Ok(vec![])
    }
}

#[derive(Debug)]
struct RecordingGeneration {
    call_count: AtomicUsize,
    last_prompt: std::sync::Mutex<Option<PromptParts>>,
}

#[async_trait]
impl GenerationPort for RecordingGeneration {
    async fn generate(&self, prompt: PromptParts) -> Result<String, RagError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        let mut last = self.last_prompt.lock().unwrap();
        *last = Some(prompt);
        Ok("Lo sportello anagrafe e' aperto dalle 9:00 alle 12:30.".into())
    }
}

#[derive(Debug, Default, cucumber::World)]
struct BotWorld {
    chunks: Vec<RetrievedChunk>,
    persona: Option<PersonaSnapshot>,
    generation: Option<Arc<RecordingGeneration>>,
    response_status: Option<u16>,
    response_body: Option<String>,
    admin_db_path: Option<String>,
    admin_session_cookie: Option<String>,
    response_set_cookie: Option<String>,
    persisted_router: Option<axum::Router>,
    upload_token: Option<String>,
    upload_db_path: Option<String>,
    upload_router: Option<axum::Router>,
    ingest_config_db_path: Option<String>,
    ingest_config_router: Option<axum::Router>,
    ingest_run_db_path: Option<String>,
    ingest_run_router: Option<axum::Router>,
    ingest_run_id: Option<i64>,
    ingest_manual_router: Option<axum::Router>,
    training_sessions_db_path: Option<String>,
    training_sessions_router: Option<axum::Router>,
    training_session_id: Option<i64>,
    training_message_id: Option<i64>,
    cited_chunk_id: Option<i64>,
}

impl Drop for BotWorld {
    fn drop(&mut self) {
        if let Some(ref path) = self.admin_db_path {
            let _ = std::fs::remove_file(path);
        }
        if let Some(ref path) = self.upload_db_path {
            let _ = std::fs::remove_file(path);
        }
        if let Some(ref path) = self.ingest_config_db_path {
            let _ = std::fs::remove_file(path);
        }
        if let Some(ref path) = self.ingest_run_db_path {
            let _ = std::fs::remove_file(path);
        }
        if let Some(ref path) = self.training_sessions_db_path {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Build an axum Router for the admin BDD scenarios.
/// Opens the KbStore at `db_path` and wires everything with stubs for
/// Embedding/Retrieval/Generation ports and a real PersonaAdminAdapter.
/// Builds a fully-wired admin router for BDD scenarios, plus a ready-to-use
/// `Cookie` header value for an already-authenticated operator session (most
/// scenarios test admin behavior, not the login flow itself, so tests skip
/// the HTTP login round-trip and seed a valid session directly). A real
/// operator credential file is also written — backed by `admin_key` as the
/// password — so the dedicated login/logout scenarios can exercise the real
/// `POST /admin/api/auth/login` flow against the same router.
async fn build_admin_router(db_path: &str, admin_key: &str) -> (axum::Router, String) {
    let store = Arc::new(
        kb_store::KbStore::open(db_path)
            .await
            .expect("failed to open test kb.db"),
    );
    let persona: Arc<dyn PersonaPort> = Arc::new(
        backend::rag_engine::persona::PersonaAdapter::new(store.clone()),
    );
    let persona_admin: Arc<dyn PersonaAdminPort> = Arc::new(
        backend::rag_engine::persona_admin::PersonaAdminAdapter::new(
            store.clone(),
            persona.clone(),
        ),
    );

    let credential_path = format!("{db_path}.operator-credential.json");
    let salt = argon2::password_hash::SaltString::generate(&mut rand::rngs::OsRng);
    let password_hash = argon2::Argon2::default()
        .hash_password(admin_key.as_bytes(), &salt)
        .expect("hash_password failed")
        .to_string();
    std::fs::write(
        &credential_path,
        format!(r#"{{"username":"operator","password_hash":"{password_hash}"}}"#),
    )
    .expect("failed to write test operator credential file");

    let config = Config {
        embed_url: "http://embed:8080".into(),
        generate_url: "http://generate:8080".into(),
        kb_path: db_path.into(),
        top_k: 5,
        min_score: 0.35,
        operator_credential_path: credential_path,
        operator_username: None,
        operator_password: None,
        session_ttl_secs: 1800,
        upload_max_bytes: 10_485_760,
        curation_allowed_hosts: vec!["halleyweb.com".to_string()],
        training_notes_dir: "/tmp/nonexistent-training-notes-dir".into(),
    };

    let rag_engine = Arc::new(RagEngine::new(
        Arc::new(StubEmbedding),
        Arc::new(ConfigurableRetrieval { chunks: vec![] }),
        persona,
        Arc::new(RecordingGeneration {
            call_count: AtomicUsize::new(0),
            last_prompt: std::sync::Mutex::new(None),
        }),
        5,
        0.35,
    ));

    let upload: Arc<dyn UploadPort> = Arc::new(StubUploadPort);
    let preview_store = Arc::new(PreviewStore::new(15));
    let upload_state = UploadState {
        upload,
        preview_store,
        config: config.clone(),
        audit: Arc::new(NoopAudit),
    };

    let ingest_config_port: Arc<dyn IngestConfigAdminPort> = Arc::new(
        backend::admin::ingest_config::adapter::KbStoreIngestConfigAdapter::new(store.clone()),
    );
    let ingest_config_state = IngestConfigState {
        ingest_config: ingest_config_port,
        audit: Arc::new(NoopAudit),
    };

    let ingest_run_port: Arc<dyn IngestRunAdminPort> =
        Arc::new(backend::admin::ingest_run::adapter::KbStoreIngestRunAdapter::new(store.clone()));
    let ingest_run_state = IngestRunState {
        ingest_run: ingest_run_port,
        audit: Arc::new(NoopAudit),
    };

    let training_session_port: Arc<dyn TrainingSessionAdminPort> = Arc::new(
        backend::admin::training_sessions::adapter::KbStoreTrainingSessionAdapter::new(
            store.clone(),
        ),
    );
    let training_session_state = TrainingSessionState {
        training_sessions: training_session_port,
        audit: Arc::new(NoopAudit),
    };

    let training_message_port: Arc<dyn TrainingMessageAdminPort> = Arc::new(
        backend::admin::training_messages::adapter::RagTrainingMessageAdapter::new(
            store.clone(),
            rag_engine.clone(),
        ),
    );
    let training_message_state = TrainingMessageState {
        training_messages: training_message_port,
        audit: Arc::new(NoopAudit),
    };

    let training_feedback_port: Arc<dyn TrainingFeedbackAdminPort> = Arc::new(
        backend::admin::training_feedback::adapter::KbStoreTrainingFeedbackAdapter::new(
            store.clone(),
        ),
    );
    let training_feedback_state = TrainingFeedbackState {
        training_feedback: training_feedback_port,
        audit: Arc::new(NoopAudit),
    };

    let session_store = Arc::new(SessionStore::new(config.session_ttl_secs));
    let session_token = session_store.insert("operator".into());
    let audit_port: Arc<dyn AuditLogPort> = Arc::new(KbStoreAuditLogAdapter::new(store.clone()));

    let router = backend::router_with(
        AppState { rag_engine },
        persona_admin,
        config,
        session_store,
        audit_port,
        backend::AdminRouterState {
            upload: upload_state,
            ingest_config: ingest_config_state,
            ingest_manual: stub_ingest_manual_state(),
            ingest_run: ingest_run_state,
            scraper_options: stub_scraper_options_state(),
            training_sessions: training_session_state,
            training_messages: training_message_state,
            training_feedback: training_feedback_state,
        },
    );

    (router, format!("session={session_token}"))
}

fn temp_db() -> String {
    let id: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let path = std::env::temp_dir()
        .join(format!("admin_bdd_{id}.db"))
        .to_string_lossy()
        .into_owned();
    let _ = std::fs::remove_file(&path);
    path
}

fn stub_ingest_config_state() -> IngestConfigState {
    IngestConfigState {
        ingest_config: Arc::new(StubIngestConfigAdmin),
        audit: Arc::new(NoopAudit),
    }
}

fn stub_scraper_options_state() -> ScraperOptionsState {
    ScraperOptionsState {
        scraper_options: Arc::new(StubScraperOptionsAdmin),
        audit: Arc::new(NoopAudit),
    }
}

fn stub_ingest_run_state() -> IngestRunState {
    IngestRunState {
        ingest_run: Arc::new(StubIngestRunAdmin),
        audit: Arc::new(NoopAudit),
    }
}

fn stub_ingest_manual_state() -> IngestManualState {
    IngestManualState {
        ingest_manual: Arc::new(StubIngestManualAdmin),
        audit: Arc::new(NoopAudit),
    }
}

fn stub_training_session_state() -> TrainingSessionState {
    TrainingSessionState {
        training_sessions: Arc::new(StubTrainingSessionAdmin),
        audit: Arc::new(NoopAudit),
    }
}

fn stub_training_message_state() -> TrainingMessageState {
    TrainingMessageState {
        training_messages: Arc::new(StubTrainingMessageAdmin),
        audit: Arc::new(NoopAudit),
    }
}

fn stub_training_feedback_state() -> TrainingFeedbackState {
    TrainingFeedbackState {
        training_feedback: Arc::new(StubTrainingFeedbackAdmin),
        audit: Arc::new(NoopAudit),
    }
}

#[given("the backend service is running")]
async fn given_backend_running(_world: &mut BotWorld) {}

#[when("the operator checks the service health")]
async fn when_check_health(world: &mut BotWorld) {
    let rag_engine = Arc::new(RagEngine::new(
        Arc::new(StubEmbedding),
        Arc::new(ConfigurableRetrieval { chunks: vec![] }),
        Arc::new(ConfigurablePersona { snapshot: None }),
        Arc::new(RecordingGeneration {
            call_count: AtomicUsize::new(0),
            last_prompt: std::sync::Mutex::new(None),
        }),
        5,
        0.35,
    ));
    let admin: Arc<dyn PersonaAdminPort> = Arc::new(StubPersonaAdmin);
    let config = Config {
        embed_url: "http://embed:8080".into(),
        generate_url: "http://generate:8080".into(),
        kb_path: "/tmp/test.db".into(),
        top_k: 5,
        min_score: 0.35,
        operator_credential_path: "/nonexistent-bdd-credential.json".into(),
        operator_username: None,
        operator_password: None,
        session_ttl_secs: 1800,
        upload_max_bytes: 10_485_760,
        curation_allowed_hosts: vec!["halleyweb.com".to_string()],
        training_notes_dir: "/tmp/nonexistent-training-notes-dir".into(),
    };
    let upload: Arc<dyn UploadPort> = Arc::new(StubUploadPort);
    let preview_store = Arc::new(PreviewStore::new(15));
    let upload_state = UploadState {
        upload,
        preview_store,
        config: config.clone(),
        audit: Arc::new(NoopAudit),
    };
    let ingest_config_state = stub_ingest_config_state();
    let ingest_run_state = stub_ingest_run_state();
    let training_session_state = stub_training_session_state();
    let training_message_state = stub_training_message_state();
    let training_feedback_state = stub_training_feedback_state();
    let router = backend::router_with(
        AppState { rag_engine },
        admin,
        config.clone(),
        Arc::new(SessionStore::new(config.session_ttl_secs)),
        Arc::new(NoopAudit),
        backend::AdminRouterState {
            upload: upload_state,
            ingest_config: ingest_config_state,
            ingest_manual: stub_ingest_manual_state(),
            ingest_run: ingest_run_state,
            scraper_options: stub_scraper_options_state(),
            training_sessions: training_session_state,
            training_messages: training_message_state,
            training_feedback: training_feedback_state,
        },
    );
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[then("the service reports it is ok")]
async fn then_service_ok(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(200));
    assert_eq!(world.response_body.as_deref(), Some(r#"{"status":"ok"}"#));
}

#[given(regex = r#"^the knowledge base contains a document titled "?([^"]+)"?$"#)]
fn given_document_title(world: &mut BotWorld, title: String) {
    world.chunks.push(RetrievedChunk {
        id: 1,
        content: "Lo sportello anagrafe e' aperto dal lunedi' al venerdi' dalle 9:00 alle 12:30"
            .into(),
        source_ref: title,
        similarity: 0.85,
        source_url: None,
    });
}

#[given(regex = r"^the document contains the text (.+)$")]
fn given_document_text(_world: &mut BotWorld, _text: String) {}

#[given(regex = r"^the knowledge base contains no document about (.+)$")]
fn given_no_document(_world: &mut BotWorld, _topic: String) {}

#[given("an active persona is configured with a system prompt and a fallback message")]
fn given_persona(world: &mut BotWorld) {
    world.persona = Some(PersonaSnapshot {
        name: "gaspare".into(),
        system_prompt: "Sei Gaspare Spontini, sindaco di Maiolati Spontini.".into(),
        fallback_message: Some(
            "Non ho trovato informazioni nei documenti comunali su questo argomento.".into(),
        ),
    });
}

#[when(regex = r#"^the citizen asks "(.+)"$"#)]
async fn when_citizen_asks(world: &mut BotWorld, question: String) {
    let counter = Arc::new(RecordingGeneration {
        call_count: AtomicUsize::new(0),
        last_prompt: std::sync::Mutex::new(None),
    });
    world.generation = Some(counter.clone());

    let admin: Arc<dyn PersonaAdminPort> = Arc::new(StubPersonaAdmin);
    let config = Config {
        embed_url: "http://embed:8080".into(),
        generate_url: "http://generate:8080".into(),
        kb_path: "/tmp/test.db".into(),
        top_k: 5,
        min_score: 0.35,
        operator_credential_path: "/nonexistent-bdd-credential.json".into(),
        operator_username: None,
        operator_password: None,
        session_ttl_secs: 1800,
        upload_max_bytes: 10_485_760,
        curation_allowed_hosts: vec!["halleyweb.com".to_string()],
        training_notes_dir: "/tmp/nonexistent-training-notes-dir".into(),
    };
    let upload: Arc<dyn UploadPort> = Arc::new(StubUploadPort);
    let preview_store = Arc::new(PreviewStore::new(15));
    let upload_state = UploadState {
        upload,
        preview_store,
        config: config.clone(),
        audit: Arc::new(NoopAudit),
    };
    let ingest_config_state = stub_ingest_config_state();
    let ingest_run_state = stub_ingest_run_state();
    let training_session_state = stub_training_session_state();
    let training_message_state = stub_training_message_state();
    let training_feedback_state = stub_training_feedback_state();
    let router = backend::router_with(
        {
            let rag_engine = Arc::new(RagEngine::new(
                Arc::new(StubEmbedding),
                Arc::new(ConfigurableRetrieval {
                    chunks: world.chunks.clone(),
                }),
                Arc::new(ConfigurablePersona {
                    snapshot: world.persona.clone(),
                }),
                counter,
                5,
                0.35,
            ));
            AppState { rag_engine }
        },
        admin,
        config.clone(),
        Arc::new(SessionStore::new(config.session_ttl_secs)),
        Arc::new(NoopAudit),
        backend::AdminRouterState {
            upload: upload_state,
            ingest_config: ingest_config_state,
            ingest_manual: stub_ingest_manual_state(),
            ingest_run: ingest_run_state,
            scraper_options: stub_scraper_options_state(),
            training_sessions: training_session_state,
            training_messages: training_message_state,
            training_feedback: training_feedback_state,
        },
    );

    let body = serde_json::json!({ "question": question });
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/chat")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[then("Spontini answers using the content of the retrieved document")]
async fn then_uses_document_content(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(200));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    let answer = body["answer"].as_str().unwrap();
    assert!(
        answer.contains("sportello"),
        "Answer should reference the document content, got: {answer}"
    );
}

#[then("Spontini cites the source document by title")]
async fn then_cites_source(world: &mut BotWorld) {
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    let sources = body["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(
        sources[0]["source_ref"].as_str().unwrap(),
        "Orari sportello anagrafe"
    );
    assert_eq!(body["fell_back"], false);
}

#[then(
    "the final prompt keeps the persona, retrieved context, and question as three separate parts"
)]
async fn then_prompt_parts_separated(world: &mut BotWorld) {
    let counter = world.generation.as_ref().unwrap();
    let call_count = counter.call_count.load(Ordering::SeqCst);

    if call_count == 0 {
        return;
    }

    let last_prompt = counter.last_prompt.lock().unwrap();
    let prompt = last_prompt.as_ref().expect("generation was not called");

    assert!(
        !prompt.system.contains("A che ore"),
        "system should not contain the question"
    );
    assert!(
        !prompt.context.contains("Sei Gaspare"),
        "context should not contain the persona"
    );
    assert!(
        !prompt.user.contains("sportello"),
        "user should not contain chunks"
    );
}

#[then("Spontini answers with the fallback message")]
async fn then_fallback_answer(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(200));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["fell_back"], true);
    let answer = body["answer"].as_str().unwrap();
    assert!(
        answer.contains("Non ho trovato informazioni"),
        "Expected fallback message, got: {answer}"
    );
}

#[then("Spontini does not cite any document")]
async fn then_no_citations(world: &mut BotWorld) {
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    let sources = body["sources"].as_array().unwrap();
    assert!(sources.is_empty());
}

#[then("Spontini does not invent any detail")]
async fn then_no_hallucination(world: &mut BotWorld) {
    let counter = world.generation.as_ref().unwrap();
    assert_eq!(
        counter.call_count.load(Ordering::SeqCst),
        0,
        "Generation should not be called in the honest-unknown path"
    );
}

#[then("Spontini answers instantly from its own persona, without retrieval or generation")]
async fn then_instant_identity_answer(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(200));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["fell_back"], false);
    let sources = body["sources"].as_array().unwrap();
    assert!(sources.is_empty(), "identity answers cite no KB document");

    let persona = world.persona.as_ref().expect("persona must be configured");
    assert_eq!(
        body["answer"].as_str().unwrap(),
        persona.system_prompt,
        "identity answer must be the persona's own system_prompt verbatim"
    );

    let counter = world.generation.as_ref().unwrap();
    assert_eq!(
        counter.call_count.load(Ordering::SeqCst),
        0,
        "generation must not be called for an identity question (ADR 0014)"
    );
}

// ---------------------------------------------------------------------------
// Admin persona BDD steps
// ---------------------------------------------------------------------------

const ADMIN_KEY: &str = "bdd-test-key";

#[given(regex = r#"^the knowledge base contains persona "([^"]+)" with (\d+) versions$"#)]
async fn given_persona_with_versions(world: &mut BotWorld, name: String, count: u32) {
    let path = temp_db();
    let store = kb_store::KbStore::open(&path)
        .await
        .expect("failed to open test db");

    for _ in 0..count {
        store
            .insert_persona(
                kb_store::NewPersona {
                    name: name.clone(),
                    system_prompt: format!("Sei {name}."),
                    tone: None,
                    fallback_message: None,
                    created_by: Some("admin".into()),
                },
                false,
            )
            .await
            .expect("insert failed");
    }

    drop(store);
    world.admin_db_path = Some(path);
}

#[given(regex = r#"^the knowledge base contains persona "([^"]+)" with version (\d+) active$"#)]
async fn given_persona_v1_active(world: &mut BotWorld, name: String, version: u32) {
    let path = temp_db();
    let store = kb_store::KbStore::open(&path)
        .await
        .expect("failed to open test db");

    // Insert the first version active
    store
        .insert_persona(
            kb_store::NewPersona {
                name: name.clone(),
                system_prompt: format!("Sei {name} v{version}."),
                tone: None,
                fallback_message: None,
                created_by: Some("admin".into()),
            },
            true,
        )
        .await
        .expect("insert failed");

    drop(store);
    world.admin_db_path = Some(path);
}

#[given(regex = r#"^persona "([^"]+)" has version (\d+) active and version (\d+) inactive$"#)]
async fn given_persona_v1_active_v2_inactive(
    world: &mut BotWorld,
    name: String,
    _active: u32,
    _inactive: u32,
) {
    let path = temp_db();
    let store = kb_store::KbStore::open(&path)
        .await
        .expect("failed to open test db");

    // Insert v1 active
    store
        .insert_persona(
            kb_store::NewPersona {
                name: name.clone(),
                system_prompt: format!("Sei {name} v1."),
                tone: None,
                fallback_message: None,
                created_by: Some("admin".into()),
            },
            true,
        )
        .await
        .expect("insert failed");

    // Insert v2 inactive
    store
        .insert_persona(
            kb_store::NewPersona {
                name: name.clone(),
                system_prompt: format!("Sei {name} v2."),
                tone: None,
                fallback_message: None,
                created_by: Some("admin".into()),
            },
            false,
        )
        .await
        .expect("insert failed");

    drop(store);
    world.admin_db_path = Some(path);
}

#[given("the persona cache contains the active persona")]
async fn given_cache_has_active_persona(world: &mut BotWorld) {
    let path = temp_db();
    let store = kb_store::KbStore::open(&path)
        .await
        .expect("failed to open test db");

    store
        .insert_persona(
            kb_store::NewPersona {
                name: "gaspare".into(),
                system_prompt: "Sei Gaspare v1.".into(),
                tone: None,
                fallback_message: None,
                created_by: Some("admin".into()),
            },
            true,
        )
        .await
        .expect("insert failed");

    drop(store);
    world.admin_db_path = Some(path);
}

#[when(regex = r#"^the operator creates a new version of persona "([^"]+)" with activation$"#)]
async fn when_insert_with_activate(world: &mut BotWorld, name: String) {
    let path = world.admin_db_path.as_ref().expect("no db path set");
    let (router, cookie) = build_admin_router(path, ADMIN_KEY).await;

    let body = serde_json::json!({
        "name": name,
        "system_prompt": format!("Sei {name}. (nuova versione)"),
        "activate": true,
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/api/persona")
                .header("content-type", "application/json")
                .header("cookie", &cookie)
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[when(regex = r#"^the operator requests all versions of persona "([^"]+)"$"#)]
async fn when_list_versions(world: &mut BotWorld, name: String) {
    let path = world.admin_db_path.as_ref().expect("no db path set");
    let (router, cookie) = build_admin_router(path, ADMIN_KEY).await;

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/admin/api/persona?name={name}"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[when(regex = r#"^the operator activates version (\d+) of persona "([^"]+)"$"#)]
async fn when_activate_version(world: &mut BotWorld, _version: u32, name: String) {
    let path = world.admin_db_path.as_ref().expect("no db path set");
    let (router, cookie) = build_admin_router(path, ADMIN_KEY).await;

    // We need the persona's id — fetch the list first
    let list_resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/admin/api/persona?name={name}"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let list_body_bytes = axum::body::to_bytes(list_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let list_body: serde_json::Value = serde_json::from_slice(&list_body_bytes).unwrap();
    let versions = list_body.as_array().unwrap();
    assert!(!versions.is_empty(), "no versions found for {name}");
    // Pick the *first* version (the one we're activating is version 2)
    let target = &versions[0];
    let id = target["id"].as_i64().unwrap();

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/admin/api/persona/{id}/activate"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[when(regex = r#"^the operator deletes version (\d+) of persona "([^"]+)"$"#)]
async fn when_delete_version(world: &mut BotWorld, version: i64, name: String) {
    let path = world.admin_db_path.as_ref().expect("no db path set");
    let (router, cookie) = build_admin_router(path, ADMIN_KEY).await;

    let list_resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/admin/api/persona?name={name}"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let list_body_bytes = axum::body::to_bytes(list_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let list_body: serde_json::Value = serde_json::from_slice(&list_body_bytes).unwrap();
    let versions = list_body.as_array().unwrap();
    let target = versions
        .iter()
        .find(|v| v["version"].as_i64() == Some(version))
        .unwrap_or_else(|| panic!("version {version} not found for persona {name}"));
    let id = target["id"].as_i64().unwrap();

    let response = router
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/admin/api/persona/{id}"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[when(regex = r"^the operator deletes persona version (\d+)$")]
async fn when_delete_unknown_persona_version(world: &mut BotWorld, id: i64) {
    let path = world.admin_db_path.as_ref().expect("no db path set");
    let (router, cookie) = build_admin_router(path, ADMIN_KEY).await;

    let response = router
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/admin/api/persona/{id}"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[then(regex = r#"^only (\d+) version of persona "([^"]+)" remains$"#)]
async fn then_n_versions_remain(world: &mut BotWorld, expected: usize, name: String) {
    let path = world.admin_db_path.as_ref().expect("no db path set");
    let (router, cookie) = build_admin_router(path, ADMIN_KEY).await;

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/admin/api/persona?name={name}"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let versions: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(versions.as_array().unwrap().len(), expected);
}

#[then("the request is rejected with 409")]
async fn then_rejected_409(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(409));
}

#[then("the request is rejected with 404")]
async fn then_rejected_404(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(404));
}

#[when("the operator reloads the persona cache")]
async fn when_reload_persona(world: &mut BotWorld) {
    let path = world.admin_db_path.as_ref().expect("no db path set");
    let (router, cookie) = build_admin_router(path, ADMIN_KEY).await;

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/api/persona/reload")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[then(regex = r"^(\d+) versions are returned$")]
async fn then_n_versions(world: &mut BotWorld, expected: usize) {
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    let versions = body.as_array().unwrap();
    assert_eq!(versions.len(), expected);
}

#[then("the latest version is listed first")]
async fn then_latest_first(world: &mut BotWorld) {
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    let versions = body.as_array().unwrap();
    assert!(versions.len() >= 2, "expected at least 2 versions");
    let v0 = versions[0]["version"].as_i64().unwrap();
    let v1 = versions[1]["version"].as_i64().unwrap();
    assert!(v0 > v1, "expected latest version first, got {v0} then {v1}");
}

#[then(regex = r"^version (\d+) becomes active$")]
async fn then_version_active(world: &mut BotWorld, _version: i64) {
    // Activate endpoint returns {"status":"activated"} with 200
    assert_eq!(world.response_status, Some(200));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["status"], "activated");
}

#[then(regex = r"^version (\d+) becomes inactive$")]
async fn then_version_inactive(world: &mut BotWorld, _version: i64) {
    // Fetch all versions and check ordering
    let path = world.admin_db_path.as_ref().expect("no db path set");
    let (router, cookie) = build_admin_router(path, ADMIN_KEY).await;

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/admin/api/persona?name=gaspare")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let versions: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let versions = versions.as_array().unwrap();

    // versions[0] is version 2 (now active), versions[1] is version 1 (now inactive)
    assert!(versions.len() >= 2, "expected at least 2 versions");
    assert!(versions[0]["is_active"].as_bool().unwrap());
    assert!(!versions[1]["is_active"].as_bool().unwrap());
}

#[then("the persona cache is refreshed")]
async fn then_cache_refreshed(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(200));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["status"], "reloaded");
}

#[when("the operator requests persona versions without admin key")]
async fn when_list_versions_no_key(world: &mut BotWorld) {
    let path = temp_db();
    let store = kb_store::KbStore::open(&path)
        .await
        .expect("failed to open test db");
    store
        .insert_persona(
            kb_store::NewPersona {
                name: "gaspare".into(),
                system_prompt: "Sei Gaspare.".into(),
                tone: None,
                fallback_message: None,
                created_by: Some("admin".into()),
            },
            false,
        )
        .await
        .expect("insert failed");
    drop(store);
    world.admin_db_path = Some(path);

    let (router, _cookie) =
        build_admin_router(world.admin_db_path.as_ref().unwrap(), ADMIN_KEY).await;
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/admin/api/persona?name=gaspare")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[then("the request is rejected with 401")]
async fn then_rejected_401(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(401));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    let error = body["error"].as_str().unwrap();
    assert!(
        error.contains("invalid or missing"),
        "Expected auth error message, got: {error}"
    );
}

#[then("the new persona version is active")]
async fn then_new_version_is_active(world: &mut BotWorld) {
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(world.response_status, Some(201));
    assert_eq!(body["is_active"], true);
}

#[then("the previous persona version is inactive")]
async fn then_previous_inactive(world: &mut BotWorld) {
    // Fetch all versions to verify
    let path = world.admin_db_path.as_ref().expect("no db path set");
    let (router, cookie) = build_admin_router(path, ADMIN_KEY).await;

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/admin/api/persona?name=gaspare")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let versions: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let versions = versions.as_array().unwrap();

    // versions[0] is the newest (active), versions[1] is the older (inactive)
    assert!(versions.len() >= 2, "expected at least 2 versions");
    assert!(versions[0]["is_active"].as_bool().unwrap());
    assert!(!versions[1]["is_active"].as_bool().unwrap());
}

// ---------------------------------------------------------------------------
// Auth / audit log BDD step definitions
// ---------------------------------------------------------------------------

#[when("the operator creates a persona version without a session")]
async fn when_create_persona_without_session(world: &mut BotWorld) {
    let path = temp_db();
    let (router, _cookie) = build_admin_router(&path, ADMIN_KEY).await;
    world.admin_db_path = Some(path);

    let body = serde_json::json!({
        "name": "gaspare",
        "system_prompt": "Sei Gaspare.",
        "activate": true,
    });

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/api/persona")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

async fn login_request(world: &mut BotWorld, password: &str) {
    let path = temp_db();
    let (router, _cookie) = build_admin_router(&path, ADMIN_KEY).await;
    world.admin_db_path = Some(path);

    let body = serde_json::json!({"username": "operator", "password": password});
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    world.response_set_cookie = response
        .headers()
        .get(axum::http::header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[when("the operator logs in with the correct password")]
async fn when_login_correct(world: &mut BotWorld) {
    login_request(world, ADMIN_KEY).await;
}

#[when("the operator logs in with an incorrect password")]
async fn when_login_incorrect(world: &mut BotWorld) {
    login_request(world, "wrong-password").await;
}

#[then("the login succeeds and a session cookie is set")]
async fn then_login_succeeds(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(200));
    let cookie = world
        .response_set_cookie
        .as_ref()
        .expect("expected a Set-Cookie header");
    assert!(cookie.starts_with("session="), "got: {cookie}");
    assert!(cookie.contains("HttpOnly"), "got: {cookie}");
}

#[then("the login is rejected with 401")]
async fn then_login_rejected(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(401));
}

#[then(regex = r#"^the audit log contains an entry for action "([^"]+)"$"#)]
async fn then_audit_log_contains_action(world: &mut BotWorld, action: String) {
    let path = world.admin_db_path.as_ref().expect("no db path set");
    let store = kb_store::KbStore::open(path)
        .await
        .expect("failed to open db");
    let entries = store
        .list_audit_entries()
        .await
        .expect("list_audit_entries failed");
    assert!(
        entries.iter().any(|e| e.action == action),
        "expected an audit entry for action {action}, got: {entries:?}"
    );
}

#[when("the operator logs out")]
async fn when_operator_logs_out(world: &mut BotWorld) {
    let path = temp_db();
    let (router, cookie) = build_admin_router(&path, ADMIN_KEY).await;
    world.admin_db_path = Some(path);

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/api/auth/logout")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);

    world.admin_session_cookie = Some(cookie);
    world.persisted_router = Some(router);
}

#[when("the operator requests persona versions again with the same, now-stale cookie")]
async fn when_list_versions_with_stale_cookie(world: &mut BotWorld) {
    let router = world
        .persisted_router
        .take()
        .expect("no persisted router — did you forget 'When the operator logs out'?");
    let cookie = world
        .admin_session_cookie
        .clone()
        .expect("no session cookie stored");

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/admin/api/persona?name=gaspare")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

// ---------------------------------------------------------------------------
// Upload BDD helpers
// ---------------------------------------------------------------------------

fn multipart_body(filename: &str, section: &str, content: &[u8]) -> (String, Vec<u8>) {
    let boundary = "----TestBoundary123";
    let mut body = Vec::new();

    // file field
    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"content-type: text/plain\r\n");
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(content);
    body.extend_from_slice(b"\r\n");

    // section field
    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"section\"\r\n");
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(section.as_bytes());
    body.extend_from_slice(b"\r\n");

    // close
    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"--\r\n");

    (boundary.to_string(), body)
}

async fn build_upload_router(db_path: &str, admin_key: &str) -> (axum::Router, String) {
    let store = Arc::new(
        kb_store::KbStore::open(db_path)
            .await
            .expect("failed to open test kb.db for upload"),
    );
    let persona: Arc<dyn PersonaPort> = Arc::new(
        backend::rag_engine::persona::PersonaAdapter::new(store.clone()),
    );
    let persona_admin: Arc<dyn PersonaAdminPort> = Arc::new(
        backend::rag_engine::persona_admin::PersonaAdminAdapter::new(store.clone(), persona),
    );

    let credential_path = format!("{db_path}.operator-credential.json");
    let salt = argon2::password_hash::SaltString::generate(&mut rand::rngs::OsRng);
    let password_hash = argon2::Argon2::default()
        .hash_password(admin_key.as_bytes(), &salt)
        .expect("hash_password failed")
        .to_string();
    std::fs::write(
        &credential_path,
        format!(r#"{{"username":"operator","password_hash":"{password_hash}"}}"#),
    )
    .expect("failed to write test operator credential file");

    let config = Config {
        embed_url: "http://embed:8080".into(),
        generate_url: "http://generate:8080".into(),
        kb_path: db_path.into(),
        top_k: 5,
        min_score: 0.35,
        operator_credential_path: credential_path,
        operator_username: None,
        operator_password: None,
        session_ttl_secs: 1800,
        upload_max_bytes: 10_485_760,
        curation_allowed_hosts: vec!["halleyweb.com".to_string()],
        training_notes_dir: "/tmp/nonexistent-training-notes-dir".into(),
    };

    let rag_engine = Arc::new(RagEngine::new(
        Arc::new(StubEmbedding),
        Arc::new(ConfigurableRetrieval { chunks: vec![] }),
        Arc::new(ConfigurablePersona { snapshot: None }),
        Arc::new(RecordingGeneration {
            call_count: AtomicUsize::new(0),
            last_prompt: std::sync::Mutex::new(None),
        }),
        5,
        0.35,
    ));

    let upload: Arc<dyn UploadPort> = Arc::new(StubUploadPort);
    let preview_store = Arc::new(PreviewStore::new(15));
    let upload_state = UploadState {
        upload,
        preview_store,
        config: config.clone(),
        audit: Arc::new(NoopAudit),
    };

    let ingest_config_port: Arc<dyn IngestConfigAdminPort> = Arc::new(
        backend::admin::ingest_config::adapter::KbStoreIngestConfigAdapter::new(store.clone()),
    );
    let ingest_config_state = IngestConfigState {
        ingest_config: ingest_config_port,
        audit: Arc::new(NoopAudit),
    };

    let ingest_run_port: Arc<dyn IngestRunAdminPort> =
        Arc::new(backend::admin::ingest_run::adapter::KbStoreIngestRunAdapter::new(store.clone()));
    let ingest_run_state = IngestRunState {
        ingest_run: ingest_run_port,
        audit: Arc::new(NoopAudit),
    };

    let training_session_port: Arc<dyn TrainingSessionAdminPort> = Arc::new(
        backend::admin::training_sessions::adapter::KbStoreTrainingSessionAdapter::new(
            store.clone(),
        ),
    );
    let training_session_state = TrainingSessionState {
        training_sessions: training_session_port,
        audit: Arc::new(NoopAudit),
    };

    let training_message_port: Arc<dyn TrainingMessageAdminPort> = Arc::new(
        backend::admin::training_messages::adapter::RagTrainingMessageAdapter::new(
            store.clone(),
            rag_engine.clone(),
        ),
    );
    let training_message_state = TrainingMessageState {
        training_messages: training_message_port,
        audit: Arc::new(NoopAudit),
    };

    let training_feedback_port: Arc<dyn TrainingFeedbackAdminPort> = Arc::new(
        backend::admin::training_feedback::adapter::KbStoreTrainingFeedbackAdapter::new(
            store.clone(),
        ),
    );
    let training_feedback_state = TrainingFeedbackState {
        training_feedback: training_feedback_port,
        audit: Arc::new(NoopAudit),
    };

    let session_store = Arc::new(SessionStore::new(config.session_ttl_secs));
    let session_token = session_store.insert("operator".into());
    let audit_port: Arc<dyn AuditLogPort> = Arc::new(KbStoreAuditLogAdapter::new(store.clone()));

    let router = backend::router_with(
        AppState { rag_engine },
        persona_admin,
        config,
        session_store,
        audit_port,
        backend::AdminRouterState {
            upload: upload_state,
            ingest_config: ingest_config_state,
            ingest_manual: stub_ingest_manual_state(),
            ingest_run: ingest_run_state,
            scraper_options: stub_scraper_options_state(),
            training_sessions: training_session_state,
            training_messages: training_message_state,
            training_feedback: training_feedback_state,
        },
    );

    (router, format!("session={session_token}"))
}

// ---------------------------------------------------------------------------
// Upload BDD step definitions
// ---------------------------------------------------------------------------

#[given("a persona is configured in the knowledge base")]
async fn given_persona_configured(world: &mut BotWorld) {
    let path = temp_db();
    let store = kb_store::KbStore::open(&path)
        .await
        .expect("failed to open test db");

    store
        .insert_persona(
            kb_store::NewPersona {
                name: "gaspare".into(),
                system_prompt: "Sei Gaspare Spontini.".into(),
                tone: None,
                fallback_message: Some("Non ho trovato informazioni.".into()),
                created_by: Some("admin".into()),
            },
            true,
        )
        .await
        .expect("insert persona failed");

    drop(store);
    world.upload_db_path = Some(path);
}

#[given("the backend service has the upload API enabled")]
async fn given_upload_api_enabled(world: &mut BotWorld) {
    let db_path = world
        .upload_db_path
        .as_ref()
        .cloned()
        .unwrap_or_else(temp_db);
    if world.upload_db_path.is_none() {
        world.upload_db_path = Some(db_path.clone());
    }
    let (router, cookie) = build_upload_router(&db_path, "test-key").await;
    world.upload_router = Some(router);
    world.admin_session_cookie = Some(cookie);
}

#[when(regex = r#"^the operator uploads a file "([^"]+)" with section "([^"]+)"$"#)]
async fn when_upload_file(world: &mut BotWorld, filename: String, section: String) {
    let router = world
        .upload_router
        .as_ref()
        .expect("upload router not initialized — did you forget 'Given the backend service has the upload API enabled'?")
        .clone();
    let content = match filename.rsplit('.').next().unwrap_or("") {
        "md" | "markdown" => b"# Test Article\n\nThis is the content of the test article for upload.\n\nIt contains multiple paragraphs to simulate a real document.\n".to_vec(),
        _ => b"fake image bytes".to_vec(),
    };
    let (boundary, body) = multipart_body(&filename, &section, &content);

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/api/upload")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .header("cookie", world.admin_session_cookie.as_ref().unwrap())
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[then("the upload returns a preview token")]
async fn then_upload_returns_token(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(201));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    let token = body["token"].as_str().expect("expected token field");
    assert!(!token.is_empty(), "token should not be empty");
    world.upload_token = Some(token.to_string());
}

#[when("the operator requests the preview with that token")]
async fn when_get_preview(world: &mut BotWorld) {
    let token = world.upload_token.as_ref().expect("no token available");
    let router = world
        .upload_router
        .as_ref()
        .expect("upload router not initialized")
        .clone();

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/admin/api/upload/preview/{token}"))
                .header("cookie", world.admin_session_cookie.as_ref().unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[then("the preview shows the extracted text and metadata")]
async fn then_preview_shows_text(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(200));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert!(
        body["extracted_text"]
            .as_str()
            .unwrap()
            .contains("Test Article"),
        "preview should contain extracted text"
    );
    assert_eq!(body["section"], "news");
    assert_eq!(body["format"], "markdown");
    assert!(body["chunk_count_estimate"].as_u64().unwrap() >= 1);
}

#[then(regex = r#"^the preview metadata has category "([^"]+)" and trust score ([0-9.]+)$"#)]
async fn then_preview_metadata_category_and_trust_score(
    world: &mut BotWorld,
    category: String,
    trust_score: f64,
) {
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(
        body["metadata"]["category"].as_str(),
        Some(category.as_str())
    );
    assert_eq!(body["metadata"]["trust_score"].as_f64(), Some(trust_score));
}

#[then("the preview metadata tags are derived from the document content")]
async fn then_preview_metadata_tags_derived(world: &mut BotWorld) {
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    let tags = body["metadata"]["tags"]
        .as_array()
        .expect("expected a tags array derived from the uploaded content");
    assert!(!tags.is_empty(), "expected at least one derived tag");
    assert!(
        tags.iter().any(|t| t.as_str() == Some("article")),
        "expected 'article' among the tags derived from the test article content, got {tags:?}"
    );
}

#[when("the operator confirms the upload with that token")]
async fn when_confirm_upload(world: &mut BotWorld) {
    let token = world.upload_token.as_ref().expect("no token available");
    let router = world
        .upload_router
        .as_ref()
        .expect("upload router not initialized")
        .clone();

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/admin/api/upload/confirm/{token}"))
                .header("cookie", world.admin_session_cookie.as_ref().unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[then("the confirm response includes document IDs and a chunk count")]
async fn then_confirm_returns_ids(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(200));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert!(
        body["document_ids"].is_array(),
        "expected document_ids array"
    );
    assert!(
        body["chunk_count"].as_u64().is_some(),
        "expected chunk_count"
    );
}

#[then("the upload is rejected with an unsupported format error")]
async fn then_unsupported_format(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(400));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert!(
        body["error"].as_str().unwrap().contains("unsupported"),
        "error should mention unsupported format"
    );
}

#[when(
    regex = r#"^the operator uploads a file "([^"]+)" with section "([^"]+)" without admin key$"#
)]
async fn when_upload_no_key(world: &mut BotWorld, filename: String, section: String) {
    let router = world
        .upload_router
        .as_ref()
        .expect("upload router not initialized")
        .clone();
    let content = b"plain text";
    let (boundary, body) = multipart_body(&filename, &section, content);

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/api/upload")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

// ---------------------------------------------------------------------------
// Ingest config BDD step definitions
// ---------------------------------------------------------------------------

#[given("the ingest config API is available")]
async fn given_ingest_config_api_available(world: &mut BotWorld) {
    let path = temp_db();
    let store = Arc::new(
        kb_store::KbStore::open(&path)
            .await
            .expect("failed to open test kb.db for ingest config"),
    );
    let persona: Arc<dyn PersonaPort> = Arc::new(
        backend::rag_engine::persona::PersonaAdapter::new(store.clone()),
    );
    let persona_admin: Arc<dyn PersonaAdminPort> = Arc::new(
        backend::rag_engine::persona_admin::PersonaAdminAdapter::new(store.clone(), persona),
    );

    let config = Config {
        embed_url: "http://embed:8080".into(),
        generate_url: "http://generate:8080".into(),
        kb_path: path.clone(),
        top_k: 5,
        min_score: 0.35,
        operator_credential_path: "/nonexistent-bdd-credential.json".into(),
        operator_username: None,
        operator_password: None,
        session_ttl_secs: 1800,
        upload_max_bytes: 10_485_760,
        curation_allowed_hosts: vec!["halleyweb.com".to_string()],
        training_notes_dir: "/tmp/nonexistent-training-notes-dir".into(),
    };

    let rag_engine = Arc::new(RagEngine::new(
        Arc::new(StubEmbedding),
        Arc::new(ConfigurableRetrieval { chunks: vec![] }),
        Arc::new(ConfigurablePersona { snapshot: None }),
        Arc::new(RecordingGeneration {
            call_count: AtomicUsize::new(0),
            last_prompt: std::sync::Mutex::new(None),
        }),
        5,
        0.35,
    ));

    let upload: Arc<dyn UploadPort> = Arc::new(StubUploadPort);
    let preview_store = Arc::new(PreviewStore::new(15));
    let upload_state = UploadState {
        upload,
        preview_store,
        config: config.clone(),
        audit: Arc::new(NoopAudit),
    };

    let ingest_config_port: Arc<dyn IngestConfigAdminPort> = Arc::new(
        backend::admin::ingest_config::adapter::KbStoreIngestConfigAdapter::new(store.clone()),
    );
    let ingest_config_state = IngestConfigState {
        ingest_config: ingest_config_port,
        audit: Arc::new(NoopAudit),
    };

    let ingest_run_port: Arc<dyn IngestRunAdminPort> =
        Arc::new(backend::admin::ingest_run::adapter::KbStoreIngestRunAdapter::new(store.clone()));
    let ingest_run_state = IngestRunState {
        ingest_run: ingest_run_port,
        audit: Arc::new(NoopAudit),
    };

    let training_session_port: Arc<dyn TrainingSessionAdminPort> = Arc::new(
        backend::admin::training_sessions::adapter::KbStoreTrainingSessionAdapter::new(
            store.clone(),
        ),
    );
    let training_session_state = TrainingSessionState {
        training_sessions: training_session_port,
        audit: Arc::new(NoopAudit),
    };

    let training_message_port: Arc<dyn TrainingMessageAdminPort> = Arc::new(
        backend::admin::training_messages::adapter::RagTrainingMessageAdapter::new(
            store.clone(),
            rag_engine.clone(),
        ),
    );
    let training_message_state = TrainingMessageState {
        training_messages: training_message_port,
        audit: Arc::new(NoopAudit),
    };

    let training_feedback_port: Arc<dyn TrainingFeedbackAdminPort> = Arc::new(
        backend::admin::training_feedback::adapter::KbStoreTrainingFeedbackAdapter::new(
            store.clone(),
        ),
    );
    let training_feedback_state = TrainingFeedbackState {
        training_feedback: training_feedback_port,
        audit: Arc::new(NoopAudit),
    };

    let session_store = Arc::new(SessionStore::new(config.session_ttl_secs));
    let session_token = session_store.insert("operator".into());
    world.admin_session_cookie = Some(format!("session={session_token}"));
    let audit_port: Arc<dyn AuditLogPort> = Arc::new(KbStoreAuditLogAdapter::new(store.clone()));

    let router = backend::router_with(
        AppState { rag_engine },
        persona_admin,
        config,
        session_store,
        audit_port,
        backend::AdminRouterState {
            upload: upload_state,
            ingest_config: ingest_config_state,
            ingest_manual: stub_ingest_manual_state(),
            ingest_run: ingest_run_state,
            scraper_options: stub_scraper_options_state(),
            training_sessions: training_session_state,
            training_messages: training_message_state,
            training_feedback: training_feedback_state,
        },
    );

    world.ingest_config_db_path = Some(path);
    world.ingest_config_router = Some(router);
}

#[given(regex = r#"^an ingest section "([^"]+)" exists$"#)]
async fn given_ingest_section_exists(world: &mut BotWorld, name: String) {
    let router = world
        .ingest_config_router
        .as_ref()
        .expect("ingest config router not initialized")
        .clone();

    let body = serde_json::json!({ "name": name, "ordering": 10 });
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/api/ingest/config/sections")
                .header("cookie", world.admin_session_cookie.as_ref().unwrap())
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 201);
}

#[given(regex = r#"^a scrape source exists in section "([^"]+)"$"#)]
async fn given_scrape_source_exists_in_section(world: &mut BotWorld, section_name: String) {
    let router = world
        .ingest_config_router
        .as_ref()
        .expect("ingest config router not initialized")
        .clone();

    let body = serde_json::json!({
        "source_type": "scrape",
        "url": "https://example.com/news",
        "enabled": true,
    });
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/admin/api/ingest/config/sources?section_id={}",
                    find_section_id(world, &section_name).await
                ))
                .header("cookie", world.admin_session_cookie.as_ref().unwrap())
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 201);
}

async fn find_section_id(world: &mut BotWorld, section_name: &str) -> i64 {
    let router = world
        .ingest_config_router
        .as_ref()
        .expect("ingest config router not initialized")
        .clone();

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/admin/api/ingest/config")
                .header("cookie", world.admin_session_cookie.as_ref().unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let config: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    for section in config["sections"].as_array().unwrap() {
        if section["name"].as_str().unwrap() == section_name {
            return section["id"].as_i64().unwrap();
        }
    }
    panic!("section '{section_name}' not found");
}

async fn find_first_source_id(world: &mut BotWorld, section_name: &str) -> i64 {
    let router = world
        .ingest_config_router
        .as_ref()
        .expect("ingest config router not initialized")
        .clone();

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/admin/api/ingest/config")
                .header("cookie", world.admin_session_cookie.as_ref().unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let config: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    for section in config["sections"].as_array().unwrap() {
        if section["name"].as_str().unwrap() == section_name {
            let sources = section["sources"].as_array().unwrap();
            if let Some(first) = sources.first() {
                return first["id"].as_i64().unwrap();
            }
        }
    }
    panic!("no source found in section '{section_name}'");
}

#[when("the operator gets the ingest configuration")]
async fn when_get_ingest_config(world: &mut BotWorld) {
    let router = world
        .ingest_config_router
        .as_ref()
        .expect("ingest config router not initialized")
        .clone();

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/admin/api/ingest/config")
                .header("cookie", world.admin_session_cookie.as_ref().unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[when(regex = r#"^the operator sets the ingest schedule to "([^"]+)" (enabled|disabled)$"#)]
async fn when_set_ingest_schedule(world: &mut BotWorld, cron_expr: String, enabled_str: String) {
    let router = world
        .ingest_config_router
        .as_ref()
        .expect("ingest config router not initialized")
        .clone();

    let body = serde_json::json!({
        "cron_expr": cron_expr,
        "enabled": enabled_str == "enabled",
    });
    let response = router
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/admin/api/ingest/config/schedule")
                .header("cookie", world.admin_session_cookie.as_ref().unwrap())
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[when(regex = r#"^the operator creates an ingest section "([^"]+)" with ordering (\d+)$"#)]
async fn when_create_ingest_section(world: &mut BotWorld, name: String, ordering: i32) {
    let router = world
        .ingest_config_router
        .as_ref()
        .expect("ingest config router not initialized")
        .clone();

    let body = serde_json::json!({ "name": name, "ordering": ordering });
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/api/ingest/config/sections")
                .header("cookie", world.admin_session_cookie.as_ref().unwrap())
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[when(regex = r#"^the operator creates an? (scrape|api) source "([^"]+)" in section "([^"]+)"$"#)]
async fn when_create_source(
    world: &mut BotWorld,
    source_type: String,
    url: String,
    section_name: String,
) {
    let router = world
        .ingest_config_router
        .as_ref()
        .expect("ingest config router not initialized")
        .clone();

    let section_id = find_section_id(world, &section_name).await;
    let body = serde_json::json!({
        "source_type": source_type,
        "url": url,
        "enabled": true,
    });
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/admin/api/ingest/config/sources?section_id={section_id}"
                ))
                .header("cookie", world.admin_session_cookie.as_ref().unwrap())
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[when(regex = r#"^the operator deletes the source from section "([^"]+)"$"#)]
async fn when_delete_source(world: &mut BotWorld, section_name: String) {
    let router = world
        .ingest_config_router
        .as_ref()
        .expect("ingest config router not initialized")
        .clone();

    let source_id = find_first_source_id(world, &section_name).await;
    let response = router
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/admin/api/ingest/config/sources/{source_id}"))
                .header("cookie", world.admin_session_cookie.as_ref().unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[when(regex = r#"^the operator deletes section "([^"]+)"$"#)]
async fn when_delete_section(world: &mut BotWorld, section_name: String) {
    let router = world
        .ingest_config_router
        .as_ref()
        .expect("ingest config router not initialized")
        .clone();

    let section_id = find_section_id(world, &section_name).await;
    let response = router
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/admin/api/ingest/config/sections/{section_id}"))
                .header("cookie", world.admin_session_cookie.as_ref().unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[given(regex = r#"^a curation bookmark exists for section "([^"]+)" at source "([^"]+)"$"#)]
async fn given_curation_bookmark_exists(
    world: &mut BotWorld,
    section_name: String,
    source_url: String,
) {
    let path = world
        .ingest_config_db_path
        .clone()
        .expect("ingest config API not initialized yet");
    let store = kb_store::KbStore::open(&path)
        .await
        .expect("failed to reopen test kb.db");
    let sections = store.list_sections().await.expect("list_sections failed");
    let section = sections
        .iter()
        .find(|s| s.name == section_name)
        .unwrap_or_else(|| panic!("section '{section_name}' not found"));
    store
        .upsert_bookmark(section.id, &source_url, "74", "2026-07-13")
        .await
        .expect("upsert_bookmark failed");
}

#[given(
    regex = r#"^a document has been ingested into section "([^"]+)" with source ref "([^"]+)"$"#
)]
async fn given_document_ingested_into_section(
    world: &mut BotWorld,
    section_name: String,
    source_ref: String,
) {
    let path = world
        .ingest_config_db_path
        .clone()
        .expect("ingest config API not initialized yet");
    let store = kb_store::KbStore::open(&path)
        .await
        .expect("failed to reopen test kb.db");
    store
        .insert_document(kb_store::NewDocument {
            source: kb_store::DocumentSource::Scrape,
            source_ref,
            content: "contenuto".into(),
            metadata: None,
            embedding: vec![0.0; kb_store::EMBEDDING_DIM],
            section: Some(section_name),
        })
        .await
        .expect("insert_document failed");
}

#[when(regex = r#"^the operator lists the documents ingested into section "([^"]+)"$"#)]
async fn when_list_section_documents(world: &mut BotWorld, section_name: String) {
    let router = world
        .ingest_config_router
        .as_ref()
        .expect("ingest config router not initialized")
        .clone();

    let section_id = find_section_id(world, &section_name).await;
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/admin/api/ingest/config/sections/{section_id}/documents"
                ))
                .header("cookie", world.admin_session_cookie.as_ref().unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[when("the operator lists the documents ingested into unknown section 999999")]
async fn when_list_unknown_section_documents(world: &mut BotWorld) {
    let router = world
        .ingest_config_router
        .as_ref()
        .expect("ingest config router not initialized")
        .clone();

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/admin/api/ingest/config/sections/999999/documents")
                .header("cookie", world.admin_session_cookie.as_ref().unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[then(
    regex = r#"^the ingested documents list for "([^"]+)" contains "([^"]+)" with (\d+) chunks?$"#
)]
async fn then_ingested_documents_list_contains(
    world: &mut BotWorld,
    _section_name: String,
    source_ref: String,
    chunk_count: i64,
) {
    assert_eq!(world.response_status, Some(200));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    let documents = body.as_array().expect("expected an array response");
    let found = documents
        .iter()
        .find(|d| d["source_ref"].as_str() == Some(source_ref.as_str()))
        .unwrap_or_else(|| panic!("expected an ingested document with source_ref {source_ref}"));
    assert_eq!(found["chunk_count"].as_i64(), Some(chunk_count));
}

async fn fetch_ingest_config(world: &mut BotWorld) -> serde_json::Value {
    let router = world
        .ingest_config_router
        .as_ref()
        .expect("ingest config router not initialized")
        .clone();

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/admin/api/ingest/config")
                .header("cookie", world.admin_session_cookie.as_ref().unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body_bytes).unwrap()
}

#[then("the ingest configuration has no schedule")]
async fn then_no_schedule(world: &mut BotWorld) {
    let body = fetch_ingest_config(world).await;
    assert!(
        body["schedule"].is_null(),
        "expected null schedule, got: {}",
        body["schedule"]
    );
}

#[then("the ingest configuration has no sections")]
async fn then_no_sections(world: &mut BotWorld) {
    let body = fetch_ingest_config(world).await;
    let sections = body["sections"]
        .as_array()
        .expect("sections should be an array");
    assert!(
        sections.is_empty(),
        "expected 0 sections, got: {}",
        sections.len()
    );
}

#[then(regex = r#"^the ingest configuration has a schedule with cron "([^"]+)"$"#)]
async fn then_schedule_cron(world: &mut BotWorld, expected_cron: String) {
    let body = fetch_ingest_config(world).await;
    let actual = body["schedule"]
        .as_object()
        .expect("schedule should be an object");
    assert_eq!(actual["cron_expr"].as_str().unwrap(), expected_cron);
}

#[then(regex = r#"^the ingest configuration has (\d+) sections? named "([^"]+)"$"#)]
async fn then_sections_named(world: &mut BotWorld, expected_count: usize, name: String) {
    let body = fetch_ingest_config(world).await;
    let sections = body["sections"]
        .as_array()
        .expect("sections should be an array");
    let matching: Vec<_> = sections
        .iter()
        .filter(|s| s["name"].as_str() == Some(name.as_str()))
        .collect();
    assert_eq!(
        matching.len(),
        expected_count,
        "expected {expected_count} section(s) named '{name}', got {}",
        matching.len()
    );
}

#[then(regex = r#"^the ingest configuration has (\d+) sources? in section "([^"]+)"$"#)]
async fn then_sources_in_section(
    world: &mut BotWorld,
    expected_count: usize,
    section_name: String,
) {
    let body = fetch_ingest_config(world).await;
    for section in body["sections"].as_array().unwrap() {
        if section["name"].as_str() == Some(section_name.as_str()) {
            let sources = section["sources"].as_array().unwrap();
            assert_eq!(
                sources.len(),
                expected_count,
                "expected {expected_count} source(s) in section '{section_name}', got {}",
                sources.len()
            );
            return;
        }
    }
    panic!("section '{section_name}' not found in response");
}

#[then(regex = r#"^the ingest configuration has (\d+) curation sources? in section "([^"]+)"$"#)]
async fn then_curation_sources_in_section(
    world: &mut BotWorld,
    expected_count: usize,
    section_name: String,
) {
    let body = fetch_ingest_config(world).await;
    for section in body["sections"].as_array().unwrap() {
        if section["name"].as_str() == Some(section_name.as_str()) {
            let curation_sources = section["curation_sources"].as_array().unwrap();
            assert_eq!(
                curation_sources.len(),
                expected_count,
                "expected {expected_count} curation source(s) in section '{section_name}', got {}",
                curation_sources.len()
            );
            return;
        }
    }
    panic!("section '{section_name}' not found in response");
}

#[then(regex = r#"^the source in section "([^"]+)" is enabled and not coming soon$"#)]
async fn then_source_enabled_not_coming_soon(world: &mut BotWorld, section_name: String) {
    let body = fetch_ingest_config(world).await;

    for section in body["sections"].as_array().unwrap() {
        if section["name"].as_str() == Some(section_name.as_str()) {
            let sources = section["sources"].as_array().unwrap();
            let source = sources.first().expect("no sources in section");
            assert_eq!(source["enabled"].as_bool(), Some(true));
            assert_eq!(source["coming_soon"].as_bool(), Some(false));
            return;
        }
    }
    panic!("section '{section_name}' not found in response");
}

#[then(regex = r#"^the source in section "([^"]+)" is disabled and coming soon$"#)]
async fn then_source_disabled_coming_soon(world: &mut BotWorld, section_name: String) {
    let body = fetch_ingest_config(world).await;

    for section in body["sections"].as_array().unwrap() {
        if section["name"].as_str() == Some(section_name.as_str()) {
            let sources = section["sources"].as_array().unwrap();
            let source = sources.first().expect("no sources in section");
            assert_eq!(source["enabled"].as_bool(), Some(false));
            assert_eq!(source["coming_soon"].as_bool(), Some(true));
            return;
        }
    }
    panic!("section '{section_name}' not found in response");
}

// ---------------------------------------------------------------------------
// Ingest run BDD step definitions
// ---------------------------------------------------------------------------

#[given("the ingest run API is available")]
async fn given_ingest_run_api_available(world: &mut BotWorld) {
    let path = temp_db();
    let store = Arc::new(
        kb_store::KbStore::open(&path)
            .await
            .expect("failed to open test kb.db for ingest run"),
    );
    let persona: Arc<dyn PersonaPort> = Arc::new(
        backend::rag_engine::persona::PersonaAdapter::new(store.clone()),
    );
    let persona_admin: Arc<dyn PersonaAdminPort> = Arc::new(
        backend::rag_engine::persona_admin::PersonaAdminAdapter::new(store.clone(), persona),
    );

    let config = Config {
        embed_url: "http://embed:8080".into(),
        generate_url: "http://generate:8080".into(),
        kb_path: path.clone(),
        top_k: 5,
        min_score: 0.35,
        operator_credential_path: "/nonexistent-bdd-credential.json".into(),
        operator_username: None,
        operator_password: None,
        session_ttl_secs: 1800,
        upload_max_bytes: 10_485_760,
        curation_allowed_hosts: vec!["halleyweb.com".to_string()],
        training_notes_dir: "/tmp/nonexistent-training-notes-dir".into(),
    };

    let rag_engine = Arc::new(RagEngine::new(
        Arc::new(StubEmbedding),
        Arc::new(ConfigurableRetrieval { chunks: vec![] }),
        Arc::new(ConfigurablePersona { snapshot: None }),
        Arc::new(RecordingGeneration {
            call_count: AtomicUsize::new(0),
            last_prompt: std::sync::Mutex::new(None),
        }),
        5,
        0.35,
    ));

    let upload: Arc<dyn UploadPort> = Arc::new(StubUploadPort);
    let preview_store = Arc::new(PreviewStore::new(15));
    let upload_state = UploadState {
        upload,
        preview_store,
        config: config.clone(),
        audit: Arc::new(NoopAudit),
    };
    let ingest_config_state = stub_ingest_config_state();

    let ingest_run_port: Arc<dyn IngestRunAdminPort> =
        Arc::new(backend::admin::ingest_run::adapter::KbStoreIngestRunAdapter::new(store.clone()));
    let ingest_run_state = IngestRunState {
        ingest_run: ingest_run_port,
        audit: Arc::new(NoopAudit),
    };

    let training_session_port: Arc<dyn TrainingSessionAdminPort> = Arc::new(
        backend::admin::training_sessions::adapter::KbStoreTrainingSessionAdapter::new(
            store.clone(),
        ),
    );
    let training_session_state = TrainingSessionState {
        training_sessions: training_session_port,
        audit: Arc::new(NoopAudit),
    };

    let training_message_port: Arc<dyn TrainingMessageAdminPort> = Arc::new(
        backend::admin::training_messages::adapter::RagTrainingMessageAdapter::new(
            store.clone(),
            rag_engine.clone(),
        ),
    );
    let training_message_state = TrainingMessageState {
        training_messages: training_message_port,
        audit: Arc::new(NoopAudit),
    };

    let training_feedback_port: Arc<dyn TrainingFeedbackAdminPort> = Arc::new(
        backend::admin::training_feedback::adapter::KbStoreTrainingFeedbackAdapter::new(
            store.clone(),
        ),
    );
    let training_feedback_state = TrainingFeedbackState {
        training_feedback: training_feedback_port,
        audit: Arc::new(NoopAudit),
    };

    let session_store = Arc::new(SessionStore::new(config.session_ttl_secs));
    let session_token = session_store.insert("operator".into());
    world.admin_session_cookie = Some(format!("session={session_token}"));
    let audit_port: Arc<dyn AuditLogPort> = Arc::new(KbStoreAuditLogAdapter::new(store.clone()));

    let router = backend::router_with(
        AppState { rag_engine },
        persona_admin,
        config,
        session_store,
        audit_port,
        backend::AdminRouterState {
            upload: upload_state,
            ingest_config: ingest_config_state,
            ingest_manual: stub_ingest_manual_state(),
            ingest_run: ingest_run_state,
            scraper_options: stub_scraper_options_state(),
            training_sessions: training_session_state,
            training_messages: training_message_state,
            training_feedback: training_feedback_state,
        },
    );

    world.ingest_run_db_path = Some(path);
    world.ingest_run_router = Some(router);
}

async fn ingest_run_request(world: &mut BotWorld, method: &str, uri: String, with_auth: bool) {
    let router = world
        .ingest_run_router
        .as_ref()
        .expect("ingest run router not initialized")
        .clone();

    let mut builder = Request::builder().method(method).uri(uri);
    if with_auth {
        builder = builder.header("cookie", world.admin_session_cookie.as_ref().unwrap());
    }
    let response = router
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[when("the operator triggers an ingest run")]
async fn when_trigger_ingest_run(world: &mut BotWorld) {
    ingest_run_request(world, "POST", "/admin/api/ingest/run".into(), true).await;

    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    world.ingest_run_id = body["id"].as_i64();
}

#[when("the operator triggers an ingest run without admin key")]
async fn when_trigger_ingest_run_no_auth(world: &mut BotWorld) {
    ingest_run_request(world, "POST", "/admin/api/ingest/run".into(), false).await;
}

#[when(regex = r"^the operator checks the status of ingest run (\d+)$")]
async fn when_check_run_status_by_id(world: &mut BotWorld, id: i64) {
    ingest_run_request(world, "GET", format!("/admin/api/ingest/run/{id}"), true).await;
}

#[when(regex = r"^the operator checks the status of ingest run (\d+) without admin key$")]
async fn when_check_run_status_by_id_no_auth(world: &mut BotWorld, id: i64) {
    ingest_run_request(world, "GET", format!("/admin/api/ingest/run/{id}"), false).await;
}

#[when("the operator checks the status of that ingest run")]
async fn when_check_run_status(world: &mut BotWorld) {
    let id = world.ingest_run_id.expect("no ingest run triggered yet");
    ingest_run_request(world, "GET", format!("/admin/api/ingest/run/{id}"), true).await;
}

#[when("the ingest service picks up and completes that run")]
async fn when_ingest_service_completes_run(world: &mut BotWorld) {
    let path = world
        .ingest_run_db_path
        .clone()
        .expect("ingest run db not initialized");
    let id = world.ingest_run_id.expect("no ingest run triggered yet");

    let store = kb_store::KbStore::open(&path)
        .await
        .expect("failed to reopen test kb.db for ingest run");
    let consumed = store
        .consume_run_request()
        .await
        .expect("consume_run_request failed")
        .expect("expected a pending run request");
    assert_eq!(
        consumed.id, id,
        "consumed a different run than the one triggered"
    );
    store
        .complete_run(id, kb_store::RunRequestStatus::Done)
        .await
        .expect("complete_run failed");
}

#[then("a new ingest run is queued with pending status")]
async fn then_run_queued_pending(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(202));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["status"].as_str(), Some("pending"));
    assert!(body["id"].as_i64().is_some());
}

#[then(regex = r#"^the ingest run status is "([^"]+)"$"#)]
async fn then_run_status_is(world: &mut BotWorld, expected_status: String) {
    assert_eq!(world.response_status, Some(200));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["status"].as_str(), Some(expected_status.as_str()));
}

#[then("the ingest run is not found")]
async fn then_run_not_found(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(404));
}

// ---------------------------------------------------------------------------
// Manual (scoped) ingest BDD step definitions — Plan 0029
// ---------------------------------------------------------------------------

/// Stands in for `PipelineIngestManualAdapter` (covered by wiremock-backed
/// adapter tests in `backend/src/admin/ingest_manual/adapter.rs`). This
/// step-definition scope tests the HTTP/auth/routing layer only, matching a
/// real robots.txt-disallowed domain by name so the scenario reads like the
/// real-world case it stands for.
struct ConfigurableIngestManual;

#[async_trait]
impl IngestManualAdminPort for ConfigurableIngestManual {
    async fn ingest(
        &self,
        section: &str,
        src: &str,
        window: RecencyWindow,
    ) -> Result<IngestManualResponse, IngestManualError> {
        if src.contains("halleyweb.com") {
            return Err(IngestManualError::RobotsTxt(format!(
                "{src} disallows scraping"
            )));
        }
        Ok(IngestManualResponse {
            section: section.to_string(),
            src: src.to_string(),
            window: window.to_string(),
            status: "ingested".to_string(),
        })
    }
}

#[given("the manual ingest API is available")]
async fn given_manual_ingest_api_available(world: &mut BotWorld) {
    let path = temp_db();
    let store = Arc::new(
        kb_store::KbStore::open(&path)
            .await
            .expect("failed to open test kb.db for manual ingest"),
    );
    let persona: Arc<dyn PersonaPort> = Arc::new(
        backend::rag_engine::persona::PersonaAdapter::new(store.clone()),
    );
    let persona_admin: Arc<dyn PersonaAdminPort> = Arc::new(
        backend::rag_engine::persona_admin::PersonaAdminAdapter::new(store.clone(), persona),
    );

    let config = Config {
        embed_url: "http://embed:8080".into(),
        generate_url: "http://generate:8080".into(),
        kb_path: path.clone(),
        top_k: 5,
        min_score: 0.35,
        operator_credential_path: "/nonexistent-bdd-credential.json".into(),
        operator_username: None,
        operator_password: None,
        session_ttl_secs: 1800,
        upload_max_bytes: 10_485_760,
        curation_allowed_hosts: vec!["halleyweb.com".to_string()],
        training_notes_dir: "/tmp/nonexistent-training-notes-dir".into(),
    };

    let rag_engine = Arc::new(RagEngine::new(
        Arc::new(StubEmbedding),
        Arc::new(ConfigurableRetrieval { chunks: vec![] }),
        Arc::new(ConfigurablePersona { snapshot: None }),
        Arc::new(RecordingGeneration {
            call_count: AtomicUsize::new(0),
            last_prompt: std::sync::Mutex::new(None),
        }),
        5,
        0.35,
    ));

    let upload: Arc<dyn UploadPort> = Arc::new(StubUploadPort);
    let preview_store = Arc::new(PreviewStore::new(15));
    let upload_state = UploadState {
        upload,
        preview_store,
        config: config.clone(),
        audit: Arc::new(NoopAudit),
    };
    let ingest_config_state = stub_ingest_config_state();
    let ingest_run_state = stub_ingest_run_state();

    let ingest_manual_port: Arc<dyn IngestManualAdminPort> = Arc::new(ConfigurableIngestManual);
    let audit_port: Arc<dyn AuditLogPort> = Arc::new(KbStoreAuditLogAdapter::new(store.clone()));
    let ingest_manual_state = IngestManualState {
        ingest_manual: ingest_manual_port,
        audit: audit_port.clone(),
    };

    let training_session_port: Arc<dyn TrainingSessionAdminPort> = Arc::new(
        backend::admin::training_sessions::adapter::KbStoreTrainingSessionAdapter::new(
            store.clone(),
        ),
    );
    let training_session_state = TrainingSessionState {
        training_sessions: training_session_port,
        audit: Arc::new(NoopAudit),
    };

    let training_message_port: Arc<dyn TrainingMessageAdminPort> = Arc::new(
        backend::admin::training_messages::adapter::RagTrainingMessageAdapter::new(
            store.clone(),
            rag_engine.clone(),
        ),
    );
    let training_message_state = TrainingMessageState {
        training_messages: training_message_port,
        audit: Arc::new(NoopAudit),
    };

    let training_feedback_port: Arc<dyn TrainingFeedbackAdminPort> = Arc::new(
        backend::admin::training_feedback::adapter::KbStoreTrainingFeedbackAdapter::new(
            store.clone(),
        ),
    );
    let training_feedback_state = TrainingFeedbackState {
        training_feedback: training_feedback_port,
        audit: Arc::new(NoopAudit),
    };

    let session_store = Arc::new(SessionStore::new(config.session_ttl_secs));
    let session_token = session_store.insert("operator".into());
    world.admin_session_cookie = Some(format!("session={session_token}"));

    let router = backend::router_with(
        AppState { rag_engine },
        persona_admin,
        config,
        session_store,
        audit_port,
        backend::AdminRouterState {
            upload: upload_state,
            ingest_config: ingest_config_state,
            ingest_manual: ingest_manual_state,
            ingest_run: ingest_run_state,
            scraper_options: stub_scraper_options_state(),
            training_sessions: training_session_state,
            training_messages: training_message_state,
            training_feedback: training_feedback_state,
        },
    );

    world.admin_db_path = Some(path);
    world.ingest_manual_router = Some(router);
}

/// Stands in for `CuratingIngestManualAdapter` dispatching to
/// `HalleyCurationAdapter` for an allow-listed source (Plan 0030). The real
/// pagination/bookmark/upload orchestration is covered by the wiremock-backed
/// adapter tests in `backend/src/admin/ingest_manual/halley/curation.rs` —
/// this step-definition scope tests only the HTTP/auth/audit contract, using
/// a call counter to simulate the bookmark narrowing a second run to "no new
/// items", matching the real adapter's observable behavior.
struct SimulatedCurationPort {
    calls: AtomicUsize,
}

#[async_trait]
impl IngestManualAdminPort for SimulatedCurationPort {
    async fn ingest(
        &self,
        section: &str,
        src: &str,
        window: RecencyWindow,
    ) -> Result<IngestManualResponse, IngestManualError> {
        let call_number = self.calls.fetch_add(1, Ordering::SeqCst);
        let status = if call_number == 0 {
            "ingested 2 document(s)".to_string()
        } else {
            "no new items".to_string()
        };
        Ok(IngestManualResponse {
            section: section.to_string(),
            src: src.to_string(),
            window: window.to_string(),
            status,
        })
    }
}

#[given("the curation API is available for an allow-listed source")]
async fn given_curation_api_available(world: &mut BotWorld) {
    let path = temp_db();
    let store = Arc::new(
        kb_store::KbStore::open(&path)
            .await
            .expect("failed to open test kb.db for curation"),
    );
    let persona: Arc<dyn PersonaPort> = Arc::new(
        backend::rag_engine::persona::PersonaAdapter::new(store.clone()),
    );
    let persona_admin: Arc<dyn PersonaAdminPort> = Arc::new(
        backend::rag_engine::persona_admin::PersonaAdminAdapter::new(store.clone(), persona),
    );

    let config = Config {
        embed_url: "http://embed:8080".into(),
        generate_url: "http://generate:8080".into(),
        kb_path: path.clone(),
        top_k: 5,
        min_score: 0.35,
        operator_credential_path: "/nonexistent-bdd-credential.json".into(),
        operator_username: None,
        operator_password: None,
        session_ttl_secs: 1800,
        upload_max_bytes: 10_485_760,
        curation_allowed_hosts: vec!["example-halley-instance.test".to_string()],
        training_notes_dir: "/tmp/nonexistent-training-notes-dir".into(),
    };

    let rag_engine = Arc::new(RagEngine::new(
        Arc::new(StubEmbedding),
        Arc::new(ConfigurableRetrieval { chunks: vec![] }),
        Arc::new(ConfigurablePersona { snapshot: None }),
        Arc::new(RecordingGeneration {
            call_count: AtomicUsize::new(0),
            last_prompt: std::sync::Mutex::new(None),
        }),
        5,
        0.35,
    ));

    let upload: Arc<dyn UploadPort> = Arc::new(StubUploadPort);
    let preview_store = Arc::new(PreviewStore::new(15));
    let upload_state = UploadState {
        upload,
        preview_store,
        config: config.clone(),
        audit: Arc::new(NoopAudit),
    };
    let ingest_config_state = stub_ingest_config_state();
    let ingest_run_state = stub_ingest_run_state();

    let ingest_manual_port: Arc<dyn IngestManualAdminPort> = Arc::new(SimulatedCurationPort {
        calls: AtomicUsize::new(0),
    });
    let audit_port: Arc<dyn AuditLogPort> = Arc::new(KbStoreAuditLogAdapter::new(store.clone()));
    let ingest_manual_state = IngestManualState {
        ingest_manual: ingest_manual_port,
        audit: audit_port.clone(),
    };

    let training_session_port: Arc<dyn TrainingSessionAdminPort> = Arc::new(
        backend::admin::training_sessions::adapter::KbStoreTrainingSessionAdapter::new(
            store.clone(),
        ),
    );
    let training_session_state = TrainingSessionState {
        training_sessions: training_session_port,
        audit: Arc::new(NoopAudit),
    };

    let training_message_port: Arc<dyn TrainingMessageAdminPort> = Arc::new(
        backend::admin::training_messages::adapter::RagTrainingMessageAdapter::new(
            store.clone(),
            rag_engine.clone(),
        ),
    );
    let training_message_state = TrainingMessageState {
        training_messages: training_message_port,
        audit: Arc::new(NoopAudit),
    };

    let training_feedback_port: Arc<dyn TrainingFeedbackAdminPort> = Arc::new(
        backend::admin::training_feedback::adapter::KbStoreTrainingFeedbackAdapter::new(
            store.clone(),
        ),
    );
    let training_feedback_state = TrainingFeedbackState {
        training_feedback: training_feedback_port,
        audit: Arc::new(NoopAudit),
    };

    let session_store = Arc::new(SessionStore::new(config.session_ttl_secs));
    let session_token = session_store.insert("operator".into());
    world.admin_session_cookie = Some(format!("session={session_token}"));

    let router = backend::router_with(
        AppState { rag_engine },
        persona_admin,
        config,
        session_store,
        audit_port,
        backend::AdminRouterState {
            upload: upload_state,
            ingest_config: ingest_config_state,
            ingest_manual: ingest_manual_state,
            ingest_run: ingest_run_state,
            scraper_options: stub_scraper_options_state(),
            training_sessions: training_session_state,
            training_messages: training_message_state,
            training_feedback: training_feedback_state,
        },
    );

    world.admin_db_path = Some(path);
    world.ingest_manual_router = Some(router);
}

#[then(regex = r#"^the manual ingest reports "([^"]+)"$"#)]
async fn then_manual_ingest_reports(world: &mut BotWorld, expected_status: String) {
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["status"].as_str(), Some(expected_status.as_str()));
}

async fn manual_ingest_request(
    world: &mut BotWorld,
    section: &str,
    src: &str,
    window: &str,
    with_auth: bool,
) {
    let router = world
        .ingest_manual_router
        .as_ref()
        .expect("manual ingest router not initialized")
        .clone();

    let body = serde_json::json!({ "section": section, "src": src, "window": window }).to_string();
    let mut builder = Request::builder()
        .method("POST")
        .uri("/admin/api/ingest/manual")
        .header("content-type", "application/json");
    if with_auth {
        builder = builder.header("cookie", world.admin_session_cookie.as_ref().unwrap());
    }
    let response = router
        .oneshot(builder.body(Body::from(body)).unwrap())
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[when(
    regex = r#"^the operator runs a manual ingest for section "([^"]+)", source "([^"]+)", window "([^"]+)"$"#
)]
async fn when_run_manual_ingest(
    world: &mut BotWorld,
    section: String,
    src: String,
    window: String,
) {
    manual_ingest_request(world, &section, &src, &window, true).await;
}

#[when("the operator runs a manual ingest without admin key")]
async fn when_run_manual_ingest_no_auth(world: &mut BotWorld) {
    manual_ingest_request(
        world,
        "storia",
        "https://it.wikipedia.org/wiki/Maiolati_Spontini",
        "30d",
        false,
    )
    .await;
}

#[then("the manual ingest succeeds")]
async fn then_manual_ingest_succeeds(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(200));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["status"].as_str(), Some("ingested"));
}

#[then("the manual ingest is rejected as disallowed by robots.txt")]
async fn then_manual_ingest_rejected_robots(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(403));
}

#[then("the manual ingest is rejected as an invalid window")]
async fn then_manual_ingest_rejected_invalid_window(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(400));
}

// ---------------------------------------------------------------------------
// Training session BDD step definitions
// ---------------------------------------------------------------------------

#[given("the training sessions API is available")]
async fn given_training_sessions_api_available(world: &mut BotWorld) {
    let path = temp_db();
    let store = Arc::new(
        kb_store::KbStore::open(&path)
            .await
            .expect("failed to open test kb.db for training sessions"),
    );
    let persona: Arc<dyn PersonaPort> = Arc::new(
        backend::rag_engine::persona::PersonaAdapter::new(store.clone()),
    );
    let persona_admin: Arc<dyn PersonaAdminPort> = Arc::new(
        backend::rag_engine::persona_admin::PersonaAdminAdapter::new(store.clone(), persona),
    );

    let config = Config {
        embed_url: "http://embed:8080".into(),
        generate_url: "http://generate:8080".into(),
        kb_path: path.clone(),
        top_k: 5,
        min_score: 0.35,
        operator_credential_path: "/nonexistent-bdd-credential.json".into(),
        operator_username: None,
        operator_password: None,
        session_ttl_secs: 1800,
        upload_max_bytes: 10_485_760,
        curation_allowed_hosts: vec!["halleyweb.com".to_string()],
        training_notes_dir: "/tmp/nonexistent-training-notes-dir".into(),
    };

    let rag_engine = Arc::new(RagEngine::new(
        Arc::new(StubEmbedding),
        Arc::new(ConfigurableRetrieval { chunks: vec![] }),
        Arc::new(ConfigurablePersona { snapshot: None }),
        Arc::new(RecordingGeneration {
            call_count: AtomicUsize::new(0),
            last_prompt: std::sync::Mutex::new(None),
        }),
        5,
        0.35,
    ));

    let upload: Arc<dyn UploadPort> = Arc::new(StubUploadPort);
    let preview_store = Arc::new(PreviewStore::new(15));
    let upload_state = UploadState {
        upload,
        preview_store,
        config: config.clone(),
        audit: Arc::new(NoopAudit),
    };
    let ingest_config_state = stub_ingest_config_state();
    let ingest_run_state = stub_ingest_run_state();

    let training_session_port: Arc<dyn TrainingSessionAdminPort> = Arc::new(
        backend::admin::training_sessions::adapter::KbStoreTrainingSessionAdapter::new(
            store.clone(),
        ),
    );
    let training_session_state = TrainingSessionState {
        training_sessions: training_session_port,
        audit: Arc::new(NoopAudit),
    };

    let training_message_port: Arc<dyn TrainingMessageAdminPort> = Arc::new(
        backend::admin::training_messages::adapter::RagTrainingMessageAdapter::new(
            store.clone(),
            rag_engine.clone(),
        ),
    );
    let training_message_state = TrainingMessageState {
        training_messages: training_message_port,
        audit: Arc::new(NoopAudit),
    };

    let training_feedback_port: Arc<dyn TrainingFeedbackAdminPort> = Arc::new(
        backend::admin::training_feedback::adapter::KbStoreTrainingFeedbackAdapter::new(
            store.clone(),
        ),
    );
    let training_feedback_state = TrainingFeedbackState {
        training_feedback: training_feedback_port,
        audit: Arc::new(NoopAudit),
    };

    let session_store = Arc::new(SessionStore::new(config.session_ttl_secs));
    let session_token = session_store.insert("operator".into());
    world.admin_session_cookie = Some(format!("session={session_token}"));
    let audit_port: Arc<dyn AuditLogPort> = Arc::new(KbStoreAuditLogAdapter::new(store.clone()));

    let router = backend::router_with(
        AppState { rag_engine },
        persona_admin,
        config,
        session_store,
        audit_port,
        backend::AdminRouterState {
            upload: upload_state,
            ingest_config: ingest_config_state,
            ingest_manual: stub_ingest_manual_state(),
            ingest_run: ingest_run_state,
            scraper_options: stub_scraper_options_state(),
            training_sessions: training_session_state,
            training_messages: training_message_state,
            training_feedback: training_feedback_state,
        },
    );

    world.training_sessions_db_path = Some(path);
    world.training_sessions_router = Some(router);
}

async fn training_session_request(
    world: &mut BotWorld,
    method: &str,
    uri: String,
    with_auth: bool,
    body: Option<serde_json::Value>,
) {
    let router = world
        .training_sessions_router
        .as_ref()
        .expect("training sessions router not initialized")
        .clone();

    let mut builder = Request::builder().method(method).uri(uri);
    if with_auth {
        builder = builder.header("cookie", world.admin_session_cookie.as_ref().unwrap());
    }
    let request = if let Some(body) = body {
        builder
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    };

    let response = router.oneshot(request).await.unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[when(regex = r#"^the operator creates a training session titled "([^"]+)"$"#)]
async fn when_create_training_session(world: &mut BotWorld, title: String) {
    let body = serde_json::json!({ "title": title, "created_by": null });
    training_session_request(
        world,
        "POST",
        "/admin/api/training/sessions".into(),
        true,
        Some(body),
    )
    .await;

    let response: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    world.training_session_id = response["id"].as_i64();
}

#[given(regex = r#"^the operator has created a training session titled "([^"]+)"$"#)]
async fn given_created_training_session(world: &mut BotWorld, title: String) {
    when_create_training_session(world, title).await;
}

#[when("the operator creates a training session without admin key")]
async fn when_create_training_session_no_auth(world: &mut BotWorld) {
    let body = serde_json::json!({ "title": "Sessione", "created_by": null });
    training_session_request(
        world,
        "POST",
        "/admin/api/training/sessions".into(),
        false,
        Some(body),
    )
    .await;
}

#[when("the operator lists training sessions without admin key")]
async fn when_list_training_sessions_no_auth(world: &mut BotWorld) {
    training_session_request(
        world,
        "GET",
        "/admin/api/training/sessions".into(),
        false,
        None,
    )
    .await;
}

#[when("the operator retrieves that training session")]
async fn when_retrieve_that_training_session(world: &mut BotWorld) {
    let id = world
        .training_session_id
        .expect("no training session created yet");
    training_session_request(
        world,
        "GET",
        format!("/admin/api/training/sessions/{id}"),
        true,
        None,
    )
    .await;
}

#[when(regex = r"^the operator retrieves training session (\d+)$")]
async fn when_retrieve_training_session_by_id(world: &mut BotWorld, id: i64) {
    training_session_request(
        world,
        "GET",
        format!("/admin/api/training/sessions/{id}"),
        true,
        None,
    )
    .await;
}

#[when(regex = r"^the operator retrieves training session (\d+) without admin key$")]
async fn when_retrieve_training_session_by_id_no_auth(world: &mut BotWorld, id: i64) {
    training_session_request(
        world,
        "GET",
        format!("/admin/api/training/sessions/{id}"),
        false,
        None,
    )
    .await;
}

#[when("the operator closes that training session")]
async fn when_close_that_training_session(world: &mut BotWorld) {
    let id = world
        .training_session_id
        .expect("no training session created yet");
    training_session_request(
        world,
        "POST",
        format!("/admin/api/training/sessions/{id}/close"),
        true,
        None,
    )
    .await;
}

#[given("the operator has closed that training session")]
async fn given_closed_that_training_session(world: &mut BotWorld) {
    when_close_that_training_session(world).await;
}

#[when("the operator closes that training session again")]
async fn when_close_that_training_session_again(world: &mut BotWorld) {
    when_close_that_training_session(world).await;
}

#[when(regex = r"^the operator closes training session (\d+) without admin key$")]
async fn when_close_training_session_by_id_no_auth(world: &mut BotWorld, id: i64) {
    training_session_request(
        world,
        "POST",
        format!("/admin/api/training/sessions/{id}/close"),
        false,
        None,
    )
    .await;
}

#[when(regex = r#"^the operator closes that training session with notes "([^"]+)"$"#)]
async fn when_close_that_training_session_with_notes(world: &mut BotWorld, notes: String) {
    let id = world
        .training_session_id
        .expect("no training session created yet");
    training_session_request(
        world,
        "POST",
        format!(
            "/admin/api/training/sessions/{id}/close?notes={}",
            urlencoding_encode(&notes)
        ),
        true,
        None,
    )
    .await;
}

fn urlencoding_encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[when("the operator deletes that training session")]
async fn when_delete_that_training_session(world: &mut BotWorld) {
    let id = world
        .training_session_id
        .expect("no training session created yet");
    training_session_request(
        world,
        "DELETE",
        format!("/admin/api/training/sessions/{id}"),
        true,
        None,
    )
    .await;
}

#[then(regex = r#"^the training session list contains a session titled "([^"]+)"$"#)]
async fn then_session_list_contains(world: &mut BotWorld, title: String) {
    training_session_request(
        world,
        "GET",
        "/admin/api/training/sessions".into(),
        true,
        None,
    )
    .await;
    let sessions: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    let found = sessions
        .as_array()
        .expect("sessions should be an array")
        .iter()
        .any(|s| s["title"].as_str() == Some(title.as_str()));
    assert!(found, "expected a session titled '{title}' in the list");
}

#[then(regex = r#"^the retrieved training session is titled "([^"]+)"$"#)]
async fn then_retrieved_session_titled(world: &mut BotWorld, title: String) {
    assert_eq!(world.response_status, Some(200));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["title"].as_str(), Some(title.as_str()));
}

#[then("the retrieved training session is open")]
async fn then_retrieved_session_open(world: &mut BotWorld) {
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert!(body["closed_at"].is_null());
}

#[then("the training session is closed")]
async fn then_session_closed(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(200));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["closed"].as_bool(), Some(true));
}

#[then(regex = r#"^the retrieved training session has notes "([^"]+)"$"#)]
async fn then_retrieved_session_has_notes(world: &mut BotWorld, notes: String) {
    let id = world
        .training_session_id
        .expect("no training session created yet");
    training_session_request(
        world,
        "GET",
        format!("/admin/api/training/sessions/{id}"),
        true,
        None,
    )
    .await;
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["notes"].as_str(), Some(notes.as_str()));
}

#[then("the training session is deleted")]
async fn then_session_deleted(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(200));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["deleted"].as_bool(), Some(true));

    let id = world
        .training_session_id
        .expect("no training session created yet");
    training_session_request(
        world,
        "GET",
        format!("/admin/api/training/sessions/{id}"),
        true,
        None,
    )
    .await;
    assert_eq!(world.response_status, Some(404));
}

#[then("closing the training session has no effect")]
async fn then_closing_has_no_effect(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(200));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["closed"].as_bool(), Some(false));
}

#[then("the training session is not found")]
async fn then_session_not_found(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(404));
}

// ---------------------------------------------------------------------------
// Admin training messages BDD steps
// ---------------------------------------------------------------------------

#[given("the training messages API is available")]
async fn given_training_messages_api_available(world: &mut BotWorld) {
    let path = temp_db();
    let store = Arc::new(
        kb_store::KbStore::open(&path)
            .await
            .expect("failed to open test kb.db for training messages"),
    );
    let persona: Arc<dyn PersonaPort> = Arc::new(
        backend::rag_engine::persona::PersonaAdapter::new(store.clone()),
    );
    let persona_admin: Arc<dyn PersonaAdminPort> = Arc::new(
        backend::rag_engine::persona_admin::PersonaAdminAdapter::new(store.clone(), persona),
    );

    let config = Config {
        embed_url: "http://embed:8080".into(),
        generate_url: "http://generate:8080".into(),
        kb_path: path.clone(),
        top_k: 5,
        min_score: 0.35,
        operator_credential_path: "/nonexistent-bdd-credential.json".into(),
        operator_username: None,
        operator_password: None,
        session_ttl_secs: 1800,
        upload_max_bytes: 10_485_760,
        curation_allowed_hosts: vec!["halleyweb.com".to_string()],
        training_notes_dir: "/tmp/nonexistent-training-notes-dir".into(),
    };

    let counter = Arc::new(RecordingGeneration {
        call_count: AtomicUsize::new(0),
        last_prompt: std::sync::Mutex::new(None),
    });
    world.generation = Some(counter.clone());

    let rag_engine = Arc::new(RagEngine::new(
        Arc::new(StubEmbedding),
        Arc::new(ConfigurableRetrieval {
            chunks: world.chunks.clone(),
        }),
        Arc::new(ConfigurablePersona {
            snapshot: world.persona.clone(),
        }),
        counter,
        5,
        0.35,
    ));

    let upload: Arc<dyn UploadPort> = Arc::new(StubUploadPort);
    let preview_store = Arc::new(PreviewStore::new(15));
    let upload_state = UploadState {
        upload,
        preview_store,
        config: config.clone(),
        audit: Arc::new(NoopAudit),
    };
    let ingest_config_state = stub_ingest_config_state();
    let ingest_run_state = stub_ingest_run_state();

    let training_session_port: Arc<dyn TrainingSessionAdminPort> = Arc::new(
        backend::admin::training_sessions::adapter::KbStoreTrainingSessionAdapter::new(
            store.clone(),
        ),
    );
    let training_session_state = TrainingSessionState {
        training_sessions: training_session_port,
        audit: Arc::new(NoopAudit),
    };

    let training_message_port: Arc<dyn TrainingMessageAdminPort> = Arc::new(
        backend::admin::training_messages::adapter::RagTrainingMessageAdapter::new(
            store.clone(),
            rag_engine.clone(),
        ),
    );
    let training_message_state = TrainingMessageState {
        training_messages: training_message_port,
        audit: Arc::new(NoopAudit),
    };

    let training_feedback_port: Arc<dyn TrainingFeedbackAdminPort> = Arc::new(
        backend::admin::training_feedback::adapter::KbStoreTrainingFeedbackAdapter::new(
            store.clone(),
        ),
    );
    let training_feedback_state = TrainingFeedbackState {
        training_feedback: training_feedback_port,
        audit: Arc::new(NoopAudit),
    };

    let session_store = Arc::new(SessionStore::new(config.session_ttl_secs));
    let session_token = session_store.insert("operator".into());
    world.admin_session_cookie = Some(format!("session={session_token}"));
    let audit_port: Arc<dyn AuditLogPort> = Arc::new(KbStoreAuditLogAdapter::new(store.clone()));

    let router = backend::router_with(
        AppState { rag_engine },
        persona_admin,
        config,
        session_store,
        audit_port,
        backend::AdminRouterState {
            upload: upload_state,
            ingest_config: ingest_config_state,
            ingest_manual: stub_ingest_manual_state(),
            ingest_run: ingest_run_state,
            scraper_options: stub_scraper_options_state(),
            training_sessions: training_session_state,
            training_messages: training_message_state,
            training_feedback: training_feedback_state,
        },
    );

    world.training_sessions_db_path = Some(path);
    world.training_sessions_router = Some(router);
}

#[when(regex = r#"^the operator asks "([^"]+)" in that training session$"#)]
async fn when_ask_in_that_training_session(world: &mut BotWorld, question: String) {
    let id = world
        .training_session_id
        .expect("no training session created yet");
    let body = serde_json::json!({ "question": question });
    training_session_request(
        world,
        "POST",
        format!("/admin/api/training/sessions/{id}/messages"),
        true,
        Some(body),
    )
    .await;

    let response: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    world.training_message_id = response["id"].as_i64();
}

#[given(regex = r#"^the operator has asked "([^"]+)" in that training session$"#)]
async fn given_asked_in_that_training_session(world: &mut BotWorld, question: String) {
    when_ask_in_that_training_session(world, question).await;
}

#[when(regex = r#"^the operator asks "([^"]+)" in training session (\d+)$"#)]
async fn when_ask_in_training_session_by_id(world: &mut BotWorld, question: String, id: i64) {
    let body = serde_json::json!({ "question": question });
    training_session_request(
        world,
        "POST",
        format!("/admin/api/training/sessions/{id}/messages"),
        true,
        Some(body),
    )
    .await;
}

#[when("the operator asks a question in that training session without admin key")]
async fn when_ask_no_auth(world: &mut BotWorld) {
    let id = world
        .training_session_id
        .expect("no training session created yet");
    let body = serde_json::json!({ "question": "domanda" });
    training_session_request(
        world,
        "POST",
        format!("/admin/api/training/sessions/{id}/messages"),
        false,
        Some(body),
    )
    .await;
}

#[when(
    regex = r#"^the operator manually records the question "([^"]+)" with answer "([^"]+)" and expected answer "([^"]+)" in that training session$"#
)]
async fn when_manually_records_question(
    world: &mut BotWorld,
    question: String,
    answer: String,
    expected_answer: String,
) {
    let id = world
        .training_session_id
        .expect("no training session created yet");
    let body = serde_json::json!({
        "question": question,
        "answer": answer,
        "expected_answer": expected_answer,
    });
    training_session_request(
        world,
        "POST",
        format!("/admin/api/training/sessions/{id}/messages"),
        true,
        Some(body),
    )
    .await;

    let response: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    world.training_message_id = response["id"].as_i64();
}

#[then(regex = r#"^the training message answer is "([^"]+)"$"#)]
async fn then_training_message_answer_is(world: &mut BotWorld, answer: String) {
    assert_eq!(world.response_status, Some(201));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["answer"].as_str(), Some(answer.as_str()));
}

#[then(regex = r#"^the training message source is "([^"]+)"$"#)]
async fn then_training_message_source_is(world: &mut BotWorld, source: String) {
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["source"].as_str(), Some(source.as_str()));
}

#[then(regex = r#"^the training message expected answer is "([^"]+)"$"#)]
async fn then_training_message_expected_answer_is(world: &mut BotWorld, expected: String) {
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["expected_answer"].as_str(), Some(expected.as_str()));
}

#[when("the operator lists that training session's messages")]
async fn when_list_that_training_session_messages(world: &mut BotWorld) {
    let id = world
        .training_session_id
        .expect("no training session created yet");
    training_session_request(
        world,
        "GET",
        format!("/admin/api/training/sessions/{id}/messages"),
        true,
        None,
    )
    .await;
}

#[when("the operator lists that training session's messages without admin key")]
async fn when_list_messages_no_auth(world: &mut BotWorld) {
    let id = world
        .training_session_id
        .expect("no training session created yet");
    training_session_request(
        world,
        "GET",
        format!("/admin/api/training/sessions/{id}/messages"),
        false,
        None,
    )
    .await;
}

#[then("the training message answers using the content of the retrieved document")]
async fn then_training_message_answers_using_content(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(201));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(
        body["answer"].as_str(),
        Some("Lo sportello anagrafe e' aperto dalle 9:00 alle 12:30.")
    );
}

#[then("the training message cites the source document by title")]
async fn then_training_message_cites_source(world: &mut BotWorld) {
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    let sources = body["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(
        sources[0]["source_ref"].as_str().unwrap(),
        "Orari sportello anagrafe"
    );
}

#[then("the training message is not a fallback")]
async fn then_training_message_not_fallback(world: &mut BotWorld) {
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["fell_back"], false);
}

#[then("the training message is a fallback")]
async fn then_training_message_is_fallback(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(201));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["fell_back"], true);
}

#[then("the training message has no cited sources")]
async fn then_training_message_no_sources(world: &mut BotWorld) {
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert!(body["sources"].as_array().unwrap().is_empty());
}

#[then(regex = r#"^the training message list contains a message with question "([^"]+)"$"#)]
async fn then_message_list_contains(world: &mut BotWorld, question: String) {
    let id = world
        .training_session_id
        .expect("no training session created yet");
    training_session_request(
        world,
        "GET",
        format!("/admin/api/training/sessions/{id}/messages"),
        true,
        None,
    )
    .await;
    let messages: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    let found = messages
        .as_array()
        .expect("messages should be an array")
        .iter()
        .any(|m| m["question"].as_str() == Some(question.as_str()));
    assert!(
        found,
        "expected a message with question '{question}' in the list"
    );
}

// ---------------------------------------------------------------------------
// Admin training feedback BDD steps
// ---------------------------------------------------------------------------

#[given("a citable document exists in the knowledge base")]
async fn given_citable_document_exists(world: &mut BotWorld) {
    let path = world
        .training_sessions_db_path
        .clone()
        .expect("training messages API not initialized yet");
    let store = kb_store::KbStore::open(&path)
        .await
        .expect("failed to reopen test kb.db");
    let document = store
        .insert_document(kb_store::NewDocument {
            source: kb_store::DocumentSource::Manual,
            source_ref: "Orari sportello anagrafe".into(),
            content: "Lo sportello anagrafe e' aperto dalle 9:00 alle 12:30".into(),
            metadata: None,
            embedding: vec![0.0; kb_store::EMBEDDING_DIM],
            section: None,
        })
        .await
        .expect("insert_document failed");
    world.cited_chunk_id = Some(document.id);
}

#[when(regex = r#"^the operator leaves positive feedback on the span "([^"]+)" of that message$"#)]
async fn when_leave_positive_feedback(world: &mut BotWorld, span: String) {
    let message_id = world
        .training_message_id
        .expect("no training message asked yet");
    let body = serde_json::json!({
        "message_id": message_id,
        "chunk_id": null,
        "answer_span": span,
        "sentiment": "positive",
        "comment": null
    });
    training_session_request(
        world,
        "POST",
        "/admin/api/training/feedback".into(),
        true,
        Some(body),
    )
    .await;
}

#[given(
    regex = r#"^the operator has left positive feedback on the span "([^"]+)" of that message$"#
)]
async fn given_left_positive_feedback(world: &mut BotWorld, span: String) {
    when_leave_positive_feedback(world, span).await;
}

#[when(
    regex = r#"^the operator leaves negative feedback on the span "([^"]+)" of that message with comment "([^"]+)"$"#
)]
async fn when_leave_negative_feedback_with_comment(
    world: &mut BotWorld,
    span: String,
    comment: String,
) {
    let message_id = world
        .training_message_id
        .expect("no training message asked yet");
    let body = serde_json::json!({
        "message_id": message_id,
        "chunk_id": null,
        "answer_span": span,
        "sentiment": "negative",
        "comment": comment
    });
    training_session_request(
        world,
        "POST",
        "/admin/api/training/feedback".into(),
        true,
        Some(body),
    )
    .await;
}

#[when(
    regex = r#"^the operator leaves positive feedback on the span "([^"]+)" of that message anchored to the cited chunk$"#
)]
async fn when_leave_feedback_anchored_to_chunk(world: &mut BotWorld, span: String) {
    let message_id = world
        .training_message_id
        .expect("no training message asked yet");
    let chunk_id = world.cited_chunk_id.expect("no cited chunk recorded yet");
    let body = serde_json::json!({
        "message_id": message_id,
        "chunk_id": chunk_id,
        "answer_span": span,
        "sentiment": "positive",
        "comment": null
    });
    training_session_request(
        world,
        "POST",
        "/admin/api/training/feedback".into(),
        true,
        Some(body),
    )
    .await;
}

#[when(regex = r#"^the operator leaves feedback with sentiment "([^"]+)" on that message$"#)]
async fn when_leave_feedback_with_sentiment(world: &mut BotWorld, sentiment: String) {
    let message_id = world
        .training_message_id
        .expect("no training message asked yet");
    let body = serde_json::json!({
        "message_id": message_id,
        "chunk_id": null,
        "answer_span": "test",
        "sentiment": sentiment,
        "comment": null
    });
    training_session_request(
        world,
        "POST",
        "/admin/api/training/feedback".into(),
        true,
        Some(body),
    )
    .await;
}

#[when(regex = r"^the operator leaves feedback on unknown message (\d+)$")]
async fn when_leave_feedback_on_unknown_message(world: &mut BotWorld, message_id: i64) {
    let body = serde_json::json!({
        "message_id": message_id,
        "chunk_id": null,
        "answer_span": "test",
        "sentiment": "positive",
        "comment": null
    });
    training_session_request(
        world,
        "POST",
        "/admin/api/training/feedback".into(),
        true,
        Some(body),
    )
    .await;
}

#[when("the operator leaves feedback without admin key")]
async fn when_leave_feedback_no_auth(world: &mut BotWorld) {
    let message_id = world
        .training_message_id
        .expect("no training message asked yet");
    let body = serde_json::json!({
        "message_id": message_id,
        "chunk_id": null,
        "answer_span": "test",
        "sentiment": "positive",
        "comment": null
    });
    training_session_request(
        world,
        "POST",
        "/admin/api/training/feedback".into(),
        false,
        Some(body),
    )
    .await;
}

#[when("the operator lists feedback for that message without admin key")]
async fn when_list_feedback_no_auth(world: &mut BotWorld) {
    let message_id = world
        .training_message_id
        .expect("no training message asked yet");
    training_session_request(
        world,
        "GET",
        format!("/admin/api/training/messages/{message_id}/feedback"),
        false,
        None,
    )
    .await;
}

#[then(regex = r#"^the feedback list for that message contains a positive entry for "([^"]+)"$"#)]
async fn then_feedback_list_contains_positive(world: &mut BotWorld, span: String) {
    let message_id = world
        .training_message_id
        .expect("no training message asked yet");
    training_session_request(
        world,
        "GET",
        format!("/admin/api/training/messages/{message_id}/feedback"),
        true,
        None,
    )
    .await;
    let feedback: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    let found = feedback
        .as_array()
        .expect("feedback should be an array")
        .iter()
        .any(|f| {
            f["answer_span"].as_str() == Some(span.as_str())
                && f["sentiment"].as_str() == Some("positive")
        });
    assert!(found, "expected a positive feedback entry for '{span}'");
}

#[then(
    regex = r#"^the feedback list for that message contains a negative entry for "([^"]+)" with comment "([^"]+)"$"#
)]
async fn then_feedback_list_contains_negative_with_comment(
    world: &mut BotWorld,
    span: String,
    comment: String,
) {
    let message_id = world
        .training_message_id
        .expect("no training message asked yet");
    training_session_request(
        world,
        "GET",
        format!("/admin/api/training/messages/{message_id}/feedback"),
        true,
        None,
    )
    .await;
    let feedback: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    let found = feedback
        .as_array()
        .expect("feedback should be an array")
        .iter()
        .any(|f| {
            f["answer_span"].as_str() == Some(span.as_str())
                && f["sentiment"].as_str() == Some("negative")
                && f["comment"].as_str() == Some(comment.as_str())
        });
    assert!(
        found,
        "expected a negative feedback entry for '{span}' with comment '{comment}'"
    );
}

#[then(regex = r"^the feedback list for that message contains (\d+) entries$")]
async fn then_feedback_list_contains_n_entries(world: &mut BotWorld, expected: usize) {
    let message_id = world
        .training_message_id
        .expect("no training message asked yet");
    training_session_request(
        world,
        "GET",
        format!("/admin/api/training/messages/{message_id}/feedback"),
        true,
        None,
    )
    .await;
    let feedback: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(feedback.as_array().unwrap().len(), expected);
}

#[then("the feedback list for that message contains an entry anchored to a chunk")]
async fn then_feedback_list_contains_entry_anchored_to_chunk(world: &mut BotWorld) {
    let message_id = world
        .training_message_id
        .expect("no training message asked yet");
    let chunk_id = world.cited_chunk_id.expect("no cited chunk recorded yet");
    training_session_request(
        world,
        "GET",
        format!("/admin/api/training/messages/{message_id}/feedback"),
        true,
        None,
    )
    .await;
    let feedback: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    let found = feedback
        .as_array()
        .expect("feedback should be an array")
        .iter()
        .any(|f| f["chunk_id"].as_i64() == Some(chunk_id));
    assert!(
        found,
        "expected a feedback entry anchored to chunk {chunk_id}"
    );
}

#[then("the training message is not found")]
async fn then_training_message_not_found(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(404));
}

#[then("the request is rejected with 400")]
async fn then_rejected_400(world: &mut BotWorld) {
    assert_eq!(world.response_status, Some(400));
}

#[tokio::main]
async fn main() {
    BotWorld::cucumber()
        .fail_on_skipped()
        .run_and_exit(concat!(env!("CARGO_MANIFEST_DIR"), "/../features"))
        .await;
}
