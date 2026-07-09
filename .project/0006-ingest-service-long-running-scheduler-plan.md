# Plan 0006: ingest service — long-running scheduler

- **Status**: closed
- **Closed**: 2026-07-09 by Sisyphus (opencode)
- **Review verdict**: approved
- **Approved**: 2026-07-09 by Sisyphus
- **Branch**: feat/ingest-service-long-running-scheduler
- **Feature ID**: 0006
- **Created**: 2026-07-09
- **Owner**: Sisyphus

## Objective

Transform the `ingest` binary from a walking-skeleton heartbeat into the always-on service that populates the knowledge base from configured URL sources on a schedule. The service reads its configuration from `kb.db` (schedule, sections, sources), runs a tokio-based scheduler that invokes `ingest_core::Pipeline` for every enabled scrape source on every cron tick, polls `kb.db` for configuration changes without restart, consumes on-demand run requests written by the admin surface, and shuts down cleanly on `SIGTERM`/`SIGINT`. This delivers the automated ingest runtime that, together with `ingest-core` (feature 0005), makes the "always-on ingest" promise from STACK.md §3.3 real: the knowledge base is no longer populated by tests alone.

## Non-Goals

- No admin-ui wiring (deferred to features 0009/0010/0011).
- No `api-client` adapter scheduling (not wired until future work enables the source type).
- No `ingest-cli` changes.
- No Dockerfile or docker-compose changes — the existing deployment already names this container `ingest`.
- No health endpoint or readiness probe — the container's health is the process's liveness.

## Phases

### Phase 1: Crate dependencies and module skeleton

Goal: Add the required dependencies (cron expression parsing, `kb-store` path dependency, wiremock) and define the module structure.

- [x] **Task 1.1** — Add dependencies to `ingest/Cargo.toml`
  - What: Add `cron` (cron expression parsing — the well-known `cron` crate, or `croner`), add `kb-store` with path dependency (already implicitly available via `ingest-core`, add explicitly), ensure `tokio` has the `time` feature, and add `wiremock` and `tempfile` as dev-dependencies.
  - Deliverables:
    - Updated `ingest/Cargo.toml` with all dependencies
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo check -p ingest` passes.

- [x] **Task 1.2** — Define module structure
  - What: Create `config` (configuration loading), `scheduler` (cron loop), and `runner` (pipeline invocation) stubbed modules. Declare them in `main.rs` and keep the walking-skeleton heartbeat so the binary still compiles and runs as a no-op.
  - Deliverables:
    - `ingest/src/config.rs` (module stub)
    - `ingest/src/scheduler.rs` (module stub)
    - `ingest/src/runner.rs` (module stub)
    - Updated `ingest/src/main.rs` with module declarations
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo check -p ingest` passes.

### Phase 2: Configuration loading

Goal: Load the active schedule, sections, and per-section enabled scrape sources from `kb.db` on startup and detect changes at runtime.

- [x] **Task 2.1** — Implement `IngestConfig` domain type and `ConfigLoader`
  - What: Define an `IngestConfig` struct with fields for `schedule` (parsed cron expression string + enabled flag), `sections` (list of section names in ordering), and `sources` (list of `(section_name, url)` tuples for enabled scrape sources). Implement `ConfigLoader::load(kb: &KbStore) -> Result<IngestConfig>` that calls `get_schedule`, `list_sections`, and for each section calls `list_sources_by_section`, filtering for `source_type == Scrape` and `enabled == true`. Unit test with a temp `kb.db` seeded with known config rows.
  - Deliverables:
    - `IngestConfig` struct and `ConfigLoader` in `config.rs`
    - Unit tests with seeded `kb.db` covering: no schedule, schedule disabled, schedule enabled with one section/source, multiple sections
  - Skills to load: spontini-tdd-rust, spontini-ingest-flow
  - Verification: `cargo test -p ingest` passes with config tests.

- [x] **Task 2.2** — Implement `ConfigWatcher` with change notification
  - What: Implement `ConfigWatcher` that wraps `ConfigLoader` and polls `load` every N seconds (default 30, configurable via env `CONFIG_POLL_SECS`). When the returned config differs from the previous one (by deep comparison), send the new config on a `tokio::sync::watch` channel.
  - Deliverables:
    - `ConfigWatcher` struct with `run` / `subscribe` methods
    - `watch::Receiver<IngestConfig>` channel for change notification
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p ingest` passes with watcher tests.

### Phase 3: Cron scheduler

Goal: Run the pipeline on the schedule defined by the cron expression, re-reading config on change.

- [x] **Task 3.1** — Implement `CronScheduler` loop
  - What: Create a `CronScheduler` that takes a `Box<dyn Pipeline>`, a `watch::Receiver<IngestConfig>`, and a shutdown `CancellationToken`. In a `tokio::select!` loop: (1) on config change, parse the new schedule's cron expression and compute next tick; (2) on cron tick, invoke `Pipeline::run` for every `(section, url)` pair in the current config; (3) on shutdown signal, break and return. Use the `cron` crate's `Schedule` to compute next occurrence.
  - Deliverables:
    - `CronScheduler` struct in `scheduler.rs`
    - `tokio::select!` loop with config change, cron tick, and shutdown arms
    - Unit tests: cron parsing, next-tick computation
  - Skills to load: spontini-tdd-rust, spontini-ingest-flow
  - Verification: `cargo test -p ingest` passes with scheduler tests.

### Phase 4: Run request consumption

Goal: Consume the `ingest_run_request` flag to trigger immediate out-of-schedule runs.

- [x] **Task 4.1** — Integrate run request polling into the scheduler
  - What: Add a periodic poll (every N seconds, configurable via `RUN_REQUEST_POLL_SECS`) arm to the scheduler's `tokio::select!` loop that calls `KbStore::consume_run_request`. When a `Pending` request is found, run the pipeline for all enabled sources immediately.
  - Deliverables:
    - Run request polling integrated into the `CronScheduler` loop
  - Skills to load: spontini-tdd-rust, spontini-ingest-flow
  - Verification: `cargo test -p ingest` passes with run-request tests.

### Phase 5: Graceful shutdown and main wiring

Goal: Wire everything together in `main.rs` with clean startup and shutdown.

- [x] **Task 5.1** — Wire the full service in `main.rs`
  - What: In `main.rs`, on startup: (1) open `KbStore` at the configured path (env `KB_PATH` or default `/data/kb.db`); (2) create `ConfigWatcher` and start it; (3) create `IngestPipeline` (from `ingest-core`) with configurable user-agent, embedder URL, chunk size/overlap; (4) create `CronScheduler` with the pipeline and watcher channel; (5) wait for `SIGTERM`/`SIGINT` via `tokio::signal`; (6) trigger the shutdown `CancellationToken` and wait for graceful completion.
  - Deliverables:
    - Updated `main.rs` with full startup sequence
    - Integration test for the end-to-end startup → run → document-written → shutdown flow
  - Skills to load: spontini-tdd-rust, spontini-ingest-flow
  - Verification: `cargo test -p ingest` passes; `cargo run -p ingest` starts and shuts down cleanly on Ctrl+C.

### Phase 6: Integration test

Goal: Write an integration test that exercises the full pipeline runner end-to-end against wiremock servers.

- [x] **Task 6.1** — Write integration test with full pipeline runner flow
  - What: Add an integration test in `runner.rs` that sets up `wiremock` for the source URL and `llama-embed`, seeds config into a temp `kb.db`, creates the pipeline via `create_pipeline`, wraps it in `PipelineRunner`, runs with a configured source, and verifies the document exists after the run.
  - Deliverables:
    - Integration test in `runner.rs` with full pipeline runner flow
  - Skills to load: spontini-ingest-flow, spontini-tdd-rust
  - Verification: `cargo test -p ingest` passes with integration test green.

## Acceptance Criteria

- The `ingest` binary starts, reads config from `kb.db`, runs `IngestPipeline::run` on the cron schedule, and writes documents to `kb.db`.
- Configuration changes (schedule, sections, sources) are picked up without restart within the poll interval.
- An `ingest_run_request` flag row triggers an immediate out-of-schedule run; the request transitions to `done` status.
- `SIGTERM`/`SIGINT` shuts down gracefully (the scheduler loop exits, in-flight runs complete or are cancelled within 30s).
- BDD scenario "a scheduled run writes a document to kb.db" passes with `wiremock` and a temp `kb-store`.
- All unit tests pass; `cargo clippy -p ingest -- -D warnings` is clean; `cargo fmt --check` is clean.
- `cargo test --workspace` is clean (no regressions in `backend`, `ingest-core`, `kb-store`).

## Risks

- **Cron expression parsing** — the `cron` crate's API may differ from the stored format. Mitigation: pin the version and add a unit test that round-trips the expected cron format from `kb-store`'s `ingest_schedule` table.
- **Race between config poll and cron tick** — if config changes mid-tick, the scheduler could use stale config for the current run. Mitigation: the scheduler re-reads config at the top of its loop *before* computing the next tick; in-flight runs always use the config that was current when they started. This is acceptable — consistency within a run, eventually consistent across runs.
- **Run request consumed but pipeline partially fails** — some source URLs fail while others succeed. Mitigation: run each source independently (not batched), log failures per source, and mark the run request as `done` regardless. A future refinement could add a `failed` status and per-source retry.
- **`CancellationToken` API stability** — `tokio::util::sync::CancellationToken` is stable since tokio 1.x. If removed, fall back to a `watch` channel.

## Out-of-Scope

- No admin-ui or backend API integration (features 0009/0010/0011).
- No `api-client` adapter scheduling.
- No Dockerfile or docker-compose changes.
- No `ingest-cli` changes.
- No persistent run history beyond the `ingest_run_request` table.
- No retry logic or failure alerts.
