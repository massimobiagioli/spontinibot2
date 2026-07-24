# Plan 0011: `/admin/api/ingest/run` — trigger an immediate ingest run

- **Status**: closed
- **Approved**: 2026-07-24 by Sisyphus
- **Implemented**: 2026-07-24 by Sisyphus
- **Closed**: 2026-07-24 by Sisyphus
- **Review verdict**: approved
- **Branch**: feat/admin-api-ingest-run
- **Feature ID**: 0011
- **Created**: 2026-07-24
- **Owner**: Sisyphus

## Objective

Add the admin endpoint that lets an operator trigger an out-of-schedule ingest run and poll its status, closing the loop on the `ingest_run_request` flag-row table that `kb-store` has provided since feature 0004 (`request_run`, `consume_run_request`, `complete_run`) but that no HTTP surface has exposed yet. `POST /admin/api/ingest/run` writes a new pending run-request row and returns `202 Accepted` with a request id; `GET /admin/api/ingest/run/:id` lets the operator poll that id for its current status (`pending` / `running` / `done` / `failed`). This follows the same Clean Architecture shape as feature 0010 (`IngestConfigAdminPort` + `KbStoreIngestConfigAdapter`): a new `IngestRunAdminPort` trait wrapping the existing `kb-store` run-request methods, exposed via axum routes guarded by the existing `X-Admin-Key` header (feature 0008). Per the [Constitution](../docs/CONSTITUTION.md) §3 (Simplicity), this is a thin CRUD-shaped wrapper around storage that already exists — no new business logic beyond mapping DB rows to response DTOs.

In scope: the `backend` admin HTTP surface (trigger + poll), a small `kb-store` addition (`get_run_request(id)`, since no existing method fetches a single run request by id — only insert/consume-next-pending/complete exist), and BDD coverage for the trigger → poll → done lifecycle.

Out of scope: any change to the `ingest` service's scheduler. The `ingest` service (feature 0006) already runs a `run_poll_secs` interval, but it currently triggers the pipeline unconditionally on every tick rather than gating on an actual pending `ingest_run_request` row — it never calls `consume_run_request` or `complete_run`. This is a pre-existing gap in feature 0006, not introduced or hidden by this plan; it is documented under Risks below and left for a future fix, since wiring the `ingest` binary is outside an "Admin Surface (Backend)" milestone feature whose contract is the `kb-store` flag table itself. This plan's BDD scenario proves the admin endpoints work correctly against that table by driving status transitions the same way the `ingest` service is meant to (calling `consume_run_request` / `complete_run` directly in the test), not by spinning up the real `ingest` binary.

## Non-Goals

- No changes to the `ingest` crate/service or its scheduler.
- No changes to the `ingest_run_request` table schema (it already exists from feature 0004's `V2__ingest_config.sql`).
- No admin-ui SPA changes (separate feature 0016).
- No cancellation endpoint (no way to cancel a pending/running request).
- No history/listing endpoint (no `GET /admin/api/ingest/run` to list past requests) — only trigger and get-by-id.
- No websocket/SSE push for status updates — polling only.

## Phases

### Phase 1: `kb-store` — fetch a single run request by id

Goal: Add the one missing read method needed to support polling.

- [x] **Task 1.1** — Add `KbStore::get_run_request(id)`
  - What: In `kb-store/src/lib.rs`, add `pub async fn get_run_request(&self, id: i64) -> Result<Option<IngestRunRequest>>` that selects `id, requested_at, status FROM ingest_run_request WHERE id = ?1` and maps the row the same way `request_run` does, returning `None` when no row matches.
  - Deliverables:
    - `get_run_request` method in `kb-store/src/lib.rs`
    - Unit tests: returns `Some` with correct status right after `request_run`; returns `Some` with `Running` after `consume_run_request`; returns `Some` with `Done`/`Failed` after `complete_run`; returns `None` for an unknown id
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store` passes with the new tests green.

### Phase 2: `IngestRunAdminPort` trait and `kb-store` adapter

Goal: Define an admin port for triggering and polling ingest runs, implemented against `kb-store`.

- [x] **Task 2.1** — Define `IngestRunAdminPort` trait and response/error types
  - What: In a new file `backend/src/admin/ingest_run/mod.rs`, define `#[async_trait] pub trait IngestRunAdminPort: Send + Sync` with `async fn trigger_run(&self) -> Result<IngestRunResponse, IngestRunError>` and `async fn get_run(&self, id: i64) -> Result<Option<IngestRunResponse>, IngestRunError>`. Define `pub struct IngestRunResponse { pub id: i64, pub status: String, pub requested_at: String }` (status serialized as one of `"pending"`/`"running"`/`"done"`/`"failed"`) and `pub enum IngestRunError { DbError(String) }` (no `NotFound` variant needed — the `get_run` signature already models "not found" as `Ok(None)`, matched by the handler).
  - Deliverables:
    - `backend/src/admin/ingest_run/mod.rs` with trait, `IngestRunResponse`, `IngestRunError`
    - Unit test constructing `IngestRunError` and asserting its `Display`/mapping shape
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo build -p backend` compiles; trait is object-safe.

- [x] **Task 2.2** — Implement `KbStoreIngestRunAdapter`
  - What: Create `backend/src/admin/ingest_run/adapter.rs` with `KbStoreIngestRunAdapter { store: Arc<kb_store::KbStore> }`. Implement `IngestRunAdminPort`: `trigger_run` calls `store.request_run()` and maps the returned `IngestRunRequest` (via its `RunRequestStatus`'s `Display` impl, i.e. `.status.to_string()`) into `IngestRunResponse`; `get_run` calls `store.get_run_request(id)` (Task 1.1) and maps `Some`/`None` through.
  - Deliverables:
    - `backend/src/admin/ingest_run/adapter.rs` with `KbStoreIngestRunAdapter`
    - Unit tests using a temp `kb.db`: `trigger_run` returns a `pending` response with an id; `get_run` returns the same row; `get_run` on an unknown id returns `Ok(None)`
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo test -p backend` passes with adapter tests.

### Phase 3: Axum routes and integration wiring

Goal: Expose the endpoints under `/admin/api/ingest/run` and `/admin/api/ingest/run/:id`.

- [x] **Task 3.1** — Add admin ingest-run route handlers
  - What: Create `backend/src/admin/ingest_run/handlers.rs` with `#[derive(Clone)] pub struct IngestRunState { pub ingest_run: Arc<dyn IngestRunAdminPort>, pub config: Config }`, and two handlers: `trigger_run(State, HeaderMap) -> Result<(StatusCode, Json<IngestRunResponse>), (StatusCode, Json<ErrorResponse>)>` returning `(StatusCode::ACCEPTED, Json(response))` on success; `get_run(State, HeaderMap, Path(id): Path<i64>) -> Result<Json<IngestRunResponse>, (StatusCode, Json<ErrorResponse>)>` returning `404` with an `ErrorResponse` when the port returns `Ok(None)`. Both call `crate::admin::check_admin_key(&headers, &state.config)?` first, mirroring `admin/ingest_config/handlers.rs`.
  - Deliverables:
    - `backend/src/admin/ingest_run/handlers.rs` with `IngestRunState`, `trigger_run`, `get_run`
    - Unit tests: 401 when `X-Admin-Key` missing/wrong; trigger returns 202 with `pending` status; get returns 200 for an existing id and 404 for an unknown id
  - Skills to load: spontini-tdd-rust, spontini-bdd-gherkin
  - Verification: `cargo build -p backend` compiles; handler unit tests pass.

- [x] **Task 3.2** — Wire ingest-run routes into the router and module tree
  - What: In `backend/src/admin/mod.rs`, add `pub mod ingest_run;`. In `backend/src/lib.rs`, construct `KbStoreIngestRunAdapter` from the shared `store` (same `Arc<KbStore>` already used for `ingest_config_port`), build an `IngestRunState`, thread it through `router_with` as a new parameter (same pattern as `ingest_config_state`), and add the two routes: `POST /admin/api/ingest/run` → `trigger_run`, `GET /admin/api/ingest/run/:id` → `get_run`.
  - Deliverables:
    - Updated `backend/src/admin/mod.rs` module declaration
    - Updated `router()` construction of `IngestRunState`
    - Updated `router_with()` signature (new `ingest_run_state: IngestRunState` parameter) and route registration
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo build -p backend` compiles; `cargo test -p backend` passes; all existing call sites of `router_with` in tests are updated to pass the new parameter.

### Phase 4: BDD scenarios

Goal: Cover the trigger → poll → done lifecycle and the auth/not-found edges.

- [x] **Task 4.1** — Write BDD steps and scenarios for ingest run trigger/poll
  - What: Add BDD scenarios to `backend/tests/bdd.rs` using the existing `ChatWorld` + `reqwest` pattern: (1) trigger a run via `POST /admin/api/ingest/run`, assert `202` and a `pending` status with an id; (2) poll `GET /admin/api/ingest/run/:id` immediately and assert `pending`; (3) drive the row to `running` then `done` the same way the `ingest` service would (call `KbStore::consume_run_request` then `KbStore::complete_run(id, Done)` directly against the shared test `kb.db`), then poll again and assert `done`; (4) poll an unknown id and assert `404`; (5) call both endpoints without `X-Admin-Key` and assert `401`.
  - Deliverables:
    - BDD scenarios for trigger→pending, poll→running→done, unknown-id 404, missing-auth 401
    - Wired step definitions reusing `ChatWorld` infrastructure
  - Skills to load: spontini-tdd-rust, spontini-bdd-gherkin
  - Verification: `cargo test -p backend --test bdd -- --nocapture` passes with new scenarios green.

## Acceptance Criteria

- `POST /admin/api/ingest/run` returns `202 Accepted` with `{ id, status: "pending", requested_at }`.
- `GET /admin/api/ingest/run/:id` returns `200` with the current `{ id, status, requested_at }` for a known id, reflecting whatever status the row currently has (`pending`/`running`/`done`/`failed`).
- `GET /admin/api/ingest/run/:id` returns `404` for an unknown id.
- Both endpoints return `401` when `X-Admin-Key` is missing or wrong.
- All existing tests in the workspace (`cargo test --workspace`) remain green.
- BDD scenarios cover trigger→pending, the full pending→running→done transition as observed through polling, the unknown-id 404, and the missing-auth 401.

## Risks

- **The `ingest` service does not yet consume `ingest_run_request` rows** — its scheduler (feature 0006) runs the pipeline unconditionally on every `run_poll_secs` tick instead of checking for a pending row via `consume_run_request`/`complete_run`. This means that in the currently running system, triggering a run via this endpoint creates a `pending` row that will never transition to `running`/`done` on its own. Mitigation: this is a pre-existing gap outside this plan's scope (an admin-API-only feature per the Milestone 2 roadmap); it is called out explicitly here rather than silently papered over, and is a natural candidate for a follow-up fix when the `ingest` scheduler is revisited. The BDD scenario in Task 4.1 proves the admin endpoints are correct by driving the transition directly against `kb-store`, independent of whether `ingest` currently does so.
- **Polling gives no progress detail beyond a status enum** — an operator cannot see partial progress (e.g. "3 of 5 sources done"). Mitigation: acceptable for a single-operator system at this stage; a richer progress model is a future enhancement, not required by the roadmap description.

## Out-of-Scope

- No admin-ui SPA changes.
- No changes to the `ingest` service or scheduler.
- No changes to the `ingest_run_request` table schema.
- No cancellation endpoint.
- No history/listing endpoint.
- No websocket/SSE push.
