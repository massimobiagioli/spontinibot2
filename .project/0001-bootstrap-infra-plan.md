# Plan 0001: Bootstrap Infrastructure — Docker Services & Walking Skeletons

- **Status**: closed
- **Approved**: 2026-07-09 by Sisyphus (opencode)
- **Implemented**: 2026-07-09 by Sisyphus (opencode)
- **Closed**: 2026-07-09 by Sisyphus (opencode)
- **Review verdict**: approved
- **Branch**: feat/bootstrap-infra
- **Feature ID**: 0001
- **Created**: 2026-07-09
- **Owner**: Sisyphus (opencode)

## Objective

Bootstrap the project from a docs-only repository to a running, containerized walking skeleton that satisfies the architecture in [docs/STACK.md §2](../docs/STACK.md#2-architecture-overview). After this plan lands, `make build && make up` brings up **all six runtime containers** — `backend`, `admin-ui`, `ingest`, `frontend`, `llama-embed`, `llama-generate` — each implementing nothing beyond a health endpoint and/or an empty home page, sharing the `kb-data` volume as the only cross-flow coupling. This proves the Constitution's **Openness** principle (every component Dockerized, reproducible with a single `make` command) and unblocks all subsequent feature work by giving every crate and every container a compilable, runnable home.

**In scope:** Cargo workspace root + 5 Rust crates (`backend`, `ingest`, `ingest-cli`, `ingest-core`, `kb-store`) as minimal skeletons that compile and pass `cargo test`/`clippy`/`fmt`; `frontend` and `admin-ui` Vue 3 + Vite + TypeScript apps with an empty home page; `docker-compose.yml` wiring all 6 services + `kb-data` volume; Dockerfiles for the 4 application containers; nginx reverse-proxy config for `admin-ui`; a `provision-models` Makefile target that downloads the GGUF model files into `models/embed/` and `models/generate/`; BDD scenario for the backend health route; the existing `Makefile` targets (`build`, `up`, `down`, `logs`, `test`, `lint`, `fmt-check`, `verify`, `compose-config`) all working end-to-end inside containers.

**Out of scope:** any real RAG / retrieval / embedding / generation logic; the `rag-engine` module; ingest adapters (scraper, api-client); chunking; libSQL schema migrations beyond an empty `kb.db` file; the `persona` table; `/chat` answering behavior; admin-ui sections (ingest config, imprinting, training); training feedback; source citation UI; Design System Italia integration (a separate plan); CI pipeline wiring.

## Non-Goals

- No retrieval, embedding, or generation logic — `/chat` returns a stub response, `rag-engine` does not exist yet.
- No ingest adapter implementation — the `ingest` container starts, logs a heartbeat, and exits cleanly on `SIGTERM`; it does not read `kb.db`.
- No libSQL schema — `kb-store` is a compiling library with no DB calls; `kb.db` is an empty file on the `kb-data` volume.
- No DSI / Bootstrap Italia styling — the Vue apps render a single empty `<div>` home page. DSI integration is deferred to a dedicated plan.
- No CI badges wiring — README badges stay `pending`.
- No model fine-tuning, quantization, or alternative model selection — the two GGUF models from [docs/STACK.md §3.4](../docs/STACK.md#34-inference--llamacppllama-server) are downloaded verbatim.

## Phases

### Phase 1: Rust workspace foundation

Goal: establish the Cargo workspace root and the 5 Rust crates as compiling, clippy-clean, fmt-clean skeletons with the dependency matrix enforced by the clean-arch guard.

- [x] **Task 1.1** — Create workspace root and toolchain pin
  - What: Add `Cargo.toml` (workspace manifest listing the 5 member crates, `edition = "2024"`, shared profile) and `rust-toolchain.toml` pinning `channel = "1.96.1"` per [docs/STACK.md §1](../docs/STACK.md#1-language-and-runtime).
  - Deliverables:
    - `Cargo.toml` (workspace root, `[workspace] members = [...]`, `[workspace.package]` with `edition = "2024"`, `license = "MIT"`)
    - `rust-toolchain.toml` with `[toolchain] channel = "1.96.1"`
  - Skills to load: spontini-clean-arch-guard
  - Verification: `test -f Cargo.toml && test -f rust-toolchain.toml && grep -q '1.96.1' rust-toolchain.toml`; `rustup show active-toolchain` confirms `1.96.1`.

- [x] **Task 1.2** — Scaffold `kb-store` crate skeleton
  - What: Create the `kb-store/` crate as a pure library with a single placeholder module (`lib.rs` exporting nothing yet) — no `libsql` calls, no DB. It compiles and has one trivial unit test so the coverage gate has a baseline.
  - Deliverables:
    - `kb-store/Cargo.toml` (`name = "kb-store"`, `license = "MIT"`, `[lib]` default)
    - `kb-store/src/lib.rs` with one `pub fn version() -> &'static str { "kb-store 0.1.0" }` and a `#[cfg(test)]` unit test asserting it
  - Skills to load: spontini-clean-arch-guard, spontini-tdd-rust
  - Verification: `cargo test -p kb-store` passes; `cargo clippy -p kb-store -- -D warnings` is clean.

- [x] **Task 1.3** — Scaffold `ingest-core` crate skeleton
  - What: Create `ingest-core/` as a library crate with a placeholder module. No adapters, no chunking, no embedding calls. Depends on nothing but std (no `kb-store` dependency yet — wiring comes in a later plan).
  - Deliverables:
    - `ingest-core/Cargo.toml`
    - `ingest-core/src/lib.rs` with `pub fn version() -> &'static str { "ingest-core 0.1.0" }` + unit test
  - Skills to load: spontini-clean-arch-guard, spontini-tdd-rust
  - Verification: `cargo test -p ingest-core` passes; clippy clean.

- [x] **Task 1.4** — Scaffold `ingest-cli` one-shot binary skeleton
  - What: Create `ingest-cli/` as a binary crate that prints a help line and exits 0. It is a developer tool, not a production container. Depends on `ingest-core` (to validate the inward dependency edge in the matrix).
  - Deliverables:
    - `ingest-cli/Cargo.toml` (`name = "ingest-cli"`, `[[bin]]`)
    - `ingest-cli/src/main.rs` that prints `ingest-cli 0.1.0 — no adapters wired yet` and returns `Ok(())`
  - Skills to load: spontini-clean-arch-guard, spontini-tdd-rust
  - Verification: `cargo run -p ingest-cli` prints the help line and exits 0; clippy clean.

- [x] **Task 1.5** — Scaffold `ingest` long-running service binary skeleton
  - What: Create `ingest/` as a binary crate (the always-on container). Uses `tokio` runtime, installs a `ctrlc` handler, logs a startup heartbeat line every 60 s, and exits 0 on `SIGTERM`. No scheduler, no `kb.db` access. Depends on `ingest-core` (inward edge).
  - Deliverables:
    - `ingest/Cargo.toml` (`name = "ingest"`, `[[bin]]`, deps: `tokio` with `rt`, `macros`, `signal` features, `tracing`, `tracing-subscriber`)
    - `ingest/src/main.rs` — `#[tokio::main]` that inits `tracing_subscriber`, logs `ingest service started (walking skeleton)`, awaits `tokio::signal::ctrl_c()`, logs `ingest service stopping`, returns `Ok(())`
  - Skills to load: spontini-clean-arch-guard, spontini-tdd-rust
  - Verification: `cargo run -p ingest` logs the startup line and stays running until `Ctrl-C`, then exits 0; clippy clean.

- [x] **Task 1.6** — Workspace-wide build gate and Cargo.lock policy fix
  - What: After all 5 crate skeletons exist, run the aggregated workspace checks: `cargo check --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`. Also commit `Cargo.lock` — this is a binary workspace (3 of 5 crates produce binaries: `backend`, `ingest`, `ingest-cli`), so the lockfile must be tracked per the README policy. Remove `Cargo.lock` from `.gitignore`.
  - Deliverables:
    - `.gitignore` patch: delete the `Cargo.lock` line (line 9) and its surrounding comment (lines 10–12)
    - `Cargo.lock` file generated and committed
  - Skills to load: spontini-clean-arch-guard, spontini-tdd-rust
  - Verification: `cargo check --workspace` exits 0; `cargo fmt --all -- --check` exits 0; `cargo clippy --workspace --all-targets -- -D warnings` exits 0; `git status` shows `Cargo.lock` is tracked (not in `.gitignore`).

### Phase 2: Backend health route with BDD

Goal: deliver the `backend` axum container with a health endpoint, an empty home page, and a `/chat` stub, driven by a Gherkin scenario written first.

- [x] **Task 2.1** — Create `backend` crate skeleton and write the health BDD scenario (Red)
  - What: Scaffold the `backend/` crate with its `Cargo.toml`, a placeholder router (`pub fn router() -> axum::Router { todo!() }`), and a `main.rs` skeleton. Then write the `features/health.feature` BDD scenario using the `cucumber` crate (the Rust Gherkin runner per [PRINCIPLES.md §5](../docs/PRINCIPLES.md#5-bdd--behavior-driven-development)) and the step definitions in `backend/tests/bdd.rs` that compile and fail (Red phase). The scenario asserts `GET /health` returns `200` with `{"status":"ok"}`.
  - Deliverables:
    - `backend/Cargo.toml` (`name = "backend"`, `[[bin]]`; deps: `axum`, `tokio` with features `rt-multi-thread` + `macros`, `serde` with feature `derive`, `serde_json`, `tower`, `tracing`, `tracing-subscriber`; dev-deps: `cucumber` with feature `macros`, `async-trait`, `tower-http` with feature `trace`; `[[test]] name = "bdd" path = "tests/bdd.rs"`)
    - `backend/src/main.rs` — `#[tokio::main]` that binds `0.0.0.0:8080` and serves `router()` (will panic with `todo!()` until Task 2.2)
    - `backend/src/lib.rs` — `pub fn router() -> axum::Router { todo!() }`
    - `backend/src/routes.rs` — placeholder module with `todo!()` handlers
    - `features/health.feature` (one `Feature:` + one `Scenario:` with `Given`/`When`/`Then`, in English, following the Gherkin structure from PRINCIPLES.md §5)
    - `backend/tests/bdd.rs` — Cucumber step definitions wired via `#[given]`/`#[when]`/`#[then]` macros; the test builds the `router()` and uses `tower::ServiceExt::oneshot` to assert the response (no network); calls `cucumber::World` trait and a `main()` that invokes the runner
  - Skills to load: spontini-bdd-gherkin, spontini-clean-arch-guard, spontini-tdd-rust
  - Verification: `cargo test -p backend --test bdd` compiles successfully but fails (Red) — the step definitions compile against the `cucumber` macros and the `axum::Router` type, but `router()` returns `todo!()` so the scenario panics.

- [x] **Task 2.2** — Implement axum router with `/health`, `/`, `/chat` stub (Green)
  - What: Implement the actual handlers in `routes.rs` and wire them in `router()` inside `lib.rs`. Replace the `todo!()` bodies from Task 2.1: `GET /health` → `{"status":"ok"}`, `GET /` → empty `200` (home page placeholder), `POST /chat` → `{"answer":"(walking skeleton)","sources":[]}` stub. The step definitions in `tests/bdd.rs` (already written in Task 2.1) now pass because `router()` returns a real router.
  - Deliverables:
    - `backend/src/lib.rs` — replace `pub fn router() -> axum::Router { todo!() }` with the actual router building the three routes
    - `backend/src/routes.rs` — replace placeholders with `health()`, `home()`, `chat()` handler functions
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard, spontini-bdd-gherkin
  - Verification: `cargo test -p backend --test bdd` passes (Green — the cucumber scenario runs and the health step returns 200); `cargo test -p backend` passes (unit tests if any); `cargo clippy -p backend -- -D warnings` clean; `curl http://localhost:8080/health` returns `{"status":"ok"}` when run locally.

### Phase 3: Frontend and admin-ui walking skeletons

Goal: deliver the two Vue 3 + Vite + TypeScript apps as empty-home-page walking skeletons, each buildable and served by its own container.

- [x] **Task 3.1** — Scaffold `frontend` (public chat) empty home page
  - What: Create `frontend/` as a Vite Vue 3 + TS app with the strictest `tsconfig.json` per [docs/STACK.md §3.8 TypeScript strictness contract](../docs/STACK.md#38-frontend-public-chat--vue-3--vite--typescript). The app renders a single empty `<div id="app">` with a single `<h1>Spontini</h1>` placeholder. No DSI, no chat UI, no `/chat` call. Pin Node LTS in `.nvmrc` and `package.json` `engines.node`.
  - Deliverables:
    - `frontend/package.json` (Vue 3, Vite, TypeScript, `engines.node`, `lint`/`format`/`test` scripts)
    - `frontend/.nvmrc` (current LTS)
    - `frontend/tsconfig.json` (every strict flag from the STACK.md contract set to `true`)
    - `frontend/index.html`, `frontend/src/main.ts`, `frontend/src/App.vue` (renders `<h1>Spontini</h1>`)
    - `frontend/vite.config.ts` (dev server on port 5174, host `0.0.0.0`)
  - Skills to load: (none of the six project skills apply — frontend-only, no Rust/Docker/rag/ingest concern)
  - Verification: `npm ci && npm run build` inside `frontend/` produces `frontend/dist/`; `npm run lint` clean; `npm run test` passes (a trivial placeholder test).

- [x] **Task 3.2** — Scaffold `admin-ui` (operator console) empty home page
  - What: Create `admin-ui/` as a Vite Vue 3 + TS app, same strictness contract as `frontend`. Renders a single `<h1>Spontini — Operator Console</h1>` placeholder. No sections, no DSI, no `/admin/api/*` calls. Same Node LTS pin.
  - Deliverables:
    - `admin-ui/package.json`, `admin-ui/.nvmrc`, `admin-ui/tsconfig.json` (strict)
    - `admin-ui/index.html`, `admin-ui/src/main.ts`, `admin-ui/src/App.vue` (renders the placeholder heading)
    - `admin-ui/vite.config.ts` (dev server on port 5173, host `0.0.0.0`)
  - Skills to load: (none of the six project skills apply)
  - Verification: `npm ci && npm run build` produces `admin-ui/dist/`; lint clean; test passes.

### Phase 4: Dockerization and Compose wiring

Goal: containerize all six services, define the shared `kb-data` volume, and make `make build` / `make up` / `make down` / `make logs` / `make compose-config` work end-to-end.

- [x] **Task 4.1** — Write Dockerfiles for the 4 application containers
  - What: Add `backend/Dockerfile` (multi-stage: `rust:1.96.1` build → `debian:bookworm-slim` runtime; build stage installs `cargo-tarpaulin` via `cargo install cargo-tarpaulin --locked` for the coverage gate, and `rustup component add clippy rustfmt` for lint/format dev commands; runtime exposes `8080`), `ingest/Dockerfile` (same multi-stage pattern, runs the `ingest` binary), `frontend/Dockerfile` (single-stage `node:<lts>-alpine` with `apk add --no-cache nginx` — keeps `npm` available for `docker compose run` dev commands while nginx serves the built `dist/`; default nginx config suffices for the walking skeleton since it serves a single `index.html` with no SPA routing), `admin-ui/Dockerfile` (same `node:<lts>-alpine` + nginx base, plus a custom `admin-ui/nginx.conf` that serves the SPA and reverse-proxies `location /admin/api/ { proxy_pass http://backend:8080; }`). Every Dockerfile is verify-gate compatible: `cargo`/`npm` commands work in `docker compose run` for the backend/frontends respectively.
  - Deliverables:
    - `backend/Dockerfile`
    - `ingest/Dockerfile`
    - `frontend/Dockerfile`
    - `admin-ui/Dockerfile`
    - `admin-ui/nginx.conf` (SPA root + try_files + `/admin/api/` proxy_pass to `backend:8080`; note: `frontend` intentionally uses default nginx config — a custom `frontend/nginx.conf` will be added if SPA routing is needed in a future plan)
  - Skills to load: spontini-verify-gate
  - Verification: `docker compose build backend ingest frontend admin-ui` exits 0; each image starts and its health endpoint responds (backend `/health`, frontends `GET /`); `docker compose run --rm backend cargo test --workspace` exits 0; `docker compose run --rm frontend npm run test` exits 0; `docker compose run --rm admin-ui npm run test` exits 0.

- [x] **Task 4.2** — Write `docker-compose.yml` wiring all 6 services + `kb-data` volume
  - What: Author `docker-compose.yml` matching the conceptual excerpt in [docs/STACK.md §5](../docs/STACK.md#5-containerization): `backend` (ports `8080:8080`, volume `kb-data:/data`, depends_on `llama-embed` + `llama-generate`), `admin-ui` (ports `5173:80`, depends_on `backend`), `ingest` (volume `kb-data:/data`, depends_on `llama-embed`), `frontend` (ports `5174:80`), `llama-embed` (image `ghcr.io/ggml-org/llama.cpp:server`, volume `./models/embed:/models`, `--embeddings` command), `llama-generate` (same image, volume `./models/generate:/models`), and the `kb-data` volume. Add healthchecks for the 4 app containers.
  - Deliverables:
    - `docker-compose.yml`
    - `models/embed/.gitkeep`, `models/generate/.gitkeep` (so the mount paths exist; `.gitignore` already excludes `*.gguf`)
  - Skills to load: spontini-verify-gate
  - Verification: `docker compose config -q` exits 0; `make compose-config` passes.

- [x] **Task 4.3** — Add `provision-models` Makefile target
  - What: Add a `provision-models` target to the `Makefile` that downloads `nomic-embed-text` (Q4 GGUF) into `models/embed/` and `qwen2.5-7b-instruct` (Q4_K_M GGUF) into `models/generate/` using `curl`/`wget` against Hugging Face, idempotently (skip if the file already exists with the right size). Document the target in the `help` output. This is the mechanism that makes the inference containers actually start.
  - Deliverables:
    - `Makefile` patch: new `provision-models` target with `## provision-models: ...` help line
    - `models/embed/README.md` and `models/generate/README.md` noting the expected filenames and origins (referenced from `AGENTS.md` Section 4 as new Markdown files)
  - Skills to load: spontini-verify-gate
  - Verification: `make provision-models` (with network) populates `models/embed/` and `models/generate/` with the two GGUF files; running it twice is a no-op the second time.

- [x] **Task 4.4** — Update `AGENTS.md` with new Markdown file references
  - What: Per [AGENTS.md §3.2](../AGENTS.md#32-documentation-indexing), register every new `.md` file introduced by this plan (`models/embed/README.md`, `models/generate/README.md`, and this plan file is already under `.project/` which is exempt) in the appropriate table. No prompts/skills/agents are added by this plan.
  - Deliverables:
    - `AGENTS.md` edit: new rows in Section 4 for the two `models/*/README.md` files
  - Skills to load: (none of the six project skills apply)
  - Verification: `grep` of `AGENTS.md` shows both new README paths listed; no orphaned Markdown files exist under the repo.

### Phase 5: End-to-end verification gate

Goal: prove the walking skeleton is up and running through the project's own verify gate, and that every service is healthy.

- [x] **Task 5.1** — Bring the full stack up and confirm all 6 services healthy
  - What: Run `make provision-models && make build && make up`, then verify `docker compose ps` shows all 6 services running/healthy. `curl` the backend `/health`, the `frontend` `/`, the `admin-ui` `/`, and both `llama-server` `/health` endpoints. Confirm `ingest` is running and logging its heartbeat.
  - Deliverables:
    - (no new files — this task is the acceptance run; its evidence is the command output)
  - Skills to load: spontini-verify-gate
  - Verification: `docker compose ps` shows 6/6 services `Up (healthy)`; `curl http://localhost:8080/health` → `{"status":"ok"}`; `curl http://localhost:5174/` → 200; `curl http://localhost:5173/` → 200; `curl http://localhost:<embed-port>/health` → 200; `curl http://localhost:<generate-port>/health` → 200.

- [x] **Task 5.2** — Run the full `make verify` gate green
  - What: Execute `make verify` (build + test + lint + fmt-check + coverage + compose-config) inside containers and confirm every gate passes, or explicitly note any pre-existing failure. For the walking skeleton, the coverage gate runs on near-empty crates — confirm it passes (the trivial `version()` functions are fully covered) or document a justified exclusion in `coverage-exclusions.txt`.
  - Deliverables:
    - `coverage-exclusions.txt` at repo root (only if a justified exclusion is needed; otherwise absent)
  - Skills to load: spontini-verify-gate
  - Verification: `make verify` exits 0; the verify-gate skill's 10 gates are reported ✅ or explicitly noted as pre-existing/n/a.

## Acceptance Criteria

- `make build` builds all 6 container images with exit code 0.
- `make up` starts all 6 containers; `docker compose ps` shows 6/6 `Up` (4 app containers `healthy`, 2 inference containers `Up`).
- `curl http://localhost:8080/health` returns `200` and `{"status":"ok"}`.
- `curl http://localhost:5174/` returns `200` and renders the `frontend` placeholder home page.
- `curl http://localhost:5173/` returns `200` and renders the `admin-ui` placeholder home page.
- `curl http://localhost:<embed-port>/health` and `curl http://localhost:<generate-port>/health` return `200` (llama-server healthy with the provisioned models).
- `docker compose logs ingest` shows the `ingest service started (walking skeleton)` line and the service stays up until `make down`.
- `make down` stops all 6 containers cleanly, preserving the `kb-data` volume.
- `cargo test --workspace` passes (unit + the `bdd` health scenario is green).
- `cargo clippy --workspace -- -D warnings` is clean; `cargo fmt --all -- --check` is clean.
- `docker compose config -q` exits 0.
- `make verify` exits 0 (or every non-passing gate is explicitly noted as pre-existing/unrelated).
- BDD scenario in `features/health.feature` is green via `cargo test -p backend --test bdd`.

## Risks

- **Inference containers refuse to start without GGUF models** — mitigation: the `provision-models` Makefile target downloads them idempotently; the acceptance run includes `make provision-models` before `make up`. If the operator skips provisioning, the 4 app containers are still healthy and the 2 inference containers are defined but exit — this state is documented, not hidden.
- **GGUF download size (~4.7 GB total: ~274 MB embed + ~4.4 GB generate)** — mitigation: downloads are idempotent and resumable; a future ADR may swap the generate model for a smaller one for dev. Out of scope for this plan.
- **`cargo tarpaulin` coverage gate on near-empty crates** — mitigation: the trivial `version()` functions are 100% covered; if tarpaulin fails to produce a report on an empty crate, a documented entry in `coverage-exclusions.txt` is added with a one-line reason. No test is deleted or `#[ignore]`d to pass.
- **Backend/ingest Docker images must serve double duty (runtime + `cargo` dev commands via `docker compose run`)** — mitigation: the multi-stage Dockerfiles keep a cargo-capable build stage reachable, or the Makefile dev targets use the build stage. Verified in Task 4.1.
- **Vue/Vite strictest `tsconfig` may surface latent type errors in freshly scaffolded code** — mitigation: the scaffold is minimal (`<h1>` placeholder, no logic), so the strictest config is satisfied trivially; no `any`/`@ts-ignore` is ever introduced.
- **No remote `main` branch yet (origin is empty)** — mitigation: this plan branches from the local `main` (single commit); pushing `feat/bootstrap-infra` and later `main` is handled at merge time, not in this plan.

## Out-of-Scope

- Real RAG / retrieval / embedding / generation logic (separate plan).
- Ingest adapters (scraper, api-client), chunking, and `kb.db` schema migrations (separate plan).
- `persona` table, prompt assembly, source-citation UI (separate plan).
- Design System Italia / Bootstrap Italia integration (separate plan).
- Admin-ui sections: ingest configuration, bot imprinting, training (separate plan).
- CI pipeline and status badges (separate plan).
- Production hardening (resource limits, non-root containers, image scanning) — separate plan.
