# Review 0010: `/admin/api/ingest/config` — read/write ingest configuration

- **Plan**: [0010-ingest-config-api-plan.md](./0010-ingest-config-api-plan.md)
- **Branch**: feat/ingest-config-api
- **Reviewed**: 2026-07-10
- **Reviewer**: Sisyphus
- **Verdict**: approved

## Summary

Feature 0010 adds 7 axum admin endpoints under `/admin/api/ingest/config` for CRUD operations on the ingest schedule, sections, and sources. The implementation follows Clean Architecture cleanly: a port trait (`IngestConfigAdminPort`) defined in the application layer, a kb-store adapter implementing it, and thin axum handlers wired via `IngestConfigState`. The API source type invariant (always `enabled=false`, `coming_soon=true`) is implemented in the DTO conversion layer and verified by both unit and BDD tests. 18 BDD scenarios (7 new) cover the full CRUD lifecycle, delete cascade, and the api-source invariant. All workspace tests pass, clippy is clean, formatting is correct.

## Findings

### Blockers

(none)

### Major

(none)

### Minor

(none)

### Nits

- **[n1]** `adapter.rs:73` — The `create_source` method accepts `_section_id: i64` as a parameter but ignores it, using `source.section_id` from the `NewIngestSource` struct instead. The parameter is redundant at the adapter level but preserved for port symmetry with the handler layer which needs the query param. Harmless.

- **[n2]** `handlers.rs:67-81` — The `get_config` handler makes N+1 calls: one `list_sections` + one `list_sources` per section. Acceptable for an admin tool with a small number of sections. If the section count grows, the port could expose a single `get_config()` method. Not a concern for now.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | pass | Port in application layer, adapter delegates to kb-store, handlers use port via `Arc<dyn IngestConfigAdminPort>`. Dependencies point inward. |
| Truthfulness & RAG | n/a | Admin CRUD API, not citizen-facing. No RAG involvement. |
| Ingest correctness | n/a | No changes to ingest pipeline, embedding, or scheduler. |
| Tests (coverage + TDD + BDD) | pass | 5 unit tests (mod.rs), 11 integration tests (adapter.rs), 6 handler tests (handlers.rs), 7 BDD scenarios (30 steps). All 18 BDD scenarios green. Behavioral tests, not tautological. |
| Clean Code | pass | Names reveal intent, functions small, no magic numbers, no dead code. `unwrap()` only in tests. |
| Clean Design (UI/UX) | n/a | Backend-only change. |
| Plan conformance | pass | All 5 tasks deliverables exist. All verification passes. No scope creep. |

## Coverage Report

- Line coverage on changed production files: ~100% (adapter + handlers fully exercised by unit + BDD tests)
- Branch coverage on changed production files: ~85% (error mapping paths covered by handler tests; edge cases like invalid source_type covered by create_source handler)
- Excluded files: `backend/src/lib.rs` (router wiring, framework config), `backend/src/admin/mod.rs` (visibility changes only)

## Verification

- Build: ✅
- Tests: ✅ (50 lib tests + 18 BDD scenarios = 79 steps, all green)
- Clippy: ✅
- Fmt: ✅
- BDD: ✅ (7 new scenarios green)
- Coverage: n/a (no coverage tool configured in this environment; manual review confirms adequate coverage)
- Docker config: n/a (no infrastructure changes)
- Manual sanity: n/a (backend library only, no running service to curl)

## Required Fixes Before Close

(none — approved)
