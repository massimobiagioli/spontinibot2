# Plan 0009: `/admin/api/upload` — per-section manual document upload

- **Status**: closed
- **Approved**: 2026-07-10 by Sisyphus
- **Implemented**: 2026-07-10 by Sisyphus
- **Closed**: 2026-07-10 by Sisyphus
- **Review verdict**: approved
- **Branch**: feat/admin-api-upload
- **Feature ID**: 0009
- **Created**: 2026-07-10
- **Owner**: Sisyphus

## Objective

Add a manual-upload surface to `backend` that lets an operator upload a document (PDF, DOCX, Markdown, or plain text) for a specific section, preview the extracted text and metadata before indexing, and confirm the upload to trigger chunking, embedding, and insertion into `kb.db`. This closes the gap between automated scraping (feature 0005) and operator-curated content: some municipal documents are not available online and must be uploaded manually. The preview/confirm split is a constitutional requirement — the operator must never index unseen content (Constitution §5: truthfulness and source citation). Text extraction uses `pdf-extract` for PDFs, `docx-rs` for DOCX, and direct reads for Markdown/text. The preview step returns a short-lived token; the confirm step delegates to `ingest-core`'s `IngestPipeline` for chunking and embedding, then writes the chunks to `kb.db` via `kb-store`. All endpoints are protected by the existing `X-Admin-Key` header (feature 0008). BDD scenarios validate the full upload → preview → confirm → searchable flow.

## Non-Goals

- No admin-ui SPA changes (separate feature 0016).
- No changes to `kb-store` schema (the existing `documents` table already supports section-tagged inserts).
- No changes to `ingest-core` chunking or embedding logic (it already handles section metadata).
- No support for formats beyond PDF, DOCX, Markdown, and plain text (no images, no audio, no HTML).
- No file size limits or rate limiting (separate hardening concern, feature 0026).
- No persistent storage of uploaded files — only the extracted text and chunks are stored in `kb.db`.
- No async background processing — the confirm step is synchronous (chunking + embedding + insert complete before the response returns).

## Phases

### Phase 1: Text extraction adapters

Goal: Build a format-agnostic text extraction layer that converts uploaded files into plain text with metadata.

- [x] **Task 1.1** — Define `TextExtractor` trait and error types
  - What: In `backend/src/admin/upload/`, define a `#[async_trait] pub trait TextExtractor: Send + Sync` with method `async fn extract(&self, file_bytes: &[u8], filename: &str) -> Result<ExtractedText, UploadError>`. The `ExtractedText` struct holds `content: String`, `format: DocumentFormat` (enum: Pdf, Docx, Markdown, PlainText), and `byte_size: usize`. The `UploadError` enum covers: UnsupportedFormat, ExtractionFailed(String), FileTooLarge. The trait is format-agnostic; implementations handle specific formats.
  - Deliverables:
    - `backend/src/admin/upload/mod.rs` module with `TextExtractor` trait, `ExtractedText`, `DocumentFormat`, `UploadError`
    - Unit tests for error types
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo build -p backend` compiles; trait is object-safe.

- [x] **Task 1.2** — Implement PDF, DOCX, Markdown, and plain text extractors
  - What: Create four implementations of `TextExtractor`: (1) `PdfExtractor` using the `pdf-extract` crate to extract visible text from PDF bytes; (2) `DocxExtractor` using `docx-rs` to extract paragraph text from DOCX bytes; (3) `MarkdownExtractor` that reads UTF-8 bytes and strips optional frontmatter; (4) `PlainTextExtractor` that reads UTF-8 bytes verbatim. Each extractor validates the file signature (magic bytes for PDF/DOCX, UTF-8 validity for text) and returns `UploadError::ExtractionFailed` on mismatch. A `CompositeExtractor` dispatches by filename extension (`.pdf`, `.docx`, `.md`, `.txt`) and returns `UnsupportedFormat` for unknown extensions.
  - Deliverables:
    - `backend/src/admin/upload/pdf.rs` with `PdfExtractor`
    - `backend/src/admin/upload/docx.rs` with `DocxExtractor`
    - `backend/src/admin/upload/markdown.rs` with `MarkdownExtractor`
    - `backend/src/admin/upload/plain.rs` with `PlainTextExtractor`
    - `backend/src/admin/upload/composite.rs` with `CompositeExtractor`
    - Unit tests for each extractor using small fixture files (a 1-page PDF, a minimal DOCX, a Markdown file, a plain text file)
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p backend` passes; each extractor handles its format correctly; unsupported formats return `UnsupportedFormat`.

### Phase 2: Preview/confirm workflow with token-based state

Goal: Implement the two-step upload flow: upload returns a preview token, preview shows extracted text, confirm triggers indexing.

- [x] **Task 2.1** — Add in-memory preview token store
  - What: Create `backend/src/admin/upload/preview_store.rs` with a `PreviewStore` struct that holds a `DashMap<String, PreviewEntry>`. Each `PreviewEntry` contains: `extracted_text: ExtractedText`, `section: String`, `metadata: UploadMetadata` (category, tags, trust_score), `created_at: DateTime<Utc>`, and a TTL of 15 minutes. The store has methods: `insert(entry) -> token`, `get(token) -> Option<&PreviewEntry>`, `remove(token)`, and a background task that evicts expired entries every 60 seconds. The token is a 32-character hex string from `rand::thread_rng()`.
  - Deliverables:
    - `backend/src/admin/upload/preview_store.rs` with `PreviewStore`, `PreviewEntry`, `UploadMetadata`
    - Unit tests for insert, get, remove, TTL expiration
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p backend` passes; expired entries are evicted.

- [x] **Task 2.2** — Implement upload, preview, and confirm handlers
  - What: Add three axum route handlers: (1) `POST /admin/api/upload` accepts `multipart/form-data` with fields `file` (the document), `section` (string), `category` (optional string), `tags` (optional comma-separated string), `trust_score` (optional f32, default 1.0). It extracts text via `CompositeExtractor`, stores the result in `PreviewStore`, and returns `{ token, preview_url }`. (2) `GET /admin/api/upload/preview/:token` returns the extracted text, metadata, and a chunk count estimate (text length / 512). (3) `POST /admin/api/upload/confirm/:token` removes the entry from `PreviewStore`, delegates to `IngestPipeline` for chunking + embedding, writes chunks to `kb.db` via `kb-store`, and returns `{ document_ids: Vec<i64>, chunk_count: usize }`. All handlers check `X-Admin-Key` header.
  - Deliverables:
    - Route handlers for upload, preview, confirm
    - Request/response DTOs with serde Serialize/Deserialize
    - Multipart parsing via `axum::extract::Multipart`
    - Unit tests for each handler (auth rejection, valid upload, preview retrieval, confirm with indexing)
  - Skills to load: spontini-tdd-rust, spontini-bdd-gherkin, spontini-ingest-flow
  - Verification: `cargo build -p backend` compiles; handlers parse multipart correctly; preview store is populated and consumed.

### Phase 3: Integration with ingest-core and kb-store

Goal: Wire the confirm handler to `IngestPipeline` for chunking, embedding, and insertion.

- [x] **Task 3.1** — Add `UploadPort` trait for ingest pipeline delegation
  - What: In `backend/src/admin/upload/ports.rs`, define a `#[async_trait] pub trait UploadPort: Send + Sync` with method `async fn ingest_uploaded(&self, text: &str, section: &str, metadata: &UploadMetadata) -> Result<Vec<i64>, UploadError>`. This port abstracts the ingest pipeline so the backend does not depend directly on `ingest-core` types. The implementation (in Task 3.2) will call `IngestPipeline::process_manual_upload`.
  - Deliverables:
    - `backend/src/admin/upload/ports.rs` with `UploadPort` trait
    - Updated `UploadError` with `IngestFailed(String)` variant
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard, spontini-ingest-flow
  - Verification: `cargo build -p backend` compiles; trait is object-safe.

- [x] **Task 3.2** — Implement `IngestCoreUploadAdapter`
  - What: Create `backend/src/admin/upload/ingest_adapter.rs` with an `IngestCoreUploadAdapter` struct holding `Arc<IngestPipeline>` (from `ingest-core`). Implement `UploadPort` for it: the adapter calls `pipeline.process_manual_upload(text, section, metadata)` which chunks the text, embeds each chunk via `llama-embed`, and inserts the chunks into `kb.db` via `kb-store`. The adapter converts `IngestError` to `UploadError::IngestFailed`. If `ingest-core` does not yet expose a `process_manual_upload` method, add it to the `IngestPipeline` struct (this is a minimal addition: it reuses the existing chunker and embedding client, skipping the scraper step).
  - Deliverables:
    - `backend/src/admin/upload/ingest_adapter.rs` with `IngestCoreUploadAdapter`
    - `UploadPort` implementation
    - If needed: `ingest-core/src/lib.rs` gains `pub async fn process_manual_upload(&self, text: &str, section: &str, metadata: &UploadMetadata) -> Result<Vec<i64>, IngestError>`
    - Unit tests using a mock `IngestPipeline` (or a test double) covering: successful ingest, ingest failure, embedding failure
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard, spontini-ingest-flow
  - Verification: `cargo test -p backend` passes; adapter delegates to pipeline correctly; errors are mapped.

### Phase 4: Wiring and AppState integration

Goal: Wire the upload surface into the backend router and `AppState`.

- [x] **Task 4.1** — Add upload fields to `AppState` and `Config`
  - What: In `backend/src/lib.rs`, add `upload: Arc<dyn UploadPort>` and `preview_store: Arc<PreviewStore>` to `AppState`. In `router()`, construct `CompositeExtractor`, `PreviewStore` (with background eviction task), `IngestCoreUploadAdapter`, and wire them into the upload handlers. The `Config` struct gains an `upload_max_bytes: usize` field (loaded from `UPLOAD_MAX_BYTES` env var, default 10MB) used by the upload handler to reject oversized files.
  - Deliverables:
    - Updated `AppState` with `upload` and `preview_store` fields
    - Updated `Config` with `upload_max_bytes`
    - Updated `router()` wiring with upload routes
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo build -p backend` compiles; `cargo test -p backend` passes.

### Phase 5: BDD scenarios

Goal: Add Gherkin scenarios for the upload → preview → confirm → searchable flow.

- [x] **Task 5.1** — Write BDD steps and scenarios for manual upload
  - What: Add BDD scenarios to `backend/tests/bdd.rs`: (1) upload a Markdown file with section "news" and metadata, retrieve the preview, confirm the upload, then query `/chat` with a question matching the uploaded content and verify the answer cites the uploaded document; (2) upload an unsupported format (e.g., `.jpg`) and verify the upload returns `UnsupportedFormat`; (3) upload a file, retrieve the preview, but do not confirm — verify the document is not searchable after 15 minutes (TTL expiration). Each scenario uses the `ChatWorld` pattern, extended with upload endpoints via `reqwest` multipart calls.
  - Deliverables:
    - BDD scenarios for upload-preview-confirm-searchable, unsupported-format, TTL-expiration
    - Wired step definitions reusing existing `ChatWorld` infrastructure
    - Fixture files for testing (small PDF, DOCX, Markdown, plain text)
  - Skills to load: spontini-tdd-rust, spontini-bdd-gherkin
  - Verification: `cargo test -p backend --test bdd -- --nocapture` passes with new scenarios green.

## Acceptance Criteria

- `POST /admin/api/upload` accepts multipart with file, section, and metadata; returns a preview token and URL.
- `GET /admin/api/upload/preview/:token` returns the extracted text, metadata, and chunk count estimate.
- `POST /admin/api/upload/confirm/:token` triggers chunking, embedding, and insertion into `kb.db`; returns document IDs and chunk count.
- Uploaded documents are searchable via `/chat` after confirmation.
- Unsupported formats (e.g., `.jpg`, `.exe`) return `400 Bad Request` with `UnsupportedFormat` error.
- Preview tokens expire after 15 minutes; unconfirmed uploads are not indexed.
- All endpoints return 401 when `X-Admin-Key` header is missing or wrong.
- All existing tests in the workspace (`cargo test --workspace`) remain green.
- BDD scenarios cover upload-preview-confirm-searchable, unsupported-format, and TTL-expiration.

## Risks

- **Text extraction quality** — PDF and DOCX extraction may miss complex layouts, tables, or embedded images. Mitigation: the preview step lets the operator verify the extracted text before confirming; if extraction is poor, the operator can cancel and upload a different format.
- **Preview store memory usage** — Large files held in memory during preview could exhaust RAM. Mitigation: `upload_max_bytes` config limits file size (default 10MB); preview entries are evicted after 15 minutes.
- **Synchronous confirm** — Chunking + embedding + insertion may take several seconds for large documents, blocking the HTTP request. Mitigation: acceptable for manual uploads (operator is waiting); if this becomes a problem, a future feature can add async background processing with polling.
- **ingest-core API addition** — Adding `process_manual_upload` to `IngestPipeline` is a minimal change, but it couples the backend to `ingest-core`'s internal chunking/embedding logic. Mitigation: the `UploadPort` trait abstracts this; the adapter is the only coupling point.

## Out-of-Scope

- No admin-ui SPA changes.
- No file size limits or rate limiting beyond `upload_max_bytes`.
- No persistent storage of uploaded files (only chunks in `kb.db`).
- No async background processing for confirm.
- No support for formats beyond PDF, DOCX, Markdown, plain text.
- No changes to `kb-store` schema.
- No changes to automated ingest (scraper) flow.
