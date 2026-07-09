# ADR 0006: Ingest Pipeline Trait and Composition Pattern

- **Status**: accepted
- **Date**: 2026-07-09
- **Deciders**: Sisyphus (opencode)
- **Related**: Feature 0005

## Context

Spontini needs an ingest pipeline that fetches content from external URLs, chunks it into manageable segments, generates embeddings, and stores the results in `kb.db`. The pipeline must be consumed by two entry points: the always-on ingest scheduler ([Feature 0006](../.project/0006-ingest-service-long-running-scheduler-plan.md)) and the backend admin manual upload ([Feature 0009](../.project/ROADMAP.md)). Each stage must be independently testable without real network calls.

The `api-client` adapter exists as a stub and is explicitly not wired — it is reserved for future use per [STACK.md §3.6](../docs/STACK.md#36-ingest).

## Decision

We will define a `Pipeline` trait with an `async fn run(&self, url: &str, section: &str) -> Result<()>` method as the core abstraction. The `IngestPipeline` struct implements this trait by composing a `ScraperAdapter` (HTTP fetch + robots.txt + content-type allowlist + HTML text extraction), a `Chunker` (recursive text splitter with ~512-token chunks and ~64-token overlap), an `EmbeddingClient` (HTTP POST to `llama-embed` with 768-dim validation), and a `KbStore` reference for document insertion. Each stage is independently testable with `wiremock`.

## Rationale

The `Pipeline` trait provides a single abstraction consumed by both the scheduler and the admin upload, eliminating duplication. Composability allows future adapters (api, folder) to be added by implementing new adapter structs without changing the orchestrator. Wiremock testing at each stage ensures reliability without real network calls or running containers. The orchestrator pattern (scrape → chunk → embed → insert) makes the data flow explicit and debuggable.

## Consequences

### Positive

- Single pipeline abstraction consumed by scheduler and admin upload
- Each stage independently testable with wiremock (no real HTTP, no real LLM)
- Future adapters added by implementing new structs, not modifying the orchestrator
- Data flow is explicit: scrape → chunk → embed → insert

### Negative

- Naive token counting (`text.len() / 4`) is approximate — accurate tokenization deferred to future refinement
- The scraper adapter couples to HTML structure (strips `<script>`, `<style>`, nav elements)
- The `Pipeline` trait's `run` method takes a URL string — source-type polymorphism is handled by the caller, not the trait

### Neutral

- The `api-client` adapter exists as a stub (returns `IngestError`) and is not wired into the orchestrator

## Alternatives Considered

### Alternative A: Monolithic pipeline function

A single async function that performs all stages. Rejected because it prevents independent testing of each stage, makes the function signature unwieldy, and doesn't support future adapter composition.

### Alternative B: Stream-based pipeline (tokio-stream)

Compose stages as async streams with backpressure. Rejected because it adds complexity without benefit for the current batch-oriented use case (fetch all, chunk all, embed all, insert all).

## Compliance

The `spontini-ingest-flow` skill enforces the two-entry-point rule: the pipeline is consumed by the scheduler (feature 0006) and the admin upload (feature 0009). The `spontini-clean-arch-guard` skill ensures `ingest-core` depends only on `kb-store` and external HTTP crates — no framework types leak into the pipeline abstraction. All stages are tested with `wiremock` (test doubles only, no real network calls).
