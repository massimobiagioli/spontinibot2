# Plan 0005: ingest-core — scraper adapter, chunking, embedding pipeline

- **Status**: review
- **Approved**: 2026-07-09 by Sisyphus
- **Implemented**: 2026-07-09 by Sisyphus
- **Branch**: feat/ingest-core-scraper-adapter-chunking-embedding-pipeline
- **Feature ID**: 0005
- **Created**: 2026-07-09
- **Owner**: Sisyphus

## Objective

Build `ingest-core` from a version-string skeleton into a real shared library that powers Spontini's ingest pipeline. The crate must implement three core capabilities: a **scraper adapter** that fetches a URL, extracts visible text, and respects `robots.txt` and content-type allowlisting; a **chunking module** that splits extracted text into overlapping ~512-token segments with section-tagged metadata; and an **embedding client** that sends chunk text to the `llama-embed` container and validates the 768-dim response against `kb_store::EMBEDDING_DIM`. These three capabilities are orchestrated by a `Pipeline` trait and an `IngestPipeline` orchestrator (scrape → chunk → embed → `KbStore::insert_document`). The `api-client` adapter exists as a stub and is explicitly NOT wired into the orchestrator. All HTTP interactions are unit-tested with `wiremock`. This feature delivers the shared library that both the `ingest` scheduler (feature 0006) and the `backend` admin upload (feature 0009) will consume; no scheduler, no container, no admin-ui wiring is built here.

## Non-Goals

- No scheduler or always-on service (deferred to feature 0006).
- No containerization or Docker wiring (deferred to feature 0006).
- No `backend` admin upload integration (deferred to feature 0009).
- No `api-client` adapter implementation beyond a stub trait.
- No folder or DB source adapters (out of project scope per STACK.md §3.6).
- No PDF/docx file parsing (deferred to feature 0009 upload flow).
- No `ingest-cli` changes (deferred to feature 0007).

## Phases

### Phase 1: Crate dependencies and module skeleton

Goal: Set up `ingest-core` with the required dependencies and module structure so subsequent phases can add implementations without Cargo.toml churn.

- [x] **Task 1.1** — Add crate dependencies and define public module layout
  - What: Add `reqwest` (with `rustls-tls`), `scraper`, `tokio` (rt, macros), `async-trait`, `serde`/`serde_json`, `thiserror`, `tracing`, `url`, `kb-store` as dependencies; add `wiremock` and `tokio` (test-util) as dev-dependencies. Create module stubs: `scraper` (pub mod), `chunking` (pub mod), `embed` (pub mod), `pipeline` (pub mod), `error` (pub mod). Re-export the public API from `lib.rs`.
  - Deliverables:
    - `ingest-core/Cargo.toml` with all dependencies
    - `ingest-core/src/scraper.rs` (module stub with `pub fn version()`)
    - `ingest-core/src/chunking.rs` (module stub)
    - `ingest-core/src/embed.rs` (module stub)
    - `ingest-core/src/pipeline.rs` (module stub)
    - `ingest-core/src/error.rs` (error types)
    - Updated `ingest-core/src/lib.rs` (module declarations + re-exports)
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo check -p ingest-core` passes; `cargo test -p ingest-core` passes (existing version test still green).

- [ ] **Task 1.2** — Define `IngestError` enum and public error types
  - What: Create an error enum in `error.rs` covering: `Http`, `RobotsTxt`, `ContentType`, `Chunking`, `Embedding`, `DimensionMismatch`, `KbStore`. Derive `thiserror::Error` and `Display`. Implement `From` conversions for `reqwest::Error`, `kb_store::KbStoreError`, and the embedding dimension validation. Unit test the error Display and From impls.
  - Deliverables:
    - `ingest-core/src/error.rs` with `IngestError` enum
    - Unit tests for error Display and From conversions
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p ingest-core` passes with new error tests.

### Phase 2: Scraper adapter

Goal: Implement the HTTP scraper adapter that fetches a URL, extracts visible text, honors `robots.txt`, and enforces a content-type allowlist.

- [ ] **Task 2.1** — Implement `ScraperAdapter` with HTTP fetch, content-type allowlist, and text extraction
  - What: Define a `ScraperAdapter` struct holding a `reqwest::Client` and a base user-agent. Implement a `fetch_text(&self, url: &str) -> Result<String, IngestError>` method that: (1) sends a GET request, (2) checks the response `Content-Type` against an allowlist (`text/html`, `text/plain`, `application/xhtml+xml` — reject anything else with `IngestError::ContentType`), (3) parses HTML with the `scraper` crate and extracts visible text from `<body>` (strip `<script>`, `<style>`, nav elements), (4) returns the plain-text content. Unit test with `wiremock` for: successful HTML page, disallowed content-type, server error (5xx).
  - Deliverables:
    - `ingest-core/src/scraper.rs` with `ScraperAdapter` struct and `fetch_text` method
    - Wiremock-based unit tests for happy path, content-type rejection, and HTTP error
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p ingest-core -- --test-threads=1` passes with all scraper tests.

- [ ] **Task 2.2** — Implement `robots.txt` fetching and honoring
  - What: Add a `fetch_robots(url: &str) -> Result<bool, IngestError>` method that downloads `{origin}/robots.txt`, parses it using the `url` crate for origin extraction and manual `robots.txt` rule matching (no external crate — parse the well-known format: `User-agent`, `Disallow`, `Allow`, `Sitemap` lines). Cache the parsed rules in memory for the adapter's lifetime. Before fetching any URL in `fetch_text`, check if the path is disallowed for the adapter's user-agent; if disallowed, return `IngestError::RobotsTxt`. Unit test with `wiremock`: allowed path, disallowed path, no robots.txt (treat as allow-all).
  - Deliverables:
    - Robots.txt parser and checker integrated into `ScraperAdapter`
    - Wiremock-based unit tests for robots.txt scenarios
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p ingest-core` passes with all robots.txt tests.

### Phase 3: Chunking module

Goal: Implement a recursive text splitter that produces ~512-token overlapping chunks with section-tagged metadata.

- [ ] **Task 3.1** — Implement naive token counting and recursive text splitter
  - What: Create a `Chunker` struct with configurable chunk size (~512 tokens), overlap (~64 tokens), and a `naive_token_count(text: &str) -> usize` helper that estimates tokens as `text.len() / 4` (rough character-to-token ratio for Italian/English text). Implement `Chunker::chunk(text: &str, section_tag: &str) -> Vec<Chunk>` that: (1) splits text on paragraph boundaries (`\n\n`) into segments, (2) accumulates segments until the token budget is reached, (3) when the next segment would exceed budget, finalizes the current chunk and starts a new one with overlap from the previous chunk's tail, (4) assigns each chunk metadata with the section tag and chunk index. Define a `Chunk` struct with `content: String`, `section_tag: String`, `chunk_index: usize`, `token_count: usize`. Unit tests for: single-chunk input, multi-chunk output, overlap boundaries, empty text.
  - Deliverables:
    - `ingest-core/src/chunking.rs` with `Chunker` struct, `Chunk` struct, `naive_token_count` helper
    - Unit tests for chunking behavior
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p ingest-core` passes with all chunking tests.

- [ ] **Task 3.2** — Section-tagged metadata and edge cases
  - What: Extend `Chunk` metadata to include the section tag in a JSON metadata field (`{"section": "<tag>", "source_url": "<url>"}`). Handle edge cases: very long single paragraph (split mid-paragraph when a single segment exceeds budget), text shorter than overlap (single chunk, no overlap), section tag with special characters. Unit tests for each edge case.
  - Deliverables:
    - Extended `Chunk` metadata with JSON serialization
    - Edge-case unit tests
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p ingest-core` passes with all edge-case tests.

### Phase 4: Embedding client

Goal: Implement an HTTP client that sends chunk text to `llama-embed` and validates the 768-dim response.

- [ ] **Task 4.1** — Implement `EmbeddingClient` that POSTs to `llama-embed`
  - What: Define an `EmbeddingClient` struct with a `reqwest::Client` and `base_url`. Implement `embed_chunk(&self, text: &str) -> Result<Vec<f32>, IngestError>` that: (1) POSTs `{"content": "<text>"}` to `{base_url}/embedding`, (2) parses the nested response (`[{"index": 0, "embedding": [[0.01, -0.02, ...]]}]`) following the same `llama.cpp` server contract as the backend's `EmbeddingAdapter`, (3) validates the returned vector length equals `kb_store::EMBEDDING_DIM` (768), (4) returns `IngestError::Embedding` or `IngestError::DimensionMismatch` on failure. Unit test with `wiremock`: successful 768-dim response, wrong-dimension response, HTTP error, empty response.
  - Deliverables:
    - `ingest-core/src/embed.rs` with `EmbeddingClient` struct and `embed_chunk` method
    - Wiremock-based unit tests
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p ingest-core` passes with all embedding client tests.

### Phase 5: Pipeline orchestrator

Goal: Define the `Pipeline` trait and implement the `IngestPipeline` orchestrator that composes scrape → chunk → embed → insert.

- [ ] **Task 5.1** — Define `Pipeline` trait with `run` method
  - What: Define `Pipeline` trait in `pipeline.rs` with an `async fn run(&self, url: &str, section: &str) -> Result<(), IngestError>` method. The trait is the public entry point for both the ingest scheduler (feature 0006) and the backend admin upload (feature 0009).
  - Deliverables:
    - `Pipeline` trait in `pipeline.rs` with `run` method
    - Object-safety test (assert `Box<dyn Pipeline>` compiles)
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo check -p ingest-core` passes.

- [ ] **Task 5.2** — Implement `IngestPipeline` orchestrator
  - What: Implement `IngestPipeline` struct that holds a `ScraperAdapter`, a `Chunker`, an `EmbeddingClient`, and a `kb_store::KbStore`. Implement `Pipeline::run`: (1) call `ScraperAdapter::fetch_text(url)` with robots.txt check, (2) call `Chunker::chunk(text, section)` to produce chunks, (3) for each chunk call `EmbeddingClient::embed_chunk` then `KbStore::insert_document` with `DocumentSource::Scrape`, (4) return `Ok(())` on success. Log progress with `tracing` at each step. Unit test with `wiremock` for the full pipeline (mock the HTML source URL and the llama-embed endpoint).
  - Deliverables:
    - `IngestPipeline` struct implementing `Pipeline` trait
    - Wiremock-based integration test for the full scrape → chunk → embed → insert flow
    - `api-client` adapter stub (a struct with a `fetch_text` returning `IngestError` — explicitly not wired)
  - Skills to load: spontini-tdd-rust, spontini-ingest-flow, spontini-clean-arch-guard
  - Verification: `cargo test -p ingest-core` passes with all pipeline tests.

### Phase 6: Final verification and cleanup

Goal: Run the full `spontini-verify-gate` on the `ingest-core` crate and confirm everything compiles and passes.

- [ ] **Task 6.1** — Run `spontini-verify-gate` on the workspace
  - What: Run `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`, and verify all pass. If any lint or formatting issues exist in `ingest-core`, fix them. The `ingest` and `ingest-cli` crates may have broken dependencies (they depend on `ingest-core` and may need updated Cargo.toml or stub code) — fix their Cargo.toml if needed to keep the workspace compiling, but do NOT implement feature 0006 or 0007 logic here.
  - Deliverables:
    - Clean `cargo check --workspace`
    - Clean `cargo test --workspace`
    - Clean `cargo clippy --workspace -- -D warnings`
    - Clean `cargo fmt --check`
  - Skills to load: spontini-verify-gate
  - Verification: All four gates pass. Report any pre-existing failures outside `ingest-core`.

## Acceptance Criteria

- `ingest-core` builds and passes all tests (`cargo test -p ingest-core`).
- A `ScraperAdapter::fetch_text(url)` call to a wiremock-served HTML page returns the visible text content.
- A `ScraperAdapter::fetch_text(url)` call to a non-allowlisted content-type returns `IngestError::ContentType`.
- A `ScraperAdapter::fetch_text(url)` call to a `robots.txt`-disallowed URL returns `IngestError::RobotsTxt`.
- `Chunker::chunk(text, section)` splits a 2000-token text into multiple ~512-token chunks with overlap.
- `EmbeddingClient::embed_chunk(text)` POSTs to `llama-embed` and returns a 768-dim `Vec<f32>`, validating dimension.
- `IngestPipeline::run(url, section)` orchestrates the full flow (scrape → chunk → embed → `KbStore::insert_document`), tested with wiremock.
- The `api-client` adapter exists as a stub and is NOT wired into the pipeline.
- All wiremock tests pass without real network calls (test doubles only).
- The workspace `ingest` and `ingest-cli` crates still compile (their `Cargo.toml` may be updated for compatibility, but no new feature logic is added to them).

## Risks

- **`scraper` crate API changes** — mitigation: pin the version in `Cargo.toml` and use its stable HTML-parsing API; fall back to `kuchiki` if `scraper` proves inadequate for visible-text extraction.
- **Token counting accuracy** — mitigation: the naive `len/4` heuristic is acceptable for this milestone; the chunker is configurable so the budget can be tuned. Accurate tokenization (e.g., via `tiktoken-rs`) is deferred to a future refinement.
- **`llama-embed` response format divergence** — mitigation: the backend's `EmbeddingAdapter` already confirms the response format with Wiremock tests; the ingest-core `EmbeddingClient` follows the same contract and is verified by wiremock, decoupling it from the real `llama-embed` container.

## Out-of-Scope

- No scheduler or daemon (feature 0006).
- No `ingest` container changes beyond Cargo.toml compatibility.
- No `ingest-cli` features (feature 0007).
- No PDF/docx extraction (feature 0009).
- No `api-client` adapter implementation (stub only).
