# Review 0014: `/admin/api/training/feedback` — point-in-answer feedback

- **Plan**: [0014-admin-api-training-feedback-plan.md](./0014-admin-api-training-feedback-plan.md)
- **Branch**: feat/admin-api-training-feedback
- **Reviewed**: 2026-07-24
- **Reviewer**: Sisyphus
- **Verdict**: changes-requested

## Summary

The feature adds a `training_feedback` table (V5 migration), a `Sentiment` enum following the exact `SourceType` precedent, a `KbStoreTrainingFeedbackAdapter`, and the `POST /admin/api/training/feedback` / `GET /admin/api/training/messages/:id/feedback` endpoints, correctly extending the `AdminRouterState` struct introduced in feature 0013 rather than growing `router_with`'s parameter list again. Architecture, DTO boundaries, and plan conformance are solid and consistent with the established 0012/0013 patterns. The one blocking issue repeats the exact class of gap found in the 0013 review: the `TrainingFeedbackError::DbError` mapping branch is never exercised by a test, in both the handler and (reachably, via an invalid `chunk_id`) the adapter.

## Findings

### Blockers

None.

### Major

- **[M1]** `backend/src/admin/training_feedback/handlers.rs:26-40` and `backend/src/admin/training_feedback/adapter.rs:19-32` — The `TrainingFeedbackError::DbError` arm of `map_feedback_error` is never hit by any test, and the underlying `DbError` path is reachable and untested at the adapter layer too: `create_feedback`'s `chunk_id` is a nullable FK to `documents.id` (per `V5__training_feedback.sql`) with no existence check before insert (unlike `message_id`, which is explicitly checked), so a `chunk_id: Some(id)` referencing a non-existent document violates the FK and surfaces as `KbStoreError` → `TrainingFeedbackError::DbError` via the `?` conversion — a real, reachable error path, not a phantom one. Expected: PRINCIPLES.md §7 requires "every `if`, `catch`, `switch` case... has a test for both sides." Actual: `adapter::tests` covers success (with/without chunk and comment), message-not-found, and listing, but never an invalid `chunk_id`; `handlers::tests` covers 401, 201, 404, and 400, but never 500. Fix: add an adapter test that calls `create_feedback` with a `chunk_id` that does not correspond to any inserted document and asserts an `Err(TrainingFeedbackError::DbError(_))`; add a handler test with a mock `TrainingFeedbackAdminPort` returning `TrainingFeedbackError::DbError`, asserting `500`.

### Minor

- **[m1]** `kb-store/src/lib.rs` (`create_training_feedback`/`list_training_feedback`, the `.parse().map_err(KbStoreError::Migration)?` lines) — the stored-`sentiment`-column parse failure is mapped with the bare parse error message, unlike the established precedent for the same situation on `IngestSource`'s `source_type` column (`kb-store/src/lib.rs:443`, `KbStoreError::Migration(format!("invalid source_type in db: {e}"))`), which wraps the error with context identifying which column/table it came from. Suggested fix: match the existing convention — `.map_err(|e| KbStoreError::Migration(format!("invalid sentiment in db: {e}")))?`.

### Nits

None.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | pass | `KbStoreTrainingFeedbackAdapter` lives in `backend`, depends inward on `kb_store::KbStore` only, matching the crate matrix. `TrainingFeedbackAdminPort::create_feedback` takes `kb_store::NewTrainingFeedback` directly as its request type — this mirrors the already-approved precedent in `TrainingSessionAdminPort::create_session` (feature 0012, `backend/src/admin/training_sessions/mod.rs:55-58`), not a new violation. The response side is a purpose-built `TrainingFeedbackResponse` DTO (`sentiment` serialized as `String`, never the raw `kb_store::Sentiment` enum) — DTO boundary respected. `AdminRouterState` is extended with one new field rather than adding an 8th/9th parameter to `router_with`, exactly per the mechanism introduced in 0013. |
| Truthfulness & RAG | n/a | This feature does not call `RagEngine` or touch the retrieval/generation/persona flow — it only persists operator-supplied metadata (span, sentiment, optional comment) about an already-recorded `training_message`. |
| Ingest correctness | n/a | Not touched — `chunk_id` is a plain nullable FK to `documents.id`, no ingest pipeline interaction. |
| Tests (coverage + TDD + BDD) | fail | See M1. TDD was followed for every other path (RED confirmed by compile failures at each new method/type before implementation). BDD: 7 new scenarios in `features/admin_training_feedback.feature` cover positive feedback, negative feedback with a comment, chunk-anchored feedback, invalid-sentiment rejection (400), unknown-message rejection (404), and missing-auth on both submit and list — all 45 scenarios / 212 steps in the full suite pass. No `#[ignore]`, no deleted tests, no hardcoded assertions. Mechanical coverage measurement (`cargo tarpaulin`) remains unavailable in the `backend` container image, the same pre-existing infra gap noted in the 0011/0012/0013 reviews; the M1 gap was found by manual enumeration. |
| Clean Code | pass | Names reveal intent (`KbStoreTrainingFeedbackAdapter`, `Sentiment`). No magic numbers, no dead code, no `unwrap()` in production code. `Sentiment`'s `Display`/`FromStr` impl is a verbatim structural match to the existing `SourceType` enum, keeping the codebase consistent. See m1 for the one wrapping-context nit. |
| Clean Design (UI/UX) | n/a | No UI touched — admin-ui's Training section (feature 0018) will build the span-selection UI against this API. |
| Plan conformance | pass | Every task's deliverables exist and every task's stated verification passed, including the plan's explicit instruction to add `KbStore::get_training_message` inside Task 2.2 rather than Task 1.2. No unrequested scope creep. |

## Coverage Report

- Line coverage on changed files: not mechanically measured — `cargo tarpaulin` is not installed in the `backend` container image (`error: no such command: tarpaulin`), the same pre-existing infra gap noted in the 0011/0012/0013 reviews. Manually enumerated: every new function has at least one direct test except the branch in M1.
- Branch coverage on changed files: not mechanically measured, same reason. Manually enumerated gap: the `TrainingFeedbackError::DbError` mapping branch (M1).
- Excluded files: none proposed for `coverage-exclusions.txt`.

## Required Fixes Before Close

1. Add an adapter test that creates feedback with a `chunk_id` referencing a non-existent document and asserts `Err(TrainingFeedbackError::DbError(_))` (M1).
2. Add a handler test with a mock `TrainingFeedbackAdminPort::create_feedback` returning `TrainingFeedbackError::DbError`, asserting a `500` response (M1).
3. Optionally address m1 (wrap the sentiment parse error with column context, matching the `source_type` precedent) while fixing M1, since it is a one-line change in the same file.

## Fix Log

- **[M1]** FIXED on 2026-07-24. Added `should_return_db_error_for_nonexistent_chunk_id` in `backend/src/admin/training_feedback/adapter.rs` (asserts `Err(TrainingFeedbackError::DbError(_))` when `chunk_id` violates the FK) and `should_return_500_when_create_feedback_hits_a_db_error` in `backend/src/admin/training_feedback/handlers.rs` (mock port returns `DbError`, asserts `500`). Verification: `cargo test -p kb-store -p backend` and `cargo test --workspace` both green (142 backend unit tests, 41+7+7+2 BDD scenarios, 79 kb-store tests), `cargo clippy --workspace --all-targets -- -D warnings` clean.
- **[m1]** FIXED on 2026-07-24. Wrapped the `sentiment` column parse error with column context in both `get_training_feedback`/`list_training_feedback` read paths in `kb-store/src/lib.rs`, matching the `source_type` precedent: `KbStoreError::Migration(format!("invalid sentiment in db: {e}"))`. Verification: same full-suite run as M1, plus `cargo fmt --check` clean.
