# Review 0002: kb-store libSQL Implementation

- **Plan**: [0002-kb-store-impl-plan.md](./0002-kb-store-impl-plan.md)
- **Branch**: feat/kb-store-impl
- **Reviewed**: 2026-07-09
- **Reviewer**: Sisyphus (opencode)
- **Verdict**: approved

## Summary

Transforms `kb-store` from a version-string skeleton to a working libSQL access layer with idempotent schema migrations, full Document CRUD (including vector similarity search via `vector_distance_cos`), and versioned Persona CRUD (inserts only, never UPDATE). Implements all 13 tasks across 4 phases. The code is clean, well-tested (33 tests + 1 doc-test, ~95% estimated line coverage), follows the Clean Architecture dependency rules, and matches the STACK.md §3.5 schema exactly. No blockers or majors. Ready to close.

## Findings

### Blockers

*(none)*

### Major

*(none)*

### Minor

- **[m1]** `kb-store/src/lib.rs:35-306` — `KbStore` combines document CRUD and persona CRUD in one struct. It has two responsibilities (documents + personas), a minor SRP concern per PRINCIPLES.md §3. The scale is small enough that this is practical for now, but when new document or persona operations are added, `KbStore` should be split into `DocumentStore` and `PersonaStore`. The plan acknowledges this (domain types live in `kb-store` for now); a follow-up plan should extract them.

- **[m2]** `kb-store/src/lib.rs:168-232` — `insert_persona()` is 64 lines, the longest function in the crate. It does three things in one function: version computation, optional deactivation, insert + created_at re-read. Consider extracting two helpers: `compute_next_version(conn, name)` and a simplified `insert_persona_row(...)`.

- **[m3]** `kb-store/src/lib.rs:526-562` — `should_return_similar_documents_when_searching` inserts two documents with different embeddings (`vec_a`, `vec_b`), queries with `vec_a`, but only asserts the first result is one of the two inserted docs. It does **not** assert that `doc_a` (the identical vector) ranks first. The test verifies the search returns results, but not the ranking quality. Add an assertion that `results[0].document.source_ref == "doc_a"`.

### Nits

- **[n1]** `kb-store/src/lib.rs:9-11` — The doc-test uses `# async fn example()` with `# #![allow(unused_imports)]` pattern, which is standard. However it references `/data/kb.db`, a path that would fail on non-containerized hosts. Adding `no_run` was correct. Consider making the example self-contained by using a temp path or adding a clarifying comment.

- **[n2]** `kb-store/src/lib.rs:186,296` — `libsql::params![]` with an empty argument list is visually unusual but correct (these are `UPDATE` statements with no parameters). Consider adding a brief inline comment like `// no params` for readability.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | pass | Dependencies point inward. `kb-store` depends only on `libsql` + std. No `axum`, `backend`, `ingest-*` imports. Domain types are framework-agnostic. Minor SRP note (m1). |
| Truthfulness & RAG | n/a | No RAG flow or prompt assembly touched. Persona schema correctly includes `fallback_message` column for honest-unknown fallback, but its wiring to the prompt is in a future plan. |
| Ingest correctness | n/a | No ingest code touched. Plan explicitly out-of-scope. |
| Tests (coverage + TDD + BDD) | pass | 33 unit tests + 1 doc-test. Every public method has one or more dedicated tests covering normal and error paths. Tests follow `should_*` naming, AAA structure, isolated (unique temp DB per test). TDD pattern respected. Minor gap: vector search test does not assert ranking order (m3). No BDD scenarios — appropriate for a library crate (not user-visible). |
| Clean Code | pass | Meaningful names, no dead code, no magic numbers (uses `EMBEDDING_DIM` constant), no `unwrap()` in production code. Functions are mostly small; `insert_persona()` at 64 lines is the outlier (m2). |
| Clean Design (UI/UX) | n/a | Backend library only. No UI/UX touched. |
| Plan conformance | pass | All 13 tasks delivered and verified. Schema matches STACK.md §3.5 exactly. No scope creep — only `kb-store/` modified. |

## Coverage Report

- Estimated line coverage on changed files: **~95%** (no formal coverage tool available in this environment; based on manual analysis)
- Every production function and every error path has at least one test
- Untested paths identified:
  - `row_to_document()`'s `source_str.parse::<DocumentSource>()` error branch — requires a manually corrupted database, classified as `Migration` error
  - `insert_persona()`'s `get::<i32>(0).unwrap_or(1)` on NULL result — impossible under the schema
- Excluded files: none
- Formal coverage measurement with `cargo tarpaulin` should be run before CI merge to confirm ≥80% branch coverage

## Required Fixes Before Close

Verdict is **approved** — no fixes required. The minor findings (m1-m3) and nits (n1-n2) can be addressed in a follow-up or during the next touch of `kb-store/`.

To close, run `/fix-review 0002` or close the plan manually.
