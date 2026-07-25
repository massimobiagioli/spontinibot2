//! End-to-end BDD suite for `features/chat.feature`, run as an external HTTP
//! client against a live, containerized stack (`make up` + `make provision-models`).
//!
//! Unlike `backend/tests/bdd.rs` (the unit-level suite, wired with stub
//! `EmbeddingPort`/`GenerationPort` implementations via an in-process
//! `axum::Router`), this binary talks real HTTP to a running `backend`
//! container, which itself calls the real `llama-embed` / `llama-generate`
//! containers and the real libSQL vector retrieval. It proves the real
//! adapters work end-to-end, closing the risk noted in Plan 0003.
//!
//! Run via `make bdd-e2e` (see plan 0025). Configurable via:
//! - `E2E_BASE_URL` (default `http://localhost:8080`)
//! - `E2E_ADMIN_API_KEY` (default `dev-key`, matching `Config::from_env`'s default)

use cucumber::{World as _, given, then, when};
use serde_json::json;

fn base_url() -> String {
    std::env::var("E2E_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

fn admin_key() -> String {
    std::env::var("E2E_ADMIN_API_KEY").unwrap_or_else(|_| "dev-key".to_string())
}

fn unquote(s: &str) -> String {
    s.trim_matches('"').to_string()
}

/// Manually-encoded multipart body — mirrors the helper in `backend/tests/bdd.rs`.
/// Avoids requiring reqwest's `multipart` Cargo feature (not enabled on the
/// `backend` crate's `reqwest` dependency).
fn multipart_body(filename: &str, section: &str, content: &[u8]) -> (String, Vec<u8>) {
    let boundary = "----SpontiniE2eBoundary";
    let mut body = Vec::new();

    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"content-type: text/markdown\r\n");
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(content);
    body.extend_from_slice(b"\r\n");

    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"section\"\r\n");
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(section.as_bytes());
    body.extend_from_slice(b"\r\n");

    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"--\r\n");

    (boundary.to_string(), body)
}

#[derive(Debug, Default, cucumber::World)]
struct E2eWorld {
    document_title: Option<String>,
    fallback_message: Option<String>,
    response_status: Option<u16>,
    response_body: Option<String>,
}

#[given(regex = r#"^the knowledge base contains a document titled "?([^"]+)"?$"#)]
fn given_document_title(world: &mut E2eWorld, title: String) {
    world.document_title = Some(title);
}

#[given(regex = r"^the document contains the text (.+)$")]
async fn given_document_text(world: &mut E2eWorld, text: String) {
    let text = unquote(&text);
    let title = world
        .document_title
        .clone()
        .expect("document title must be given before its text");
    let filename = format!("{title}.md");
    let content = format!("# {title}\n\n{text}\n");

    let client = reqwest::Client::new();
    let (boundary, body) = multipart_body(&filename, "news", content.as_bytes());

    let upload_resp = client
        .post(format!("{}/admin/api/upload", base_url()))
        .header("x-admin-key", admin_key())
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(body)
        .send()
        .await
        .expect("upload request failed — is the stack up? (make up)");
    assert_eq!(
        upload_resp.status().as_u16(),
        201,
        "upload should return 201"
    );
    let upload_body: serde_json::Value = upload_resp.json().await.expect("invalid upload response");
    let token = upload_body["token"]
        .as_str()
        .expect("upload response missing token")
        .to_string();

    let confirm_resp = client
        .post(format!("{}/admin/api/upload/confirm/{token}", base_url()))
        .header("x-admin-key", admin_key())
        .send()
        .await
        .expect("confirm request failed");
    assert_eq!(
        confirm_resp.status().as_u16(),
        200,
        "confirm should return 200 — real chunk/embed/insert via ingest-core"
    );
}

#[given(regex = r"^the knowledge base contains no document about (.+)$")]
fn given_no_document(_world: &mut E2eWorld, _topic: String) {
    // No action: relies on the freshly-seeded live kb.db genuinely having
    // nothing related to this topic (see plan 0025, Task 1.1 and Risks).
}

#[given("an active persona is configured with a system prompt and a fallback message")]
async fn given_persona(world: &mut E2eWorld) {
    let fallback_message =
        "Non ho trovato informazioni nei documenti comunali su questo argomento.".to_string();
    let body = json!({
        "name": "gaspare-e2e",
        "system_prompt": "Sei Gaspare Spontini, sindaco di Maiolati Spontini.",
        "fallback_message": fallback_message,
        "activate": true,
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/admin/api/persona", base_url()))
        .header("x-admin-key", admin_key())
        .json(&body)
        .send()
        .await
        .expect("persona insert request failed — is the stack up? (make up)");
    assert_eq!(
        resp.status().as_u16(),
        201,
        "persona insert+activate should return 201"
    );

    world.fallback_message = Some(fallback_message);
}

#[when(regex = r"^the citizen asks (.+)$")]
async fn when_citizen_asks(world: &mut E2eWorld, question: String) {
    let question = unquote(&question);
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/chat", base_url()))
        .json(&json!({ "question": question }))
        .send()
        .await
        .expect("chat request failed — is the stack up? (make up)");

    world.response_status = Some(resp.status().as_u16());
    world.response_body = Some(resp.text().await.expect("failed to read /chat response"));
}

#[then("Spontini answers using the content of the retrieved document")]
async fn then_uses_document_content(world: &mut E2eWorld) {
    assert_eq!(world.response_status, Some(200));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(
        body["fell_back"], false,
        "expected the real RAG path, not the honest-unknown fallback"
    );
    let answer = body["answer"].as_str().unwrap();
    assert!(!answer.is_empty(), "answer should not be empty");
    // Real generation output is not deterministic — assert it is NOT the
    // fallback message (proving generation actually ran) rather than
    // asserting exact wording.
    let fallback = world.fallback_message.as_deref().unwrap_or_default();
    assert_ne!(
        answer, fallback,
        "answer should be model-generated content, not the configured fallback"
    );
}

#[then("Spontini cites the source document by title")]
async fn then_cites_source(world: &mut E2eWorld) {
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    let sources = body["sources"].as_array().unwrap();
    assert!(!sources.is_empty(), "expected at least one cited source");
    let title = world
        .document_title
        .as_deref()
        .expect("no document title recorded");
    let cited = sources[0]["source_ref"].as_str().unwrap();
    assert!(
        cited.contains(title),
        "expected cited source_ref '{cited}' to reference the seeded document title '{title}'"
    );
}

#[then(
    "the final prompt keeps the persona, retrieved context, and question as three separate parts"
)]
async fn then_prompt_parts_separated(_world: &mut E2eWorld) {
    // Not observable from outside the HTTP boundary — the real GenerationPort
    // adapter has no introspection hook for the prompt it sent. This
    // architectural invariant is already proven by the in-process `make bdd`
    // suite (backend/tests/bdd.rs), which has direct access to `PromptParts`.
}

#[then("Spontini answers with the fallback message")]
async fn then_fallback_answer(world: &mut E2eWorld) {
    assert_eq!(world.response_status, Some(200));
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["fell_back"], true);
    let answer = body["answer"].as_str().unwrap();
    let fallback = world
        .fallback_message
        .as_deref()
        .expect("fallback message not seeded");
    // The honest-unknown path is config-driven, not model-generated — even
    // against the real stack this is a deterministic exact match.
    assert_eq!(answer, fallback);
}

#[then("Spontini does not cite any document")]
async fn then_no_citations(world: &mut E2eWorld) {
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    let sources = body["sources"].as_array().unwrap();
    assert!(sources.is_empty());
}

#[then("Spontini does not invent any detail")]
async fn then_no_hallucination(world: &mut E2eWorld) {
    // The honest-unknown path never calls the generation model (Constitution
    // §5). There is no external hook to count generation calls against a
    // live stack, so the black-box proxy is: the answer is byte-identical to
    // the configured fallback message (asserted in `then_fallback_answer`)
    // and `fell_back` is true.
    let body: serde_json::Value =
        serde_json::from_str(world.response_body.as_ref().unwrap()).unwrap();
    assert_eq!(body["fell_back"], true);
}

#[tokio::main]
async fn main() {
    E2eWorld::cucumber()
        .fail_on_skipped()
        .run_and_exit(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../features/chat.feature"
        ))
        .await;
}
