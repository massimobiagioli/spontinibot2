# Plan 0013: `/admin/api/training/sessions/:id/messages` — ask/answer with recording

- **Status**: closed
- **Approved**: 2026-07-24 by Sisyphus
- **Implemented**: 2026-07-24 by Sisyphus
- **Closed**: 2026-07-24 by Sisyphus
- **Review verdict**: changes-requested (resolved)
- **Branch**: feat/admin-api-training-messages
- **Feature ID**: 0013
- **Created**: 2026-07-24
- **Owner**: Sisyphus

## Objective

Add the second piece of the operator Training surface (Milestone 2): a `training_message` table in `kb-store` (a new `V4__training_messages.sql` migration) and an admin endpoint that lets an operator ask a question inside an existing training session, get the answer from the same `RagEngine` that powers the citizen-facing `/chat`, and have the exchange persisted for later review. This directly serves the [Constitution](../docs/CONSTITUTION.md) mission of an honest, groundable bot: training sessions let an operator exercise the exact retrieval-and-generation path citizens will experience, including the honest-unknown fallback, and keep an auditable record of what was asked and answered. The endpoint must not fork the RAG logic — it reuses `RagEngine::answer` verbatim so the recorded exchange is provably the same shape (`answer`, `sources`, `fell_back`) as `/chat`.

In scope: `kb-store` schema + CRUD for `training_message` (foreign key to `training_session.id`, feature 0012), a `TrainingMessageAdminPort` wrapping the `RagEngine` call plus persistence, and the `backend` admin HTTP surface `POST /admin/api/training/sessions/:id/messages` (ask + record) and `GET /admin/api/training/sessions/:id/messages` (list a session's exchanges).

Out of scope: point-in-answer feedback (feature 0014) — a separate roadmap feature that will reference `training_message.id` as a foreign key in its own migration. No admin-ui SPA changes (feature 0018 builds the Training section). No changes to `RagEngine` itself or to `/chat`.

## Non-Goals

- No `training_feedback` table or endpoint (feature 0014).
- No admin-ui SPA changes.
- No message editing or deletion — a training message is an append-only log entry once created.
- No pagination on the list endpoint (single-operator system; per-session message count stays small).
- No authentication changes beyond the existing shared-secret `X-Admin-Key` header.
- No change to `RagEngine::answer`'s signature or behavior — the training path calls the exact same method `/chat` uses.

## Phases

### Phase 1: `kb-store` — `training_message` schema and CRUD

Goal: Add the `V4` migration and the storage-layer methods a training message needs.

- [x] **Task 1.1** — Add `V4__training_messages.sql` migration
  - What: Create `kb-store/src/migrations/V4__training_messages.sql` with `CREATE TABLE IF NOT EXISTS training_message (id INTEGER PRIMARY KEY, session_id INTEGER NOT NULL REFERENCES training_session(id), question TEXT NOT NULL, answer TEXT NOT NULL, sources TEXT NOT NULL, fell_back INTEGER NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')))` (`sources` stores a JSON array as TEXT, `fell_back` stores a SQLite boolean as 0/1, mirroring how `is_active` is stored on `persona`). Wire it into `kb-store/src/migrations/mod.rs` following the existing `V1`–`V3` pattern: `include_str!`, a version-4 check against `_migrations`, `execute_batch`, and an insert into `_migrations` recording version 4 as `training_messages_schema`.
  - Deliverables:
    - `kb-store/src/migrations/V4__training_messages.sql`
    - Updated `kb-store/src/migrations/mod.rs` with the version-4 migration step
    - Unit test asserting the `training_message` table exists after `run_migrations`, and that running migrations twice stays idempotent
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store` passes with the new migration test.

- [x] **Task 1.2** — Add `TrainingMessage`/`NewTrainingMessage` types and `KbStore` CRUD methods
  - What: In `kb-store/src/types.rs`, add `pub struct TrainingMessage { pub id: i64, pub session_id: i64, pub question: String, pub answer: String, pub sources: String, pub fell_back: bool, pub created_at: String }` (`sources` stays a raw JSON string at the storage layer — serialization/deserialization to a typed shape happens in `backend`) and `pub struct NewTrainingMessage { pub session_id: i64, pub question: String, pub answer: String, pub sources: String, pub fell_back: bool }`. In `kb-store/src/lib.rs`, add `pub async fn create_training_message(&self, message: NewTrainingMessage) -> Result<TrainingMessage>` (INSERT + re-select by last_insert_rowid, mirroring `create_training_session`) and `pub async fn list_training_messages(&self, session_id: i64) -> Result<Vec<TrainingMessage>>` (ordered by `created_at ASC, id ASC` — chronological conversation order, unlike the newest-first session list).
  - Deliverables:
    - `TrainingMessage`, `NewTrainingMessage` in `kb-store/src/types.rs`, re-exported from `kb-store/src/lib.rs`
    - `create_training_message`, `list_training_messages` on `KbStore`
    - Unit tests: create returns a message with all fields set; create against a nonexistent `session_id` still succeeds at the storage layer (FK enforcement is not assumed — SQLite FKs are off by default and this layer does not turn them on); list returns messages oldest-first for a session; list returns an empty vec for a session with no messages; list only returns messages for the requested `session_id` (not other sessions')
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store` passes with all new tests green.

### Phase 2: `TrainingMessageAdminPort` trait and RAG-backed adapter

Goal: Define the admin port that asks the question through `RagEngine` and persists the exchange, and implement it.

- [x] **Task 2.1** — Define `TrainingMessageAdminPort` trait and response/error types
  - What: In a new file `backend/src/admin/training_messages/mod.rs`, define `#[async_trait] pub trait TrainingMessageAdminPort: Send + Sync` with `async fn ask(&self, session_id: i64, question: String) -> Result<TrainingMessageResponse, TrainingMessageError>` and `async fn list_messages(&self, session_id: i64) -> Result<Vec<TrainingMessageResponse>, TrainingMessageError>`. Define `pub struct TrainingMessageSource { pub document_id: i64, pub source_ref: String }` (serde `Serialize`/`Deserialize`, same shape as `routes::ChatSource`), `pub struct TrainingMessageResponse { pub id: i64, pub session_id: i64, pub question: String, pub answer: String, pub sources: Vec<TrainingMessageSource>, pub fell_back: bool, pub created_at: String }` (serde `Serialize`), and `pub enum TrainingMessageError { SessionNotFound(i64), DbError(String), Rag(String) }` with `Display`/`std::error::Error`/`From<kb_store::KbStoreError>` impls, mirroring `IngestRunError`'s shape (feature 0011) but with the extra `SessionNotFound` and `Rag` variants this port needs.
  - Deliverables:
    - `backend/src/admin/training_messages/mod.rs` with trait, `TrainingMessageSource`, `TrainingMessageResponse`, `TrainingMessageError`
    - Unit tests: `TrainingMessageError` Display format for all three variants; `TrainingMessageResponse` serializes `sources` as a JSON array of `{document_id, source_ref}` objects
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo build -p backend` compiles; trait is object-safe.

- [x] **Task 2.2** — Implement `RagTrainingMessageAdapter`
  - What: Create `backend/src/admin/training_messages/adapter.rs` with `RagTrainingMessageAdapter { store: Arc<kb_store::KbStore>, rag_engine: Arc<crate::rag_engine::engine::RagEngine> }`. Implement `TrainingMessageAdminPort::ask`: first call `store.get_training_session(session_id)` and return `TrainingMessageError::SessionNotFound(session_id)` if `None`; otherwise call `rag_engine.answer(&question)` (mapping `RagError` to `TrainingMessageError::Rag`), serialize the returned `sources: Vec<CitedSource>` to a JSON string via `serde_json::to_string`, persist via `store.create_training_message(NewTrainingMessage { session_id, question, answer: answer.text, sources: <json>, fell_back: answer.fell_back })`, and map the persisted `TrainingMessage` back into a `TrainingMessageResponse` (deserializing `sources` back into `Vec<TrainingMessageSource>` via `serde_json::from_str`). Implement `list_messages` by calling `store.list_training_messages(session_id)` and mapping each row the same way (existence of the session is NOT re-checked here — an empty vec is returned for both "session exists with no messages" and "session does not exist", matching the roadmap's read-only listing scope).
  - Deliverables:
    - `backend/src/admin/training_messages/adapter.rs` with `RagTrainingMessageAdapter`
    - Unit tests using a temp `kb.db` and a stub `RagEngine`-compatible test double (constructed the same way `rag_engine::engine` tests build one, via the `EmbeddingPort`/`RetrievalPort`/`PersonaPort`/`GenerationPort` traits): `ask` against an unknown session returns `SessionNotFound`; `ask` against a known session persists a message and returns a response whose `sources` round-trip correctly; `ask` on the honest-unknown path (no retrieved chunks) persists `fell_back: true` and an empty `sources` list; `list_messages` returns messages oldest-first
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard, spontini-rag-build
  - Verification: `cargo test -p backend` passes with adapter tests.

### Phase 3: Axum routes and integration wiring

Goal: Expose the endpoints under `/admin/api/training/sessions/:id/messages`.

- [x] **Task 3.1** — Add admin training-message route handlers
  - What: Create `backend/src/admin/training_messages/handlers.rs` with `#[derive(Clone)] pub struct TrainingMessageState { pub training_messages: Arc<dyn TrainingMessageAdminPort>, pub config: Config }` and two handlers: `create_message` (`POST /:id/messages`, body `{ question: String }`, returns `201` with the recorded `TrainingMessageResponse`, `404` when the session does not exist, `502` when the RAG call fails) and `list_messages` (`GET /:id/messages`, returns `200` with a JSON array, chronological order). Both call `crate::admin::check_admin_key(&headers, &state.config)?` first.
  - Deliverables:
    - `backend/src/admin/training_messages/handlers.rs` with `TrainingMessageState` and the two handlers
    - Unit tests: 401 on each handler when `X-Admin-Key` missing/wrong; create returns 201 with the answer shape for a known session; create returns 404 for an unknown session; list returns the recorded messages for a known session
  - Skills to load: spontini-tdd-rust, spontini-bdd-gherkin
  - Verification: `cargo build -p backend` compiles; handler unit tests pass.

- [x] **Task 3.2** — Wire training-message routes into the router and module tree
  - What: In `backend/src/admin/mod.rs`, add `pub mod training_messages;`. In `backend/src/lib.rs`, construct `RagTrainingMessageAdapter` from the shared `store` and the already-built `rag_engine` Arc, build a `TrainingMessageState`, thread it through `router_with` as a new parameter (same pattern as `training_session_state`), and add two routes: `POST /admin/api/training/sessions/:id/messages`, `GET /admin/api/training/sessions/:id/messages`.
  - Deliverables:
    - Updated `backend/src/admin/mod.rs` module declaration
    - Updated `router()` construction of `TrainingMessageState`
    - Updated `router_with()` signature (new `training_message_state: TrainingMessageState` parameter) and route registration
    - All existing `router_with` call sites in `backend/tests/bdd.rs` updated to pass the new parameter (stub state where no real `kb.db`/RAG engine is available, real `RagTrainingMessageAdapter`-backed state where one is)
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo build -p backend` compiles; `cargo test -p backend` passes; `cargo test -p backend --test bdd` (existing scenarios) stays green.

### Phase 4: BDD scenarios

Goal: Cover the ask → record → list flow and the honest-unknown and auth/not-found edges.

- [x] **Task 4.1** — Write BDD steps and scenarios for training message ask/answer
  - What: Add `features/admin_training_messages.feature` with scenarios in domain language (no HTTP verbs/status codes in the Gherkin text): (1) operator asks a question in an open training session and receives an answer with cited sources, matching the same shape citizens get from `/chat`; (2) the recorded exchange appears in the session's message list; (3) operator asks a question with no matching knowledge-base content and the honest-unknown fallback is recorded (`fell_back` true, no sources); (4) operator asking a question in an unknown session gets a not-found result; (5) operator is rejected without an admin key on both ask and list. Wire step definitions in `backend/tests/bdd.rs` using the `ChatWorld`/`BotWorld` + `reqwest`-via-`oneshot` pattern already used for training sessions, with a new `given_training_messages_api_available` step building a real `RagTrainingMessageAdapter`-backed router wired to a test-double `RagEngine` (reusing the same test port doubles as `rag_engine::engine`'s unit tests, e.g. a `TestRetrieval` returning no chunks for the honest-unknown scenario).
  - Deliverables:
    - `features/admin_training_messages.feature`
    - Step definitions in `backend/tests/bdd.rs` (new `BotWorld` fields as needed, e.g. `training_messages_router`, `training_message_id`, `last_training_message_response`)
  - Skills to load: spontini-tdd-rust, spontini-bdd-gherkin
  - Verification: `cargo test -p backend --test bdd` passes with the new scenarios green, and all pre-existing scenarios remain green.

## Acceptance Criteria

- `POST /admin/api/training/sessions/:id/messages` with `{ question }` against a known session calls `RagEngine::answer`, persists the exchange, and returns `201` with `{ id, session_id, question, answer, sources: [{document_id, source_ref}, ...], fell_back, created_at }`.
- `POST /admin/api/training/sessions/:id/messages` against an unknown session returns `404`.
- `GET /admin/api/training/sessions/:id/messages` returns the session's exchanges in chronological (oldest-first) order.
- The honest-unknown path (no retrieved chunks) is recorded with `fell_back: true` and an empty `sources` array, exactly mirroring `/chat`'s fallback behavior.
- Both endpoints return `401` when `X-Admin-Key` is missing or wrong.
- All existing tests in the workspace (`cargo test --workspace`) remain green.
- BDD scenarios cover ask→record→list, the honest-unknown fallback, unknown-session 404, and missing-auth on both endpoints.

## Risks

- **Reusing a live `RagEngine` in tests is expensive/networked** — the real `EmbeddingAdapter`/`GenerationAdapter` call out to `llama-embed`/`llama-generate` over HTTP. Mitigation: unit and BDD tests build `RagEngine` from the same in-process test-double ports (`EmbeddingPort`/`RetrievalPort`/`PersonaPort`/`GenerationPort`) already used in `rag_engine::engine`'s test module, never the real HTTP adapters (consistent with how `/chat`'s own BDD scenarios are tested today).
- **`sources` stored as a JSON TEXT column has no schema enforcement** — a malformed JSON string could fail to deserialize when read back. Mitigation: the only writer is `RagTrainingMessageAdapter::ask`, which always serializes a `Vec<CitedSource>` it just received from `RagEngine::answer`; no other code path writes to this column.
- **No cross-feature validation yet for feedback** — this plan does not yet know about `training_feedback` (feature 0014), so there is no check preventing feedback anchored to a message from a closed session. Mitigation: feature 0014 will add its own foreign-key reference to `training_message.id`; this plan's schema is additive and does not need to anticipate it.

## Out-of-Scope

- No `training_feedback` table/endpoint.
- No admin-ui SPA changes.
- No message editing or deletion.
- No pagination.
- No auth changes beyond the existing `X-Admin-Key` header.
- No changes to `RagEngine` or `/chat`.
