# Spontini Bot 2 — Roadmap

This file is the **single source of truth** for the project's feature roadmap. It lists every feature — already implemented and yet to be built — grouped by milestone. It is the entry point for the plan lifecycle: `/create-plan` reads this file to pick up the next unchecked feature.

Each feature row has:

- **ID** — a stable 4-digit Feature ID (the same sequence used by `.project/<ID>-<name>-plan.md`).
- **Title** — a short, human-readable title.
- **Description** — the brief passed to `/create-plan` when the feature is picked up. It is rich enough for the planning agent to author a concrete plan (Objective + Non-Goals), and concise enough to fit on one row.
- **Status** — a checklist checkbox: `- [x]` closed, `- [ ]` pending.

## Workflow

1. Pick the next unchecked feature of the current milestone (in ID order).
2. Run `/create-plan` with no argument — the command reads this roadmap, resolves the first unchecked feature, and uses its **title** (for the branch and file name) and its **description** (to seed the Objective and Non-Goals). An explicit argument still overrides the roadmap lookup.
3. Follow the standard plan lifecycle: `/approve-plan` → `/implement-plan` → `/review-plan` → `/fix-review`.
4. When the plan is closed, run `/create-adr` to record the binding architectural decision (if the feature introduced one).
5. **Tick the feature** in this file: change its `- [ ]` to `- [x]` and add a `Closed:` line linking the plan and the ADR. This is the last action of the feature-close sequence.

## Conventions

- A feature is **closed** only after its plan's status is `closed` AND the resulting ADR (if any) is `accepted`. Ticking the roadmap without both is forbidden.
- Features are implemented **in ID order within a milestone**. Skipping ahead requires an explicit ADR.
- A milestone is **done** when every feature it contains is ticked. The next milestone then becomes current.
- New features are appended to the **current milestone** with the next free 4-digit ID. They are never inserted before a ticked feature.

---

## Milestone 0 — Foundation (DONE)

The walking skeleton, the shared data layer, and the citizen-facing RAG flow. After Milestone 0, `make up` runs the full stack and `POST /chat` answers citizens from the knowledge base with cited sources and an honest-unknown fallback.

- [x] **0001** — Bootstrap Infrastructure
  - Description: Stand up the Cargo workspace (5 Rust crates: `backend`, `ingest`, `ingest-cli`, `ingest-core`, `kb-store`), the two Vue 3 + Vite + TypeScript apps (`frontend`, `admin-ui`), the `docker-compose.yml` wiring all 6 runtime containers + the shared `kb-data` volume, the multi-stage Dockerfiles, the `provision-models` Makefile target, and the `make verify` gate. Every container exposes only a health endpoint / empty home page; no business logic yet.
  - Closed: Plan [0001](../.project/0001-bootstrap-infra-plan.md), ADR [0002](../.adr/0002-multi-stage-docker-compose-target.md).

- [x] **0002** — kb-store libSQL Implementation
  - Description: Transform `kb-store` from a version-string skeleton into a working libSQL access layer. Add the `libsql` dependency, an idempotent embedded-SQL migration runner creating the `documents` and `persona` tables per STACK.md §3.5, and a Clean-Architecture public API: `KbStore::open`, document CRUD (`insert`, `get_by_id`, `get_by_source`, `search_similar` via `vector_distance_cos`, `delete`), and versioned persona CRUD (`insert`, `get_active`, `get_by_id`, `get_versions`, `activate`) honoring the `is_active` partial unique index. No wiring into `backend` or `ingest` yet.
  - Closed: Plan [0002](../.project/0002-kb-store-impl-plan.md), ADR [0004](../.adr/0004-libsql-storage-layer.md).

- [x] **0003** — rag-engine: Retrieval-Augmented Generation for `/chat`
  - Description: Transform `POST /chat` from a walking-skeleton stub into a real RAG flow. Build the `rag_engine` module inside `backend` with framework-agnostic domain types (`Answer`, `CitedSource`, `PromptParts`, `RagError`, `PersonaSnapshot`), four `#[async_trait]` ports (`EmbeddingPort`, `RetrievalPort`, `PersonaPort`, `GenerationPort`), KbStore-backed and HTTP-backed adapters, a 3-part prompt assembler (persona / context / question structurally separated), and a `RagEngine` use case orchestrating the ports with the honest-unknown fallback that never calls the generation model when no chunks are retrieved (Constitution §5). Wire `/chat` via dependency-injected `AppState` with `Config::from_env`, and add BDD scenarios for the answerable and honest-unknown paths.
  - Closed: Plan [0003](../.project/0003-rag-engine-plan.md), ADR [0001](../.adr/0001-generation-model-3b.md), ADR [0003](../.adr/0003-rag-engine-ports-adapters.md).

---

## Milestone 1 — Ingest Pipeline

The always-on ingest service that populates `kb.db` from configured URL sources and from per-section manual uploads, on a schedule and on demand. After Milestone 1, the knowledge base is no longer fed by tests alone.

- [x] **0004** — kb-store ingest configuration schema
  - Description: Extend `kb-store` with the configuration tables that drive the ingest service: `ingest_schedule` (cron expression, enabled flag), `ingest_section` (name, e.g. sport/news/delibere/storia, ordering), `ingest_source` (section_id, source_type `scrape`|`api`, url, enabled; `api` rows are stored but never wired), and a `ingest_run_request` flag-row table used by `/admin/api/ingest/run`. Add a `V2__ingest_config.sql` migration (idempotent, transactional) and public CRUD methods on `KbStore` (`get_schedule`, `upsert_schedule`, `list_sections`, `upsert_section`, `list_sources_by_section`, `upsert_source`, `request_run`, `consume_run_request`). Unit tests for every method; no `backend` or `ingest` wiring.
  - Closed: Plan [0004](../.project/0004-kb-store-ingest-config-schema-plan.md), ADR [0005](../.adr/0005-ingest-configuration-data-model.md).

- [x] **0005** — ingest-core: scraper adapter, chunking, embedding pipeline
  - Description: Build `ingest-core` into a real shared library. Implement the `scraper` adapter (HTTP GET a URL, extract visible text via `scraper`/`kuchiki`, honor `robots.txt` and a content-type allowlist), a chunking module (recursive text splitter, ~512-token chunks with ~64-token overlap, section-tagged metadata), and an embedding client that POSTs chunk text to `llama-embed` `/embedding` and validates the 768-dim response against `kb_store::EMBEDDING_DIM`. Define a `Pipeline` trait and a `IngestPipeline` orchestrator (scrape → chunk → embed → `KbStore::insert_document`). The `api-client` adapter exists as a stub and is explicitly NOT wired. Unit tests with `wiremock` for HTTP; no scheduler, no container.
  - Closed: Plan [0005](../.project/0005-ingest-core-scraper-adapter-chunking-embedding-pipeline-plan.md), ADR [0006](../.adr/0006-ingest-pipeline-trait.md).

- [x] **0006** — ingest service: long-running scheduler
  - Description: Turn the `ingest` binary from a heartbeat skeleton into the always-on service. On startup, load the active schedule and sections from `kb.db` via `kb-store`. Run a tokio cron task that, on each tick, invokes the `IngestPipeline` for every enabled `scrape` source of every enabled section. Poll `kb.db` for configuration changes every N seconds (configurable) and apply them without restart. Consume the `ingest_run_request` flag row to trigger an immediate out-of-schedule run. Honor `SIGTERM` for clean shutdown. Integration test for the pipeline runner end-to-end with wiremock.
  - Closed: Plan [0006](../.project/0006-ingest-service-long-running-scheduler-plan.md), ADR [0007](../.adr/0007-cron-based-ingest-scheduler.md).

- [x] **0007** — ingest-cli: one-shot manual run developer tool
  - Description: Upgrade `ingest-cli` from a help-line skeleton into a thin one-shot developer tool over `ingest-core`. Support `ingest-cli run --url <URL> --section <name>` (scrape + chunk + embed + insert into `kb.db`) and `ingest-cli run --section <name> --all-sources` (read the section's configured sources from `kb.db` and run them once). No scheduling, no daemon. This is a developer convenience, not a production container. Unit tests for argument parsing; integration test against a `wiremock` source URL.
  - Closed: Plan [0007](../.project/0007-ingest-cli-one-shot-manual-run-developer-tool-plan.md). No ADR — thin developer convenience CLI, no architectural decision to record.

---

## Milestone 2 — Admin Surface (Backend)

The operator-facing HTTP surface of `backend`, behind `/admin/api/*`. After Milestone 2, an operator can configure persona, ingest, and run training sessions via the API (the admin-ui SPA comes in Milestone 3).

- [x] **0008** — `/admin/api/persona` — bot imprinting CRUD + reload
  - Description: Add the admin persona surface to `backend`: `GET /admin/api/persona` (list versions of a persona by name), `POST /admin/api/persona` (insert a new versioned row, optionally activate; never UPDATE), `POST /admin/api/persona/:id/activate`, and `POST /admin/api/persona/reload` (drop the cached active persona so the next `/chat` request re-reads from `kb.db`). Add a `PersonaAdminPort` and wire `kb-store` behind it. BDD scenarios for: insert deactivates previous active when `activate=true`; reload picks up a newly-activated persona; version increments within a name. Auth is a static shared-secret header for now (a dedicated auth plan follows).
  - Closed: Plan [0008](../.project/0008-admin-api-persona-bot-imprinting-crud-reload-plan.md). No ADR — shared-secret auth follows from Constitution §3; port separation extends ADR 0003.

- [x] **0009** — `/admin/api/upload` — per-section manual document upload
  - Description: Add a manual-upload endpoint that accepts a multipart file (pdf/docx/md/txt), a section name, and a metadata form (category, tags, priority/trust_score). Extract text (`pdf-extract`/`docx-rs`/plain read), return a preview (`GET /admin/api/upload/preview/:token` showing the extracted text and metadata before indexing), and on `POST /admin/api/upload/confirm/:token` delegate chunking + embedding to `ingest-core` and write the chunks to `kb.db` via `kb-store`. The preview/confirm split guarantees the operator never indexes unseen content. BDD scenario for the upload → preview → confirm → searchable flow.
  - Closed: Plan [0009](../.project/0009-admin-api-upload-plan.md).

- [ ] **0010** — `/admin/api/ingest/config` — read/write ingest configuration
  - Description: Add endpoints to read and write the ingest configuration authored in feature 0004: `GET /admin/api/ingest/config` (schedule + sections + sources tree), `PUT /admin/api/ingest/config/schedule`, `POST /admin/api/ingest/config/sections`, `PUT /admin/api/ingest/config/sections/:id`, `DELETE /admin/api/ingest/config/sections/:id`, `POST /admin/api/ingest/config/sources`, `PUT /admin/api/ingest/config/sources/:id`, `DELETE /admin/api/ingest/config/sources/:id`. The `api` source type is writable but always returned with `enabled=false` and a `coming_soon: true` flag. All writes go through `kb-store`; the `ingest` service picks them up on its next config poll. BDD scenarios for create/update/delete and for the disabled-api-source invariant.

- [ ] **0011** — `/admin/api/ingest/run` — trigger an immediate ingest run
  - Description: Add an endpoint that writes a run-request flag row to `kb.db` (via `KbStore::request_run`) so the `ingest` service picks it up on its next poll and runs the enabled sources out of schedule. Returns 202 with a request-id the operator can poll via `GET /admin/api/ingest/run/:id` for status (pending / running / done / failed). BDD scenario for the trigger → poll → done flow using a mock source URL.

- [ ] **0012** — `/admin/api/training/sessions` — training session CRUD
  - Description: Add a `training_session` table to `kb-store` (V3 migration: `id`, `title`, `created_at`, `created_by`, `closed_at`) and admin endpoints to create, list, get, and close a session. A session groups an operator's training messages and feedback. BDD scenario for create → list → close.

- [ ] **0013** — `/admin/api/training/sessions/:id/messages` — ask/answer with recording
  - Description: Add a `training_message` table (V4 migration: `id`, `session_id`, `question`, `answer`, `sources` JSON, `fell_back`, `created_at`) and a `POST /admin/api/training/sessions/:id/messages` endpoint that delegates to the same `RagEngine` as `/chat`, persists the exchange, and returns the answer with cited sources. `GET /admin/api/training/sessions/:id/messages` lists the session's exchanges. BDD scenario: a training message records the same answer shape as `/chat`, including the honest-unknown fallback.

- [ ] **0014** — `/admin/api/training/feedback` — point-in-answer feedback
  - Description: Add a `training_feedback` table (V5 migration: `id`, `message_id`, `chunk_id` nullable, `answer_span` text, `sentiment` `positive`|`negative`, `comment` text, `created_at`) and a `POST /admin/api/training/feedback` endpoint that records point-in-answer feedback anchored to a span of the answer and optionally to a retrieved chunk. `GET /admin/api/training/messages/:id/feedback` lists the feedback for a message. This data drives future retrieval-quality analysis (a later, out-of-roadmap analytics plan). BDD scenario for positive + negative + comment feedback on the same message.

---

## Milestone 3 — Operator Console (admin-ui SPA)

The Vue 3 + TypeScript SPA served by the `admin-ui` container, built on Design System Italia, with the three first-class sections (Ingest configuration, Bot imprinting, Training). After Milestone 3, an operator can drive the whole system from a browser.

- [ ] **0015** — admin-ui Design System Italia integration + /dev catalog
  - Description: Integrate `bootstrap-italia` + `design-tokens-italia` into `admin-ui` (Vite + Dart Sass, `@use`/`@forward` only). Build thin Vue wrapper components (`<DsButton>`, `<DsInput>`, `<DsCallout>`, `<DsNav>`, …) under `src/components/ds/` re-exported from a barrel. Add a `/dev` route listing every wrapper component in isolation (Storybook-lite). Configure `axe-core` + `pa11y` in the test suite with zero-violation CI gate. No business sections yet.

- [ ] **0016** — admin-ui Ingest configuration section
  - Description: Build the Ingest configuration section as a first-class route. Left-rail navigation (Ingest · Imprinting · Training). The section shows the schedule (cron, enabled toggle), the section list (sport/news/delibere/storia), and per-section sources (URL scraper enabled; API greyed-out with "Coming soon" tooltip) and a per-section manual upload dropzone that calls the feature 0009 upload flow (preview → confirm). All calls go to `/admin/api/ingest/config` and `/admin/api/upload`. BDD scenario for: add a section, add a scraper source, trigger a run, see the run status.

- [ ] **0017** — admin-ui Bot imprinting section
  - Description: Build the Bot imprinting section: a form to edit the active persona (name, system_prompt, tone, fallback_message), a "save as new version" action (calls `POST /admin/api/persona` with `activate=true`), a version history list with per-version activate buttons, and a "reload active persona" action. Destructive actions (activate an old version, delete a draft) behind an explicit DSI confirmation dialog. BDD scenario for: save a new version, see it in history, activate a previous version, reload.

- [ ] **0018** — admin-ui Training section with point-in-answer feedback
  - Description: Build the Training section: a session list, a session view with an ask/answer box, the answer rendered with inline expandable source citations (from the `sources` DTO, not by parsing the answer text), and a point-in-answer feedback marker — select a span of the answer, mark positive/negative, leave a comment, submit to `/admin/api/training/feedback`. The same `RagEngine` answers; the exchange is recorded. BDD scenario for: ask a question, see cited sources, leave negative feedback on a span, see the feedback persisted.

- [ ] **0019** — admin-ui accessibility + keyboard audit
  - Description: Audit every admin-ui section against WCAG 2.1 AA: keyboard navigability (every interactive element reachable + operable), visible focus ring (never removed), screen-reader labels (`aria-label`/`aria-labelledby`), color contrast, reduced-motion honored, touch targets ≥ 44×44 px where applicable, semantic HTML before ARIA. Run `axe-core` and `pa11y` on every route, fix every violation, add the zero-violation gate to the test suite. Manual screen-reader smoke test documented in a BDD scenario. This is a dedicated feature because accessibility is non-negotiable (STACK.md §4.2) and benefits from a focused pass.

---

## Milestone 4 — Public Chat (frontend SPA)

The Vue 3 + TypeScript SPA served by the `frontend` container, built on Design System Italia, with the citizen-facing chat widget. After Milestone 4, citizens can use Spontini from a browser.

- [ ] **0020** — frontend Design System Italia integration + /dev catalog
  - Description: Integrate `bootstrap-italia` + `design-tokens-italia` into `frontend` (Vite + Dart Sass, `@use`/`@forward` only). Build the thin Vue wrapper components under `src/components/ds/` re-exported from a barrel, and add a `/dev` route listing them in isolation. Configure `axe-core` + `pa11y` with zero-violation gate. No chat UI yet.

- [ ] **0021** — frontend chat widget with citation rendering
  - Description: Build the public chat widget: a single primary action (send a message), a conversation view, an input box with a forgiving placeholder ("Scrivi la tua domanda…"), and answer rendering with inline expandable source citations built from the `sources` DTO returned by `/chat` (never by parsing the answer text). Touch targets ≥ 44×44 px. Loading / empty / error states are designed, not accidental. BDD scenario for: ask a question, see the answer with expandable citations.

- [ ] **0022** — frontend honest-unknown + error state UI
  - Description: Build the honest-unknown state (when `fell_back=true`, render the fallback message with no citations and a calm "non ho trovato informazioni" tone) and the error state (when `/chat` returns 502/503, render an honest "non riesco a rispondere ora" state, never a raw "Error 500"). Both states are designed per STACK.md §4.5 — honest states, no lying spinners. BDD scenarios for the honest-unknown and the 502/503 paths.

- [ ] **0023** — frontend accessibility + reduced-motion + keyboard audit
  - Description: Audit the public chat against WCAG 2.1 AA with the same rigor as feature 0019: keyboard navigability, visible focus, screen-reader labels, color contrast, reduced-motion honored, touch targets ≥ 44×44 px, semantic HTML. Run `axe-core` + `pa11y` on the chat route, fix every violation, add the zero-violation gate. Manual screen-reader smoke test documented in a BDD scenario. The public chat is the most intuitive surface Spontini offers — accessibility is non-negotiable.

---

## Milestone 5 — Quality, CI, and Production Hardening

The closing milestone: CI, end-to-end BDD against live containers, and production hardening. After Milestone 5, the system is shippable to the Comune di Maiolati Spontini.

- [ ] **0024** — CI pipeline (GitHub Actions) + README status badges
  - Description: Add a GitHub Actions workflow that runs `make verify` on every push and PR (build + test + lint + fmt-check + coverage + compose config), caches the Docker layers and the cargo registry, and fails on any non-zero gate. Wire the build/test/coverage badges in `README.md` (currently `pending`). Coverage gate enforced at 100% line / 80% branch per PRINCIPLES.md §7. No deployment step — deployment is a separate, future plan.

- [ ] **0025** — End-to-end BDD against live containers
  - Description: Add an end-to-end BDD suite that runs the `features/*.feature` scenarios against the full running stack (all 6 containers up via `make up`, real `llama-embed` and `llama-generate`, a seeded `kb.db` with an active persona and a known document). The honest-unknown scenario uses a question that matches no document. Wire the suite as a `make bdd-e2e` target (separate from the unit-level `make bdd` which uses test doubles). This proves the real adapters work against the real `llama.cpp` containers, closing the gap noted in Plan 0003's risks.

- [ ] **0026** — Production hardening: non-root containers, resource limits, image scanning
  - Description: Harden the runtime for production: run every container as a non-root user, set memory and CPU limits in `docker-compose.yml` tuned for the Mac Intel i7 / 16 GB RAM target, add a `healthcheck` to every service (the inference containers included), and add a `make scan` target running `trivy` (or equivalent) on every image with a zero-high-cve gate. Document the hardened compose in an ADR. No functionality change — this is a non-functional hardening pass.

- [ ] **0027** — Operator auth + audit log
  - Description: Replace the static shared-secret auth placeholder from feature 0008 with a real operator auth scheme (single operator for now: a hashed password in an env-loaded credential file, a short-lived session cookie). Every `/admin/api/*` write is recorded in an `audit_log` table (V6 migration: `id`, `actor`, `action`, `target`, `payload` JSON, `at`). BDD scenarios for: unauthenticated write is rejected; authenticated write succeeds and is audited. This closes the security gap left open by the admin surface plans.
