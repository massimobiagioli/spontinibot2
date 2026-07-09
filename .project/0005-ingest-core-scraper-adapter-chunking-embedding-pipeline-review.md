# Review 0005: ingest-core — scraper adapter, chunking, embedding pipeline

- **Plan**: [0005-ingest-core-scraper-adapter-chunking-embedding-pipeline-plan.md](./0005-ingest-core-scraper-adapter-chunking-embedding-pipeline-plan.md)
- **Branch**: feat/ingest-core-scraper-adapter-chunking-embedding-pipeline
- **Reviewed**: 2026-07-09
- **Reviewer**: Sisyphus
- **Verdict**: changes-requested

## Summary

The implementation delivers all six phases of the plan: scraper adapter with robots.txt honoring and content-type allowlist, recursive text chunker with section-tagged metadata, embedding client validating 768-dim responses against `kb_store::EMBEDDING_DIM`, an `IngestPipeline` orchestrator composing scrape → chunk → embed → `KbStore::insert_document`, and a `Pipeline` trait for future consumers. The crate is in good shape — tests pass, clippy is clean, wiremock decouples HTTP tests. One blocker prevents closing: a `todo!()` in place of an object-safety assertion, and a few minors around test coverage and minor coupling.

## Findings

### Blockers

- **[B1]** `pipeline.rs:148` — `_assert_pipeline_is_object_safe` uses `todo!()`, which panics at runtime if ever reached. The plan requires an object-safety test. Replace with a proper test that compiles and passes: `let _: Box<dyn Pipeline> = todo!();` inside a `#[test]` that never panics, or use `#[allow(dead_code)]` on a function that compiles the assertion without executing it.

### Major

- **[M1]** No test verifies the `IngestPipeline::run` error path when `ScraperAdapter::fetch_text` returns an error (e.g., robots.txt disallow). The plan's acceptance criteria require all error types to be reachable. Add a wiremock test for the error path (e.g., robots.txt disallows the target URL, pipeline returns `IngestError::RobotsTxt`).

### Minor

- **[m1]** No test for the `text/plain` content-type path in `ScraperAdapter::fetch_text`. The content-type allowlist includes `text/plain`, but the code path that skips HTML parsing and returns raw text is untested. Add a wiremock test serving `text/plain` content.

### Nits

- **[n1]** `IngestPipeline::new` creates concrete adapter types (`ScraperAdapter`, `EmbeddingClient`) internally rather than accepting trait implementations. This couples the pipeline to concrete types. For a library crate this is acceptable at this scope, but a `PipelineConfig` struct or constructor injection would improve testability before the crate gains more consumers.
- **[n2]** `Chunker::chunk` (`chunking.rs:55-135`) is ~80 lines handling both accumulation and boundary logic. The paragraph-splitting and overlap computation could be extracted into helpers for readability.
- **[n3]** The overlap test (`should_include_overlap_between_chunks`) uses approximate contains() matching, which is fragile with short text. A more deterministic assertion (e.g., comparing a known-overlap substring) would be more reliable.
- **[n4]** `Chunker::chunk` takes `source_url: &str` but the `Pipeline` trait's `Pipeline::run` method already passes `url` — the section tag is in the plan's API but the `url` parameter on `chunk` duplicates information the caller already has. Minor API surface wart.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | pass | Crate depends only inward; `ingest-core` does not import `axum` or `ingest-cli`. No framework types in domain logic. The `Pipeline` trait is a clean port. |
| Truthfulness & RAG | n/a | No RAG flow changes. Embedding dimension validated against shared `kb_store::EMBEDDING_DIM` (768), ensuring ingest/query consistency. |
| Ingest correctness | pass | Same embedding model constraint enforced. Adapters do not embed or write to `kb.db` directly. Chunking is configurable. `api-client` is a stub, not wired. `DocumentSource::Scrape` used correctly. ingest-flow rules all met. |
| Tests (coverage + TDD + BDD) | fail | 29 unit tests covering most paths. **Blocker B1** (todo!() assertion) and **major M1** (missing pipeline error-path test) lower the coverage below the 80% branch standard. One uncovered `text/plain` path (minor). |
| Clean Code | pass | Names reveal intent. Functions are small to moderate. Error handling is thorough (`IngestError` enum with `thiserror`). No magic numbers - chunk size/overlap are configurable. |
| Clean Design (UI/UX) | n/a | No UI or UX changes in this feature. |
| Plan conformance | pass | Every task's deliverables exist. All phases are implemented. No unrequested scope creep (api-client is a stub, not wired). Verification gates pass. |

## Coverage Report

- Line coverage on changed files: ~92% (estimated — `todo!()` in pipeline.rs is uncovered)
- Branch coverage on changed files: ~85% (estimated — missing pipeline error-path and text/plain branch)
- Excluded files: none

## Required Fixes Before Close

1. **[B1]** Replace `todo!()` in `_assert_pipeline_is_object_safe` with a proper compile-time assertion that does not panic.
2. **[M1]** Add a wiremock test for the pipeline error path (robots.txt disallow → `IngestError::RobotsTxt`).
3. **[m1]** Add a wiremock test for the `text/plain` content-type path in `ScraperAdapter::fetch_text`.

## Fix Log

- **[B1]** FIXED on 2026-07-09. Replaced `todo!()` with `Option<Box<dyn Pipeline>>` compile-time assertion in `pipeline.rs`. Verification: 40/40 tests pass, clippy clean, fmt clean.
- **[M1]** FIXED on 2026-07-09. Added `should_return_robots_txt_error_when_url_disallowed_by_robots` test in `pipeline.rs` — sets up robots.txt with `Disallow: /private/`, calls `IngestPipeline::run` with a disallowed URL, asserts `IngestError::RobotsTxt`. Verification: test passes, 40/40.
- **[m1]** FIXED on 2026-07-09. Added `should_return_raw_text_when_content_type_is_text_plain` test in `scraper.rs` — serves `text/plain` content, verifies scraper returns content verbatim. Verification: test passes, 40/40.
