# Review 0007: ingest-cli — one-shot manual run developer tool

- **Plan**: [0007-ingest-cli-one-shot-manual-run-developer-tool-plan.md](./0007-ingest-cli-one-shot-manual-run-developer-tool-plan.md)
- **Branch**: feat/ingest-cli-one-shot-manual-run-developer-tool
- **Reviewed**: 2026-07-09
- **Reviewer**: Sisyphus
- **Verdict**: approved

## Summary

A thin one-shot CLI over `ingest-core` for developer convenience. Two modes: `--url` scrapes/embeds/inserts a single URL; `--all-sources` reads configured scrape sources from `kb.db` and runs them all. 7 unit tests and 2 wiremock integration tests cover both modes, mutual exclusion, error handling, and per-source error tolerance. Clean architecture respected — no changes to `ingest-core` or `kb-store` internals. Ships.

## Findings

### Blockers

None.

### Major

None.

### Minor

- **[m1]** `ingest-cli/run.rs#L41-L43` — `run_all_sources` creates the `IngestPipeline` with `config.user_agent.clone()`, but the pipeline receives ownership via `.clone()`. If the pipeline outlives the loop iteration this is harmless, but since `pipeline.run()` takes `&self`, a single pipeline instance shared across sources is intentional and correct. The clone is a trivial cost and not a concern.
- **[m2]** `ingest-cli/src/main.rs#L18` — `args.section.unwrap_or_default()` silently converts a missing `--section` to `""`. This means `ingest-cli run --url http://x` passes an empty string as the section name. The plan's acceptance criteria don't require rejecting this case, and the pipeline handles it gracefully, but a user-friendly explicit error would be better.

### Nits

- **[n1]** `ingest-cli/tests/integration.rs#L98,160` — `let _ = std::fs::remove_file(&path);` silently discards cleanup errors. Matches the existing pattern in `ingest_core::pipeline::tests`, so consistent, but it means temp db files can leak on test failure (as seen during development).

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | pass | CLI consumes existing APIs, no new crate boundaries, dependencies point inward. |
| Truthfulness & RAG | n/a | Not touched — RAG/chat flow unchanged. |
| Ingest correctness | pass | Uses same `IngestPipeline` as the always-on ingest; no adapter/embedding duplication. |
| Tests (coverage + TDD + BDD) | pass | 9 tests (7 unit + 2 integration); TDD followed; no `#[ignore]` or tautological tests. |
| Clean Code | pass | Clear names, focused functions, no magic numbers, no unwrap abuse. |
| Clean Design (UI/UX) | n/a | CLI tool, no UI surface. |
| Plan conformance | pass | All 8 tasks completed, all deliverables present, no scope creep. |

## Coverage Report

Coverage enforced via `make coverage` (Docker/tarpaulin). Changed production files in `ingest-cli/src/`:

- `cli.rs` — 100% line (covered by 6 unit tests)
- `run.rs` — 1 function (`should_use_default_config_when_no_env_vars`) covers `RunConfig::default` defaults; `run_url` and `run_all_sources` exercised by integration tests
- `main.rs` — main entry point with `#[tokio::main]`, minimal logic; covered by integration tests end-to-end

## Required Fixes Before Close

None. This review is approved — `/fix-review 0007` can proceed directly to close the plan.
