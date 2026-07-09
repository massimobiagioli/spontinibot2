# Spontini Bot 2 — Technical Stack Specification

## Purpose

A knowledge-base-driven chatbot. The knowledge base is fed from web URLs (scraper) and per-section manual file uploads; an API source type is reserved for future use. All models run locally, on hardware without a GPU (target: Mac Intel i7 / 16GB RAM). Fully containerized, across five runtime containers.

---

## 1. Language and runtime

| Component | Version | Notes |
|---|---|---|
| Rust | **1.96.1** (stable) | Rust has no LTS channel: only the latest stable release receives security patches. Pin the version in `rust-toolchain.toml` and review it periodically (every 2-3 releases, roughly every 3 months). |
| Edition | 2024 | Compatible with 1.96.1 |
| Node.js | Current LTS (never legacy) | Frontend build only, not used at runtime. Must always track the latest LTS line (e.g. via `package.json` `engines` field and `.nvmrc`). Legacy Node versions are forbidden. |

```toml
# rust-toolchain.toml
[toolchain]
channel = "1.96.1"
```

---

## 2. Architecture overview

Five runtime containers, plus the shared `kb.db` volume that is the only coupling between the chat flow and the ingest flow.

```
Chat runtime (public):
  citizen → frontend → backend(/chat) → rag-engine
    → embed query (llama-embed) → retrieval (libSQL/kb.db)
    → prompt (persona + context + query) → llama-generate → answer

Admin runtime (operator):
  operator → admin-ui(SPA) → backend(/admin/api/*)
    → configure ingest sources & schedule
    → configure persona (imprinting) + manual document upload
    → run training sessions (ask / answer / feedback)

Ingest runtime (automated, decoupled):
  scheduler (inside ingest container) reads config from kb.db
    → adapters (scraper (URL) / api (disabled, future) ) + per-section manual uploads → ingest-core
    → embed (llama-embed) → write to kb.db
```

The chat flow and the ingest flow never communicate directly; they share only the `kb.db` file. The admin flow configures both (persona + ingest config + manual uploads) through the backend's `/admin/api/*` endpoints; the ingest container polls `kb.db` for its schedule and source configuration and acts autonomously.

---

## 3. Components

### 3.1 Backend (core) — `axum`
Async HTTP framework, pure Rust. This is the **core** container. Exposes two clearly separated surfaces:

**Public surface (citizen-facing):**
- `/chat` — public endpoint for end users

**Admin surface (operator-facing, protected):**
- `/admin/api/persona` — bot identity (imprinting) management
- `/admin/api/persona/reload` — reload active persona without restart
- `/admin/api/upload` — manual document upload (tagged)
- `/admin/api/ingest/config` — read/write ingest configuration (schedule + sections + sources)
- `/admin/api/ingest/run` — trigger an immediate ingest run out of schedule
- `/admin/api/training/sessions` — create / list training sessions
- `/admin/api/training/sessions/:id/messages` — ask a question, retrieve the answer (delegates to the same `rag-engine` as `/chat`, but records the exchange and the operator's feedback)
- `/admin/api/training/feedback` — record point-in-answer feedback (positive / negative + comment, anchored to a chunk offset)

Hosts the `rag-engine` module: query embedding, retrieval, prompt building, generation. The admin training surface reuses the same `rag-engine` — it is not a second implementation.

### 3.2 Admin UI — `admin-ui` (separate container)
A dedicated container serving the operator-facing SPA. Three sections, each a first-class route inside the SPA:

| Section | Purpose | Backend endpoints |
|---|---|---|
| **Ingest configuration** | Configure the scheduler (cron expressions, enabled/disabled) and the section list (e.g. *sport*, *news*, *delibere*, *storia*). For each section, configure the source(s) and upload documents: **URL (scraper) — enabled**; **API — disabled, future use** (visible but non-clickable, greyed out with a "Coming soon" tooltip). Folder and DB sources are NOT offered. Additionally, per-section **manual file upload** with tagging (category, tags, priority/trust) and extracted-text preview before indexing — e.g. inside the *Sport* section, an operator drops a PDF and it is ingested into that section. | `/admin/api/ingest/config`, `/admin/api/ingest/run`, `/admin/api/upload` (per-section) |
| **Bot imprinting** | Configure the bot identity: who it is, its history, its tone of voice. Edit the active `persona` row (new version inserted on every save, never `UPDATE`). No document upload here — uploads live in the Ingest configuration section above. | `/admin/api/persona` |
| **Training** | Open a training session, ask questions, see the answer with inline source citations, and give **point-in-answer feedback** (select a span of the answer, mark it positive/negative, leave a comment). Feedback is persisted and linked to the retrieved chunks for future retrieval-quality analysis. | `/admin/api/training/sessions`, `/admin/api/training/feedback` |

The admin-ui container is a static SPA (Vue build) served by nginx, with a reverse proxy from `/admin/api/*` to the `backend` container's admin surface. No business logic lives in the SPA — it only calls the backend's admin API.

### 3.3 Ingest — `ingest` container (separate, always-on)
A dedicated container running the **ingest service** (not a one-shot CLI anymore). It is a long-running Rust binary built on top of `ingest-core` (the shared library). Responsibilities:

- On startup, reads the active ingest configuration from `kb.db` (schedule + sections + sources).
- Runs an internal scheduler (a tokio task with cron expressions) that triggers the configured adapters at the configured times.
- On each run: invokes the enabled adapters (scraper only for now; the API adapter exists in `ingest-core` but is not wired to the scheduler yet — folder and DB adapters are NOT part of this project), embeds the chunks via `llama-embed`, writes to `kb.db`. Manual file uploads (per-section, from the admin-ui) are ingested on demand — they do not go through the scheduler; the backend writes them directly and indexes them via `ingest-core` + `llama-embed`.
- Polls `kb.db` periodically (or is signaled by the backend on config save) to pick up configuration changes without restart.
- Can be triggered on demand by the admin-ui via `/admin/api/ingest/run` (the backend writes a "run requested" flag row; the ingest service picks it up).

The `ingest-cli` crate remains as a thin one-shot binary for manual/ad-hoc runs (kept as a developer tool, not a production container).

### 3.4 Inference — `llama.cpp` (`llama-server`)
Two separate containerized instances, same engine, different models:

| Instance | Model | Purpose | Called by |
|---|---|---|---|
| `llama-embed` | nomic-embed-text (or bge-small), GGUF Q4/F16 | Text → vector embedding | `ingest-core` and `rag-engine` |
| `llama-generate` | Qwen2.5-7B-Instruct, GGUF Q4_K_M | Answer generation | `rag-engine` only |

**Constraint:** the same embedding model must be used for writing (ingest) and reading (query). Changing it requires a full re-ingest of the KB.

### 3.5 Storage — `libSQL`
- SQLite-compatible engine with a native vector column type (`F32_BLOB`), no extension to load
- Single `kb.db` file, mounted on a Docker volume
- Exact search (`vector_distance_cos`), no DiskANN index — stays under 100ms at the expected data volume and avoids the disk-space blowup an ANN index causes on small datasets
- Rust crate: `libsql` (native client, `core` feature)

```sql
CREATE TABLE documents (
    id INTEGER PRIMARY KEY,
    source TEXT,              -- 'scrape' | 'api' | 'manual'
    source_ref TEXT,
    content TEXT,
    metadata TEXT,             -- JSON: tags, category, priority/trust_score
    embedding F32_BLOB(768)
);

CREATE TABLE persona (
    id INTEGER PRIMARY KEY,
    version INTEGER NOT NULL,
    name TEXT NOT NULL,
    system_prompt TEXT NOT NULL,
    tone TEXT,
    fallback_message TEXT,
    is_active BOOLEAN DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    created_by TEXT
);
CREATE UNIQUE INDEX idx_persona_active ON persona(is_active) WHERE is_active = 1;
```

### 3.6 Ingest core — `ingest-core` crate
Shared library containing the adapters and the chunking/embedding pipeline. Consumed by two entry points:

- **`ingest` container** (long-running service) — scheduled runs driven by config stored in `kb.db`. Only the **`scraper` adapter (URL sources)** is wired to the scheduler. The **`api-client` adapter** exists in the crate for future use but is **not enabled** in the admin-ui and not wired to the scheduler yet. Folder and DB adapters are **not part of this project**.
- **Backend admin surface** (on-demand, per-section manual upload) — drag & drop file upload (pdf/docx/md/txt), preview of extracted text before indexing, manual metadata form (category, tags, priority/trust). The backend delegates the actual chunking + embedding to `ingest-core` and writes the result to `kb.db`.

Both entry points write to the same `kb.db`; neither knows about the other. The `ingest` container and the backend communicate only through `kb.db` (config rows written by the backend, polled by the ingest container; documents written by either, read by both).

### 3.7 Bot identity — `persona` table
- Not a document in the KB — must never compete during retrieval
- Active row read/cached by `rag-engine` at startup, reloadable via `/admin/api/persona/reload` or a short TTL
- Every edit inserts a new row (history/versioning; never `UPDATE`)
- Final prompt always keeps three parts separate:
  ```
  [SYSTEM: persona.system_prompt]
  [CONTEXT: chunks retrieved from documents]
  [USER: question]
  ```

### 3.8 Frontend (public chat) — Vue 3 + Vite + TypeScript

Lightweight client for the public chat interface. No business logic — only consumes `/chat`.

| Concern | Choice | Notes |
|---|---|---|
| Framework | **Vue 3** (latest stable, never legacy) | Composition API + `<script setup>` |
| Build tool | **Vite** (latest stable) | Dev server, HMR, production bundling |
| Language | **TypeScript** (latest stable) | Configured in the strictest possible mode — see below |
| Pinia | Latest stable | State management (if/when needed) |
| Vue Router | Latest stable | Only if the public surface grows beyond a single chat view |

#### TypeScript strictness contract (NON-NEGOTIABLE)

`tsconfig.json` must enable every strict flag available in the current TypeScript version. The following must all be `true`:

```jsonc
{
  "compilerOptions": {
    "strict": true,
    "noImplicitAny": true,
    "strictNullChecks": true,
    "strictFunctionTypes": true,
    "strictBindCallApply": true,
    "strictPropertyInitialization": true,
    "noImplicitThis": true,
    "alwaysStrict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noImplicitReturns": true,
    "noFallthroughCasesInSwitch": true,
    "noUncheckedIndexedAccess": true,
    "noPropertyAccessFromIndexSignature": true,
    "exactOptionalPropertyTypes": true,
    "forceConsistentCasingInFileNames": true,
    "skipLibCheck": true
  }
}
```

**`any` is forbidden.** This includes:

- Explicit `any` in type annotations.
- Implicit `any` (the `noImplicitAny` flag must catch every case).
- `as any` casts.
- `// @ts-ignore` and `// @ts-expect-error` directives (use proper type narrowing instead).
- `any` smuggled in via `// eslint-disable` exceptions.

When a type is genuinely unknown at a system boundary (e.g. a third-party payload), use `unknown` and narrow it with a type guard or a validation library (e.g. `zod`). Never fall back to `any`.

#### Versioning policy

- Every frontend dependency must track the latest stable major at the time of upgrade. Legacy majors are forbidden.
- `package.json` must declare an `engines.node` constraint pinned to the current LTS major.
- `.nvmrc` (or equivalent) must pin the exact LTS version in use.
- `npm ci` (or the chosen lockfile's strict install command) is mandatory in CI and Docker builds.

---

## 4. UX / UI

This section governs the **visual and interaction layer** of every user-facing surface of Spontini: the public chat (`frontend`) and the operator console (`admin-ui`). It is the technical counterpart of [PRINCIPLES.md §6 — Clean Design](./PRINCIPLES.md#6-clean-design-ui-and-ux--the-jobs-aesthetic).

The guiding ambition: **the most intuitive municipal chatbot an Italian citizen has ever used**, effortless even for people with no confidence in technology. Every rule below serves that ambition.

### 4.1 Design system — Design System Italia (binding)

Both `frontend` and `admin-ui` MUST be built on **Design System Italia** (DSI), the official design system of the Italian Pubblica Amministrazione.

- **Official site**: https://designers.italia.it/design-system/
- **Developer entry point**: https://designers.italia.it/design-system/come-iniziare/per-sviluppatori/

DSI's web implementation is **Bootstrap Italia** (built on Bootstrap 5). There is **no official Vue wrapper** for DSI. The canonical integration approach for a Vue 3 + Vite app is:

1. Install the CSS/JS bundles and design tokens:
   - `bootstrap-italia` — the base stylesheet + interactive JS components
   - `design-tokens-italia` — CSS custom properties + SCSS variables for theming
2. Consume DSI as **stylesheets + markup classes**; write thin Vue wrapper components (`<DsButton>`, `<DsInput>`, `<DsCallout>`, …) around the DSI markup, using the official class names.
3. Initialize DSI's interactive JS components in `onMounted()` (or a dedicated composable) — never globally, never on `window.load`.

| Concern | Choice | Notes |
|---|---|---|
| Base CSS framework | `bootstrap-italia` (latest stable) | DSI web kit. Pin the major version in `package.json`. |
| Design tokens | `design-tokens-italia` (latest stable) | Color, spacing, typography, shadow tokens as CSS variables + SCSS. |
| Vue component layer | In-repo wrappers (e.g. `src/components/ds/`) | One Vue SFC per DSI component used. Re-exported from a barrel. |
| Bootstrap Italia JS | Imported per-feature, initialized in `onMounted` | Avoid global `window` side effects. |
| Versioning | Latest stable major, never legacy | Same policy as the rest of the frontend (see §3.8). |

**Vite integration caveat**: Bootstrap Italia 2.x has a known issue resolving `@splidejs/splide/src/css/core/index` under Vite. Prefer importing the prebuilt bundle (`bootstrap-italia/dist/css/bootstrap-italia.min.css`) and, when a Sass pipeline is needed, use Bootstrap Italia 3.x (modular `@use`/`@forward` architecture, Dart Sass compatible).

### 4.2 Accessibility (NON-NEGOTIABLE)

Accessibility is a **first-class requirement**, not a checklist at the end. DSI conforms to **WCAG 2.1 level AA** — the level mandated for Italian PA sites by the harmonized European standard UNI CEI EN 301549:2021.

Every screen in `frontend` and `admin-ui` must satisfy:

- **WCAG 2.1 AA** at minimum (WCAG 2.2 recommended where DSI already supports it). Reference: https://designers.italia.it/design-system/fondamenti/accessibilita/
- **Keyboard navigable**: every interactive element reachable and operable via keyboard, with a visible focus indicator (DSI's default focus ring must never be removed).
- **Screen-reader labeled**: every meaningful element has an accessible name (`aria-label`, `aria-labelledby`, or visible text). Decorative icons are `aria-hidden`.
- **Color contrast**: text and interactive elements meet AA contrast ratios. Never rely on color alone to convey meaning.
- **Reduced-motion honored**: `@media (prefers-reduced-motion: reduce)` disables non-essential animation.
- **Semantic HTML**: use native elements (`<button>`, `<nav>`, `<main>`, `<dialog>`) before ARIA. ARIA only when a native element is impossible.
- **Touch targets ≥ 44×44 px** on the public chat (per [PRINCIPLES.md §6.2](./PRINCIPLES.md)).
- **Automated + manual audits**: every PR touching UI runs `axe-core` (zero violations) and `pa11y`; manual screen-reader smoke tests are part of the BDD scenarios for citizen-facing flows.

### 4.3 Styling architecture — BEM + Sass

- **BEM** (`block__element--modifier`) is the naming convention for every custom CSS class that is NOT a DSI class. DSI classes keep their official names; project-specific classes follow BEM strictly.
- **Sass** (SCSS syntax) is the stylesheet language. Each Vue SFC uses `<style scoped lang="scss">` for component-local styles; global styles live under `src/styles/` and are imported once in the entry point.
- **Dart Sass** (`sass` package, latest stable) is the compiler — `node-sass` (LibSass) is deprecated and forbidden.
- **Design tokens come first**: colors, spacing, typography, radii, shadows MUST be sourced from `design-tokens-italia` (CSS variables or SCSS variables). Hard-coded hex/px values in custom SCSS are forbidden — use `var(--color-...)` or the SCSS token map.
- **`@use` / `@forward` only**: the legacy `@import` Sass syntax is forbidden (Bootstrap Italia 3.x and Dart Sass v3 compliant).
- **No CSS-in-JS**: Vue SFC `<style>` blocks only. Keeps the build simple and the runtime fast.

### 4.4 Component strategy — reuse first, create only when necessary

1. **Reuse DSI components** whenever one exists for the need (buttons, inputs, callouts, alerts, modals, nav, tables, etc.). Do not reinvent a DSI component. The DSI docs are the single source of truth for what already exists.
2. **Wrap, don't fork**: when a DSI component is used, wrap it in a thin Vue SFC (`<DsButton>`) that forwards props/slots and applies the DSI class names. Never copy-paste DSI markup into business components — the wrapper is the single integration point, so DSI upgrades touch one file.
3. **Create new components only when DSI has no equivalent** (e.g. the chat message bubble with inline source citation, the point-in-answer feedback marker). New components MUST:
   - Be built on DSI design tokens (no bespoke color palette, no bespoke type scale).
   - Follow BEM class naming for their custom parts.
   - Match DSI's visual language (radii, spacing, motion, typography) as if they were part of the system.
   - Be documented in a component catalog (`admin-ui` and `frontend` each have a `/dev` route listing their components in isolation, à la Storybook-lite).
4. **One component = one responsibility** (per [PRINCIPLES.md §3 SOLID](./PRINCIPLES.md#3-solid)). A chat bubble does not fetch data; a feedback marker does not persist itself.

### 4.5 Usability ambition — "the most intuitive ever"

Every interaction must be intelligible without instructions, by a citizen who has never used a chatbot. Concrete rules:

- **One primary action per screen.** The public chat shows the conversation and the input. Nothing else competes.
- **Zero jargon.** "Send the message", not "Submit query". "I don't have an answer in the municipal documents", not "No retrieval results".
- **Every answer cites its source, inline, expandable.** Trust is built by verifiability (per [Constitution](./CONSTITUTION.md) and [PRINCIPLES.md §6.2](./PRINCIPLES.md#62-ux-principles)).
- **Honest states.** Loading, empty, error, and "I don't know" states are designed, not accidental. No spinner that lies; no generic "Error 500" to a citizen.
- **Forgiving input.** Typos, lowercase, informal Italian, voice-to-text gibberish — all accepted. The system never tells the user they "asked wrong".
- **Operator console (`admin-ui`) is equally intuitive** despite being for internal staff: progressive disclosure (simple first, advanced behind a disclosure), inline help (`?` tooltips on every field), destructive actions behind an explicit confirmation, and a consistent left-rail navigation (Ingest · Imprinting · Training) that never moves.
- **Tested with real users.** BDD scenarios (per [PRINCIPLES.md §5](./PRINCIPLES.md#5-bdd--behavior-driven-development)) include usability-oriented scenarios written with the operator's language; usability is verified, not asserted.

### 4.6 Reference: Design System Italia links

| Resource | URL |
|---|---|
| Design System Italia (main) | https://designers.italia.it/design-system/ |
| Getting started for developers | https://designers.italia.it/design-system/come-iniziare/per-sviluppatori/ |
| Bootstrap Italia docs | https://italia.github.io/bootstrap-italia/ |
| Bootstrap Italia GitHub | https://github.com/italia/bootstrap-italia |
| Design Tokens Italia | https://designers.italia.it/design-system/fondamenti/design-tokens/ |
| Accessibility (WCAG 2.1 AA) | https://designers.italia.it/design-system/fondamenti/accessibilita/ |

---

## 5. Containerization

Fully dockerized, **5 runtime containers**:

```yaml
# docker-compose.yml (conceptual excerpt)
services:
  backend:                # 1. core — axum, rag-engine, /chat + /admin/api/*
    build: ./backend
    ports: ["8080:8080"]
    volumes:
      - kb-data:/data      # kb.db
    depends_on:
      - llama-embed
      - llama-generate

  admin-ui:               # 2. operator SPA (Vue build served by nginx)
    build: ./admin-ui
    ports: ["5173:80"]
    depends_on:
      - backend

  ingest:                 # 3. always-on ingest service (long-running)
    build: ./ingest
    volumes:
      - kb-data:/data      # reads config + writes documents
    depends_on:
      - llama-embed

  llama-embed:            # 4. embedding inference
    image: ghcr.io/ggml-org/llama.cpp:server
    volumes:
      - ./models/embed:/models
    command: ["--model", "/models/nomic-embed-text-q4.gguf", "--embeddings"]

  llama-generate:         # 5. generation inference
    image: ghcr.io/ggml-org/llama.cpp:server
    volumes:
      - ./models/generate:/models
    command: ["--model", "/models/qwen2.5-7b-q4.gguf"]

volumes:
  kb-data:
```

Notes:
- `backend` and `ingest` share the `kb-data` volume (the `kb.db` file) but never call each other over the network. `backend` writes config rows; `ingest` polls them. Both write documents.
- `admin-ui` is a static SPA. Its nginx config reverse-proxies `/admin/api/*` to `backend:8080`.
- `ingest-cli` remains as a one-shot developer tool (`cargo run -p ingest-cli`), not a production container.
- `frontend` (public chat) is built and served separately (see §3.8); in production it can be hosted by the same nginx or a dedicated static host.

---

## 6. Cargo workspace layout

```
spontini-bot-2/
├── Cargo.toml            # workspace root
├── rust-toolchain.toml
├── LICENSE                # MIT
├── README.md              # project overview, quick start, architecture pointer
├── Makefile               # operator entry point — every target runs inside containers
├── backend/              # axum, rag-engine, /chat + /admin/api/* (core container)
├── ingest-core/          # shared ingest library (adapters: scraper, api-client; chunking; embedding calls)
├── ingest/               # always-on ingest service binary (scheduler + adapters) (ingest container)
├── ingest-cli/           # thin one-shot CLI binary over ingest-core (developer tool only)
├── kb-store/             # libSQL access layer, shared by backend and ingest
├── frontend/             # public chat Vue app (served to citizens)
├── admin-ui/             # operator-facing Vue SPA (ingest config, bot imprinting, training)
└── docker-compose.yml
```

---

## 7. Root-level project files

These three files live at the repository root and are the **public contract** of the project. They are mandatory; a repository without them is incomplete.

### 7.1 `LICENSE` — MIT

The project is released under the **MIT License** (SPDX identifier: `MIT`).

- The root `LICENSE` file contains the canonical MIT text (the same text as https://opensource.org/license/mit/ ).
- The copyright line uses the project's copyright holder and the current year.
- Every `Cargo.toml` and `package.json` in the workspace declares `license = "MIT"` / `"license": "MIT"`.
- No file in the repository carries a different, conflicting license header unless explicitly approved by an ADR (per [`.adr/`](./.adr/)). If a third-party snippet is vendored under a compatible license, its license is noted in the file header and recorded in a `THIRD_PARTY_NOTICES.md` (if/when one is introduced).

### 7.2 `README.md`

The project's front door. It is written in English (per [AGENTS.md §3.1](./AGENTS.md#31-language)) and must contain, in this order:

1. **Project name and one-line description** — what Spontini is, for whom (Comune di Maiolati Spontini citizens).
2. **Status badge** — build / test / coverage status from CI (added when CI exists).
3. **Mission pointer** — a short paragraph linking to [docs/CONSTITUTION.md](./docs/CONSTITUTION.md) and [docs/PRINCIPLES.md](./docs/PRINCIPLES.md). The README never duplicates the Constitution; it points to it.
4. **Prerequisites** — Docker + Docker Compose, Node LTS (for frontend dev only), Rust 1.96.1 (for native dev only), with the exact versions pinned in `.nvmrc` / `rust-toolchain.toml`.
5. **Quick start** — the minimum-viable path to a running system, expressed exclusively as `make` targets (see §7.3). No raw `docker compose` / `cargo` / `npm` incantations in the quick start — the Makefile is the only entry point.
6. **Architecture overview** — a one-paragraph summary + a pointer to [docs/STACK.md §2](./docs/STACK.md#2-architecture-overview). No duplication.
7. **Repository layout** — the tree from [§6](#6-cargo-workspace-layout), reproduced for orientation, with links into the docs.
8. **Contributing** — pointer to [AGENTS.md](./AGENTS.md), to the [opencode commands](./AGENTS.md#commands) (`/create-plan`, `/approve-plan`, `/implement-plan`, `/review-plan`, `/fix-review`), and to [docs/PRINCIPLES.md](./docs/PRINCIPLES.md) (TDD, BDD, coverage gate).
9. **License** — "MIT — see [LICENSE](./LICENSE)".

The README is a **living document**. It is updated in the same PR that introduces a breaking change to the architecture, the quick start, or the prerequisites.

### 7.3 `Makefile` — container-first operator entry point

The Makefile is the **single entry point** for every operator and developer action. Every target runs **inside the containers** — the host machine needs only Docker + Docker Compose + `make`, nothing else. No `cargo`, `npm`, or `node` command is ever invoked directly on the host by a Makefile target.

#### Rules (NON-NEGOTIABLE)

1. **Default target is `help`.** Running `make` with no arguments prints the available targets with a one-line description each. The `help` target is the first target in the file and the `.DEFAULT_GOAL := help` directive is set.
2. **Self-documenting.** Every target has a `## target: description` comment parsed by the `help` target (standard `awk`-based help generator). Targets without a description are internal and not shown.
3. **Container-first.** All commands that touch the codebase execute via `docker compose run --rm <service> <cmd>` or `docker compose exec <service> <cmd>`. The host never runs Rust/Node directly.
4. **No hidden state.** Every target is idempotent and cleans up after itself. `make` never leaves dangling containers, volumes, or build artifacts on the host.
5. **Idiomatic names.** `build`, `up`, `down`, `test`, `lint`, `fmt`, `check`, `clean`, `logs`, `shell`. No abbreviations that a newcomer cannot decode.
6. **One concern per target.** `test-backend` runs backend tests; `test-frontend` runs frontend tests; `test` depends on both. No mega-target.

#### Mandatory targets

| Target | What it does (inside containers) |
|---|---|
| `help` (default) | Prints all documented targets with descriptions. |
| `build` | `docker compose build` — builds all images. |
| `up` | `docker compose up -d` — starts the full stack. |
| `down` | `docker compose down` — stops the stack, preserves volumes. |
| `logs` | `docker compose logs -f` — tails logs from all services. |
| `shell` | Opens an interactive shell inside the `backend` container (override with `SERVICE=ingest make shell`). |
| `test` | Runs backend + ingest + kb-store + frontend test suites, each in its own container. |
| `test-backend` | `cargo test` inside the `backend` container (workspace tests). |
| `test-frontend` | `npm run test` inside the `frontend` container (and `admin-ui` when present). |
| `lint` | `cargo clippy` + `npm run lint` across the relevant containers. |
| `fmt` | `cargo fmt` + `npm run format` (write mode). |
| `fmt-check` | `cargo fmt --check` + `npm run format -- --check`. |
| `check` | `cargo check` (workspace) — fast compile gate. |
| `coverage` | `cargo tarpaulin` (or the chosen tool) inside the backend container; enforces the 100% line / 80% branch gate from [PRINCIPLES.md §7](./docs/PRINCIPLES.md#7-100-test-coverage-on-the-codebase). |
| `bdd` | Runs Gherkin scenarios end-to-end against the running stack (see [PRINCIPLES.md §5](./docs/PRINCIPLES.md#5-bdd--behavior-driven-development)). |
| `ingest-run` | Triggers an immediate ingest run via `/admin/api/ingest/run` against the running `backend`. |
| `migrate` | Runs the libSQL migrations inside the `backend` (or `ingest`) container. |
| `clean` | `cargo clean` + `rm -rf frontend/dist admin-ui/dist` inside the relevant containers; `docker compose down -v` only when `CLEAN_VOLUMES=1` is passed (destructive — requires the confirmation flag). |
| `verify` | The pre-completion gate from the `spontini-verify-gate` skill: `build` + `test` + `lint` + `fmt-check` + `coverage` + `docker compose config`. Runs end-to-end inside containers. |

#### Pattern (excerpt)

```makefile
.DEFAULT_GOAL := help

SERVICE ?= backend

## help: show this help
help:
	@awk 'BEGIN { \
		printf "Usage:\n  make \033[36m<target>\033[0m [SERVICE=<svc>]\n\nTargets:\n"; \
	} /^## / { \
		sub(/^## /, ""); split($$0, a, ":"); name = a[1]; \
		sub(/^[^:]*:/, "", $$0); sub(/^ /, "", $$0); \
		printf "  \033[36m%-16s\033[0m %s\n", name, $$0; \
	}' $(MAKEFILE_LIST)

## build: build all container images
build:
	docker compose build

## up: start the full stack in the background
up:
	docker compose up -d

## down: stop the stack (preserves volumes)
down:
	docker compose down

## test: run every test suite, in containers
test: test-backend test-frontend

## test-backend: cargo test (workspace) inside the backend container
test-backend:
	docker compose run --rm backend cargo test --workspace

## lint: clippy + eslint
lint:
	docker compose run --rm backend cargo clippy --workspace -- -D warnings
	docker compose run --rm frontend npm run lint

## fmt: format the whole codebase
fmt:
	docker compose run --rm backend cargo fmt
	docker compose run --rm frontend npm run format

## verify: pre-completion gate (build + test + lint + fmt-check + coverage + compose config)
verify: build test lint fmt-check coverage
	docker compose config -q

## shell: open a shell inside a service (SERVICE=ingest to switch)
shell:
	docker compose run --rm $(SERVICE) bash
```

**Constraint:** the Makefile is the only documented way to operate the project. Documentation, README, and CI must all use the `make` targets, never raw `docker compose` / `cargo` / `npm` commands. If a new operator action is needed, a new target is added; the README and CI are updated in the same change.
