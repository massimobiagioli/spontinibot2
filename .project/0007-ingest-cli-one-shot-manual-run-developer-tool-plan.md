# Plan 0007: ingest-cli — one-shot manual run developer tool

- **Status**: closed
- **Approved**: 2026-07-09 by Sisyphus
- **Implemented**: 2026-07-09 by Sisyphus
- **Closed**: 2026-07-09 by Sisyphus
- **Review verdict**: approved
- **Branch**: feat/ingest-cli-one-shot-manual-run-developer-tool
- **Feature ID**: 0007
- **Created**: 2026-07-09
- **Owner**: Sisyphus

## Objective

Upgrade `ingest-cli` from a help-line skeleton into a thin one-shot developer tool over `ingest-core`. The CLI supports two modes: (1) `ingest-cli run --url <URL> --section <name>` which scrapes, chunks, embeds, and inserts a single URL into `kb.db`; and (2) `ingest-cli run --section <name> --all-sources` which reads the section's configured scrape sources from `kb.db` and runs them all once. This is a developer convenience tool, not a production container — no scheduling, no daemon, no Dockerfile changes. Per the [Constitution](../docs/CONSTITUTION.md) §4 (Locality, Not Cloud), the developer experience toolchain stays local and minimal. Unit tests for argument parsing and an integration test against a `wiremock` source URL validate correctness.

## Non-Goals

- No scheduling or daemon mode (feature 0006 already covers the always-on scheduler).
- No Dockerfile or docker-compose changes — this is a developer-only binary, not a production container.
- No `admin-ui` or backend API integration.
- No `api-client` adapter wiring.
- No changes to `ingest-core` or `kb-store` — the CLI consumes their existing APIs only.

## Phases

### Phase 1: Crate dependencies and CLI skeleton

Goal: Add clap argument parsing and kb-store path dependency to ingest-cli, then define the CLI command structure.

- [x] **Task 1.1** — Add dependencies to `ingest-cli/Cargo.toml`
  - What: Add `clap` with `derive` feature for argument parsing, add `kb-store` as a path dependency (needed for `--all-sources` mode to read sections/sources from `kb.db`), add `tokio` with `rt-multi-thread` and `macros` features for the async runtime, and add `wiremock` + `tempfile` as dev-dependencies for integration testing.
  - Deliverables:
    - Updated `ingest-cli/Cargo.toml` with all dependencies
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo check -p ingest-cli` passes.

- [x] **Task 1.2** — Define CLI command structure with clap
  - What: Define a `Cli` struct with `#[derive(Parser)]` supporting `run` as a subcommand. The `run` subcommand has two mutually exclusive modes: `--url <URL> --section <name>` (single URL scrape) and `--section <name> --all-sources` (read section sources from kb.db). Use clap's `conflicts_with` for mutual exclusion. Write unit tests for argument parsing covering: valid `--url` + `--section`, valid `--all-sources` + `--section`, missing required args, mutually exclusive args together.
  - Deliverables:
    - `ingest-cli/src/cli.rs` module with `Cli` and `RunArgs` structs
    - Updated `main.rs` to parse args and print help on no args
    - Unit tests for all argument combinations
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p ingest-cli` passes with arg parsing tests.

### Phase 2: `--url` mode — single URL scrape pipeline

Goal: Implement the `ingest-cli run --url <URL> --section <name>` code path that creates an `IngestPipeline`, runs it against the given URL, and writes the result to `kb.db`.

- [x] **Task 2.1** — Implement single-URL run command
  - What: In a new `run.rs` module, implement `async fn run_url(url: &str, section: &str, kb_path: &str, embedder_url: &str, user_agent: &str, chunk_size: usize, chunk_overlap: usize) -> Result<()>`. This function: (1) opens `KbStore` at `kb_path`; (2) creates `IngestPipeline` with the given configuration; (3) calls `pipeline.run(url, section).await`; (4) prints a success message with the URL and section. Add a helper that loads defaults from environment variables: `KB_PATH` (default `/data/kb.db`), `EMBEDDER_BASE_URL` (default `http://localhost:8081`), `CHUNK_SIZE` (default 512), `CHUNK_OVERLAP` (default 64), `USER_AGENT` (default `ingest-cli/0.1.0`).
  - Deliverables:
    - `ingest-cli/src/run.rs` module with `run_url` function
    - Environment-variable-based defaults helper in `run.rs` or a new `config.rs` module
  - Skills to load: spontini-tdd-rust, spontini-ingest-flow
  - Verification: `cargo build -p ingest-cli` compiles; unit tests cover the defaults helper.

- [x] **Task 2.2** — Wire `run_url` into the CLI `run` subcommand
  - What: In `main.rs`, match the `run` subcommand. If `--url` is provided, call `run_url` with the parsed args and env-based defaults. Print errors to stderr and exit with code 1 on failure.
  - Deliverables:
    - Updated `main.rs` wiring the `run` subcommand
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo run -p ingest-cli -- run --url http://localhost:9999/page --section test` fails gracefully (no server); `cargo run -p ingest-cli -- run --section test --all-sources` fails with appropriate message when no `--url`.

### Phase 3: `--all-sources` mode — run all configured sources for a section

Goal: Implement `ingest-cli run --section <name> --all-sources` that reads the section's configured scrape sources from `kb.db` and runs them all.

- [x] **Task 3.1** — Implement `run_all_sources` function
  - What: In `run.rs`, implement `async fn run_all_sources(section_name: &str, kb_path: &str, embedder_url: &str, user_agent: &str, chunk_size: usize, chunk_overlap: usize) -> Result<()>`. This function: (1) opens `KbStore` at `kb_path`; (2) calls `list_sections()` to find the section by name; (3) if not found, return an error; (4) calls `list_sources_by_section(section.id)`; (5) filters for `SourceType::Scrape` and `enabled == true`; (6) creates `IngestPipeline`; (7) iterates over filtered sources and calls `pipeline.run(url, section_name)` for each; (8) prints progress and per-source success/failure. Each source runs independently — a failure in one does not abort others.
  - Deliverables:
    - `run_all_sources` function in `run.rs`
    - Unit tests for: section not found error, empty sources case
  - Skills to load: spontini-tdd-rust, spontini-ingest-flow
  - Verification: `cargo test -p ingest-cli` passes.

- [x] **Task 3.2** — Wire `run_all_sources` into the CLI `run` subcommand
  - What: In `main.rs`, extend the `run` subcommand match: if `--all-sources` is set (without `--url`), call `run_all_sources`. Use env-based defaults as in task 2.1.
  - Deliverables:
    - Updated `main.rs` wiring the `--all-sources` path
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo build -p ingest-cli` compiles; `cargo run -p ingest-cli -- --help` shows both modes.

### Phase 4: Integration test against wiremock

Goal: Write a full end-to-end integration test that exercises both CLI modes against wiremock servers and a temp `kb.db`.

- [x] **Task 4.1** — Write integration test for `--url` mode
  - What: Add an integration test (gated with `#[cfg(test)]` in a tests module or a `tests/` integration directory) that: (1) starts wiremock servers for the source URL and the embedder; (2) creates a temp `kb.db`; (3) calls `run_url` pointing at the mock servers; (4) verifies the document was inserted by re-opening `kb.db` and querying `get_documents_by_source`. Follow the same pattern as the existing `ingest_core::pipeline::tests` module.
  - Deliverables:
    - Integration test for `--url` mode
  - Skills to load: spontini-tdd-rust, spontini-ingest-flow
  - Verification: `cargo test -p ingest-cli` passes with the integration test green.

- [x] **Task 4.2** — Write integration test for `--all-sources` mode
  - What: Add an integration test that: (1) starts wiremock servers for the source URL and embedder; (2) creates a temp `kb.db`; (3) inserts a section and two scrape sources (one enabled, one disabled) using `KbStore` directly; (4) calls `run_all_sources`; (5) verifies only the enabled source's document was stored. This tests the config-read path end-to-end.
  - Deliverables:
    - Integration test for `--all-sources` mode
  - Skills to load: spontini-tdd-rust, spontini-ingest-flow
  - Verification: `cargo test -p ingest-cli` passes with both integration tests green.

## Acceptance Criteria

- `ingest-cli run --url <URL> --section <name>` scrapes, chunks, embeds, and inserts the document into `kb.db`.
- `ingest-cli run --section <name> --all-sources` reads the section's enabled scrape sources from `kb.db` and runs them all, skipping disabled sources.
- Both modes use sensible environment-variable defaults (`KB_PATH`, `EMBEDDER_BASE_URL`, `CHUNK_SIZE`, `CHUNK_OVERLAP`, `USER_AGENT`).
- Missing `--all-sources` or `--url` results in a clear error message.
- Both `--url` and `--all-sources` together is rejected (mutually exclusive).
- Unit tests cover argument parsing, defaults, and error cases.
- Integration tests against wiremock cover both modes end-to-end.
- All existing tests in the workspace (`cargo test --workspace`) remain green.

## Risks

- **libSQL in CLI context** — `KbStore::open` creates a full libSQL database, which the CLI opens, uses briefly, and drops. Mitigation: test with `tempfile` to ensure clean close; the libSQL embedded library handles this correctly.
- **Pipeline environment mismatch** — the CLI runs outside Docker and may not reach `llama-embed` at `localhost:8081` if the stack is not up. Mitigation: document this in `--help` and in the error message when the embedder connection fails. The default `EMBEDDER_BASE_URL` is configurable.
- **Wiremock tests leaking temp db files** — Mitigation: use `std::fs::remove_file` in a drop guard or `Drop` implementation, matching the existing pattern in `ingest_core::pipeline::tests`.

## Out-of-Scope

- No scheduling or daemon mode.
- No Dockerfile or docker-compose changes — this is a developer-only binary.
- No `api-client` adapter wiring.
- No changes to `ingest-core` or `kb-store` internals.
- No admin-ui or backend API integration.
- No persistent run history or progress tracking beyond stdout.
