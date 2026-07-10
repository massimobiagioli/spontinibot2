# Plan 0010: `/admin/api/ingest/config` — read/write ingest configuration

- **Status**: closed
- **Approved**: 2026-07-10 by Sisyphus
- **Implemented**: 2026-07-10 by Sisyphus
- **Closed**: 2026-07-10 by Sisyphus
- **Review verdict**: approved
- **Branch**: feat/ingest-config-api
- **Feature ID**: 0010
- **Created**: 2026-07-10
- **Owner**: Sisyphus

## Objective

Add the admin ingest configuration surface to `backend`: endpoints to read and write the ingest schedule, sections, and per-section sources that were defined in feature 0004's `kb-store` schema. This enables an operator to configure the ingest pipeline (what sections exist, which URLs are scraped, how often) via the API, laying the foundation for the admin-ui SPA ingest configuration view (feature 0016). The existing `kb-store` methods (`get_schedule`, `upsert_schedule`, `list_sections`, `upsert_section`, `delete_section`, `list_sources_by_section`, `upsert_source`, `delete_source`) are the storage layer; this feature wraps them behind a Clean Architecture port and exposes them via axum routes under `/admin/api/ingest/config`. The `api` source type is writable but always returned with `enabled=false` and `coming_soon: true` — the API adapter is not wired to the scheduler yet (feature 0005, feature 0006). All admin routes are protected by the existing `X-Admin-Key` header (feature 0008). BDD scenarios validate the full CRUD lifecycle and the disabled-api-source invariant. Per the [Constitution](../docs/CONSTITUTION.md) §3 (Simplicity), this is a thin CRUD wrapper — no business logic beyond the `api` source transformation.

## Non-Goals

- No admin-ui SPA changes (separate feature 0016).
- No changes to `kb-store` schema (it already has `ingest_schedule`, `ingest_section`, `ingest_source` tables from feature 0004).
- No changes to the `ingest` service scheduler (it already polls `kb.db` for config changes).
- No validation of cron expressions beyond accepting a string (the ingest service handles parse errors).
- No ordering management beyond the `ordering` field on sections (no drag-and-drop reordering).
- No source URL validation or reachability checks.
- No bulk operations (batch create/update/delete).

## Phases

### Phase 1: `IngestConfigAdminPort` trait and `kb-store` adapter

Goal: Define an admin port for ingest config CRUD and implement it against `kb-store`.

- [x] **Task 1.1** — Define `IngestConfigAdminPort` trait
  - What: In a new file `backend/src/admin/ingest_config/mod.rs`, define a `#[async_trait] pub trait IngestConfigAdminPort: Send + Sync` with methods: `async fn get_config(&self) -> Result<IngestConfigResponse, IngestConfigError>`, `async fn upsert_schedule(&self, schedule: NewIngestSchedule) -> Result<IngestScheduleResponse, IngestConfigError>`, `async fn create_section(&self, section: NewIngestSection) -> Result<IngestSectionResponse, IngestConfigError>`, `async fn update_section(&self, id: i64, section: NewIngestSection) -> Result<IngestSectionResponse, IngestConfigError>`, `async fn delete_section(&self, id: i64) -> Result<bool, IngestConfigError>`, `async fn create_source(&self, section_id: i64, source: NewIngestSource) -> Result<IngestSourceResponse, IngestConfigError>`, `async fn update_source(&self, id: i64, source: NewIngestSource) -> Result<IngestSourceResponse, IngestConfigError>`, `async fn delete_source(&self, id: i64) -> Result<bool, IngestConfigError>`. The `IngestConfigResponse` struct contains `schedule: Option<IngestScheduleResponse>` and `sections: Vec<IngestSectionWithSources>` (where `IngestSectionWithSources` has the section fields + `sources: Vec<IngestSourceResponse>`). The `IngestSourceResponse` adds `coming_soon: bool` for API source types. The `IngestConfigError` enum covers: NotFound(String), DbError(String).
  - Deliverables:
    - `backend/src/admin/ingest_config/mod.rs` with trait, response types, and error type
    - Unit tests for error type construction
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo build -p backend` compiles; trait is object-safe.

- [x] **Task 1.2** — Implement `KbStoreIngestConfigAdapter`
  - What: Create `backend/src/admin/ingest_config/adapter.rs` with a `KbStoreIngestConfigAdapter` struct holding `Arc<KbStore>`. Implement `IngestConfigAdminPort` for it, delegating to `kb-store` methods. The `get_config` method calls `get_schedule`, `list_sections`, and for each section calls `list_sources_by_section` to build the tree. The `api` source type transformation is applied here: for sources with `source_type == Api`, set `enabled = false` and `coming_soon = true` in the response. For `create_section`, call `upsert_section`. For `update_section`, the section has no mutable fields beyond `name` and `ordering` which are immutable after creation — the method returns `NotFound` for now (sections cannot be renamed/reordered after creation; this is a deliberate simplicity choice per Constitution §3). The `delete_section` method deletes by ID and returns whether it existed. For `create_source`/`update_source`/`delete_source`, delegate to `upsert_source`/`delete_source`.
  - Deliverables:
    - `backend/src/admin/ingest_config/adapter.rs` with `KbStoreIngestConfigAdapter`
    - `IngestConfigAdminPort` implementation
    - Unit tests using a temp `kb.db` covering: get_config returns tree, upsert_schedule updates, create_section + create_source builds tree, api source gets coming_soon, delete operations work
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo test -p backend` passes with adapter tests.

### Phase 2: Axum routes and integration wiring

Goal: Expose the admin ingest config endpoints via axum under `/admin/api/ingest/config/`.

- [x] **Task 2.1** — Add admin ingest config route handlers
  - What: Create `backend/src/admin/ingest_config/handlers.rs` with axum route handlers: `GET /admin/api/ingest/config` → returns the full config tree (schedule + sections with sources); `PUT /admin/api/ingest/config/schedule` → upserts schedule (body: `{ cron_expr, enabled }`); `POST /admin/api/ingest/config/sections` → creates a section (body: `{ name, ordering }`); `PUT /admin/api/ingest/config/sections/:id` → updates a section (body: `{ name, ordering }`); `DELETE /admin/api/ingest/config/sections/:id` → deletes a section; `POST /admin/api/ingest/config/sources` → creates a source in a section (query param `section_id`, body: `{ source_type, url, enabled }`); `PUT /admin/api/ingest/config/sources/:id` → updates a source (body: `{ source_type, url, enabled }`); `DELETE /admin/api/ingest/config/sources/:id` → deletes a source. All handlers check `X-Admin-Key` header via the existing `check_admin_key` helper from `admin/mod.rs`. The handlers use a new `IngestConfigState` struct holding `Arc<dyn IngestConfigAdminPort>` and `Config`.
  - Deliverables:
    - `backend/src/admin/ingest_config/handlers.rs` with all route handlers
    - Request/response DTOs with serde Serialize/Deserialize
    - Unit tests for each handler (auth rejection, valid CRUD operations, not-found responses)
  - Skills to load: spontini-tdd-rust, spontini-bdd-gherkin
  - Verification: `cargo build -p backend` compiles; handlers parse requests correctly.

- [x] **Task 2.2** — Wire admin ingest config routes into the router and `AppState`
  - What: In `backend/src/lib.rs`, add `ingest_config_admin: Arc<dyn IngestConfigAdminPort>` to `AppState`. In `router()`, construct `KbStoreIngestConfigAdapter` and wire it. Add axum `route()` calls for all 8 admin ingest config endpoints. The `router_with` function gains a new `IngestConfigState` parameter. Update the existing `router_with` signature to accept the new state.
  - Deliverables:
    - Updated `AppState` with `ingest_config_admin` field
    - Updated `router()` wiring
    - Updated `router_with()` signature and route registration
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo build -p backend` compiles; `cargo test -p backend` passes.

### Phase 3: BDD scenarios

Goal: Add Gherkin scenarios for the ingest config CRUD lifecycle.

- [x] **Task 3.1** — Write BDD steps and scenarios for ingest config admin
  - What: Add BDD scenarios to `backend/tests/bdd.rs`: (1) full CRUD lifecycle: create a section "sport", add a scraper source, read the config tree, verify the section and source appear, update the schedule, verify the schedule is updated; (2) delete a section and verify it and its sources are gone; (3) create an API source and verify it is returned with `enabled=false` and `coming_soon=true`. Each scenario uses the `ChatWorld` pattern, extended with ingest config endpoints via `reqwest` calls.
  - Deliverables:
    - BDD scenarios for CRUD lifecycle, delete cascade, api-source-invariant
    - Wired step definitions reusing existing `ChatWorld` infrastructure
  - Skills to load: spontini-tdd-rust, spontini-bdd-gherkin
  - Verification: `cargo test -p backend --test bdd -- --nocapture` passes with new scenarios green.

## Acceptance Criteria

- `GET /admin/api/ingest/config` returns a JSON object with `schedule` (nullable) and `sections` (array), each section containing its `sources` array.
- `PUT /admin/api/ingest/config/schedule` with `{ cron_expr, enabled }` upserts the schedule and returns the saved schedule.
- `POST /admin/api/ingest/config/sections` with `{ name, ordering }` creates a section and returns it.
- `DELETE /admin/api/ingest/config/sections/:id` deletes the section and its sources; returns 200 with `{ deleted: true }` or 404 if not found.
- `POST /admin/api/ingest/config/sources?section_id=N` with `{ source_type, url, enabled }` creates a source in the section and returns it.
- `DELETE /admin/api/ingest/config/sources/:id` deletes the source; returns 200 with `{ deleted: true }` or 404 if not found.
- API source type is always returned with `enabled=false` and `coming_soon=true`, regardless of the `enabled` value in the request body.
- All endpoints return 401 when `X-Admin-Key` header is missing or wrong.
- All existing tests in the workspace (`cargo test --workspace`) remain green.
- BDD scenarios cover the CRUD lifecycle, delete cascade, and api-source-invariant.

## Risks

- **Section immutability** — Sections cannot be renamed or reordered after creation (no PUT for section fields beyond initial values). Mitigation: this is a deliberate simplicity choice; if reordering is needed, a future feature can add it. The delete + recreate pattern works for now.
- **Source update semantics** — `update_source` uses `upsert_source` which is actually an INSERT, not an UPDATE. Mitigation: the kb-store method is named `upsert_source` but currently only does INSERT. If UPDATE is needed, kb-store will need a `update_source` method — add it to kb-store in Task 1.2 if the test reveals the gap.
- **No transactional config updates** — Updating schedule + sections + sources requires multiple API calls; a partial failure leaves the config in an inconsistent state. Mitigation: acceptable for a single-operator system; the operator can manually fix inconsistencies. A future feature can add batch updates.

## Out-of-Scope

- No admin-ui SPA changes.
- No changes to `kb-store` schema.
- No changes to the `ingest` service scheduler.
- No cron expression validation.
- No source URL validation or reachability checks.
- No bulk operations.
- No section rename/reorder after creation.
