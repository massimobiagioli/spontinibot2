# ADR 0004: libSQL as Storage Layer with Vector Search and Versioned Persona

- **Status**: accepted
- **Date**: 2026-07-09
- **Deciders**: Sisyphus (opencode)
- **Related**: Feature 0002

## Context

Spontini needs a local storage layer for documents, persona configuration, and (later) ingest configuration. The [Constitution §1](../docs/CONSTITUTION.md#1-mission) requires a fully local stack with no external database servers. The RAG flow ([Feature 0003](../.project/0003-rag-engine-plan.md)) requires vector similarity search to retrieve relevant document chunks. The persona system needs versioned configuration with exactly one active version at a time.

The storage layer must be shared between `backend` (RAG retrieval) and `ingest` (document insertion), with a clean public API that other crates consume without depending on database internals.

## Decision

We will use libSQL (SQLite-compatible, embedded) as the sole storage engine. Embeddings are stored as `F32_BLOB(768)` columns and searched via `vector_distance_cos`. Persona uses an append-only versioned pattern: new versions are inserted (never updated), and a partial unique index enforces at most one active version. Domain types (`Document`, `Persona`, etc.) live in the `kb-store` crate. A hand-rolled embedded-SQL migration runner applies schema changes idempotently on startup.

## Rationale

libSQL is SQLite-compatible (no separate server process), supports `F32_BLOB` for binary vector storage, and provides `vector_distance_cos` for native cosine similarity search. This satisfies the Constitution's local-stack requirement and eliminates the need for a separate vector database. The versioned persona pattern avoids UPDATE races and provides a natural audit trail. Keeping domain types in `kb-store` keeps the dependency graph simple — downstream crates (`backend`, `ingest`) depend on `kb-store` for both data access and domain types.

## Consequences

### Positive

- No external database server — the entire stack runs from a single `kb.db` file
- Vector search is native to the storage engine — no separate vector DB or index
- Versioned persona provides a complete audit trail of configuration changes
- Embedded migration runner is idempotent and requires no external tooling
- Single `KbStore` struct consumed by both `backend` and `ingest`

### Negative

- libSQL is pre-1.0 — the Rust crate API may change between versions
- `F32_BLOB(768)` has a fixed dimension — changing the embedding dimension requires a migration
- Domain types in `kb-store` may need extraction to a separate Domain crate when the type graph grows
- The embedded migration runner is hand-rolled, not using a migration framework

### Neutral

- Each `KbStore` method opens its own connection via `db.connect()` — the caller provides the async runtime (tokio)

## Alternatives Considered

### Alternative A: PostgreSQL with pgvector

A full PostgreSQL database with the pgvector extension for vector search. Rejected because it requires a separate database server process, violating the Constitution's local-stack requirement and adding operational complexity for a single-operator system.

### Alternative B: SurrealDB

A multi-model database with built-in vector search. Rejected because it is not SQLite-compatible, adds a separate server process, and its Rust SDK is less mature than the libSQL crate.

### Alternative C: Separate Domain crate from the start

Extract `Document`, `Persona`, and other domain types into a dedicated `domain` crate immediately. Rejected because it adds a crate and a dependency edge before the type graph warrants it. The types can be extracted later when a Domain crate is introduced.

## Compliance

The `spontini-clean-arch-guard` skill enforces that `kb-store` depends only on `libsql` and `std` — no framework types leak into domain or application code. The `EMBEDDING_DIM` constant in `kb-store` enforces the 768-dimension constraint at insert time. Unit tests validate the versioned persona pattern (inserts only, never UPDATE) and the vector search round-trip.
