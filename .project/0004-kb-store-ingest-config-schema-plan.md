# Plan 0004: kb-store ingest configuration schema

- **Status**: closed
- **Approved**: 2026-07-09 by Sisyphus (opencode)
- **Implemented**: 2026-07-09 by Sisyphus (opencode)
- **Closed**: 2026-07-09 by Sisyphus (opencode)
- **Review verdict**: approved
- **Branch**: feat/kb-store-ingest-config-schema
- **Feature ID**: 0004
- **Created**: 2026-07-09
- **Owner**: Sisyphus (opencode)

## Objective

Extend `kb-store` with the configuration tables that drive the always-on ingest service. After this plan, the ingest service (feature 0006) and the admin surface (features 0010, 0011) can read and write ingest configuration through the same `KbStore` library that already owns `documents` and `persona`. The new tables are: `ingest_schedule` (cron expression, enabled flag — a singleton row), `ingest_section` (name, ordering), `ingest_source` (section_id, source_type, url, enabled), and `ingest_run_request` (a flag-row table consumed by the ingest service to trigger out-of-schedule runs). A `V2__ingest_config.sql` migration creates all four tables idempotently inside the existing migration runner. `KbStore` gains eight public CRUD methods and one module re-exporting the new domain types. The `api` source type is stored but never wired to a real adapter — it is reserved for future use per STACK.md §3.6. Unit tests exercise every method against an in-memory libSQL database; no `backend` or `ingest` wiring in this plan.

## Non-Goals

- No wiring into `backend` or `ingest` — consumers of these methods are in separate features (0010, 0011, 0006).
- No `ingest-core` changes — the pipeline crate remains untouched.
- No scheduler logic — this plan provides the data layer; the cron-driven scheduler is in feature 0006.
- No HTTP surface — admin endpoints are in feature 0010; this plan is a pure library change.
- No Docker Compose or Dockerfile changes — `kb-store` is a library, not a container.
- No domain type extraction — the new types live in `kb-store/src/types.rs` alongside `Document` and `Persona`, following the pattern from plan 0002. A separate Domain crate may be extracted later.

## Phases

### Phase 1: V2 migration — ingest configuration tables

Goal: add the four new tables to `kb-store` via the existing migration runner, proving idempotency.

- [x] **Task 1.1** — Write the `V2__ingest_config.sql` migration file
  - What: Create `kb-store/src/migrations/V2__ingest_config.sql` with four `CREATE TABLE IF NOT EXISTS` statements and one index:
    - `ingest_schedule`: `id INTEGER PRIMARY KEY DEFAULT 1`, `cron_expr TEXT NOT NULL DEFAULT '0 */6 * * *'`, `enabled BOOLEAN NOT NULL DEFAULT 0`, `updated_at TEXT NOT NULL DEFAULT (datetime('now'))`. Constraint: at most one row (enforced at the application layer, not via CHECK).
    - `ingest_section`: `id INTEGER PRIMARY KEY`, `name TEXT NOT NULL UNIQUE`, `ordering INTEGER NOT NULL DEFAULT 0`, `created_at TEXT NOT NULL DEFAULT (datetime('now'))`.
    - `ingest_source`: `id INTEGER PRIMARY KEY`, `section_id INTEGER NOT NULL REFERENCES ingest_section(id) ON DELETE CASCADE`, `source_type TEXT NOT NULL CHECK(source_type IN ('scrape','api'))`, `url TEXT NOT NULL`, `enabled BOOLEAN NOT NULL DEFAULT 1`, `created_at TEXT NOT NULL DEFAULT (datetime('now'))`. `api` rows are stored but never wired to an adapter (reserved for future use).
    - `ingest_run_request`: `id INTEGER PRIMARY KEY`, `requested_at TEXT NOT NULL DEFAULT (datetime('now'))`, `status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','running','done','failed'))`.
    - Index: `CREATE INDEX IF NOT EXISTS idx_source_section ON ingest_source(section_id)`.
  - Deliverables:
    - `kb-store/src/migrations/V2__ingest_config.sql`
  - Skills to load: spontini-tdd-rust
  - Verification: `cat kb-store/src/migrations/V2__ingest_config.sql` — the file exists and contains all four tables plus the index.

- [x] **Task 1.2** — Wire `V2__ingest_config.sql` into the migration runner
  - What: Update `kb-store/src/migrations/mod.rs`:
    1. Add `const V2_SCHEMA: &str = include_str!("V2__ingest_config.sql");`
    2. Append a new block after the V1 block: check `_migrations` for `version = 2`; if absent, execute `V2_SCHEMA` inside a transaction and record `(2, 'ingest_config_schema')`.
    3. Update the existing V1 block to NOT include the V2 migration — the runner remains linear.
  - Deliverables:
    - `kb-store/src/migrations/mod.rs` — V2 block added
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo test -p kb-store should_create_tables_when_migrations_run` — the existing test still passes; additionally, run a new test (Task 1.3) that asserts all four new tables exist after `run_migrations`.

- [x] **Task 1.3** — Write migration test for the four new tables
  - What: Add to `kb-store/src/migrations/mod.rs` (or a dedicated `tests/` file) a test `should_create_ingest_config_tables_when_migrations_run` that:
    1. Opens an in-memory `:memory:` database.
    2. Calls `run_migrations(&conn)`.
    3. Queries `sqlite_master` for each table name (`ingest_schedule`, `ingest_section`, `ingest_source`, `ingest_run_request`) and asserts all four exist.
    4. Runs the migration a second time and asserts no error (idempotency).
  - Deliverables:
    - `kb-store/src/migrations/mod.rs` — new test in `#[cfg(test)] mod tests`
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store should_create_ingest_config_tables_when_migrations_run -- --nocapture` — test compiles and passes.

### Phase 2: Domain types for ingest configuration

Goal: define the types that represent ingest config rows, following the pattern of `Document` and `Persona` in `kb-store/src/types.rs`.

- [x] **Task 2.1** — Define `IngestSchedule`, `NewIngestSchedule`, `IngestSection`, `NewIngestSection`, `IngestSource`, `NewIngestSource`, `IngestRunRequest`, `RunRequestStatus`
  - What: Add to `kb-store/src/types.rs`:
    - `#[derive(Debug, Clone, PartialEq)] pub struct IngestSchedule { pub cron_expr: String, pub enabled: bool, pub updated_at: String }` — a single-row singleton, no `id` field (always row 1).
    - `#[derive(Debug, Clone)] pub struct NewIngestSchedule { pub cron_expr: String, pub enabled: bool }` — for `upsert_schedule`.
    - `#[derive(Debug, Clone, PartialEq)] pub struct IngestSection { pub id: i64, pub name: String, pub ordering: i32, pub created_at: String }`.
    - `#[derive(Debug, Clone)] pub struct NewIngestSection { pub name: String, pub ordering: i32 }`.
    - `#[derive(Debug, Clone, PartialEq)] pub struct IngestSource { pub id: i64, pub section_id: i64, pub source_type: SourceType, pub url: String, pub enabled: bool, pub created_at: String }`.
    - `#[derive(Debug, Clone)] pub struct NewIngestSource { pub section_id: i64, pub source_type: SourceType, pub url: String, pub enabled: bool }`.
    - `#[derive(Debug, Clone, PartialEq)] pub enum SourceType { Scrape, Api }` with `Display` and `FromStr` impls (following the `DocumentSource` pattern).
    - `#[derive(Debug, Clone, PartialEq)] pub enum RunRequestStatus { Pending, Running, Done, Failed }` with `Display`/`FromStr` and a `to_string()` mapping.
    - `#[derive(Debug, Clone, PartialEq)] pub struct IngestRunRequest { pub id: i64, pub requested_at: String, pub status: RunRequestStatus }`.
  - Re-export all new types from `kb-store/src/lib.rs` (`pub use types::{IngestSchedule, ...}`).
  - Deliverables:
    - `kb-store/src/types.rs` — new types appended
    - `kb-store/src/lib.rs` — updated `pub use types::{...}` line
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo test -p kb-store` — all existing tests still pass; the new types compile.

- [x] **Task 2.2** — Unit tests for `SourceType` and `RunRequestStatus` round-trip
  - What: Add unit tests in `types.rs` (`#[cfg(test)] mod tests`) asserting:
    - `SourceType::Scrape.to_string() == "scrape"`, `SourceType::Api.to_string() == "api"`, round-trip `FromStr` for both.
    - `RunRequestStatus::Pending.to_string() == "pending"`, and round-trip `FromStr` for all four variants.
  - Deliverables:
    - `kb-store/src/types.rs` — new tests in `#[cfg(test)] mod tests`
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store should_round_trip_source_type` and `should_round_trip_run_request_status` — both pass.

### Phase 3: Schedule + section + source CRUD

Goal: implement the six CRUD methods on `KbStore` for `ingest_schedule`, `ingest_section`, and `ingest_source`, following the existing pattern of opening a connection per call.

- [x] **Task 3.1** — Implement `get_schedule` and `upsert_schedule`
  - What: Add to `KbStore` in `kb-store/src/lib.rs`:
    - `pub async fn get_schedule(&self) -> Result<Option<IngestSchedule>>` — `SELECT cron_expr, enabled, updated_at FROM ingest_schedule WHERE id = 1`. Returns `None` if no row.
    - `pub async fn upsert_schedule(&self, schedule: NewIngestSchedule) -> Result<IngestSchedule>` — INSERT OR REPLACE `ingest_schedule (id, cron_expr, enabled) VALUES (1, ?1, ?2)` (singleton row). Returns the inserted schedule with the current `updated_at`.
  - Deliverables:
    - `kb-store/src/lib.rs` — two new methods on `KbStore`
    - Unit test `should_return_none_when_no_schedule` — `get_schedule` on a fresh DB returns `None`.
    - Unit test `should_upsert_schedule_and_return_it` — upsert a schedule, then `get_schedule` returns the same cron and enabled flag.
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store should_return_none_when_no_schedule` and `should_upsert_schedule_and_return_it` — both pass.

- [x] **Task 3.2** — Implement `list_sections`, `upsert_section`, `delete_section`
  - What: Add to `KbStore`:
    - `pub async fn list_sections(&self) -> Result<Vec<IngestSection>>` — `SELECT id, name, ordering, created_at FROM ingest_section ORDER BY ordering ASC, id ASC`.
    - `pub async fn upsert_section(&self, section: NewIngestSection) -> Result<IngestSection>` — `INSERT INTO ingest_section (name, ordering) VALUES (?1, ?2)` (libSQL auto-generates `id`). Returns the inserted section with its `id`.
    - `pub async fn delete_section(&self, id: i64) -> Result<bool>` — `DELETE FROM ingest_section WHERE id = ?1`. Returns `true` if a row was deleted. CASCADE deletes the section's sources automatically (per the foreign key constraint).
  - Deliverables:
    - `kb-store/src/lib.rs` — three new methods
    - Unit test `should_list_sections_in_ordering_asc` — insert two sections (ordering 10, 20), assert list returns them in order.
    - Unit test `should_delete_section_and_cascade_delete_sources` — insert a section, insert a source under it, delete the section, assert the source is gone.
    - Unit test `should_return_false_when_deleting_missing_section`.
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store list_sections` and `upsert_section` and `delete_section` — all pass.

- [x] **Task 3.3** — Implement `list_sources_by_section`, `upsert_source`, `delete_source`
  - What: Add to `KbStore`:
    - `pub async fn list_sources_by_section(&self, section_id: i64) -> Result<Vec<IngestSource>>` — `SELECT id, section_id, source_type, url, enabled, created_at FROM ingest_source WHERE section_id = ?1 ORDER BY id ASC`.
    - `pub async fn upsert_source(&self, source: NewIngestSource) -> Result<IngestSource>` — validates `source_type` is one of the allowed values, then `INSERT INTO ingest_source (section_id, source_type, url, enabled) VALUES (?1, ?2, ?3, ?4)`.
    - `pub async fn delete_source(&self, id: i64) -> Result<bool>` — `DELETE FROM ingest_source WHERE id = ?1`.
  - Deliverables:
    - `kb-store/src/lib.rs` — three new methods
    - Unit test `should_list_sources_for_section` — insert two sources under the same section, assert list returns both.
    - Unit test `should_insert_scrape_and_api_source_types` — insert a scrape and an api source, assert both are returned with the correct `SourceType`.
    - Unit test `should_return_false_when_deleting_missing_source`.
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store list_sources_by_section` and `upsert_source` and `delete_source` — all pass.

### Phase 4: Run request CRUD + workspace verification gate

Goal: implement the run-request methods and run the full verification gate.

- [x] **Task 4.1** — Implement `request_run` and `consume_run_request`
  - What: Add to `KbStore`:
    - `pub async fn request_run(&self) -> Result<IngestRunRequest>` — `INSERT INTO ingest_run_request DEFAULT VALUES` (auto-sets `requested_at` and `status = 'pending'`). Returns the new row with its `id`.
    - `pub async fn consume_run_request(&self) -> Result<Option<IngestRunRequest>>` — in a transaction: `SELECT id, requested_at, status FROM ingest_run_request WHERE status = 'pending' ORDER BY id ASC LIMIT 1`. If a row exists, `UPDATE ingest_run_request SET status = 'running' WHERE id = ?1`. Return the row (now in `Running` state). If no pending row, return `None`. The caller (the ingest service) is responsible for later updating the status to `done` or `failed` (a simple UPDATE; expose a `complete_run(id, status)` method as well).
    - `pub async fn complete_run(&self, id: i64, status: RunRequestStatus) -> Result<()>` — `UPDATE ingest_run_request SET status = ?2 WHERE id = ?1`. Returns `NotFound` if `id` does not exist. Called by the ingest service after a run finishes.
  - Deliverables:
    - `kb-store/src/lib.rs` — three new methods
    - Unit test `should_request_run_and_return_pending` — call `request_run`, assert `status == RunRequestStatus::Pending` and `id > 0`.
    - Unit test `should_consume_first_pending_run` — insert two run requests, call `consume_run_request`, assert the returned `id` matches the first and `status == Running`.
    - Unit test `should_return_none_when_no_pending_run` — `consume_run_request` on a fresh DB returns `None`.
    - Unit test `should_complete_run_with_done_status` — consume a run, call `complete_run(id, Done)`, then re-query and assert `status == Done`.
    - Unit test `should_return_error_when_completing_missing_run`.
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p kb-store request_run` and `consume_run_request` and `complete_run` — all pass.

- [x] **Task 4.2** — Run the full verification gate on `kb-store`
  - What: Run, in order, capturing the output of each:
    ```bash
    cargo test -p kb-store -- --nocapture
    cargo clippy -p kb-store -- -D warnings
    cargo fmt -p kb-store -- --check
    cargo test --workspace --all-targets
    cargo clippy --workspace --all-targets -- -D warnings
    ```
    No task is blocked on other crates' pre-existing failures. Fix all warnings/errors in `kb-store`. Do NOT fix warnings in other crates.
  - Deliverables:
    - (No file changes — verification only. Create `coverage-exclusions.txt` only if a justified exclusion is needed, e.g., the `main.rs` composition root of `kb-store` if one existed.)
  - Skills to load: spontini-verify-gate
  - Verification: All commands above exit 0. Output captured and reported.

## Acceptance Criteria

- `cargo test -p kb-store` passes all existing tests plus the new tests for `IngestSchedule`, `IngestSection`, `IngestSource`, `IngestRunRequest`, `SourceType`, and `RunRequestStatus` CRUD.
- `cargo test --workspace` passes with no regression in `backend`, `ingest-core`, `ingest-cli`, or `ingest`.
- `cargo clippy -p kb-store -- -D warnings` is clean; `cargo fmt --check` is clean.
- `KbStore::get_schedule()` returns `None` on a fresh DB and the upserted schedule after `upsert_schedule()`.
- `KbStore::list_sections()` returns sections ordered by `ordering ASC, id ASC`.
- Deleting a section cascades to its sources (the foreign key constraint is enforced by libSQL).
- `KbStore::request_run()` returns a row with `status = Pending`; `consume_run_request()` atomically marks the first pending row as `Running` and returns it; calling `consume_run_request()` again on an empty queue returns `None`.
- `KbStore::complete_run(id, Done)` updates the status and returns `Ok(())`; calling it with a nonexistent `id` returns `Err(KbStoreError::NotFound(...))`.
- All four `CREATE TABLE IF NOT EXISTS` statements in `V2__ingest_config.sql` are idempotent — running `run_migrations` twice on the same DB produces no errors and no duplicate tables.
- The `api` source type can be inserted and retrieved, but no adapter or scheduler in this plan consumes it — it is explicitly reserved for future use.
- `cargo doc -p kb-store --no-deps` succeeds without warnings; the new public types appear in the generated docs.

## Risks

- **`ON DELETE CASCADE` support in libSQL** — The `ingest_source` foreign key uses `ON DELETE CASCADE` so deleting a section removes its sources atomically. libSQL inherits SQLite's FK enforcement. If the in-memory test database does not enforce FKs (SQLite requires `PRAGMA foreign_keys = ON`), the cascade test may not trigger the cascade. Mitigation: enable `PRAGMA foreign_keys = ON` explicitly in the test setup via `conn.execute_batch("PRAGMA foreign_keys = ON")`. Verify by checking the source count after section deletion.
- **Singleton schedule row** — `ingest_schedule` enforces at most one row via the `id = 1` constraint, not via a database-level CHECK. If two `upsert_schedule` calls race, the second wins (INSERT OR REPLACE is atomic). This is acceptable for a single-operator system (STACK.md §3.3 — one ingest service, one operator). No mutex or lock is needed at the library layer; the caller handles serialization if needed.
- **Migration version number gap** — If future features add `V3`, `V4` etc. migrations, the runner must apply them in order. The linear pattern (check version N, apply if absent) is simple and correct but does not handle out-of-order application. This is acceptable because the ingest config tables (V2) are always applied after V1 and before any later migrations.
- **No `tokio` dependency in `kb-store`** — All methods are `async fn` returning futures; the caller provides the tokio runtime. The `run_migrations` function opens its own connection via the `conn` parameter (not `self.db`), so there is no runtime dependency leak. The new methods follow the same pattern.
- **`SourceType::Api` reserved but not wired** — The roadmap description explicitly reserves the `api` source type. The migration stores it, the CRUD methods write and read it, but no adapter consumes it. This is intentional and matches STACK.md §3.6 ("The `api-client` adapter exists in the crate for future use but is not enabled").

## Out-of-Scope

- `ingest-core` changes (scraper adapter, chunking, embedding pipeline) — feature 0005.
- `ingest` service scheduler — feature 0006.
- `ingest-cli` one-shot runs — feature 0007.
- `backend` admin endpoints for ingest config — feature 0010.
- `backend` admin endpoint for triggering an ingest run — feature 0011.
- Any change to `backend`, `ingest`, `ingest-core`, `ingest-cli`, `frontend`, `admin-ui`, `docker-compose.yml`, or the Dockerfiles.
- A separate Domain crate — domain types live in `kb-store/src/types.rs` for now.
