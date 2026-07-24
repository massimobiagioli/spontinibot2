# Plan 0014: `/admin/api/training/feedback` — point-in-answer feedback

- **Status**: review
- **Approved**: 2026-07-24 by Sisyphus
- **Implemented**: 2026-07-24 by Sisyphus
- **Branch**: feat/admin-api-training-feedback
- **Feature ID**: 0014
- **Created**: 2026-07-24
- **Owner**: Sisyphus

## Objective

Close out the Training data model (Milestone 2) by adding the point-in-answer feedback mechanism: a `training_feedback` table in `kb-store` (a new `V5__training_feedback.sql` migration) and admin endpoints to record and list feedback anchored to a specific span of a recorded training message's answer, optionally tied to one of the cited chunks. This directly serves the [Constitution](../docs/CONSTITUTION.md) mission of a truthful, verifiable bot: by letting an operator mark exactly which part of an answer was right or wrong (and against which retrieved document, if any), the system accumulates the structured signal a future retrieval-quality analysis needs — without building that analysis here. This follows the same Clean Architecture shape already used for `/admin/api/training/sessions` (0012) and `/admin/api/training/sessions/:id/messages` (0013): a `TrainingFeedbackAdminPort` trait wrapping new `kb-store` CRUD methods, implemented by a `KbStoreTrainingFeedbackAdapter`, exposed via axum handlers guarded by the existing `X-Admin-Key` header (feature 0008).

In scope: `kb-store` schema + CRUD for `training_feedback` (foreign key to `training_message.id`, feature 0013; optional foreign key to `documents.id` for the cited chunk), and the `backend` admin HTTP surface to record feedback (`POST /admin/api/training/feedback`) and list it per message (`GET /admin/api/training/messages/:id/feedback`).

Out of scope: any retrieval-quality analytics or aggregation over the recorded feedback (explicitly deferred to "a later, out-of-roadmap analytics plan" per the roadmap description). No admin-ui SPA changes (feature 0018 builds the Training section, including the span-selection UI). No feedback editing or deletion.

## Non-Goals

- No analytics, aggregation, or scoring over `training_feedback` rows — this plan only records them.
- No admin-ui SPA changes.
- No feedback editing or deletion — feedback is an append-only log once submitted.
- No validation that `answer_span` is a substring of the referenced message's `answer` — the admin-ui (future feature 0018) is responsible for producing a valid span from the citizen's selection; this plan trusts the recorded text.
- No pagination on the list endpoint (single-operator system; per-message feedback count stays small).
- No authentication changes beyond the existing shared-secret `X-Admin-Key` header.

## Phases

### Phase 1: `kb-store` — `training_feedback` schema and CRUD

Goal: Add the `V5` migration and the storage-layer methods point-in-answer feedback needs.

- [x] **Task 1.1** — Add `V5__training_feedback.sql` migration
  - What: Create `kb-store/src/migrations/V5__training_feedback.sql` with `CREATE TABLE IF NOT EXISTS training_feedback (id INTEGER PRIMARY KEY, message_id INTEGER NOT NULL REFERENCES training_message(id), chunk_id INTEGER REFERENCES documents(id), answer_span TEXT NOT NULL, sentiment TEXT NOT NULL, comment TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')))`. Wire it into `kb-store/src/migrations/mod.rs` following the existing `V1`-`V4` pattern: `include_str!`, a version-5 check against `_migrations`, `execute_batch`, and an insert into `_migrations` recording version 5 as `training_feedback_schema`.
  - Deliverables:
    - `kb-store/src/migrations/V5__training_feedback.sql`
    - Updated `kb-store/src/migrations/mod.rs` with the version-5 migration step
    - Unit test asserting the `training_feedback` table exists after `run_migrations`, and that running migrations twice stays idempotent
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store` passes with the new migration test.

- [x] **Task 1.2** — Add `Sentiment` enum, `TrainingFeedback`/`NewTrainingFeedback` types, and `KbStore` CRUD methods
  - What: In `kb-store/src/types.rs`, add `pub enum Sentiment { Positive, Negative }` with `Display`/`FromStr` impls (`"positive"`/`"negative"`, mirroring `SourceType`'s pattern), `pub struct TrainingFeedback { pub id: i64, pub message_id: i64, pub chunk_id: Option<i64>, pub answer_span: String, pub sentiment: Sentiment, pub comment: Option<String>, pub created_at: String }`, and `pub struct NewTrainingFeedback { pub message_id: i64, pub chunk_id: Option<i64>, pub answer_span: String, pub sentiment: Sentiment, pub comment: Option<String> }`. In `kb-store/src/lib.rs`, add `pub async fn create_training_feedback(&self, feedback: NewTrainingFeedback) -> Result<TrainingFeedback>` (INSERT + re-select by last_insert_rowid, mirroring `create_training_message`; storing `sentiment.to_string()` as TEXT) and `pub async fn list_training_feedback(&self, message_id: i64) -> Result<Vec<TrainingFeedback>>` (ordered by `created_at ASC, id ASC`, mirroring `list_training_messages`; parsing the stored TEXT back through `Sentiment::from_str`).
  - Deliverables:
    - `Sentiment`, `TrainingFeedback`, `NewTrainingFeedback` in `kb-store/src/types.rs`, re-exported from `kb-store/src/lib.rs`
    - `create_training_feedback`, `list_training_feedback` on `KbStore`
    - Unit tests: `Sentiment` round-trips through `Display`/`FromStr` for both variants and rejects an unknown string; create returns feedback with all fields set (including `chunk_id: None` and `chunk_id: Some(id)` cases); create against a nonexistent `message_id` fails (the `training_feedback.message_id` foreign key, enforced the same way `training_message.session_id` is per feature 0013); list returns feedback oldest-first for a message; list returns an empty vec for a message with no feedback; list only returns feedback for the requested `message_id` (not other messages')
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store` passes with all new tests green.

### Phase 2: `TrainingFeedbackAdminPort` trait and `kb-store` adapter

Goal: Define the admin port for feedback CRUD and implement it against `kb-store`.

- [x] **Task 2.1** — Define `TrainingFeedbackAdminPort` trait and response/error types
  - What: In a new file `backend/src/admin/training_feedback/mod.rs`, define `#[async_trait] pub trait TrainingFeedbackAdminPort: Send + Sync` with `async fn create_feedback(&self, req: kb_store::NewTrainingFeedback) -> Result<TrainingFeedbackResponse, TrainingFeedbackError>` and `async fn list_feedback(&self, message_id: i64) -> Result<Vec<TrainingFeedbackResponse>, TrainingFeedbackError>`. Define `pub struct TrainingFeedbackResponse { pub id: i64, pub message_id: i64, pub chunk_id: Option<i64>, pub answer_span: String, pub sentiment: String, pub comment: Option<String>, pub created_at: String }` with a `From<kb_store::TrainingFeedback>` impl (serializing `sentiment` via `.to_string()`), and `pub enum TrainingFeedbackError { MessageNotFound(i64), DbError(String) }` with `Display`/`std::error::Error`/`From<kb_store::KbStoreError>` impls, mirroring `TrainingSessionError`'s shape (feature 0012).
  - Deliverables:
    - `backend/src/admin/training_feedback/mod.rs` with trait, `TrainingFeedbackResponse`, `TrainingFeedbackError`
    - Unit tests: `TrainingFeedbackResponse::from` maps all fields including `None`/`Some` `chunk_id` and `None`/`Some` `comment`; `TrainingFeedbackError` Display format for both variants; `TrainingFeedbackResponse` serializes cleanly via serde
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo build -p backend` compiles; trait is object-safe.

- [x] **Task 2.2** — Implement `KbStoreTrainingFeedbackAdapter`
  - What: Create `backend/src/admin/training_feedback/adapter.rs` with `KbStoreTrainingFeedbackAdapter { store: Arc<kb_store::KbStore> }`. Implement `TrainingFeedbackAdminPort::create_feedback`: first call `store.get_training_message(message_id)`-equivalent existence check — since `kb-store` does not yet have a single-message getter, add `pub async fn get_training_message(&self, id: i64) -> Result<Option<kb_store::TrainingMessage>>` to `KbStore` in this task (SELECT by id, mirroring `get_training_session`) — return `TrainingFeedbackError::MessageNotFound(message_id)` if `None`; otherwise call `store.create_training_feedback(req)` and map through `TrainingFeedbackResponse::from`. Implement `list_feedback` by calling `store.list_training_feedback(message_id)` and mapping each row (existence of the message is NOT re-checked here — an empty vec is returned for both "message exists with no feedback" and "message does not exist", matching the read-only listing scope already used by `TrainingMessageAdminPort::list_messages`).
  - Deliverables:
    - `backend/src/admin/training_feedback/adapter.rs` with `KbStoreTrainingFeedbackAdapter`
    - New `KbStore::get_training_message` method in `kb-store/src/lib.rs` with a unit test (found/not-found cases)
    - Unit tests using a temp `kb.db`: `create_feedback` against an unknown message returns `MessageNotFound`; `create_feedback` against a known message persists and returns a response with all fields round-tripping (including a `chunk_id: Some` case referencing a real inserted document and a `chunk_id: None` case); `list_feedback` returns feedback oldest-first
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo test -p backend` passes with adapter tests.

### Phase 3: Axum routes and integration wiring

Goal: Expose the endpoints under `/admin/api/training/feedback` and `/admin/api/training/messages/:id/feedback`.

- [x] **Task 3.1** — Add admin training-feedback route handlers
  - What: Create `backend/src/admin/training_feedback/handlers.rs` with `#[derive(Clone)] pub struct TrainingFeedbackState { pub training_feedback: Arc<dyn TrainingFeedbackAdminPort>, pub config: Config }` and two handlers: `create_feedback` (`POST /admin/api/training/feedback`, body `{ message_id, chunk_id, answer_span, sentiment, comment }` where `sentiment` is `"positive"` or `"negative"` — parsed via `kb_store::Sentiment::from_str`, returning `400` on an invalid value — returns `201` with the created `TrainingFeedbackResponse`, `404` when the message does not exist) and `list_feedback` (`GET /admin/api/training/messages/:id/feedback`, returns `200` with a JSON array, chronological order). Both call `crate::admin::check_admin_key(&headers, &state.config)?` first.
  - Deliverables:
    - `backend/src/admin/training_feedback/handlers.rs` with `TrainingFeedbackState` and the two handlers
    - Unit tests: 401 on each handler when `X-Admin-Key` missing/wrong; create returns 201 for a known message (with and without `chunk_id`/`comment`); create returns 404 for an unknown message; create returns 400 for an invalid `sentiment` string; list returns the recorded feedback for a known message
  - Skills to load: spontini-tdd-rust, spontini-bdd-gherkin
  - Verification: `cargo build -p backend` compiles; handler unit tests pass.

- [x] **Task 3.2** — Wire training-feedback routes into the router and module tree
  - What: In `backend/src/admin/mod.rs`, add `pub mod training_feedback;`. In `backend/src/lib.rs`, construct `KbStoreTrainingFeedbackAdapter` from the shared `store`, build a `TrainingFeedbackState`, add it as a new field on `AdminRouterState` (the struct introduced in feature 0013 to keep `router_with` under the clippy argument-count gate), and add two routes: `POST /admin/api/training/feedback`, `GET /admin/api/training/messages/:id/feedback`.
  - Deliverables:
    - Updated `backend/src/admin/mod.rs` module declaration
    - Updated `router()` construction of `TrainingFeedbackState`
    - Updated `AdminRouterState` with a new `training_feedback: TrainingFeedbackState` field, and route registration in `router_with`
    - All `backend::AdminRouterState { ... }` construction sites in `backend/tests/bdd.rs` updated to include the new field (stub state where no real `kb.db` is available, real `KbStoreTrainingFeedbackAdapter`-backed state where one is)
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo build -p backend` compiles; `cargo test -p backend` passes; `cargo test -p backend --test bdd` (existing scenarios) stays green.

### Phase 4: BDD scenarios

Goal: Cover positive, negative, and commented feedback on the same message, plus the not-found and auth edges.

- [x] **Task 4.1** — Write BDD steps and scenarios for point-in-answer feedback
  - What: Add `features/admin_training_feedback.feature` with scenarios in domain language (no HTTP verbs/status codes in the Gherkin text): (1) operator leaves positive feedback on a span of a recorded answer and it appears in that message's feedback list; (2) operator leaves negative feedback with a comment on the same message and both entries are listed; (3) operator leaves feedback anchored to a specific cited chunk; (4) operator leaving feedback with an invalid sentiment value is rejected; (5) operator leaving feedback on an unknown message gets a not-found result; (6) operator is rejected without an admin key on create and list. Wire step definitions in `backend/tests/bdd.rs` using the `BotWorld` + `reqwest`-via-`oneshot` pattern already used for training sessions/messages, with a new `given_training_feedback_api_available` step building a real `KbStoreTrainingFeedbackAdapter`-backed router wired to the same `RagTrainingMessageAdapter` setup as feature 0013 (so a real training message exists to anchor feedback to).
  - Deliverables:
    - `features/admin_training_feedback.feature`
    - Step definitions in `backend/tests/bdd.rs` (new `BotWorld` fields as needed, e.g. `training_feedback_message_id`)
  - Skills to load: spontini-tdd-rust, spontini-bdd-gherkin
  - Verification: `cargo test -p backend --test bdd` passes with the new scenarios green, and all pre-existing scenarios remain green.

## Acceptance Criteria

- `POST /admin/api/training/feedback` with `{ message_id, chunk_id, answer_span, sentiment, comment }` against a known message creates a feedback row and returns `201` with the full recorded shape.
- `POST /admin/api/training/feedback` against an unknown `message_id` returns `404`.
- `POST /admin/api/training/feedback` with an invalid `sentiment` value (not `"positive"`/`"negative"`) returns `400`.
- `GET /admin/api/training/messages/:id/feedback` returns all feedback for that message in chronological (oldest-first) order.
- Both endpoints return `401` when `X-Admin-Key` is missing or wrong.
- All existing tests in the workspace (`cargo test --workspace`) remain green.
- BDD scenarios cover positive feedback, negative feedback with a comment, chunk-anchored feedback, invalid-sentiment rejection, unknown-message 404, and missing-auth on both endpoints.

## Risks

- **`chunk_id` references `documents.id` but is never validated against the message's actual cited sources** — an operator (or a future admin-ui bug) could anchor feedback to a chunk that was never cited in that message's answer. Mitigation: out of scope per this plan's Non-Goals (no cross-referencing validation); the foreign key only guarantees the chunk exists as a document, not that it was cited by this specific message. A future analytics plan can add that check if it proves necessary.
- **`answer_span` is unvalidated free text** — nothing guarantees it is an actual substring of the message's `answer`. Mitigation: explicitly a Non-Goal; the admin-ui (feature 0018) is the boundary responsible for producing a valid span from the operator's selection.
- **Growing `AdminRouterState`** — this is the second feature to add a field to the struct introduced in 0013. Mitigation: the struct exists precisely to absorb this growth without re-triggering the `clippy::too_many_arguments` gate on `router_with`; no further action needed.

## Out-of-Scope

- No analytics or aggregation over feedback.
- No admin-ui SPA changes.
- No feedback editing or deletion.
- No pagination.
- No auth changes beyond the existing `X-Admin-Key` header.
- No validation that `answer_span` is a substring of the message's answer, or that `chunk_id` was actually cited.
