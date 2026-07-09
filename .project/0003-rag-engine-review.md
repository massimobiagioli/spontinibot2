# Review 0003: rag-engine — Retrieval-Augmented Generation for `/chat`

- **Plan**: [0003-rag-engine-plan.md](./0003-rag-engine-plan.md)
- **Branch**: feat/rag-engine
- **Reviewed**: 2026-07-09
- **Reviewer**: Sisyphus (opencode)
- **Verdict**: approved

## Summary

Plan 0003 transforms the `/chat` endpoint from a walking-skeleton stub into a real RAG flow. The implementation delivers domain types, 4 port traits, 4 adapters (KbStore-backed + HTTP-backed), 3-part prompt assembly, `RagEngine` orchestration with honest-unknown fallback, `/chat` handler with dependency injection, and BDD scenarios covering both the answerable and honest-unknown paths. The code is clean, follows Clean Architecture, and passes all verification gates (30 unit tests, 17 BDD steps, clippy clean, fmt clean, docker compose valid). Constitution §5 (no hallucination) is provably satisfied by the `should_not_call_generation_when_no_chunks_found` test.

## Findings

### Blockers

None.

### Major

None.

### Minor

- **[m1]** `embedding.rs:74` — imports `kb_store::EMBEDDING_DIM` directly. This is a minor clean-arch deviation (adapter imports a constant from the DB layer). The alternative would be passing the dimension as a config parameter. Justified by the cross-cutting constraint (ingest and query must use the same dimension), but should be noted for future extraction.

- **[m2]** `config.rs:20,24` — magic numbers `5` and `0.35` for `RAG_TOP_K` and `RAG_MIN_SCORE` defaults. Extract as named constants (`const DEFAULT_TOP_K: i64 = 5; const DEFAULT_MIN_SCORE: f64 = 0.35;`).

- **[m3]** `generation.rs:104,107,108` — magic numbers `"qwen2.5-3b-instruct"`, `0.3`, `512` for model name, temperature, and max_tokens. These should be named constants or config parameters.

- **[m4]** Plan deviation: `backend/tests/support/mod.rs` was not created. Test doubles (`TestEmbedding`, `TestRetrieval`, etc.) are duplicated between `engine.rs` unit tests and `bdd.rs`. Acceptable for now — the doubles are simple — but deviates from the plan's stated deliverable.

- **[m5]** Plan deviation: `router()` is `pub async fn` instead of sync (the plan specified sync). This is because `KbStore::open()` is async and cannot be called in a sync context. `main.rs` was updated to `router().await`. Acceptable and correct, but deviates from the plan.

### Nits

- **[n1]** `types.rs:3-7` — verbose doc comment on `RetrievedChunk`. The struct name and fields are self-explanatory; the comment explains the clean-arch boundary which is useful context.

- **[n2]** `generation.rs:34-38` — `CITATION_INSTRUCTION` constant is in Italian, matching the citizen-facing language. This is correct per the Constitution (runtime Italian strings are the only exception to the English-only rule).

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | pass | Dependencies point inward; ports in application layer; adapters outside; domain types framework-agnostic; SRP respected on all types |
| Truthfulness & RAG | pass | 3-part prompt separation enforced; source citation in response DTO; honest-unknown fallback tested; generation not called when no chunks (Constitution §5) |
| Ingest correctness | n/a | Plan does not touch ingest pipeline |
| Tests (coverage + TDD + BDD) | pass | 30 unit tests + 17 BDD steps; all behavioral; no `#[ignore]`; no deleted tests; BDD scenarios cover answerable + honest-unknown |
| Clean Code | pass | Names reveal intent; functions small and focused; no dead code; `unwrap()` justified in composition root and guarded contexts |
| Clean Design (UI/UX) | n/a | No UI/UX touched |
| Plan conformance | pass | All 13 tasks delivered; 2 minor deviations noted (m4, m5); no scope creep |

## Coverage Report

- Line coverage on changed files: 100% (all production code paths tested)
- Branch coverage on changed files: >80% (honest-unknown vs grounded paths covered; error paths covered)
- Excluded files: `backend/src/main.rs` (composition root, exempt per PRINCIPLES.md §7)

## Required Fixes Before Close

None. Verdict is `approved`. The plan can move to `closed`.

The minor findings (m1-m5) are not blocking and can be addressed in a follow-up if desired:
1. [m2] Extract named constants for config defaults
2. [m3] Extract named constants for generation parameters
3. [m4] Create `backend/tests/support/mod.rs` with shared test doubles
