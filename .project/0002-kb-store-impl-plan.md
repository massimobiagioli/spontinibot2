# Plan 0002: kb-store libSQL Implementation

- **Status**: closed
- **Approved**: 2026-07-09 by Sisyphus (opencode)
- **Implemented**: 2026-07-09 by Sisyphus (opencode)
- **Closed**: 2026-07-09 by Sisyphus (opencode)
- **Review verdict**: approved
- **Branch**: feat/kb-store-impl
- **Feature ID**: 0002
- **Created**: 2026-07-09
- **Owner**: Sisyphus (opencode)

## Objective

Transform `kb-store` from a version-string skeleton into a working libSQL access layer that `backend` and `ingest` will share. After this plan lands, `kb-store` opens a local `kb.db` file, runs idempotent schema migrations on startup (creating the `documents` and `persona` tables per [STACK.md §3.5](../docs/STACK.md#35-storage--libsql)), and exposes a Clean-Architecture-friendly public API for document CRUD (including vector similarity search via `vector_distance_cos`) and persona CRUD (versioned inserts, never UPDATE, with the `is_active` partial unique index).

This serves the [Constitution §1](../docs/CONSTITUTION.md#1-mission) mission by building the data foundation the bot needs to store and retrieve municipal documents. It keeps the stack local (libSQL, no external DB). It reduces complexity by providing a single shared access layer instead of each crate rolling its own SQL.

**In scope:** libSQL dependency, embedded SQL migration runner, public `KbStore` struct with `open(path)` + document CRUD (`insert`, `get_by_id`, `get_by_source`, `search_similar`, `delete`) + persona CRUD (`insert`, `get_active`, `get_by_id`, `get_versions`, `activate`), unit tests for every public function, integration test against an in-memory libSQL database, clean-arch-compliant dependency rule (`kb-store` depends only on `libsql` crate + std).

**Out of scope:** Wiring `kb-store` into `backend` or `ingest` (separate plan). The `rag-engine` module. Real embedding calls (llama-embed). The `ingest-core` scraper adapter. The `admin-ui` sections. Any HTTP or network layer. A separate Domain crate (domain types live in `kb-store` for now; they can be extracted when a Domain crate is introduced).

## Non-Goals

- This plan does NOT wire `kb-store` into `backend` or `ingest` — those are separate plans. `kb-store` is a library that other crates will consume.
- This plan does NOT call `llama-embed` or `llama-generate` — embeddings are provided by the caller as `&[f32]`.
- This plan does NOT create a separate Domain crate — domain types (`Document`, `Persona`, etc.) live in `kb-store/src/types/` and can be extracted later when a Domain crate is introduced.
- This plan does NOT implement the `rag-engine` — retrieval from `kb-store` is exercised only in unit/integration tests.
- This plan does NOT change `backend`, `ingest`, `ingest-core`, or `ingest-cli` — only `kb-store/` is modified.
- This plan does NOT change the Docker Compose config — `kb-store` is a library, not a container.

## Phases

### Phase 1: libSQL Foundation + Migration Runner

Goal: Add the `libsql` dependency, implement a hand-rolled embedded-SQL migration runner, and verify it creates both tables with the correct schema idempotently.

- [x] **Task 1.1** — Add `libsql` dependency to `kb-store`
  - What: Update `kb-store/Cargo.toml` with `[dependencies] libsql = { version = "0.9", default-features = false, features = ["core"] }` and add `thiserror = "1"` for typed errors. No other dependencies (`tokio` is not needed at the library layer — the caller provides the async runtime; `libsql`'s API is already async).
  - Deliverables:
    - `kb-store/Cargo.toml` updated
  - Skills to load: spontini-clean-arch-guard
  - Verification: `cargo check -p kb-store` compiles successfully

- [x] **Task 1.2** — Write embedded SQL migration runner
  - What: Create `kb-store/src/migrations/` with:
    - `V1__initial_schema.sql` — `CREATE TABLE IF NOT EXISTS documents (...)` with all columns from STACK.md §3.5 (`id INTEGER PRIMARY KEY`, `source TEXT`, `source_ref TEXT`, `content TEXT`, `metadata TEXT`, `embedding F32_BLOB(768)`) and `CREATE TABLE IF NOT EXISTS persona (...)` with all columns and the partial unique index `idx_persona_active`.
    - `mod.rs` — A `run_migrations(conn: &Connection) -> Result<()>` function that:
      1. Creates a `_migrations` tracking table (`CREATE TABLE IF NOT EXISTS _migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at TEXT NOT NULL DEFAULT (datetime('now')))`)
      2. Applies the base schema via `conn.execute_batch(V1_SCHEMA)` (idempotent via `IF NOT EXISTS`)
      3. Records `(1, 'initial_schema')` in `_migrations` if not already present
      4. Reads `INCREMENTAL_MIGRATIONS` from a const array (empty for now, structured for future additions) and applies any not yet recorded, wrapped in a transaction
    - The public function signature: `pub async fn run_migrations(conn: &Connection) -> Result<()>`
  - Deliverables:
    - `kb-store/src/migrations/mod.rs`
    - `kb-store/src/migrations/V1__initial_schema.sql`
    - Unit test in `mod.rs` that creates an in-memory DB, runs migrations, and asserts both tables exist (query `SELECT name FROM sqlite_master WHERE type='table'`)
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo test -p kb-store should_create_tables_when_migrations_run` passes; second run is idempotent (no errors, no duplicate rows)

- [x] **Task 1.3** — Define `KbStoreError` and `Result` type
  - What: Create `kb-store/src/error.rs` with a `#[derive(Error, Debug)]` enum:
    - `Database(#[from] libsql::Error)` — wraps libSQL errors
    - `InvalidDimension { expected: usize, actual: usize }` — for embedding vector size mismatch
    - `NotFound(String)` — for missing document/persona lookups
    - `Migration(String)` — for migration failures
    - Public type alias: `pub type Result<T> = std::result::Result<T, KbStoreError>`
  - Deliverables:
    - `kb-store/src/error.rs` with the `KbStoreError` enum
    - Unit test that the error types display correctly via `Display` derive
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store` passes; clippy clean

### Phase 2: Document CRUD + Vector Search

Goal: Define the `Document` domain type and implement full document CRUD with vector similarity search.

- [x] **Task 2.1** — Define `Document`, `DocumentSource`, `NewDocument`, and `ScoredDocument` types
  - What: Create `kb-store/src/types.rs` with:
    - `#[derive(Debug, Clone, PartialEq)]` `pub struct Document { pub id: i64, pub source: DocumentSource, pub source_ref: String, pub content: String, pub metadata: Option<String>, pub embedding: Option<Vec<f32>> }`
    - `#[derive(Debug, Clone, PartialEq)]` `pub enum DocumentSource { Scrape, Api, Manual }` with `Display` and `FromStr` impls
    - `#[derive(Debug)]` `pub struct NewDocument { pub source: DocumentSource, pub source_ref: String, pub content: String, pub metadata: Option<String>, pub embedding: Vec<f32> }` (embedding required for new docs, 768-dimension validated at insert time)
    - `#[derive(Debug, Clone)]` `pub struct ScoredDocument { pub document: Document, pub similarity: f64 }` — for search results
    - All types implement the traits needed by the public API (`Debug`, `Clone`, `PartialEq` for testing)
  - Deliverables:
    - `kb-store/src/types.rs`
    - Unit tests for `DocumentSource` round-trip (Display + FromStr) and dimension validation logic
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store` passes; clippy clean

 - [x] **Task 2.2** — Implement `KbStore::open()` and `KbStore::insert_document()`
  - What: Create `kb-store/src/lib.rs` with:
    - `pub struct KbStore { db: Database }` - wraps libsql `Database`, not `Connection` (caller gets connections as needed)  
    - `pub async fn open(path: &str) -> Result<Self>` — calls `Builder::new_local(path).build().await`, runs `migrations::run_migrations()`, returns `KbStore`
    - Helper function `fn f32_slice_to_blob(embedding: &[f32]) -> Vec<u8>` — converts via `iter().flat_map(|f| f.to_le_bytes()).collect()`
    - Helper function `fn blob_to_f32_vec(blob: Vec<u8>) -> Result<Vec<f32>>` — reverses via `chunks_exact(4).map(f32::from_le_bytes).collect()`, returns `InvalidDimension` if blob length not divisible by 4
    - `pub async fn insert_document(&self, doc: NewDocument) -> Result<Document>` — validates embedding is 768-dim, opens a connection, executes `INSERT INTO documents (source, source_ref, content, metadata, embedding) VALUES (?1, ?2, ?3, ?4, ?5)`, uses `Value::Blob(blob)` for embedding, returns the inserted `Document` with the auto-generated `id` (via `last_insert_rowid()`)
  - Deliverables:
    - `kb-store/src/lib.rs` rewritten with `KbStore` struct + `open()` + `insert_document()`
    - `kb-store/src/lib.rs` updated `mod` declarations (mod types, mod error, mod migrations)
    - Unit test: `should_open_database_when_path_given` (in-memory `:memory:`)
    - Unit test: `should_insert_document_when_valid_embedding_provided`
    - Unit test: `should_reject_document_when_wrong_dimension`
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo test -p kb-store should_insert_document_when_valid_embedding_provided` passes; `cargo test -p kb-store should_reject_document_when_wrong_dimension` passes

- [x] **Task 2.3** — Implement `get_document()` and `get_documents_by_source()`
  - What: Add to `KbStore`:
    - `pub async fn get_document(&self, id: i64) -> Result<Option<Document>>` — `SELECT * FROM documents WHERE id = ?1`, reconstructs `Document` from row (including blob→f32_vec conversion). Returns `None` if no row.
    - `pub async fn get_documents_by_source(&self, source: DocumentSource, limit: i64, offset: i64) -> Result<Vec<Document>>` — `SELECT * FROM documents WHERE source = ?1 ORDER BY id DESC LIMIT ?2 OFFSET ?3`
    - Internal helper `fn row_to_document(row: &Row) -> Result<Document>` that extracts each column and converts the embedding blob
  - Deliverables:
    - `kb-store/src/lib.rs` updated (or a new `kb-store/src/documents.rs` — preference: keep in `lib.rs` if <200 lines, extract to `documents.rs` if larger)
    - Unit tests: `should_return_document_when_get_by_existing_id`, `should_return_none_when_get_by_missing_id`, `should_return_documents_filtered_by_source`
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store` passes all doc query tests; clippy clean

- [x] **Task 2.4** — Implement `search_similar()` vector search
  - What: Add to `KbStore`:
    - `pub async fn search_similar(&self, query_embedding: &[f32], top_k: i64, min_score: f64) -> Result<Vec<ScoredDocument>>` — validates embedding dimension (768), executes:
      ```sql
      SELECT id, source, source_ref, content, metadata, embedding,
             1 - vector_distance_cos(embedding, vector32(?1)) AS similarity
      FROM documents
      WHERE embedding IS NOT NULL
        AND (1 - vector_distance_cos(embedding, vector32(?1))) >= ?3
      ORDER BY similarity DESC
      LIMIT ?2
      ```
      Converts each row to `ScoredDocument { document: Document {...}, similarity: f64 }`.
    - `vector32(?1)` takes the blob parameter and interprets it as a vector for the distance computation.
  - Deliverables:
    - `kb-store/src/lib.rs` updated with `search_similar()`
    - Unit test: `should_return_similar_documents_when_searching` — insert two documents with known embeddings, query with a vector close to one of them, assert the close one is returned first with similarity > threshold
    - Unit test: `should_return_empty_when_no_matching_documents`
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store should_return_similar_documents_when_searching` passes; verify the similarity scores are in the expected range (0-2 distance, 1 - distance = cosine similarity in [-1,1])

- [x] **Task 2.5** — Implement `delete_document()`
  - What: Add to `KbStore`:
    - `pub async fn delete_document(&self, id: i64) -> Result<bool>` — `DELETE FROM documents WHERE id = ?1`, returns `true` if a row was deleted, `false` if no row matched
  - Deliverables:
    - `kb-store/src/lib.rs` updated
    - Unit test: `should_delete_document_when_exists`, `should_return_false_when_deleting_missing_document`
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store` passes

### Phase 3: Persona CRUD

Goal: Implement versioned persona operations — inserts only, never UPDATE, with the partial unique index on `is_active`.

- [x] **Task 3.1** — Define `Persona` and `NewPersona` types
  - What: Add to `kb-store/src/types.rs`:
    - `#[derive(Debug, Clone, PartialEq)] pub struct Persona { pub id: i64, pub version: i32, pub name: String, pub system_prompt: String, pub tone: Option<String>, pub fallback_message: Option<String>, pub is_active: bool, pub created_at: String, pub created_by: Option<String> }`
    - `#[derive(Debug)] pub struct NewPersona { pub name: String, pub system_prompt: String, pub tone: Option<String>, pub fallback_message: Option<String>, pub created_by: Option<String> }`
    - `version` starts at 1 and increments within each `name` group (managed by `KbStore`, not the caller)
  - Deliverables:
    - `kb-store/src/types.rs` updated
    - Unit test: all fields round-trip correctly
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store` passes

- [x] **Task 3.2** — Implement `insert_persona()` and `get_active_persona()`
  - What: Add to `KbStore`:
    - `pub async fn insert_persona(&self, persona: NewPersona, activate: bool) -> Result<Persona>` — in a transaction:
      1. Compute `version`: `SELECT COALESCE(MAX(version), 0) + 1 FROM persona WHERE name = ?1`
      2. If `activate` is true: `UPDATE persona SET is_active = 0 WHERE is_active = 1` (deactivate current active)
      3. `INSERT INTO persona (version, name, system_prompt, tone, fallback_message, is_active, created_by) VALUES (...)` with `is_active = activate as integer`
      4. Returns the inserted `Persona`
    - `pub async fn get_active_persona(&self) -> Result<Option<Persona>>` — `SELECT * FROM persona WHERE is_active = 1 LIMIT 1`
    - Internal helper `fn row_to_persona(row: &Row) -> Result<Persona>` — converts `is_active` from `i64`/`bool`
  - Deliverables:
    - `kb-store/src/lib.rs` updated
    - Unit test: `should_insert_persona_with_incrementing_version` — insert two personas with same name, assert versions are 1 and 2
    - Unit test: `should_have_one_active_persona_when_inserting_with_activate_true` — insert persona A (activate=true), then persona B (activate=true), assert A.is_active=false and B.is_active=true
    - Unit test: `should_return_none_when_no_active_persona`, `should_return_active_persona_when_exists`
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store` passes all persona tests

- [x] **Task 3.3** — Implement `get_persona()`, `get_persona_versions()`, and `activate_persona()`
  - What: Add to `KbStore`:
    - `pub async fn get_persona(&self, id: i64) -> Result<Option<Persona>>` — `SELECT * FROM persona WHERE id = ?1`
    - `pub async fn get_persona_versions(&self, name: &str) -> Result<Vec<Persona>>` — `SELECT * FROM persona WHERE name = ?1 ORDER BY version DESC`
    - `pub async fn activate_persona(&self, id: i64) -> Result<()>` — in a transaction: deactivates all (`UPDATE persona SET is_active = 0`), then activates the specified persona (`UPDATE persona SET is_active = 1 WHERE id = ?1`). Returns `NotFound` if `id` doesn't exist.
  - Deliverables:
    - `kb-store/src/lib.rs` updated
    - Unit test: `should_return_all_versions_when_querying_by_name`, `should_activate_persona_and_deactivate_others`
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store` passes

### Phase 4: Clean-up and Verification

Goal: Finalize module structure, run the full verification gate, ensure everything is ready for consumers.

- [x] **Task 4.1** — Review `kb-store` public API surface (module structure + visibility)
  - What: Audit the `kb-store/src/lib.rs` and ensure:
    - Only intended types are `pub` — the `KbStore` struct, its methods, the `Result`/`KbStoreError` types, and the domain types (`Document`, `Persona`, etc.)
    - Internal helpers (`f32_slice_to_blob`, `blob_to_f32_vec`, `row_to_document`, `row_to_persona`) are `pub(crate)` or private
    - Migration module is `pub(crate)` — consumers should not call migrations directly
    - Re-export key types at the crate root (`pub use types::{Document, Persona, NewDocument, NewPersona, ScoredDocument, DocumentSource}`)
    - Add module-level doc comments explaining the crate's purpose per STACK.md §3.5
  - Deliverables:
    - `kb-store/src/lib.rs` — clean public API surface
    - Doc comments on `KbStore`, `KbStoreError`, and each public method
  - Skills to load: spontini-clean-arch-guard
  - Verification: `cargo doc -p kb-store --no-deps` succeeds without warnings; only intended symbols appear in the generated docs

- [x] **Task 4.2** — Full verification gate
  - What: Run the complete verify cycle on `kb-store`:
    ```bash
    cargo test -p kb-store -- --nocapture
    cargo clippy -p kb-store -- -D warnings
    cargo fmt -p kb-store -- --check
    cargo test --workspace --all-targets  # ensure no workspace regression
    cargo clippy --workspace --all-targets -- -D warnings
    ```
    No task is blocked on other crates' pre-existing failures. Fix all warnings/errors in `kb-store`. Do NOT fix warnings in other crates.
  - Deliverables:
    - (No file changes — verification only. Create `coverage-exclusions.txt` only if justified.)
  - Skills to load: spontini-verify-gate
  - Verification: All commands above exit 0. Output captured and reported.

## Acceptance Criteria

- `cargo test -p kb-store` passes all tests (unit + integration with in-memory DB)
- `cargo test --workspace` passes (no regression in other crates)
- `cargo clippy -p kb-store -- -D warnings` is clean
- `cargo fmt --check` is clean on `kb-store/`
- `KbStore::open(":memory:")` returns a working instance with all tables created
- `KbStore::insert_document()` stores a document and returns it with a valid `id`
- `KbStore::insert_document()` rejects embeddings that are not 768-dimensional with `KbStoreError::InvalidDimension`
- `KbStore::search_similar()` returns documents ordered by cosine similarity, with the expected nearest neighbor first
- `KbStore::insert_persona()` auto-increments `version` within each `name` group
- `KbStore::insert_persona(activate=true)` ensures exactly one active persona exists
- `KbStore::activate_persona()` atomically deactivates all and activates the specified one
- Running `KbStore::open(":memory:")` twice on an existing DB file is idempotent — no errors, no duplicate migrations
- The `documents` and `persona` tables match the schema in STACK.md §3.5 exactly (column names, types, constraints, indexes)

## Risks

- **libSQL version compatibility** — The `libsql` crate API changes rapidly (pre-1.0). Mitigation: pin to `0.9.x` (the latest stable at planning time). If the API changes during implementation, adjust to match. The research confirmed the `0.9` API patterns above.
- **`vector32()` vs `F32_BLOB` interaction** — The `F32_BLOB(768)` column stores raw LE f32 bytes. The `vector32(?1)` SQL function interprets a blob parameter as a vector for distance computation. These must use the same byte layout. Mitigation: the blob is produced/consumed by `f32_slice_to_blob`/`blob_to_f32_vec` consistently; the test in Task 2.4 validates that a round-trip insert→search returns the correct nearest neighbor.
- **libSQL in-memory mode for tests** — Some libSQL features (like WAL mode) fail on `:memory:`. Mitigation: migration runner handles `PRAGMA journal_mode=WAL` failure gracefully (logs a warning, continues). Tests use `:memory:` and skip WAL.
- **No tokio dependency in kb-store** — `libsql`'s API is async (returns futures), but `kb-store` as a library does not own the runtime. The caller (backend, ingest) provides tokio. Mitigation: all `kb-store` methods are `async fn` that return futures; they do not call `tokio::spawn` or block on futures internally. The test harness wraps tests in `#[tokio::test]`.
- **`last_insert_rowid()` thread safety** — If two connections insert concurrently, `last_insert_rowid()` returns the last insert on THAT connection, which is correct. Mitigation: each public method opens its own connection via `db.connect()`. The `KbStore` struct holds `Database` (connection factory), not `Connection`.

## Out-of-Scope

- Wiring `kb-store` into `backend` (caller integration is a separate plan).
- Wiring `kb-store` into `ingest-core` (separate plan).
- The `rag-engine` module (separate plan).
- Real embedding calls against `llama-embed` (separate plan).
- A separate Domain crate (domain types live in `kb-store` for now).
- `Dockerfile` or `docker-compose.yml` changes (kb-store is a library, not a container).
- Changes to `backend/`, `ingest/`, `ingest-core/`, or `ingest-cli/`.
