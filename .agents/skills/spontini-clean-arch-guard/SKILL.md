---
name: spontini-clean-arch-guard
description: Clean Architecture dependency-rule guard for the Spontini Rust workspace. Use WHEN adding a crate, a module, a port/adapter, or any `use`/`use crate` import. Enforces that dependencies point only inward toward the domain and that no framework types leak into domain or application code.
---

# Spontini Clean Architecture Guard

You are adding a module, a crate, a port, an adapter, or any import in this workspace. Load this skill to keep the dependency rule intact.

## The Workspace Crates

```
backend/        — axum HTTP, rag-engine, admin-ui routes
ingest-core/    — shared ingest library (adapters, chunking, embed calls)
ingest-cli/     — thin CLI binary over ingest-core
kb-store/       — libSQL access layer, shared by backend and ingest-core
frontend/       — Vue app (not Rust; own rules below)
```

## The Dependency Rule

Source code dependencies point **only inward**:

```
Domain  ←  Application  ←  Interface Adapters  ←  Frameworks & Drivers
```

### Crate-level dependency matrix (enforced)

| Crate           | May depend on                  | May NOT depend on                          |
|-----------------|--------------------------------|--------------------------------------------|
| Domain modules | std + domain modules           | any framework, any adapter, `axum`, `libsql`, `reqwest`, `tokio` (only `tokio::sync` types if async trait needed, documented case-by-case) |
| Application (use cases) | Domain + Port traits   | concrete adapters, framework runtime types |
| `kb-store`      | Domain, `libsql` crate         | `axum`, `backend`, `ingest-*`              |
| `ingest-core`   | Domain, `kb-store` (via port), Port traits | `axum`, `ingest-cli`           |
| `ingest-cli`    | `ingest-core`, Port trait impls | —                                          |
| `backend`       | Application, `axum`, adapter impls, `kb-store` | —                              |

### Forbidden imports (auto-reject in review)

- Any `use axum::*`, `use sqlx::*`, `use libsql::*`, `use reqwest::*`, `use tokio::*` inside a Domain or Application module.
- Any `use crate::infrastructure::*` (or equivalent) inside a Domain or Application module.
- `backend` importing `ingest_cli`.
- `ingest-core` importing `backend`.
- A Domain type carrying an ORM annotation, a serde derive tied to a transport, or a framework trait.

## Ports and Adapters

- **Ports** are traits defined in the Application layer (or Domain, for pure domain policy). They describe what the system needs (`RetrievalPort`, `EmbeddingPort`, `LlmPort`, `DocumentPort`).
- **Adapters** are concrete implementations of ports, living outside the application layer (`OllamaEmbeddingAdapter`, `LibSqlRetrievalAdapter`).
- Use cases depend on ports, never on adapter types. Adapters are injected at the composition root in `backend`.

## DTO Boundary Rule

When data crosses a layer boundary, it travels in a purpose-built DTO. Domain entities never carry framework annotations:

- Controller receives a request DTO → calls use case → use case returns a response DTO → controller serializes it.
- Never return a Domain entity directly from an HTTP handler.
- Never annotate a Domain entity with serde derives used for transport. Use a dedicated response type.

## Workflow When Adding a Crate or Module

1. **Identify the layer.** Domain? Application? Adapter? Framework?
2. **Check the matrix.** Does the proposed dependency point inward? If not, invert it: define a port in the inner layer, implement it in the outer one.
3. **Name by responsibility.** `Ingestor`, `Retriever`, `Embedder` — one verb, one noun. No `Manager`, no `Service` without a single-sentence responsibility.
4. **Verify imports compile under the rule.** If `cargo build -p <crate>` fails because an inner crate tried to import an outer one, the architecture is wrong, not the import.
5. **Add the wiring at the composition root**, not scattered across modules.

## Single Responsibility Audit

Any type named `*Service`, `*Manager`, `*Handler` must be auditable:

> "This type has exactly one reason to change: ___"

If you cannot fill the blank with one clause, split the type.

## Forbidden

- `as Any`, `as any` casts to bypass the type system.
- God traits with more than 5 methods — segregate.
- Adapter types appearing in use case signatures.
- Domain modules importing `tokio` runtime types (only `tokio::sync` and only with justification).
- Circular dependencies between crates — the workspace is a DAG pointing inward.
