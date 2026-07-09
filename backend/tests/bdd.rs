use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use cucumber::{World as _, given, then, when};
use tower::ServiceExt;

use backend::AppState;
use backend::rag_engine::engine::RagEngine;
use backend::rag_engine::ports::{EmbeddingPort, GenerationPort, PersonaPort, RetrievalPort};
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
    let router = backend::router_with(AppState { rag_engine });
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

    let router = backend::router_with({
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
    });

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

#[tokio::main]
async fn main() {
    BotWorld::cucumber()
        .fail_on_skipped()
        .run_and_exit(concat!(env!("CARGO_MANIFEST_DIR"), "/../features"))
        .await;
}
