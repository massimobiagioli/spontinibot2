# Spontini Bot 2

**Spontini** is a conversational AI assistant for the **Comune di Maiolati Spontini**. It answers citizens' questions exclusively from official municipal documents, citing the source of every answer. Fully local LLM stack, no GPU required. Fully containerized.

---

## Status

[![Build](https://github.com/massimobiagioli/spontinibot2/actions/workflows/ci.yml/badge.svg)](https://github.com/massimobiagioli/spontinibot2/actions/workflows/ci.yml)
[![Tests](https://github.com/massimobiagioli/spontinibot2/actions/workflows/ci.yml/badge.svg)](https://github.com/massimobiagioli/spontinibot2/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/badge/coverage-100%25%20line%20%2F%2080%25%20branch%20gate-brightgreen)](https://github.com/massimobiagioli/spontinibot2/actions/workflows/ci.yml)

> The CI workflow (`.github/workflows/ci.yml`) runs `make verify` on every push and pull request. The Coverage badge reflects the enforced gate threshold, not a live percentage — see the workflow run for the actual `cargo tarpaulin` output.

---

## Mission

Spontini exists to give every citizen of Maiolati Spontini a trustworthy, always-available channel to consult the municipal knowledge base — in plain Italian, with verifiable sources, and without ever inventing details.

The mission, identity, scope, and decision-making criteria are binding and live in **[docs/CONSTITUTION.md](./docs/CONSTITUTION.md)**. The engineering and design standards live in **[docs/PRINCIPLES.md](./docs/PRINCIPLES.md)**. This README is a pointer; it never duplicates those documents.

---

## Prerequisites

The host needs only three things:

| Tool | Version | Why |
|---|---|---|
| **Docker** | Latest stable | Runs every container. |
| **Docker Compose** | v2 (bundled with Docker Desktop / `docker compose` plugin) | Orchestrates the 5-runtime-container stack. |
| **GNU Make** | Any recent version | The only entry point for operator actions. |

For native development (optional — the Makefile runs everything inside containers):

| Tool | Version | File |
|---|---|---|
| Rust | **1.96.1** (stable) | pinned in [`rust-toolchain.toml`](./rust-toolchain.toml) |
| Node.js | Current LTS (never legacy) | pinned in `frontend/.nvmrc` and `admin-ui/.nvmrc` |
| Edition | Rust 2024 | `Cargo.toml` |

Legacy versions of any dependency are forbidden (see [docs/STACK.md §1](./docs/STACK.md#1-language-and-runtime)).

---

## Quick start

Everything runs through the `Makefile`. The default target is `help`.

```bash
# See every available action
make

# Build all container images
make build

# Start the full stack (backend, admin-ui, ingest, llama-embed, llama-generate)
make up

# Tail logs from every service
make logs

# Stop the stack (volumes preserved)
make down

# Run the full verification gate (build + test + lint + fmt-check + coverage + compose config)
make verify

# End-to-end BDD against the live stack — real llama-embed / llama-generate,
# not test doubles. Requires `make provision-models` and `make up` first;
# not part of `make verify` or CI (needs multi-gigabyte models + a live stack).
make bdd-e2e
```

Once the stack is up:

- **Public chat** (citizens): served by the `frontend` container — http://localhost:5174 (see [docs/STACK.md §3.8](./docs/STACK.md#38-frontend-public-chat--vue-3--vite--typescript))
- **Operator console** (admin-ui): http://localhost:5173 — Ingest configuration · Bot imprinting · Training (see [docs/STACK.md §3.2](./docs/STACK.md#32-admin-ui--admin-ui-separate-container))
- **Backend API**: http://localhost:8080 — `/chat` (public) + `/admin/api/*` (protected)

The first build downloads the GGUF model files into `models/embed/` and `models/generate/` — see [docs/STACK.md §5](./docs/STACK.md#5-containerization). Until those models are present, the inference containers will refuse to start.

### Production

`docker-compose.yml` alone is dev-first: `backend`/`ingest` build to their `target: build` stage (full Rust toolchain) so `docker compose run --rm <svc> cargo ...` works without installing Rust on the host. `docker-compose.prod.yml` is an overlay for what actually ships: `target: runtime` (slim, non-root) for every owned service, memory/CPU limits sized for the Mac Intel i7 / 16 GB RAM target, and healthchecks for every one of the 6 containers.

```bash
# Build and start the hardened, resource-limited production stack
make prod-build
make prod-up

# Stop it (volumes preserved)
make prod-down

# Zero-HIGH/CRITICAL-CVE gate on every owned image (run `make prod-build` first)
make scan
```

---

## Architecture overview

Five runtime containers, plus a shared `kb.db` volume that is the **only** coupling between the chat flow and the ingest flow.

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

The chat flow and the ingest flow never communicate directly; they share only the `kb.db` file.

Full specification: **[docs/STACK.md §2 — Architecture overview](./docs/STACK.md#2-architecture-overview)**.

---

## Repository layout

```
spontini-bot-2/
├── Cargo.toml            # workspace root
├── rust-toolchain.toml
├── LICENSE                # MIT
├── README.md              # this file
├── Makefile               # operator entry point — every target runs inside containers
├── AGENTS.md              # root index for every AI agent
├── docs/                  # CONSTITUTION, PRINCIPLES, STACK
├── .adr/                  # Architecture Decision Records
├── .agents/skills/        # project skills (TDD, BDD, clean-arch, rag, ingest, verify)
├── .opencode/commands/    # plan / approve / implement / review / fix commands
├── backend/              # axum, rag-engine, /chat + /admin/api/* (core container)
├── ingest-core/          # shared ingest library (adapters: scraper, api-client; chunking; embedding calls)
├── ingest/               # always-on ingest service binary (scheduler + adapters) (ingest container)
├── ingest-cli/           # thin one-shot CLI binary over ingest-core (developer tool only)
├── kb-store/             # libSQL access layer, shared by backend and ingest
├── frontend/             # public chat Vue app (served to citizens)
├── admin-ui/             # operator-facing Vue SPA (ingest config, bot imprinting, training)
├── models/               # GGUF model files (embed + generate)
└── docker-compose.yml
```

---

## Contributing

This repository is governed by [AGENTS.md](./AGENTS.md) — read it first.

### Workflow

Feature work follows the opencode plan lifecycle:

1. **[/create-plan](./.opencode/commands/create-plan.md)** — create a feature plan on a `feat/<name>` branch (status: `draft`).
2. **[/approve-plan](./.opencode/commands/approve-plan.md)** — transition the plan to `open`.
3. **[/implement-plan](./.opencode/commands/implement-plan.md)** — implement phase by phase (status → `review`).
4. **[/review-plan](./.opencode/commands/review-plan.md)** — code-review the implementation; produces a review file.
5. **[/fix-review](./.opencode/commands/fix-review.md)** — apply review fixes (status → `closed`).

Architecture decisions are recorded as ADRs via **[/create-adr](./.opencode/commands/create-adr.md)**.

### Engineering standards

All work complies with [docs/PRINCIPLES.md](./docs/PRINCIPLES.md). In short:

- **Clean Code, Clean Architecture, SOLID** — dependencies point inward toward the domain.
- **TDD** — Red-Green-Refactor. No production code without a failing test.
- **BDD** — Gherkin scenarios written *before* implementation, wired to real use cases.
- **Clean Design** — Jobs-era Apple aesthetic; Design System Italia on every UI surface.
- **100% line coverage / 80% branch coverage** on production code — enforced by CI and by `make coverage`.

The full pre-completion gate runs with:

```bash
make verify
```

This invokes the `spontini-verify-gate` skill's checks end-to-end, inside containers.

---

## License

MIT — see [LICENSE](./LICENSE).

Copyright (c) 2026 Massimo Biagioli.
