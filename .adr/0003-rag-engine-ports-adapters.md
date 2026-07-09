# ADR 0003: RAG Engine as Backend Module with Ports/Adapters Architecture

- **Status**: accepted
- **Date**: 2026-07-09
- **Deciders**: Sisyphus (opencode)
- **Related**: 0001, 0002, 0003 (Feature ID)

## Context

The `/chat` endpoint was a walking-skeleton stub returning a hardcoded string. To fulfill the Constitution §1 mission — answering citizens from official municipal documents with source citations — we needed a Retrieval-Augmented Generation (RAG) flow. The system must embed queries, retrieve relevant chunks from `kb.db`, assemble a 3-part prompt, generate answers, and cite sources while honestly admitting when it doesn't know.

Key constraints:
- Constitution §5: No hallucination. If answer not found in documents, explicitly say so.
- Constitution §3: Every answer must trace back to a document.
- PRINCIPLES.md §2: Clean Architecture — ports/adapters, no framework types in domain.
- STACK.md §3.1: rag-engine lives as a module inside `backend`.

## Decision

We will implement the RAG engine as a module inside `backend` using Clean Architecture ports/adapters. The module contains domain types (`Answer`, `CitedSource`, `PromptParts`, `RagError`), port traits (`EmbeddingPort`, `RetrievalPort`, `PersonaPort`, `GenerationPort`), thin adapters (KbStore-backed for retrieval/persona, HTTP-backed for embedding/generation), 3-part prompt assembly, and `RagEngine` orchestration with honest-unknown fallback.

## Rationale

The ports/adapters pattern isolates the domain logic from infrastructure. Test doubles can replace adapters in tests, enabling full unit test coverage without live `llama.cpp` containers or `kb.db`. The 3-part prompt separation (`PromptParts` with separate `system`, `context`, `user` fields) is enforced structurally — only `GenerationAdapter` combines them into chat messages, and only with explicit delimiters. The honest-unknown path (`chunks.is_empty() → return fallback without calling generation`) provably satisfies Constitution §5.

## Consequences

### Positive

- Domain types are framework-agnostic (no `kb_store`, `reqwest`, or `axum` imports in `types.rs` or `ports.rs`)
- Full unit test coverage with test doubles — no live services needed
- Honest-unknown path is tested: `should_not_call_generation_when_no_chunks_found` proves generation is not called
- BDD scenarios validate both answerable and honest-unknown paths end-to-end

### Negative

- Adapter duplication between unit tests and BDD tests (shared test doubles not extracted to `backend/tests/support/`)
- `embedding.rs` imports `kb_store::EMBEDDING_DIM` directly — minor clean-arch deviation for cross-cutting constraint

### Neutral

- `router()` is async (due to `KbStore::open()` being async) — minor deviation from plan's sync spec
- Config defaults (`RAG_TOP_K=5`, `RAG_MIN_SCORE=0.35`) are magic numbers — should be named constants

## Alternatives Considered

### Alternative A: Separate Domain crate

Extract domain types into a `rag-engine-domain` crate. Rejected because only `backend` currently consumes these types — extraction deferred until a second consumer (e.g., admin surface) needs them.

### Alternative B: Direct kb-store coupling

Call `kb-store` directly in `RagEngine` without ports. Rejected because it violates Clean Architecture dependency rule and makes testing impossible without a real database.

## Compliance

The `spontini-clean-arch-guard` skill enforces that `backend/src/rag_engine/types.rs` and `ports.rs` have no `kb_store` imports. The `spontini-rag-build` skill enforces 3-part prompt separation and honest-unknown fallback. The `spontini-tdd-rust` skill enforces test-first development. The `spontini-verify-gate` skill validates build, test, clippy, and fmt at completion.
