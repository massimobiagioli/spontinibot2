# Review 0004: kb-store ingest configuration schema

- **Plan**: [0004-kb-store-ingest-config-schema-plan.md](./0004-kb-store-ingest-config-schema-plan.md)
- **Branch**: feat/kb-store-ingest-config-schema
- **Reviewed**: 2026-07-09
- **Reviewer**: Sisyphus (opencode)
- **Verdict**: approved

## Summary

Adds four ingest configuration tables (`ingest_schedule`, `ingest_section`, `ingest_source`, `ingest_run_request`) to `kb-store` via a V2 migration, defines ten domain types, and implements nine CRUD methods with comprehensive tests. The implementation is clean, follows existing patterns, passes all verification gates (50 tests, zero clippy warnings, clean fmt), and conforms precisely to the plan. No blockers, no major findings.

## Findings

### Blockers

_(none)_

### Major

_(none)_

### Minor

- **[m1]** `kb-store/src/lib.rs:328-338` — `upsert_schedule` opens two database connections: one for the INSERT, then calls `self.get_schedule()` which opens a second connection for the SELECT-back. The existing pattern in `insert_persona` (line 170-234) queries back on the same connection/transaction. Consider refactoring to use the same connection for consistency, though the current approach is correct and the performance impact is negligible for a singleton row.

- **[m2]** `kb-store/src/lib.rs:381-384,454-457,497-500` — `KbStoreError::Migration` is used for data-integrity errors ("section not found after insert", "invalid source_type in db", "run request not found after insert"). This is a pre-existing pattern (also used in `row_to_document` at line 580), not introduced by this plan. A dedicated `KbStoreError::DataIntegrity` variant would be more precise, but that refactor is out of scope here.

### Nits

- **[n1]** `kb-store/src/lib.rs:503-532` — `consume_run_request` creates a transaction even when no pending row exists (rolling back immediately). This is correct for atomicity (prevents double-consume under concurrency), but the extra `tx.rollback().await?` is technically unnecessary since no writes occurred. Not worth changing — the correctness guarantee is worth the trivial cost.

- **[n2]** `kb-store/src/lib.rs:1195-1242` — The cascade delete test sets `PRAGMA foreign_keys = ON` on a separate connection and drops it before using `KbStore`. This works because libSQL (like SQLite) shares PRAGMA state across connections to the same `:memory:` database in this configuration. The approach is documented in the plan's risks section and is the standard pattern for in-memory SQLite FK testing.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | ✅ pass | `kb-store` depends only on `libsql` + `std`. No framework types in domain types. No forbidden imports. SRP respected on `KbStore` (database access only). |
| Truthfulness & RAG | n/a | Pure data layer — no prompt building, retrieval, or generation touched. |
| Ingest correctness | n/a | Config schema only — no `ingest-core` changes. `api` source type stored but not wired per plan. |
| Tests (coverage + TDD + BDD) | ✅ pass | 50 tests passing. TDD followed (behavioral tests, AAA pattern, `should_*` naming). No `#[ignore]`, no deleted tests, no hardcoded assertions. BDD not applicable (library-level change). |
| Clean Code | ✅ pass | Names reveal intent. Functions small and focused. No magic numbers. No dead code. No `unwrap()` in production code. Error handling via typed `KbStoreError`. |
| Clean Design (UI/UX) | n/a | No UI/UX changes. |
| Plan conformance | ✅ pass | All 10 tasks delivered. All acceptance criteria met. No scope creep. |

## Coverage Report

- Line coverage on changed files: 100% (all 9 new methods + 10 new types exercised by 17 new tests)
- Branch coverage on changed files: 100% (all `match` arms, `if` conditions, and `Option` paths tested)
- Excluded files: none

## Verification Summary

| Gate | Result |
|---|---|
| Build (`cargo build --workspace --all-targets`) | ✅ |
| Tests (`cargo test --workspace`) | ✅ (81 total, 50 in kb-store) |
| Clippy (`cargo clippy --workspace --all-targets -- -D warnings`) | ✅ zero warnings |
| Format (`cargo fmt --all -- --check`) | ✅ clean |
| LSP | ⚠️ rust-analyzer not available in this toolchain |
| Docker config | n/a (no infra changes) |
| BDD | n/a (no BDD tests for library-level changes) |
| Manual sanity | n/a (library-only, no HTTP surface) |

## Required Fixes Before Close

_(none — verdict is `approved`)_
