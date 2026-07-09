---
name: spontini-verify-gate
description: Pre-completion verification gate for the Spontini Rust workspace. Use BEFORE claiming any task done. Runs the full build, test, lint, format, and coverage checks, plus the Docker Compose config validation. No task is complete until every gate passes or pre-existing failures are explicitly noted.
---

# Spontini Verify Gate

You are about to report a task as complete. Before you do, run every gate in this skill. Reporting "done" without these passing is a contract violation.

## Gate 1 — Workspace Build

```bash
cargo build --workspace --all-targets
```

Exit code must be 0. Any compile error is a blocker; fix it before proceeding.

## Gate 2 — Tests

```bash
cargo test --workspace --all-targets -- --nocapture
```

- Every test passes, OR
- Pre-existing failures are explicitly listed in your final report with `# pre-existing, unrelated to this change`.

Never delete or `#[ignore]` a test to make this green. Never hardcode a value to satisfy an assertion.

## Gate 3 — Clippy (Warnings = Errors)

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Zero warnings. If clippy suggests a fix, apply it, do not suppress with an attribute.

## Gate 4 — Format Check

```bash
cargo fmt --all -- --check
```

If it fails, run `cargo fmt --all` and re-check. Do not leave formatting diffs for the reviewer.

## Gate 5 — Coverage Threshold

100% line coverage and 80% branch coverage on production code. Run the workspace coverage tool and confirm:

- Every changed production file meets the threshold.
- Any uncovered branch either gains a test or is added to `coverage-exclusions.txt` with a documented reason in the PR.

Coverage is measured on production code only. Exclusions allowed (with justification): `main` entry points, pure framework config, value-object DTOs with no behavior.

## Gate 6 — LSP Diagnostics (Type Errors)

Run LSP diagnostics on every changed file. Zero type errors. `as any` / `@ts-ignore` / `unwrap()`-to-bypass-the-type-system are forbidden.

## Gate 7 — Docker Compose Config (If Infrastructure Touched)

```bash
docker compose config --quiet
```

Validates the compose file syntax. Run this if you touched `docker-compose.yml`, any `Dockerfile`, or service definitions.

## Gate 8 — BDD Scenarios (If Feature Touched)

```bash
# Run the BDD test target that wires features/*.feature
cargo test -p backend --test bdd -- --nocapture
```

Every Gherkin scenario for the touched feature is green. A feature without a passing scenario is not done.

## Gate 9 — Embedding Model Consistency (If RAG or Ingest Touched)

If you touched `llama-embed`, the embedding endpoint, or the embedding model config:

- Confirm `ingest-core` and `rag-engine` read the same model name and endpoint from the same config source.
- Confirm no mixed-model vectors exist in `kb.db` (if the model changed, re-ingest was performed).

## Gate 10 — Manual Sanity (End-to-End Surface)

Per the project verification contract, "tests pass" is not enough for end-to-end work. If the change is user-visible:

- **HTTP API** → `curl` the running service against `/chat` (and `/admin/upload` if touched).
- **Frontend** → load the chat popup in a browser, send a test question, confirm the answer renders with a citation.
- **CLI** → run `ingest-cli --source <adapter>` against a tiny test fixture, confirm `kb.db` gains rows.

If you cannot run the surface, say so explicitly. Do not imply it works.

## Reporting

Your final report must state, for each gate:

- ✅ passed, OR
- ⚠️ pre-existing failure (listed), OR
- ❌ blocked (with reason)

Example:

```
Verification:
- Build: ✅
- Tests: ✅ (142 passed, 0 failed)
- Clippy: ✅
- Fmt: ✅
- Coverage: ✅ (100% line, 84% branch on changed files)
- LSP: ✅
- Docker config: ✅
- BDD: ✅ (3 scenarios green)
- Embedding consistency: n/a (not touched)
- Manual sanity: ✅ (curl /chat returned 200 with cited answer)
```

## Forbidden

- Reporting "done" with any gate not run.
- Reporting a gate as passed because you assume it would.
- Marking a test `#[ignore]` to pass.
- Suppressing clippy warnings.
- Bypassing the type system to silence an error.
- Claiming end-to-end behavior works without having run it.
