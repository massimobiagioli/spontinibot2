# Plan 0003: rag-engine — Retrieval-Augmented Generation for `/chat`

- **Status**: closed
- **Approved**: 2026-07-09 by Sisyphus (opencode)
- **Closed**: 2026-07-09 by Sisyphus (opencode)
- **Review verdict**: approved (no required fixes)
- **Branch**: feat/rag-engine
- **Feature ID**: 0003
- **Created**: 2026-07-09
- **Owner**: Sisyphus (opencode)

## Objective

Transform the `backend`'s `/chat` endpoint from a walking-skeleton stub (`{"answer":"(walking skeleton)","sources":[]}`) into a real Retrieval-Augmented-Generation flow that serves the [Constitution §1 mission](../docs/CONSTITUTION.md#1-mission): a citizen asks a question, Spontini answers **only** from retrieved municipal documents, **cites the source**, and **honestly admits when it does not know** ([Constitution §5](../docs/CONSTITUTION.md#5-knowledge-base-rule)).

The flow, per [docs/STACK.md §2](../docs/STACK.md#2-architecture-overview) and the `spontini-rag-build` skill:

```
citizen question
  → embed query (llama-embed, HTTP /embedding)
  → retrieve chunks (kb-store, vector_distance_cos)
  → assemble 3-part prompt (persona + context + question — structurally separated)
  → generate (llama-generate, HTTP /v1/chat/completions)
  → answer + cited document ids
```

This plan delivers the `rag-engine` as a **module inside `backend`** (per [STACK.md §3.1](../docs/STACK.md#31-backend-core--axum) — "Hosts the `rag-engine` module"), built with Clean Architecture ports/adapters so the flow is fully unit-testable with test doubles and only the thin HTTP adapters touch the `llama.cpp` containers. After this plan, `POST /chat` returns a real answer grounded in `kb.db` with inline source citations, and the honest-unknown path is exercised by a BDD scenario.

**In scope:** `backend` dependency on `kb-store` and `reqwest`; `rag_engine/` module with domain types (`Answer`, `CitedSource`, `PromptParts`, `RagError`), ports (`EmbeddingPort`, `GenerationPort`, `RetrievalPort`, `PersonaPort`), adapters (KbStore-backed retrieval + persona, HTTP-backed embedding + generation), 3-part prompt assembly, the `RagEngine` use case orchestrating the ports with the honest-unknown fallback, `/chat` handler refactor to use `RagEngine` via dependency-injected `AppState`, BDD scenarios (answerable + not-answerable), unit tests with test doubles for every port and the orchestration.

**Out of scope:** the admin surface (`/admin/api/persona`, `/admin/api/upload`, `/admin/api/ingest/*`, `/admin/api/training/*`) — separate plans; `ingest-core` adapters (scraper, chunking) — separate plan; the frontend chat UI (popup widget, citation rendering) — separate plan; Design System Italia integration — separate plan; streaming responses (Constitution explicitly excludes real-time streaming); persona reload endpoint (`/admin/api/persona/reload`) — admin surface plan; any change to `kb-store`, `ingest-core`, `ingest`, `ingest-cli`, `frontend`, `admin-ui`, `docker-compose.yml`, or the Dockerfiles.

## Non-Goals

- This plan does NOT implement any `/admin/api/*` endpoint. Only the public `/chat` surface is wired.
- This plan does NOT implement the ingest pipeline. Documents are assumed to already exist in `kb.db` (inserted by tests via `kb-store`, or by a future `ingest-core` plan).
- This plan does NOT implement persona management UI or reload endpoint. The active persona is read from `kb-store` at request time (caching is deferred to a later optimization plan).
- This plan does NOT stream the generation. The full answer is awaited and returned in one JSON response (Constitution §3 excludes real-time streaming).
- This plan does NOT introduce a separate Domain crate. Domain types live in `backend/src/rag_engine/types.rs` and can be extracted later.
- This plan does NOT change the generation model (Qwen2.5-3B-Instruct per [ADR-0001](../.adr/0001-generation-model-3b.md)) or the embedding model (`nomic-embed-text`, 768-dim per `kb-store::EMBEDDING_DIM`).

## Phases

### Phase 1: rag-engine foundation — dependencies, domain types, ports

Goal: establish the `backend`'s new dependencies and the application-layer contract (domain types + port traits) that every adapter and the use case will build on. No external calls yet.

- [x] **Task 1.1** — Add `kb-store`, `reqwest`, `async-trait`, `thiserror` dependencies to `backend`
  - What: Update `backend/Cargo.toml` to add `kb-store = { path = "../kb-store" }` (validates the clean-arch matrix edge `kb-store ← backend`), `reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }` (outbound HTTP to `llama.cpp` containers, no native-tls to keep the image lean), `async-trait = "0.1"` (dyn-compatible ports), and `thiserror = "1"` (typed `RagError`). Keep `axum`, `tokio`, `serde`, `serde_json`, `tower`, `tracing`, `tracing-subscriber` as-is. Add `reqwest` to dev-deps only if a mock HTTP server is used in tests (Task 2.3/2.4 decide).
  - Deliverables:
    - `backend/Cargo.toml` updated with the four new deps under `[dependencies]`
  - Skills to load: spontini-clean-arch-guard
  - Verification: `cargo check -p backend` exits 0; `cargo check --workspace` exits 0 (no regression); the `kb-store` edge appears in the dependency graph.

- [x] **Task 1.2** — Define `rag_engine` module skeleton with domain types and `RagError`
  - What: Create `backend/src/rag_engine/mod.rs` declaring the submodules (`mod types; mod ports; mod prompt; mod retrieval; mod persona; mod embedding; mod generation; mod engine;`) and re-exporting the public surface. Create `backend/src/rag_engine/types.rs` with pure domain types (NO `kb-store`, NO `reqwest` imports — framework-agnostic per [PRINCIPLES.md §2](../docs/PRINCIPLES.md#2-clean-architecture)):
    - `#[derive(Debug, Clone, PartialEq)] pub struct RetrievedChunk { pub id: i64, pub source_ref: String, pub content: String, pub similarity: f64 }`
    - `#[derive(Debug, Clone, PartialEq)] pub struct CitedSource { pub document_id: i64, pub source_ref: String }`
    - `#[derive(Debug, Clone, PartialEq)] pub struct Answer { pub text: String, pub sources: Vec<CitedSource>, pub fell_back: bool }` — `fell_back` distinguishes the honest-unknown path for tests and UI
    - `#[derive(Debug, Clone, PartialEq)] pub struct PromptParts { pub system: String, pub context: String, pub user: String }` — the 3-part separation lives in code, enforced by the prompt assembler (Task 3.1)
    - `#[derive(Debug, thiserror::Error)] pub enum RagError { #[error("embedding service error: {0}")] Embedding(String), #[error("generation service error: {0}")] Generation(String), #[error("retrieval error: {0}")] Retrieval(String), #[error("persona error: {0}")] Persona(String), #[error("no active persona configured")] NoActivePersona }`
  - Add `mod rag_engine;` to `backend/src/lib.rs`.
  - Deliverables:
    - `backend/src/rag_engine/mod.rs` (submodule declarations + re-exports)
    - `backend/src/rag_engine/types.rs` (the five domain types)
    - `backend/src/lib.rs` patch (`mod rag_engine;`)
  - Skills to load: spontini-clean-arch-guard, spontini-tdd-rust, spontini-rag-build
  - Verification: `cargo test -p backend` compiles; `cargo doc -p backend --no-deps` succeeds without warnings; `grep -q 'use kb_store' backend/src/rag_engine/types.rs` returns 1 (no leak — types.rs is framework-agnostic).

- [x] **Task 1.3** — Define the four port traits (application-layer interfaces)
  - What: Create `backend/src/rag_engine/ports.rs` with four `#[async_trait]` traits, each returning domain types (never `kb-store` or `reqwest` types):
    - `#[async_trait] pub trait EmbeddingPort: Send + Sync { async fn embed(&self, text: &str) -> Result<Vec<f32>, RagError>; }`
    - `#[async_trait] pub trait RetrievalPort: Send + Sync { async fn retrieve(&self, query_embedding: &[f32], top_k: i64, min_score: f64) -> Result<Vec<RetrievedChunk>, RagError>; }`
    - `#[async_trait] pub trait PersonaPort: Send + Sync { async fn active_persona(&self) -> Result<Option<PersonaSnapshot>, RagError>; }` where `PersonaSnapshot { pub system_prompt: String, pub fallback_message: Option<String> }` is added to `types.rs` (a read-only projection of `kb_store::Persona` so the rag-engine does not depend on the full `kb_store::Persona` shape)
    - `#[async_trait] pub trait GenerationPort: Send + Sync { async fn generate(&self, prompt: PromptParts) -> Result<String, RagError>; }` — takes the 3-part `PromptParts` so the assembler (Task 3.1) owns the structural separation, the generation adapter only forwards it
  - Re-export the traits from `rag_engine/mod.rs`.
  - Deliverables:
    - `backend/src/rag_engine/ports.rs` (four traits)
    - `backend/src/rag_engine/types.rs` updated with `PersonaSnapshot`
  - Skills to load: spontini-clean-arch-guard, spontini-tdd-rust, spontini-rag-build
  - Verification: `cargo check -p backend` exits 0; each trait is dyn-compatible (`Box<dyn EmbeddingPort>` compiles in a smoke test inside `ports.rs` `#[cfg(test)]` module); no trait method accepts or returns a `kb_store::*` or `reqwest::*` type.

### Phase 2: Adapters — retrieval, persona, embedding, generation

Goal: implement the four port traits as thin adapters. The KbStore-backed adapters (`retrieval`, `persona`) convert between `kb-store` types and the rag-engine domain types. The HTTP-backed adapters (`embedding`, `generation`) call the `llama.cpp` containers and are tested with a mock HTTP server (or `wiremock`) so no live container is required for unit tests.

- [ ] **Task 2.1** — Implement `RetrievalAdapter` (KbStore-backed)
  - What: Create `backend/src/rag_engine/retrieval.rs` with `pub struct RetrievalAdapter { store: Arc<KbStore>, }` implementing `RetrievalPort`. `retrieve()` calls `store.search_similar(query_embedding, top_k, min_score)` and maps each `kb_store::ScoredDocument` to `RetrievedChunk { id, source_ref, content, similarity }` (dropping `source` and `metadata` — the rag-engine does not need them for v1; the document `id` is preserved for citation). Errors are wrapped via `RagError::Retrieval`. Add a unit test using a temp `KbStore` (`:memory:` path) that inserts two documents with known embeddings and asserts the adapter returns the nearest one first with the expected `id` and `source_ref`.
  - Deliverables:
    - `backend/src/rag_engine/retrieval.rs` (`RetrievalAdapter` + `impl RetrievalPort` + unit test)
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard, spontini-rag-build
  - Verification: `cargo test -p backend retrieval` passes; the test inserts docs via `KbStore::insert_document` (real libSQL) and asserts retrieval order — no test double for the DB.

- [ ] **Task 2.2** — Implement `PersonaAdapter` (KbStore-backed)
  - What: Create `backend/src/rag_engine/persona.rs` with `pub struct PersonaAdapter { store: Arc<KbStore>, }` implementing `PersonaPort`. `active_persona()` calls `store.get_active_persona()` and maps `Option<kb_store::Persona>` to `Option<PersonaSnapshot>` (only `system_prompt` + `fallback_message`). Errors are wrapped via `RagError::Persona`. Add a unit test using a temp `KbStore` that inserts an active persona and asserts the adapter returns its snapshot; add a second test with no active persona asserting `None`.
  - Deliverables:
    - `backend/src/rag_engine/persona.rs` (`PersonaAdapter` + `impl PersonaPort` + unit tests)
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard, spontini-rag-build
  - Verification: `cargo test -p backend persona` passes; both the active-persona and no-active-persona paths are covered.

- [ ] **Task 2.3** — Implement `EmbeddingAdapter` (HTTP `llama-embed`)
  - What: Create `backend/src/rag_engine/embedding.rs` with `pub struct EmbeddingAdapter { client: reqwest::Client, base_url: String }` implementing `EmbeddingPort`. `embed(text)` POSTs `{"content": text}` to `{base_url}/embedding` and parses `{"embedding": [f32; 768]}`, validating the dimension equals `kb_store::EMBEDDING_DIM` (returns `RagError::Embedding` on mismatch — the same model must be used for ingest and query per the rag-build skill). The `base_url` comes from config (Task 1.1 wiring; default `http://llama-embed:8080`). Add a unit test using `wiremock` (added to dev-deps) that stands up a mock `/embedding` endpoint returning a 768-dim vector and asserts the adapter returns it; add a second test that the adapter rejects a wrong-dimension response with `RagError::Embedding`. Before implementing, verify the actual `llama.cpp` server `/embedding` request/response shape by `curl`-ing the running `llama-embed` container (`make up` then `docker compose exec llama-embed ...` or `curl` from the `backend` container); document the verified shape in a module doc comment so Task 2.4 and future ingest work share it.
  - Deliverables:
    - `backend/src/rag_engine/embedding.rs` (`EmbeddingAdapter` + `impl EmbeddingPort` + wiremock unit tests)
    - `backend/Cargo.toml` dev-deps: `wiremock = "0.6"`
    - Module doc comment in `embedding.rs` recording the verified `POST /embedding` request/response contract
  - Skills to load: spontini-tdd-rust, spontini-rag-build, spontini-clean-arch-guard
  - Verification: `cargo test -p backend embedding` passes against the wiremock server; the dimension-validation test fails when the mock returns 512-dim; `cargo clippy -p backend -- -D warnings` clean.

- [ ] **Task 2.4** — Implement `GenerationAdapter` (HTTP `llama-generate`)
  - What: Create `backend/src/rag_engine/generation.rs` with `pub struct GenerationAdapter { client: reqwest::Client, base_url: String }` implementing `GenerationPort`. `generate(prompt: PromptParts)` assembles the OpenAI-compatible chat request body from the 3-part `PromptParts`:
    - `messages: [ { "role": "system", "content": prompt.system + "\n\n" + citation_instruction }, { "role": "user", "content": "<context>\n" + prompt.context + "\n</context>\n\n<question>\n" + prompt.user + "\n</question>" } ]`
    - `citation_instruction` is a constant: `"Rispondi UNICAMENTE usando il contesto fornito. Cita il documento di origine indicandone il titolo. Se il contesto non contiene la risposta, di' che non hai trovato l'informazione nei documenti comunali."`
    - `stream: false`, `max_tokens` from a constant (e.g., 512), `temperature: 0.3` (low temperature for grounded synthesis per the Constitution's truthfulness principle)
    - POST to `{base_url}/v1/chat/completions` (OpenAI-compatible endpoint exposed by `llama.cpp` server; applies the Qwen chat template automatically)
    - Parse `choices[0].message.content` as the answer string; wrap HTTP/parse errors via `RagError::Generation`
    - The `base_url` comes from config (default `http://llama-generate:8080`)
    - Add a unit test using `wiremock` that returns a canned OpenAI completion body and asserts the adapter extracts the content; add a test for the 500-error path returning `RagError::Generation`. Before implementing, verify the actual `llama.cpp` server `/v1/chat/completions` shape by `curl`-ing the running `llama-generate` container; record the contract in a module doc comment.
  - Deliverables:
    - `backend/src/rag_engine/generation.rs` (`GenerationAdapter` + `impl GenerationPort` + wiremock unit tests)
    - Module doc comment recording the verified `POST /v1/chat/completions` request/response contract
  - Skills to load: spontini-tdd-rust, spontini-rag-build, spontini-clean-arch-guard
  - Verification: `cargo test -p backend generation` passes against wiremock; the 500-error test asserts `RagError::Generation`; clippy clean.

### Phase 3: Prompt assembly + RagEngine use case

Goal: assemble the 3-part prompt from the retrieved chunks + active persona + citizen question (keeping the three parts structurally separated per the rag-build skill), and orchestrate the full flow in the `RagEngine` use case with the honest-unknown fallback.

- [ ] **Task 3.1** — Implement `PromptParts` assembler (3-part separation, non-negotiable)
  - What: Create `backend/src/rag_engine/prompt.rs` with a pure function `pub fn assemble(persona: &PersonaSnapshot, chunks: &[RetrievedChunk], question: &str) -> PromptParts` that builds the three structurally-separated parts:
    - `system = persona.system_prompt.clone()` — persona instructions ONLY, never chunks, never the question
    - `context = chunks.iter().map(|c| format!("[Fonte: {}]\n{}", c.source_ref, c.content)).collect::<Vec<_>>().join("\n\n---\n\n")` — retrieved chunks ONLY, never persona, never the question; each chunk is prefixed with its `source_ref` so the generation model can cite it
    - `user = question.to_string()` — the citizen question ONLY, never persona, never chunks
    - The `PromptParts` struct keeps them as three separate `String` fields; the `GenerationAdapter` (Task 2.4) is the ONLY place that concatenates them into chat messages, and it does so with explicit delimiters — no other code path may merge the parts.
    - Add unit tests: (a) the three fields contain exactly persona, chunks, and question respectively (assert `!system.contains(question)`, `!context.contains(persona.system_prompt)`, `!user.contains(&chunks[0].content)`); (b) empty chunks list produces an empty `context` string (the honest-unknown path still calls generate with empty context so the model says "not found" rather than the adapter silently bypassing generation — but the `RagEngine` Task 3.2 decides whether to fall back BEFORE calling generate); (c) multiple chunks are joined with the separator and each prefixed with its source.
  - Deliverables:
    - `backend/src/rag_engine/prompt.rs` (`assemble` + unit tests)
  - Skills to load: spontini-rag-build, spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo test -p backend prompt` passes; the separation-invariant tests (persona never in context, question never in system, chunks never in user) are explicit and green.

- [ ] **Task 3.2** — Implement `RagEngine` use case orchestrating the four ports
  - What: Create `backend/src/rag_engine/engine.rs` with `pub struct RagEngine { embedding: Arc<dyn EmbeddingPort>, retrieval: Arc<dyn RetrievalPort>, persona: Arc<dyn PersonaPort>, generation: Arc<dyn GenerationPort>, top_k: i64, min_score: f64 }` and a single public method `pub async fn answer(&self, question: &str) -> Result<Answer, RagError>`:
    1. `let persona = self.persona.active_persona().await?.ok_or(RagError::NoActivePersona)?;`
    2. `let qe = self.embedding.embed(question).await?;`
    3. `let chunks = self.retrieval.retrieve(&qe, self.top_k, self.min_score).await?;`
    4. **Honest-unknown branch**: if `chunks.is_empty()`, return `Answer { text: persona.fallback_message.unwrap_or_else(|| "Non ho trovato informazioni nei documenti comunali su questo argomento.".into()), sources: vec![], fell_back: true }` — do NOT call the generation model, do NOT invent, do NOT cite any document (Constitution §5).
    5. **Grounded branch**: `let prompt = prompt::assemble(&persona, &chunks, question); let text = self.generation.generate(prompt).await?; let sources = chunks.iter().map(|c| CitedSource { document_id: c.id, source_ref: c.source_ref.clone() }).collect(); Answer { text, sources, fell_back: false }`.
    - Add a constructor `pub fn new(embedding, retrieval, persona, generation, top_k, min_score) -> Self`.
    - Add unit tests with hand-written test doubles implementing the four ports (a `TestEmbedding` returning a fixed vector, a `TestRetrieval` configurable to return chunks or empty, a `TestPersona` returning a fixed snapshot, a `TestGeneration` returning a fixed string):
      - `should_return_grounded_answer_with_cited_sources_when_chunks_found`
      - `should_return_fallback_answer_with_no_sources_when_no_chunks` (the honest-unknown path — assert `fell_back == true` and `sources.is_empty()` and `text` is the fallback)
      - `should_return_no_active_persona_error_when_persona_missing` (asserts `Err(RagError::NoActivePersona)`)
      - `should_propagate_embedding_error` (asserts `Err(RagError::Embedding(_))`)
      - `should_not_call_generation_when_no_chunks_found` (asserts the `TestGeneration` counter is zero — proves the honest-unknown path does not invoke the model)
  - Deliverables:
    - `backend/src/rag_engine/engine.rs` (`RagEngine` + `answer()` + test doubles + unit tests)
  - Skills to load: spontini-rag-build, spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo test -p backend engine` passes all five tests; the "does not call generation when no chunks" test is explicit and green (proves Constitution §5).

### Phase 4: Wire `/chat` to `RagEngine` + BDD scenarios

Goal: replace the stub `/chat` handler with a real RAG flow driven by `RagEngine`, using dependency injection so the BDD scenarios can run with test doubles (no live `llama.cpp` containers needed for tests).

- [ ] **Task 4.1** — Refactor `router()` to accept `AppState` with the `RagEngine` (dependency injection)
  - What: Refactor `backend/src/lib.rs` so the router carries an axum `State<AppState>`:
    - `#[derive(Clone)] pub struct AppState { rag_engine: Arc<RagEngine> }`
    - `pub fn router() -> Router` — builds the production `AppState` from env vars (`LLAMA_EMBED_URL`, `LLAMA_GENERATE_URL`, `KB_DB_PATH` with defaults `http://llama-embed:8080`, `http://llama-generate:8080`, `/data/kb.db`; `RAG_TOP_K` default `5`, `RAG_MIN_SCORE` default `0.35`) and wires the four real adapters (`EmbeddingAdapter`, `GenerationAdapter`, `RetrievalAdapter`, `PersonaAdapter` sharing an `Arc<KbStore::open(kb_path)>`). Expose `pub fn router_with(state: AppState) -> Router` for tests. The existing `GET /health` and `GET /` routes keep working unchanged (they ignore state). Update `backend/src/main.rs` if needed to call `router()` (it already does — no change expected).
    - Add a `pub mod config;` with a `pub struct Config { embed_url, generate_url, kb_path, top_k, min_score }` and a `pub fn from_env() -> Config` reading the env vars with defaults. `router()` builds `Config::from_env()` then the adapters.
  - Deliverables:
    - `backend/src/lib.rs` refactored (`AppState`, `router()`, `router_with()`)
    - `backend/src/config.rs` (`Config` + `from_env`)
    - `backend/src/routes.rs` — `/health` and `/` unchanged; `/chat` updated in Task 4.2
  - Skills to load: spontini-clean-arch-guard, spontini-tdd-rust
  - Verification: `cargo test -p backend --test bdd` (the existing health scenario) still passes — `router()` still returns a working router with `/health`; `cargo check -p backend` exits 0.

- [ ] **Task 4.2** — Implement the `/chat` handler using `RagEngine`
  - What: Replace the stub `chat()` handler in `backend/src/routes.rs` with an `axum::extract::State<AppState>` handler that:
    - Accepts `Json<ChatRequest>` where `ChatRequest { question: String }` (derive `Deserialize`)
    - Calls `state.rag_engine.answer(&req.question).await`
    - On `Ok(answer)`: returns `Json(ChatResponse { answer: answer.text, sources: answer.sources.iter().map(|s| ChatSource { document_id: s.document_id, source_ref: s.source_ref.clone() }).collect(), fell_back: answer.fell_back })` with HTTP 200
    - On `Err(RagError::NoActivePersona)`: returns HTTP 503 with `{"error":"no active persona configured"}` (the operator must configure a persona via the admin surface — out of scope here, but the error is surfaced honestly)
    - On `Err(RagError::Embedding(_) | Generation(_) | Retrieval(_) | Persona(_))`: logs the error via `tracing::error!`, returns HTTP 502 with `{"error":"upstream service unavailable"}` (the citizen-facing frontend will render the honest "I can't answer right now" state — no 500 leak)
    - Define `ChatRequest`, `ChatSource`, `ChatResponse` as `Serialize`/`Deserialize` structs in `routes.rs` (or a new `backend/src/dto.rs` if cleaner). Keep the existing `HealthResponse`.
  - Deliverables:
    - `backend/src/routes.rs` — `chat()` handler refactored; `ChatRequest`/`ChatResponse`/`ChatSource` DTOs
    - `backend/src/lib.rs` — `/chat` route wired with `State<AppState>` (`route("/chat", post(routes::chat).with_state(...))` or axum-idiomatic `Router::with_state`)
  - Skills to load: spontini-rag-build, spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo test -p backend` (unit tests) passes; `cargo check -p backend` exits 0; a manual `curl -X POST http://localhost:8080/chat -d '{"question":"test"}' -H 'content-type: application/json'` against a running stack (with an active persona inserted) returns a real answer or a 503/502 — not the stub string.

- [ ] **Task 4.3** — Write the BDD scenarios for `/chat` (answerable + honest-unknown) with test doubles
  - What: Write `features/chat.feature` in English, in the domain language (no HTTP jargon), following [PRINCIPLES.md §5](../docs/PRINCIPLES.md#5-bdd--behavior-driven-development):
    ```gherkin
    Feature: Answering citizen questions from the knowledge base

      Scenario: A citizen asks a question answerable from a municipal document
        Given the knowledge base contains a document titled "Orari sportello anagrafe"
        And an active persona is configured for Spontini
        When the citizen asks "A che ora apre l'anagrafe?"
        Then Spontini answers using the content of the retrieved document
        And Spontini cites the source document

      Scenario: A citizen asks a question not answerable from any document
        Given the knowledge base contains no document about "tasse comunali"
        And an active persona is configured with a fallback message
        When the citizen asks "Quanto pago di tasse comunali?"
        Then Spontini answers with the fallback message
        And Spontini does not cite any document
        And Spontini does not invent details
    ```
    Extend `backend/tests/bdd.rs` with a second `World` (`ChatWorld`) and step definitions. The steps build a `router_with(test_state)` where the test state injects hand-written test doubles for the four ports (the same doubles from Task 3.2, extracted to `backend/tests/support/` or a `#[cfg(test)]` module so both unit and BDD tests reuse them). The answerable scenario configures `TestRetrieval` to return a chunk with `source_ref = "Orari sportello anagrafe"` and `TestGeneration` to return a string that mentions that title; the honest-unknown scenario configures `TestRetrieval` to return `vec![]` and asserts the response body's `answer` equals the persona fallback and `sources` is empty. The HTTP call is made via `tower::ServiceExt::oneshot` (no network — same pattern as the existing health BDD). Keep the existing `HealthWorld` and its steps intact; add the chat world alongside.
  - Deliverables:
    - `features/chat.feature` (two scenarios, English, domain language)
    - `backend/tests/bdd.rs` extended (`ChatWorld` + step definitions + shared test doubles)
    - `backend/tests/support/mod.rs` (shared test doubles: `TestEmbedding`, `TestRetrieval`, `TestPersona`, `TestGeneration` — extracted so unit and BDD tests both use them)
  - Skills to load: spontini-bdd-gherkin, spontini-rag-build, spontini-tdd-rust
  - Verification: `cargo test -p backend --test bdd` passes — both the existing health scenario and the two new chat scenarios are green; the honest-unknown scenario's `sources` assertion is explicit (empty array); `cargo test -p backend` (all targets) passes.

### Phase 5: End-to-end verification gate

Goal: prove the whole `backend` (and the workspace) is buildable, tested, linted, formatted, and that the Docker compose config is still valid.

- [ ] **Task 5.1** — Run the full `spontini-verify-gate` on the workspace
  - What: Run, in order, capturing the output of each:
    - `cargo check --workspace`
    - `cargo test --workspace --all-targets` (unit tests for `kb-store`, `backend`, the other skeleton crates, and the `bdd` test)
    - `cargo clippy --workspace --all-targets -- -D warnings`
    - `cargo fmt --all -- --check`
    - `cargo doc -p backend --no-deps` (no warnings — the rag-engine public surface is documented)
    - `docker compose config -q` (compose still valid — this plan does NOT touch compose, but the gate checks it)
    - A manual smoke test against the running stack: `make up`, insert an active persona via a one-off `ingest-cli` or a `docker compose exec backend` SQL insert (or a small `cargo run -p ingest-cli` if it can insert a persona — otherwise document the SQL), `curl -X POST http://localhost:8080/chat -d '{"question":"hello"}' -H 'content-type: application/json'` and confirm a 200 with a real answer (or a 503 if no persona is active — both prove the stub is gone). If the models are not provisioned, document that the smoke test requires `make provision-models` and skip with an explicit note.
    - Do NOT fix pre-existing failures in other crates; note them explicitly if present.
  - Deliverables:
    - (no new files — verification only; create `coverage-exclusions.txt` only if a justified exclusion is needed, e.g., the `main.rs` composition root per [PRINCIPLES.md §7](../docs/PRINCIPLES.md#7-100-test-coverage-on-the-codebase))
  - Skills to load: spontini-verify-gate
  - Verification: every command above exits 0, or any non-zero is explicitly noted as pre-existing/unrelated to this plan's changes; the rag-engine module's line coverage meets the 100% line / 80% branch gate (tarpaulin via `make coverage` if available in the container, otherwise documented).

## Acceptance Criteria

- `cargo test -p backend` passes (all rag-engine unit tests: adapters, prompt assembly, engine orchestration, test-doubles).
- `cargo test -p backend --test bdd` passes — the existing `features/health.feature` scenario AND the two new `features/chat.feature` scenarios are green.
- `cargo test --workspace` passes (no regression in `kb-store` or the skeleton crates).
- `cargo clippy --workspace --all-targets -- -D warnings` is clean; `cargo fmt --all -- --check` is clean; `cargo doc -p backend --no-deps` is warning-free.
- `POST /chat` with a question, against a stack with an active persona and at least one relevant document, returns `{"answer": "...", "sources": [{"document_id": N, "source_ref": "..."}], "fell_back": false}` — NOT the `"(walking skeleton)"` stub.
- `POST /chat` with a question that matches no document returns `{"answer": "<fallback>", "sources": [], "fell_back": true}` — the honest-unknown path is observable (Constitution §5).
- `POST /chat` with no active persona configured returns HTTP 503 `{"error":"no active persona configured"}`.
- `POST /chat` when `llama-embed` or `llama-generate` is unreachable returns HTTP 502 `{"error":"upstream service unavailable"}`.
- The 3-part prompt separation is enforced in code: `PromptParts` has three separate `String` fields; the `prompt::assemble` tests assert persona never leaks into context, the question never leaks into system, and chunks never leak into user.
- The honest-unknown path is provably not calling the generation model (the `should_not_call_generation_when_no_chunks_found` test asserts the `TestGeneration` call counter is zero).
- The same embedding model constraint is respected: `EmbeddingAdapter` validates the response dimension against `kb_store::EMBEDDING_DIM` and returns `RagError::Embedding` on mismatch.
- `docker compose config -q` exits 0 (compose untouched but validated).
- The backend public API surface (`cargo doc -p backend --no-deps`) exposes `rag_engine::{RagEngine, Answer, CitedSource, PromptParts, RagError, ...}` with doc comments; the port traits are `pub` (for test doubles) and the adapters are `pub` (for `router()` wiring).

## Risks

- **`llama.cpp` server API shape** — The `/embedding` and `/v1/chat/completions` request/response contracts are assumed from the `llama.cpp` server README. Mitigation: Tasks 2.3 and 2.4 begin by `curl`-ing the running containers to verify the exact shape; the verified contract is recorded in a module doc comment so future ingest work (`ingest-core`) reuses it. If the endpoint differs, the adapter is adjusted to match — the port trait stays the same.
- **`llama.cpp` server default port** — The `docker-compose.yml` does not expose `llama-embed`/`llama-generate` ports to the host; they are reachable inside the Docker network on the default `llama-server` port (`8080`). Mitigation: `Config::from_env()` defaults to `http://llama-embed:8080` and `http://llama-generate:8080`; if the actual port differs, override via `LLAMA_EMBED_URL` / `LLAMA_GENERATE_URL` env vars in compose (a follow-up compose patch, NOT this plan).
- **OpenAI-compatible endpoint availability** — The `/v1/chat/completions` endpoint must be enabled in the `llama.cpp` server image. Mitigation: if it is not available, fall back to the raw `/completion` endpoint with a manually-rendered Qwen chat template (the `PromptParts` 3-part structure is preserved either way); document the fallback as a risk note in `generation.rs`.
- **Qwen2.5-3B citation quality** — The 3B model ([ADR-0001](../.adr/0001-generation-model-3b.md)) may not always cite the source title even when instructed. Mitigation: the response DTO returns the retrieved `CitedSource` list independent of the model's prose — the frontend renders citations from the DTO, not by parsing the answer text. The model's job is synthesis, not citation mechanics.
- **Honest-unknown threshold tuning** — `RAG_MIN_SCORE` default `0.35` is a guess; too high and the bot falls back too often, too low and it hallucinates from weak matches. Mitigation: the value is a config constant tunable via env without code change; the BDD honest-unknown scenario uses `TestRetrieval` returning empty (threshold-independent) so the fallback path is tested regardless of the production default.
- **`reqwest` with `rustls-tls` in the container** — The `llama.cpp` containers serve plain HTTP, so TLS is only relevant if the embed/generate URLs are HTTPS (not the default). Mitigation: `default-features = false, features = ["json", "rustls-tls"]` keeps the image lean; plain HTTP calls work without TLS. If the build stage lacks the rustls ring assembly, fall back to `default-tls` — note in the plan review if discovered.
- **Coverage gate on the `main.rs` composition root** — `backend/src/main.rs` is exempt per [PRINCIPLES.md §7](../docs/PRINCIPLES.md#7-100-test-coverage-on-the-codebase) §7.1 (main entry points). Mitigation: if `cargo tarpaulin` flags it, add a justified line to `coverage-exclusions.txt`. No test is deleted or `#[ignore]`d to pass.
- **BDD test doubles vs. real adapters** — The BDD scenarios run with test doubles, so they do not exercise the real HTTP adapters against live `llama.cpp` containers. Mitigation: the HTTP adapters have their own `wiremock` unit tests (Tasks 2.3, 2.4); the BDD proves the orchestration and the honest-unknown contract; a separate end-to-end smoke test (Task 5.1 manual step) against the running stack validates the real adapters. Full end-to-end BDD against live containers is deferred to a future integration-test plan.
- **`async-trait` dyn-compatibility** — All four ports must be dyn-compatible to live behind `Arc<dyn Port>`. Mitigation: each trait uses `&self` (not `&mut self`) and returns `Future<Output = Result<...>>` via `#[async_trait]`; a `#[cfg(test)]` smoke test in `ports.rs` asserts `Box<dyn EmbeddingPort>` compiles.

## Out-of-Scope

- The admin surface (`/admin/api/persona`, `/admin/api/persona/reload`, `/admin/api/upload`, `/admin/api/ingest/config`, `/admin/api/ingest/run`, `/admin/api/training/*`) — separate plans.
- The `ingest-core` crate (scraper adapter, chunking, embedding pipeline for ingest) — separate plan.
- The frontend chat UI (popup widget, citation rendering, honest-unknown state UI) — separate plan.
- Design System Italia / Bootstrap Italia integration — separate plan.
- Streaming responses — Constitution §3 explicitly excludes real-time streaming.
- Persona caching and `/admin/api/persona/reload` — the active persona is read per-request in this plan; caching is a later optimization.
- Metadata-filtered retrieval (category, tags, priority/trust_score before vector distance) — the `RetrievalPort` signature in this plan is filter-free; a future plan extends it when the admin upload + metadata UI lands.
- A separate Domain crate — domain types live in `backend/src/rag_engine/types.rs` and are extracted only when a second consumer (e.g., the admin surface) needs them.
- Changes to `kb-store`, `ingest-core`, `ingest`, `ingest-cli`, `frontend`, `admin-ui`, `docker-compose.yml`, or any Dockerfile.
- CI pipeline wiring and README status badges — separate plan.
