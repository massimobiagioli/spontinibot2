# Review 0011: `/admin/api/ingest/run` — trigger an immediate ingest run

- **Plan**: [0011-admin-api-ingest-run-plan.md](./0011-admin-api-ingest-run-plan.md)
- **Branch**: feat/admin-api-ingest-run
- **Reviewed**: 2026-07-24
- **Reviewer**: Sisyphus
- **Verdict**: approved

## Summary

Implements `POST /admin/api/ingest/run` (202, writes a pending run-request row) and `GET /admin/api/ingest/run/:id` (200/404) behind the existing `X-Admin-Key` guard, following the exact Clean Architecture shape of feature 0010 (`IngestRunAdminPort` trait + `KbStoreIngestRunAdapter`, thin axum handlers). Adds the one missing `kb-store` read method (`get_run_request`) needed to support polling. BDD scenarios cover trigger→pending, the full pending→running→done transition (driven directly against `kb-store` to simulate what the `ingest` service is meant to do), an unknown-id 404, and missing-auth 401 on both endpoints. All new code is small, well-tested, and consistent with surrounding conventions. Ships as-is.

## Findings

### Blockers

None.

### Major

None.

### Minor

None.

### Nits

- **[n1]** `backend/src/admin/ingest_run/mod.rs:27` — `IngestRunError` has a single variant (`DbError`). This mirrors `IngestConfigError`'s two-variant shape minus `NotFound`, which is correct here since "not found" is modeled as `Ok(None)` per the trait signature rather than an error — just noting the asymmetry with the sibling module is intentional, not an oversight.
- **[n2]** `backend/tests/bdd.rs` — the new `given_ingest_run_api_available` step duplicates the router-assembly boilerplate already present in `build_admin_router`/`build_upload_router`/`given_ingest_config_api_available` (same stub embed/retrieval/generation wiring, same `Config` literal). Pre-existing duplication pattern in this file, not introduced by scope creep in this change; a future refactor could extract a single `build_test_router(db_path, admin_key)` helper, but that's out of scope for this plan.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | pass | `IngestRunAdminPort` defined in `backend/src/admin/ingest_run/mod.rs`, implemented by `KbStoreIngestRunAdapter` in `adapter.rs`. Handlers depend only on the port trait object, never on the concrete adapter or `kb_store::KbStore` directly. Dependency direction is inward: `kb-store` has no knowledge of `backend`. No framework types (`axum`, `Json`) leak into `mod.rs`'s trait or response types — `IngestRunResponse` is a plain DTO with serde derives only. |
| Truthfulness & RAG | n/a | This feature does not touch retrieval, generation, persona, or the citizen `/chat` answer path. |
| Ingest correctness | pass (with documented gap) | The endpoint correctly delegates to `kb-store`'s existing `request_run`/`get_run_request`. The plan explicitly documents (Objective, Non-Goals, Risks) that the `ingest` service's scheduler does not yet consume `ingest_run_request` rows — a pre-existing gap in feature 0006, not hidden or newly introduced by this change. This is the right call: fixing the `ingest` scheduler is out of scope for an Admin Surface (Backend) feature and is called out honestly rather than glossed over. |
| Tests (coverage + TDD + BDD) | pass | `kb-store`: 4 new tests for `get_run_request` (pending/running/done/unknown-id). `backend`: 2 unit tests on `IngestRunResponse`/`IngestRunError`, 3 adapter tests, 5 handler tests (401×2, 202, 200, 404) — every branch of every new function is exercised. 5 new BDD scenarios in `features/admin_ingest_run.feature` cover trigger→pending, the full pending→running→done polling lifecycle, unknown-id 404, and missing-auth 401 on both routes. All 23 scenarios / 97 steps across the whole BDD suite pass; no `#[ignore]`, no deleted or weakened tests. |
| Clean Code | pass | Names are intent-revealing (`trigger_run`, `get_run`, `KbStoreIngestRunAdapter`). Functions are small and single-purpose. No magic numbers. No `unwrap()` in production code paths (only in test helpers, consistent with existing test style in this file). |
| Clean Design (UI/UX) | n/a | No UI touched — this is a backend-only admin API feature (admin-ui section is feature 0016). |
| Plan conformance | pass | All 6 tasks across 4 phases are checked off with their exact stated deliverables present: `KbStore::get_run_request` (Task 1.1), `IngestRunAdminPort`/`IngestRunResponse`/`IngestRunError` (Task 2.1), `KbStoreIngestRunAdapter` (Task 2.2), `IngestRunState`/handlers (Task 3.1), router wiring in `lib.rs`/`admin/mod.rs` (Task 3.2), BDD scenarios (Task 4.1). No unrequested scope creep — the `ingest` service was deliberately left untouched per the plan's Non-Goals. |

## Coverage Report

- Line coverage on changed files: not mechanically measured — `cargo tarpaulin` requires the Docker-based `make coverage` target, and Docker is not available in this environment. Every new function (`get_run_request`, `trigger_run`, `get_run`, `KbStoreIngestRunAdapter::{trigger_run,get_run}`, both handlers, both `From`/`Display` impls) has at least one direct test, and every conditional branch (Some/None; all 4 `RunRequestStatus` variants; auth pass/fail; 404 vs 200) is exercised by an existing test. High confidence in 100%/80%+ coverage, but this is an assessment, not a tool-verified number.
- Branch coverage on changed files: see above — not mechanically measured, same reason.
- Excluded files: none proposed for `coverage-exclusions.txt`.

## Required Fixes Before Close

None. Verdict is `approved`; `/fix-review 0011` can close the plan directly (no findings to fix).
