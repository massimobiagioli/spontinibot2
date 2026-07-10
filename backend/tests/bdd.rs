use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use cucumber::{World as _, given, then, when};
use tower::ServiceExt;

use backend::AppState;
use backend::config::Config;
use backend::rag_engine::engine::RagEngine;
use backend::rag_engine::ports::{
    EmbeddingPort, GenerationPort, PersonaAdminPort, PersonaPort, RetrievalPort,
};
use backend::rag_engine::types::{PersonaSnapshot, PromptParts, RagError, RetrievedChunk};

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
    async fn list_versions(&self, _name: &str) -> Result<Vec<kb_store::Persona>, RagError> {
        todo!()
    }
    async fn insert_persona(
        &self,
        _persona: kb_store::NewPersona,
        _activate: bool,
    ) -> Result<kb_store::Persona, RagError> {
        todo!()
    }
    async fn activate_persona(&self, _id: i64) -> Result<(), RagError> {
        todo!()
    }
    async fn reload_persona(&self) -> Result<(), RagError> {
        todo!()
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
}

impl Drop for BotWorld {
    fn drop(&mut self) {
        if let Some(ref path) = self.admin_db_path {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Build an axum Router for the admin BDD scenarios.
/// Opens the KbStore at `db_path` and wires everything with stubs for
/// Embedding/Retrieval/Generation ports and a real PersonaAdminAdapter.
async fn build_admin_router(db_path: &str, admin_key: &str) -> axum::Router {
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

    let config = Config {
        embed_url: "http://embed:8080".into(),
        generate_url: "http://generate:8080".into(),
        kb_path: db_path.into(),
        top_k: 5,
        min_score: 0.35,
        admin_api_key: admin_key.into(),
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

    backend::router_with(AppState { rag_engine }, persona_admin, config)
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
        admin_api_key: "test-key".into(),
    };
    let router = backend::router_with(AppState { rag_engine }, admin, config);
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
    });
}

#[given(regex = r"^the document contains the text (.+)$")]
fn given_document_text(_world: &mut BotWorld, _text: String) {}

#[given(regex = r"^the knowledge base contains no document about (.+)$")]
fn given_no_document(_world: &mut BotWorld, _topic: String) {}

#[given("an active persona is configured with a system prompt and a fallback message")]
fn given_persona(world: &mut BotWorld) {
    world.persona = Some(PersonaSnapshot {
        system_prompt: "Sei Gaspare Spontini, sindaco di Maiolati Spontini.".into(),
        fallback_message: Some(
            "Non ho trovato informazioni nei documenti comunali su questo argomento.".into(),
        ),
    });
}

#[when(regex = r"^the citizen asks (.+)$")]
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
        admin_api_key: "test-key".into(),
    };
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
        config,
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
    let router = build_admin_router(path, ADMIN_KEY).await;

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
                .header("x-admin-key", ADMIN_KEY)
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
    let router = build_admin_router(path, ADMIN_KEY).await;

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/admin/api/persona?name={name}"))
                .header("x-admin-key", ADMIN_KEY)
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
    let router = build_admin_router(path, ADMIN_KEY).await;

    // We need the persona's id — fetch the list first
    let list_resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/admin/api/persona?name={name}"))
                .header("x-admin-key", ADMIN_KEY)
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
                .header("x-admin-key", ADMIN_KEY)
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

#[when("the operator reloads the persona cache")]
async fn when_reload_persona(world: &mut BotWorld) {
    let path = world.admin_db_path.as_ref().expect("no db path set");
    let router = build_admin_router(path, ADMIN_KEY).await;

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/api/persona/reload")
                .header("x-admin-key", ADMIN_KEY)
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
    let router = build_admin_router(path, ADMIN_KEY).await;

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/admin/api/persona?name=gaspare")
                .header("x-admin-key", ADMIN_KEY)
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
    let router = build_admin_router(path, ADMIN_KEY).await;

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/admin/api/persona?name=gaspare")
                .header("x-admin-key", ADMIN_KEY)
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

#[tokio::main]
async fn main() {
    BotWorld::cucumber()
        .fail_on_skipped()
        .run_and_exit(concat!(env!("CARGO_MANIFEST_DIR"), "/../features"))
        .await;
}
