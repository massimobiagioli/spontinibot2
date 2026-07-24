# Review 0013: `/admin/api/training/sessions/:id/messages` — ask/answer with recording

- **Plan**: [0013-admin-api-training-messages-plan.md](./0013-admin-api-training-messages-plan.md)
- **Branch**: feat/admin-api-training-messages
- **Reviewed**: 2026-07-24
- **Reviewer**: Sisyphus
- **Verdict**: changes-requested

## Summary

The feature adds a `training_message` table (V4 migration), a `TrainingMessageAdminPort`/`RagTrainingMessageAdapter` that reuses `RagEngine::answer` verbatim, and the `POST`/`GET /admin/api/training/sessions/:id/messages` endpoints, following the same port/adapter shape as feature 0012. Architecture, truthfulness/RAG correctness, and plan conformance are solid, and the `AdminRouterState` refactor is a genuine improvement (fixed a real clippy `too_many_arguments` violation instead of suppressing it). The one blocking issue is a real gap in branch coverage: the error-mapping branches for `TrainingMessageError::Rag` and `TrainingMessageError::DbError` are never exercised by any test, in both the adapter and the handlers, which violates the project's explicit "no untested branches" rule.

## Findings

### Blockers

None.

### Major

- **[M1]** `backend/src/admin/training_messages/adapter.rs:29-31` and `backend/src/admin/training_messages/handlers.rs:26-40` — The `RagError → TrainingMessageError::Rag` mapping in `RagTrainingMessageAdapter::ask` and the `TrainingMessageError::DbError` / `TrainingMessageError::Rag` arms of `map_message_error` are never hit by any test. Expected: PRINCIPLES.md §7 requires "every `if`, `catch`, `switch` case... has a test for both sides" — the happy path and `SessionNotFound` are covered, but a RAG failure (e.g. embedding/generation error) and a raw DB error are not. Actual: `adapter::tests` only exercises success, session-not-found, and honest-unknown; `handlers::tests` only exercises 401, 201, and 404. Fix: add an adapter test using an error-returning `EmbeddingPort` double (mirroring `rag_engine::engine::tests::TestEmbeddingError`) asserting `Err(TrainingMessageError::Rag(_))`, and add two handler tests with a mock port returning `TrainingMessageError::DbError` / `TrainingMessageError::Rag`, asserting `500` / `502` respectively.

### Minor

- **[m1]** `backend/src/admin/training_messages/adapter.rs:42-51` — `TrainingMessageError::Rag` is reused for two unrelated failure kinds: genuine `RagEngine` errors (line 40) and JSON serialization/deserialization failures (lines 50, 79). This blurs the error variant's meaning for anyone reading logs or matching on it. Suggested fix: add a dedicated `TrainingMessageError::Serialization(String)` variant for the JSON ser/de paths, keeping `Rag` for actual RAG-engine failures only.
- **[m2]** `backend/src/admin/training_messages/adapter.rs:47-51` — The `serde_json::to_string(&sources)` failure branch is effectively unreachable (`Vec<TrainingMessageSource>` of two plain fields cannot fail to serialize), yet it is left as a live, untested error path rather than being documented as an exemption per PRINCIPLES.md §7 ("only the following may be excluded... each exclusion must be justified"). Suggested fix: either add a `coverage-exclusions.txt` entry with justification, or replace with `.expect("Vec<TrainingMessageSource> is always serializable")` to make the invariant explicit instead of threading a phantom error path through the `Result`.

### Nits

- **[n1]** `.project/0013-admin-api-training-messages-plan.md` Task 3.2 — the plan's literal deliverable text describes adding a `training_message_state: TrainingMessageState` parameter to `router_with`; the actual implementation instead introduces an `AdminRouterState` struct bundling all five admin sub-states (`backend/src/lib.rs:36-43`). This was necessary to satisfy the `clippy::too_many_arguments` gate (the function would have grown to 8 parameters) and is a strict improvement, not scope creep — noting it here only for plan-conformance transparency.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | pass | `RagTrainingMessageAdapter` lives in `backend` (interface-adapters layer), depends inward on `kb_store::KbStore` and `rag_engine::engine::RagEngine`, matching the crate dependency matrix. DTO boundary respected: `TrainingMessageResponse`/`TrainingMessageSource` are purpose-built response types with serde derives; the rag-engine domain type `CitedSource` is never annotated for transport — sources are explicitly converted at the adapter boundary (adapter.rs:42-49). `AdminRouterState` bundling is a genuine SRP/parameter-count improvement (router_with went from a would-be 8 params to 4). |
| Truthfulness & RAG | pass | `ask()` calls `RagEngine::answer` verbatim — the exact same use case `/chat` uses — so the persona/context/question separation and the honest-unknown fallback are inherited unchanged, not reimplemented. The honest-unknown path (empty `sources`, `fell_back: true`) is covered by both a unit test (`should_record_honest_unknown_fallback_with_no_sources`) and a BDD scenario. Every recorded message carries its cited sources through to persistence and back. |
| Ingest correctness | n/a | This feature does not touch `ingest-core`, `ingest-cli`, or the embedding/ingest pipeline. |
| Tests (coverage + TDD + BDD) | fail | See M1/m2. TDD was followed (tests written before/alongside each implementation increment; RED confirmed by compile failures at each step). BDD: 6 new scenarios in `features/admin_training_messages.feature` cover ask+cite, list, honest-unknown fallback, unknown-session 404, and missing-auth on ask/list — all 38 scenarios / 164 steps in the full suite pass. No `#[ignore]`, no deleted tests, no hardcoded assertions. Mechanical coverage measurement (`cargo tarpaulin`) is unavailable — the tool is not installed in the `backend` container image (`error: no such command: tarpaulin`), a pre-existing infra gap already noted in the 0011/0012 reviews and out of scope for this feature; the branch gaps above were found by manual enumeration instead. |
| Clean Code | pass | Names reveal intent (`RagTrainingMessageAdapter`, `TrainingMessageAdminPort`). No magic numbers, no dead code, no `unwrap()` in production code (only `.expect()` in tests). `ask()` is ~30 lines doing one coherent orchestration (check session → answer → serialize → persist), consistent with the existing `RagEngine::answer`'s own length. See m1 for the one naming-clarity nit. |
| Clean Design (UI/UX) | n/a | No UI touched — this is an HTTP admin endpoint only. |
| Plan conformance | pass | Every task's deliverables exist and every task's stated verification passed. No unrequested scope creep — the only deviation (n1) was a necessary, smaller-footprint fix for a hard gate, not a scope expansion. |

## Coverage Report

- Line coverage on changed files: not mechanically measured — `cargo tarpaulin` is not installed in the `backend` container image (pre-existing infra gap, same as noted in the 0011/0012 reviews); Docker itself is available in this environment but the tool binary is missing from the image. Manually enumerated: every new function has at least one direct test except the branches in M1/m2.
- Branch coverage on changed files: not mechanically measured, same reason. Manually enumerated gaps: `TrainingMessageError::Rag`/`DbError` mapping branches in `adapter.rs` and `handlers.rs` (M1); the unreachable JSON-serialization error branch (m2).
- Excluded files: none proposed for `coverage-exclusions.txt` (m2 recommends either adding one or removing the phantom error path).

## Required Fixes Before Close

1. Add an adapter test that wires `RagEngine` with an error-returning `EmbeddingPort` (or `GenerationPort`) double and asserts `RagTrainingMessageAdapter::ask` returns `Err(TrainingMessageError::Rag(_))` (M1).
2. Add two handler tests: one where the mock `TrainingMessageAdminPort` returns `TrainingMessageError::DbError`, asserting `500`; one where it returns `TrainingMessageError::Rag`, asserting `502` (M1).
3. Optionally address m1 (split the `Rag` variant) and m2 (document or eliminate the unreachable serialization-error branch) while fixing M1, since the same test additions touch this code.

## Fix Log

- **[M1]** FIXED on 2026-07-24. Added `should_map_rag_engine_error_to_training_message_rag_error` (adapter, uses a `FailingEmbedding` test double) and `should_return_500_when_ask_hits_a_db_error` / `should_return_502_when_ask_hits_a_rag_error` (handlers, mock returns the two error variants). Verification: `cargo test -p backend --lib training_messages` — 19 passed, 0 failed.
- **[m1]** FIXED on 2026-07-24. Added a dedicated `TrainingMessageError::Serialization(String)` variant, used for the JSON deserialize path in `message_to_response`; `Rag` now carries only genuine `RagEngine` failures. Verification: `cargo build -p backend` compiles; `should_format_serialization_error_display` passes.
- **[m2]** FIXED on 2026-07-24. Replaced the phantom `serde_json::to_string` error branch with `.expect("Vec<TrainingMessageSource> of plain fields is always serializable")`, removing the untested unreachable path instead of documenting an exclusion. Added `should_map_malformed_sources_json_to_serialization_error`, a direct unit test on `message_to_response` covering the (reachable) deserialize-failure branch, plus `should_return_500_when_ask_hits_a_serialization_error` on the handler side. Verification: full workspace gate green — `cargo test --workspace --all-targets` (125/0/0/0/... across all targets), `cargo test -p backend --test bdd` (38 scenarios/164 steps passed), `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo fmt --all -- --check` clean.
