# Plan 0025: End-to-end BDD against live containers

- **Status**: closed
- **Approved**: 2026-07-25 by agent
- **Implemented**: 2026-07-25 by agent
- **Closed**: 2026-07-25 by agent
- **Review verdict**: approved
- **Branch**: feat/end-to-end-bdd-against-live-containers
- **Feature ID**: 0025
- **Created**: 2026-07-25
- **Owner**: agent

## Objective

The [Constitution](../docs/CONSTITUTION.md) demands that Spontini's citizen-facing answers are trustworthy — every answer cited, every honest-unknown fallback honest. Today that guarantee is proven only at the unit/in-process level: `make bdd` (`backend/tests/bdd.rs`) exercises `features/chat.feature` through an in-memory `axum::Router` wired with stub `EmbeddingPort`/`GenerationPort` implementations that never touch the real `llama-embed` / `llama-generate` containers. This leaves exactly the gap Plan 0003 flagged as an open risk: the real HTTP-backed adapters (`backend/src/rag_engine/embedding.rs`, `backend/src/rag_engine/generation.rs`), the real libSQL `vector_distance_cos` retrieval, and the real `ingest-core` chunk/embed pipeline behind `/admin/api/upload` have never been proven to work together against the actual containerized stack. This feature closes that gap: it adds a new, separate end-to-end BDD test binary that runs the two scenarios of `features/chat.feature` (answerable-from-document, honest-unknown) as an external HTTP client against a `make up`'d stack with real models provisioned via `make provision-models`, seeding the knowledge base through the real `/admin/api/persona` and `/admin/api/upload` endpoints — never by writing to `kb.db` directly. In scope: the new e2e test binary, its step definitions, and a `make bdd-e2e` Makefile target, wired as documented, optional, separate tooling. Out of scope: extending e2e coverage to the admin/training/ingest-config feature files (`admin_*.feature`) — those remain covered by the existing in-process `make bdd`; making `bdd-e2e` part of `make verify` or the CI workflow from feature 0024 (it needs multi-gigabyte GGUF models and a live multi-container stack, which do not belong in the fast per-push gate); and any change to the RAG engine, ports, or adapters themselves.

## Non-Goals

- No e2e coverage for `admin_persona.feature`, `admin_upload.feature`, `admin_training_*.feature`, `admin_ingest_*.feature`, or `health.feature` — only `features/chat.feature`'s two scenarios, which are what Plan 0003's risk note is about. A follow-up feature can extend coverage later.
- No addition of `bdd-e2e` to `make verify` or to `.github/workflows/ci.yml` — it requires `make provision-models` (multi-gigabyte downloads) and a live `make up` stack, neither of which belong in the fast, always-on CI gate from feature 0024.
- No automatic wiping/reset of the `kb-data` Docker volume before each e2e run. The target assumes the operator runs it against a stack they control (fresh via `make down && make up`, or an existing dev stack); forcing a destructive volume wipe by default is out of scope.
- No change to `rag_engine`, its ports, its adapters, `ingest-core`, or `kb-store`. This feature only adds a test harness that exercises the existing, already-implemented real code paths.

## Phases

### Phase 1: End-to-end BDD test harness

Goal: a new, independent Cucumber test binary proves `features/chat.feature`'s two scenarios pass against the real, running, containerized stack — real embeddings, real retrieval, real generation.

- [x] **Task 1.1** — Write the e2e step definitions against the live HTTP API
  - What: Create `backend/tests/bdd_e2e.rs`, a new Cucumber `World` and step-definition set that runs `features/chat.feature` (only) as an external `reqwest` HTTP client against `E2E_BASE_URL` (env var, default `http://localhost:8080`), authenticating admin calls with `E2E_ADMIN_API_KEY` (env var, default `dev-key`, matching `backend::Config::from_env`'s default). Seed the "a document titled X" / "the document contains the text Y" Given-steps by POSTing a markdown file to `/admin/api/upload` (manually-encoded multipart body — mirror the existing helper in `backend/tests/bdd.rs`, no new `reqwest` Cargo features needed) with section `"news"`, then `POST /admin/api/upload/confirm/:token` — this is the real `ingest-core` chunk/embed/insert path, not a stub. Seed "an active persona is configured..." by `POST /admin/api/persona` with `activate: true`. Implement "the citizen asks" as `POST /chat`. For the answerable scenario, assert HTTP 200, `fell_back == false`, `sources[0].source_ref` equals the exact seeded document title (proves real retrieval + citation), and `answer` is non-empty and not equal to the configured fallback message (proves the real generation model produced content) — do not assert exact generated wording, since the real `qwen2.5-3b-instruct` model's output is not deterministic. For the honest-unknown scenario, assert HTTP 200, `fell_back == true`, `sources` empty, and `answer` exactly equals the configured fallback message (this path is config-driven, not model-generated, so it is deterministic even against the live stack). Implement the "the final prompt keeps the persona, retrieved context, and question as three separate parts" and "the knowledge base contains no document about X" Given/Then steps as documented no-ops (with a one-line comment explaining why): the first is an internal architectural invariant already proven by the in-process `make bdd` suite and is not observable from outside the HTTP boundary; the second relies on the freshly-seeded live `kb.db` genuinely having nothing related to "tasse comunali" — no code action needed.
  - Deliverables:
    - `backend/tests/bdd_e2e.rs`
  - Skills to load: spontini-bdd-gherkin, spontini-rag-build
  - Verification: `cargo check --test bdd_e2e -p backend` compiles cleanly (verifies wiring without requiring a live stack); manual read-through confirms every step in `features/chat.feature` has a matching implementation.

- [x] **Task 1.2** — Add the `make bdd-e2e` Makefile target
  - What: Add a `bdd-e2e` target to the `Makefile` that runs `cargo test --test bdd_e2e -p backend -- --nocapture` natively on the host (a deliberate, documented exception to the project's "every target runs inside containers" convention — this suite tests the containerized stack as an external client over its published `localhost:8080` port; running it *inside* another `backend`-service container via `docker compose run` would collide with the already-running `backend` service's Compose DNS name and is why this target is host-native). Precede it with an `## bdd-e2e:` help comment stating the prerequisites (`make provision-models` and `make up` must have completed first).
  - Deliverables:
    - Updated `Makefile` (`bdd-e2e` target, `.PHONY` entry, help comment)
  - Skills to load: spontini-verify-gate
  - Verification: `make help` lists the new `bdd-e2e` target with its description; the target is a single `cargo test` invocation (no inline conditionals/loops, honoring `docs/STACK.md §7.3` Rule 7).

### Phase 2: Documentation

Goal: operators and CI-readers understand `make bdd-e2e` exists, what it needs, and how it differs from `make bdd`.

- [x] **Task 2.1** — Document the target
  - What: Add a `make bdd-e2e` line to `README.md`'s Quick start block (immediately after the existing `make verify` line) with a one-line comment distinguishing it from `make bdd`/`make verify` (needs `make provision-models` + `make up` first; not part of CI). Add the target to the `docs/STACK.md §7.3` Makefile-targets table alongside the existing `bdd` row.
  - Deliverables:
    - Updated `README.md`
    - Updated `docs/STACK.md`
  - Skills to load: (none — documentation edit)
  - Verification: manual read-through confirms both docs mention `bdd-e2e`, its prerequisites, and that it is intentionally excluded from `make verify`/CI.

## Acceptance Criteria

- `backend/tests/bdd_e2e.rs` exists, compiles (`cargo check --test bdd_e2e -p backend`), and implements every step used by `features/chat.feature`.
- Running `make provision-models && make up && make bdd-e2e` against a real stack passes both `features/chat.feature` scenarios (answerable-from-document and honest-unknown), exercising the real `llama-embed`/`llama-generate` HTTP adapters and the real libSQL vector retrieval — not test doubles.
- `make bdd` (unit-level, in-process, test-doubled) is unchanged and still passes.
- `bdd-e2e` is not part of `make verify` and is not added to `.github/workflows/ci.yml`.
- `README.md` and `docs/STACK.md` document the new target and its prerequisites.

## Risks

- The real generation model's non-deterministic phrasing could make an exact-wording assertion flaky — mitigation: Task 1.1's e2e assertions check citation correctness, `fell_back`, and non-emptiness instead of exact generated text (see Task 1.1 What).
- The `kb-data` Docker volume persists across local runs, so repeated `make bdd-e2e` runs accumulate persona versions and duplicate document rows — mitigation: documented as a known limitation (Non-Goals); CI never runs this target so it is not a CI-flakiness concern, and a fresh `make down -v && make up` (opt-in, destructive) gives a clean slate for local reruns when desired.
- Running the suite natively on the host requires the pinned Rust 1.96.1 toolchain (already required for native dev per `README.md` Prerequisites and `rust-toolchain.toml`) — mitigation: none needed, this is already a documented supported path, not a new requirement.
- Provisioning the real GGUF models (~2.2 GB total) takes time and bandwidth — mitigation: this is exactly why the target is separate from `make verify`/CI and requires an explicit `make provision-models` prerequisite, never run automatically.

## Out-of-Scope

- E2E coverage for any feature file other than `features/chat.feature`.
- Adding `bdd-e2e` to `make verify` or CI.
- Automatic/destructive volume reset before each run.
- Any change to `rag_engine`, `ingest-core`, `kb-store`, or their ports/adapters.
