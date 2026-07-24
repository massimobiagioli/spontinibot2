# Plan 0012: `/admin/api/training/sessions` — training session CRUD

- **Status**: closed
- **Approved**: 2026-07-24 by Sisyphus
- **Implemented**: 2026-07-24 by Sisyphus
- **Closed**: 2026-07-24 by Sisyphus
- **Review verdict**: approved
- **Branch**: feat/admin-api-training-sessions
- **Feature ID**: 0012
- **Created**: 2026-07-24
- **Owner**: Sisyphus

## Objective

Add the first piece of the operator Training surface (Milestone 2): a `training_session` table in `kb-store` (a new `V3__training_sessions.sql` migration) and admin endpoints to create, list, get, and close a training session. A session is the grouping construct that later features (0013 — ask/answer with recording, 0014 — point-in-answer feedback) attach their rows to; this plan only establishes the session lifecycle itself, with no messages or feedback yet. This follows the same Clean Architecture shape already used for `/admin/api/ingest/config` (0010) and `/admin/api/ingest/run` (0011): a `TrainingSessionAdminPort` trait wrapping new `kb-store` CRUD methods, implemented by a `KbStoreTrainingSessionAdapter`, exposed via axum handlers guarded by the existing `X-Admin-Key` header (feature 0008). Per the [Constitution](../docs/CONSTITUTION.md) §3 (Simplicity), a session has the minimum fields needed to group later exchanges (`id`, `title`, `created_at`, `created_by`, `closed_at`) — no tagging, ownership transfer, or archival beyond a single `closed_at` timestamp.

In scope: `kb-store` schema + CRUD for `training_session`, and the `backend` admin HTTP surface to create/list/get/close a session.

Out of scope: training messages (feature 0013) and point-in-answer feedback (feature 0014) — those are separate roadmap features that will reference `training_session.id` as a foreign key in their own migrations. No admin-ui SPA changes (feature 0018 builds the Training section).

## Non-Goals

- No `training_message` or `training_feedback` tables or endpoints (features 0013, 0014).
- No admin-ui SPA changes.
- No session editing beyond create and close — a closed session's `title`/`created_by` cannot be changed, and a closed session cannot be reopened.
- No deletion endpoint for a session (sessions are an append-only training log; closing is the only lifecycle transition after creation).
- No pagination on the list endpoint (single-operator system; session count stays small).
- No authentication changes beyond the existing shared-secret `X-Admin-Key` header.

## Phases

### Phase 1: `kb-store` — `training_session` schema and CRUD

Goal: Add the `V3` migration and the storage-layer methods a training session needs.

- [x] **Task 1.1** — Add `V3__training_sessions.sql` migration
  - What: Create `kb-store/src/migrations/V3__training_sessions.sql` with `CREATE TABLE IF NOT EXISTS training_session (id INTEGER PRIMARY KEY, title TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')), created_by TEXT, closed_at TEXT)`. Wire it into `kb-store/src/migrations/mod.rs` following the existing `V1`/`V2` pattern: `include_str!`, a version-3 check against `_migrations`, `execute_batch`, and an insert into `_migrations` recording version 3 as `training_sessions_schema`.
  - Deliverables:
    - `kb-store/src/migrations/V3__training_sessions.sql`
    - Updated `kb-store/src/migrations/mod.rs` with the version-3 migration step
    - Unit test asserting the `training_session` table exists after `run_migrations`, and that running migrations twice stays idempotent
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store` passes with the new migration test.

- [x] **Task 1.2** — Add `TrainingSession`/`NewTrainingSession` types and `KbStore` CRUD methods
  - What: In `kb-store/src/types.rs`, add `pub struct TrainingSession { pub id: i64, pub title: String, pub created_at: String, pub created_by: Option<String>, pub closed_at: Option<String> }` and `pub struct NewTrainingSession { pub title: String, pub created_by: Option<String> }`. In `kb-store/src/lib.rs`, add `pub async fn create_training_session(&self, session: NewTrainingSession) -> Result<TrainingSession>` (INSERT + re-select, mirroring `insert_document`'s insert-then-fetch-by-rowid pattern), `pub async fn list_training_sessions(&self) -> Result<Vec<TrainingSession>>` (ordered by `created_at DESC`), `pub async fn get_training_session(&self, id: i64) -> Result<Option<TrainingSession>>`, and `pub async fn close_training_session(&self, id: i64) -> Result<bool>` (sets `closed_at = datetime('now')` where `id = ?` and `closed_at IS NULL`; returns whether a row was actually updated, so closing an already-closed or nonexistent session returns `false`).
  - Deliverables:
    - `TrainingSession`, `NewTrainingSession` in `kb-store/src/types.rs`, re-exported from `kb-store/src/lib.rs`
    - `create_training_session`, `list_training_sessions`, `get_training_session`, `close_training_session` on `KbStore`
    - Unit tests: create returns a session with `closed_at: None`; list returns sessions newest-first; get returns `None` for an unknown id; close sets `closed_at` and returns `true`; closing an already-closed session returns `false`; closing an unknown id returns `false`
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store` passes with all new tests green.

### Phase 2: `TrainingSessionAdminPort` trait and `kb-store` adapter

Goal: Define the admin port for training session CRUD and implement it against `kb-store`.

- [x] **Task 2.1** — Define `TrainingSessionAdminPort` trait and response/error types
  - What: In a new file `backend/src/admin/training_sessions/mod.rs`, define `#[async_trait] pub trait TrainingSessionAdminPort: Send + Sync` with `async fn create_session(&self, req: NewTrainingSession) -> Result<TrainingSessionResponse, TrainingSessionError>`, `async fn list_sessions(&self) -> Result<Vec<TrainingSessionResponse>, TrainingSessionError>`, `async fn get_session(&self, id: i64) -> Result<Option<TrainingSessionResponse>, TrainingSessionError>`, `async fn close_session(&self, id: i64) -> Result<bool, TrainingSessionError>`. Define `pub struct TrainingSessionResponse { pub id: i64, pub title: String, pub created_at: String, pub created_by: Option<String>, pub closed_at: Option<String> }` with a `From<kb_store::TrainingSession>` impl, and `pub enum TrainingSessionError { DbError(String) }` with `Display`/`std::error::Error`/`From<kb_store::KbStoreError>` impls, mirroring `IngestRunError`'s shape (feature 0011).
  - Deliverables:
    - `backend/src/admin/training_sessions/mod.rs` with trait, `TrainingSessionResponse`, `TrainingSessionError`
    - Unit tests: `TrainingSessionResponse::from` maps all fields including `None` created_by/closed_at; `TrainingSessionError` Display format
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo build -p backend` compiles; trait is object-safe.

- [x] **Task 2.2** — Implement `KbStoreTrainingSessionAdapter`
  - What: Create `backend/src/admin/training_sessions/adapter.rs` with `KbStoreTrainingSessionAdapter { store: Arc<kb_store::KbStore> }`. Implement `TrainingSessionAdminPort`, delegating each method directly to the matching `kb-store` method (Task 1.2) and mapping the result through `TrainingSessionResponse::from`.
  - Deliverables:
    - `backend/src/admin/training_sessions/adapter.rs` with `KbStoreTrainingSessionAdapter`
    - Unit tests using a temp `kb.db`: create returns an open session; list returns newest-first; get returns `None` for unknown id; close transitions an open session and returns `true`; closing twice returns `false` the second time
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo test -p backend` passes with adapter tests.

### Phase 3: Axum routes and integration wiring

Goal: Expose the endpoints under `/admin/api/training/sessions`.

- [x] **Task 3.1** — Add admin training-session route handlers
  - What: Create `backend/src/admin/training_sessions/handlers.rs` with `#[derive(Clone)] pub struct TrainingSessionState { pub training_sessions: Arc<dyn TrainingSessionAdminPort>, pub config: Config }` and four handlers: `create_session` (`POST`, body `{ title, created_by }`, returns `201` with the created session), `list_sessions` (`GET`, returns `200` with a JSON array), `get_session` (`GET /:id`, returns `200` or `404`), `close_session` (`POST /:id/close`, returns `200` with the updated `{ closed: bool }` shape — `false` when the session was already closed or does not exist, mirroring the `DeletedResponse`-style boolean-result pattern from `admin/ingest_config/handlers.rs`). All four call `crate::admin::check_admin_key(&headers, &state.config)?` first.
  - Deliverables:
    - `backend/src/admin/training_sessions/handlers.rs` with `TrainingSessionState` and the four handlers
    - Unit tests: 401 on each handler when `X-Admin-Key` missing/wrong; create returns 201; list returns the created sessions; get returns 200 for a known id and 404 for unknown; close returns `{closed: true}` for an open session and `{closed: false}` for an already-closed one
  - Skills to load: spontini-tdd-rust, spontini-bdd-gherkin
  - Verification: `cargo build -p backend` compiles; handler unit tests pass.

- [x] **Task 3.2** — Wire training-session routes into the router and module tree
  - What: In `backend/src/admin/mod.rs`, add `pub mod training_sessions;`. In `backend/src/lib.rs`, construct `KbStoreTrainingSessionAdapter` from the shared `store`, build a `TrainingSessionState`, thread it through `router_with` as a new parameter (same pattern as `ingest_run_state`), and add four routes: `POST /admin/api/training/sessions`, `GET /admin/api/training/sessions`, `GET /admin/api/training/sessions/:id`, `POST /admin/api/training/sessions/:id/close`.
  - Deliverables:
    - Updated `backend/src/admin/mod.rs` module declaration
    - Updated `router()` construction of `TrainingSessionState`
    - Updated `router_with()` signature (new `training_session_state: TrainingSessionState` parameter) and route registration
    - All existing `router_with` call sites in `backend/tests/bdd.rs` updated to pass the new parameter (stub state where no real `kb.db` is available, real `KbStoreTrainingSessionAdapter`-backed state where one is)
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo build -p backend` compiles; `cargo test -p backend` passes; `cargo test -p backend --test bdd` (existing scenarios) stays green.

### Phase 4: BDD scenarios

Goal: Cover the create → list → close lifecycle and the auth/not-found edges.

- [x] **Task 4.1** — Write BDD steps and scenarios for training session lifecycle
  - What: Add `features/admin_training_sessions.feature` with scenarios in domain language (no HTTP verbs/status codes in the Gherkin text): (1) operator creates a training session and it appears in the session list; (2) operator retrieves a single session by id; (3) operator closes an open session and it is reflected as closed; (4) operator closing an already-closed session is a no-op; (5) operator looking up an unknown session id gets a not-found result; (6) operator is rejected without an admin key on each of create/list/get/close. Wire step definitions in `backend/tests/bdd.rs` using the `ChatWorld`/`BotWorld` + `reqwest`-via-`oneshot` pattern already used for ingest config and ingest run, with a new `given_training_sessions_api_available` step building a real `KbStoreTrainingSessionAdapter`-backed router.
  - Deliverables:
    - `features/admin_training_sessions.feature`
    - Step definitions in `backend/tests/bdd.rs` (new `BotWorld` fields as needed, e.g. `training_sessions_db_path`, `training_sessions_router`, `training_session_id`)
  - Skills to load: spontini-tdd-rust, spontini-bdd-gherkin
  - Verification: `cargo test -p backend --test bdd` passes with the new scenarios green, and all pre-existing scenarios remain green.

## Acceptance Criteria

- `POST /admin/api/training/sessions` with `{ title, created_by }` creates a session and returns `201` with `{ id, title, created_at, created_by, closed_at: null }`.
- `GET /admin/api/training/sessions` returns all sessions, newest-first.
- `GET /admin/api/training/sessions/:id` returns `200` with the session or `404` for an unknown id.
- `POST /admin/api/training/sessions/:id/close` sets `closed_at` on an open session and returns `{ closed: true }`; returns `{ closed: false }` when the session is already closed or does not exist.
- All four endpoints return `401` when `X-Admin-Key` is missing or wrong.
- All existing tests in the workspace (`cargo test --workspace`) remain green.
- BDD scenarios cover create→list, get-by-id (found and not-found), close (open→closed and already-closed no-op), and missing-auth on all four endpoints.

## Risks

- **`close_training_session` semantics** — using a conditional `UPDATE ... WHERE closed_at IS NULL` to detect "already closed" relies on `closed_at IS NULL` as the single source of truth for open/closed state, consistent with the schema in the roadmap description. Mitigation: covered directly by a unit test (Task 1.2) asserting a second close returns `false` without erroring.
- **No cross-feature validation yet** — this plan does not yet know about `training_message`/`training_feedback` (features 0013/0014), so there is no check preventing a session with unknown future-referenced state. Mitigation: those features will add their own foreign-key references to `training_session.id`; this plan's schema is additive and does not need to anticipate them.

## Out-of-Scope

- No `training_message` or `training_feedback` tables/endpoints.
- No admin-ui SPA changes.
- No session deletion.
- No pagination.
- No auth changes beyond the existing `X-Admin-Key` header.
