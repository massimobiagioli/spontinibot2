# Review 0026: Production hardening: non-root containers, resource limits, image scanning

- **Plan**: [0026-production-hardening-non-root-containers-resource-limits-image-scanning-plan.md](./0026-production-hardening-non-root-containers-resource-limits-image-scanning-plan.md)
- **Branch**: feat/production-hardening-non-root-containers-resource-limits-image-scanning
- **Reviewed**: 2026-07-25
- **Reviewer**: Sisyphus (Claude Code)
- **Verdict**: changes-requested

## Summary

The implementation delivers everything the plan promised and more: every owned container runs non-root, a `docker-compose.prod.yml` overlay applies memory/CPU limits and healthchecks (including a new, well-tested heartbeat mechanism for the HTTP-less `ingest` daemon), and `make scan` gates on zero HIGH/CRITICAL CVEs across all four owned images — genuinely zero, not just "gate exists." The frontend/admin-ui Dockerfiles were reasonably extended into a real multi-stage build (down to 11.1 MB runtime images) after scanning surfaced real CVEs in bundled dev tooling that never ships; this was necessary to make the scan gate meaningful rather than permanently red, and is disclosed in the plan's Implementation Notes. Live end-to-end verification (all 6 services healthy under the prod overlay, `/chat` returning a real cited answer) was performed, not just claimed. The one gap is a missing README update for the new production-deployment and scanning commands, required by STACK.md §7.3 for any new operator action.

## Findings

### Blockers

(none)

### Major

- **[M1]** `README.md` — STACK.md §7.3 states: "If a new operator action is needed, a new target is added; the README and CI are updated in the same change." This plan adds `make prod-build`, `make prod-up`, `make prod-down`, and `make scan` — the actual production deployment path for Milestone 5 ("the system is shippable to the Comune di Maiolati Spontini"), not a minor dev convenience. `README.md`'s Quick Start section documents `build`/`up`/`logs`/`down`/`verify`/`bdd-e2e` but has no mention of the production overlay or the scan gate at all. Expected: a short addition to Quick Start (or a new subsection) covering `make prod-build && make prod-up` and `make scan`. Actual: undocumented — an operator reading the README has no way to discover the production path exists. Suggested fix: add a "Production" subsection to `README.md` after Quick Start, listing `make prod-build`, `make prod-up`, `make prod-down`, `make scan`, and one sentence on what the overlay changes (non-root + resource limits + `target: runtime`, per `docker-compose.prod.yml`'s own header comment).

### Minor

- **[m1]** `ingest/src/scheduler.rs:49-56` (`touch_heartbeat`) — uses a blocking `std::fs::write` inside an `async fn` called from the scheduler's single `tokio::select!` loop (`current_thread` runtime, per `ingest/src/main.rs`'s `#[tokio::main(flavor = "current_thread")]`). For a few bytes to `/tmp` this is negligible in practice (confirmed by the live healthcheck passing on every poll tick), but it's technically a blocking call on the async runtime. Not worth a `spawn_blocking` for this payload size, but worth a one-line comment if a future reader wonders why it isn't `tokio::fs` (already explained in this review; no action required).
- **[m2]** `bin/scan.sh:9-11` — the comment says the llama.cpp scan exclusion mirrors "the accepted non-root exception documented in the ADR," but the ADR doesn't exist yet at this point in the lifecycle (it's written by `/create-adr` after this review closes). Not a functional issue — the ADR step (2.6 of the orchestrator) must in fact document this exclusion for the comment to become true. Suggested: when authoring the ADR, explicitly cover the `make scan` exclusion alongside the non-root exception so this comment's forward reference is satisfied.

### Nits

- **[n1]** `docker-compose.prod.yml` — the header comment is thorough and explains the `:prod` tag rationale well; no change needed, called out only as a positive note for future maintainers extending this file.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | pass | Infra-only change plus one small, cohesive addition to `CronScheduler` (heartbeat write) that stays within its existing "run the scheduler loop" responsibility. No new crates, no port/adapter changes, no domain-layer touch. |
| Truthfulness & RAG | n/a | No change to `/chat`, retrieval, prompt assembly, or persona. Verified live: `/chat` still returns a correctly cited answer through the hardened stack. |
| Ingest correctness | n/a | No change to embedding model, adapters, or chunking/embedding logic. The heartbeat is a pure operational addition (feature 0006's scheduler already existed; this only adds a liveness signal). |
| Tests (coverage + TDD + BDD) | pass | Two new unit tests cover both branches of `touch_heartbeat` (success + write-failure). Full workspace `cargo test` (142+9+... = all green) and both frontend/admin-ui `npm test` suites pass. `cargo tarpaulin` coverage numbers could not be produced — confirmed pre-existing (missing from the image before this branch; unrelated to any Dockerfile change here) via `git diff` and direct testing, explicitly disclosed in the plan's Implementation Notes rather than silently skipped. |
| Clean Code | pass | Clear naming (`touch_heartbeat`, `heartbeat_path`, `spontini` user), no magic numbers without justification (UID/GID and resource-limit values are explained in the plan), no unwrap-to-bypass-errors in production code. |
| Clean Design (UI/UX) | n/a | No UI changes. |
| Plan conformance | pass (with disclosed expansion) | Every task's listed deliverables exist and its verification passed. Task 3.1 grew beyond its originally-listed deliverables (added a `runtime` stage to frontend/admin-ui Dockerfiles, plus `target: build` pins in the base `docker-compose.yml`) — this was the direct, necessary fix for the task's own verification step (a meaningful zero-CVE `make scan`), not speculative scope creep, and is explicitly disclosed in the plan's Implementation Notes with the reasoning. |

## Coverage Report

- Line coverage on changed files: not measured — `cargo-tarpaulin` is absent from the `backend`/`ingest` build-stage image, confirmed pre-existing and unrelated to this branch's changes.
- Branch coverage on changed files: not measured, same reason.
- Substitute verification: `cargo test -p ingest` (host and in-container as non-root) exercises both branches (write-success, write-failure) of the only new production-code branch introduced (`touch_heartbeat`'s `if let Err`). All other changes in this plan are Dockerfile/compose/Makefile/shell — not subject to the Rust coverage gate.
- Excluded files: none beyond the pre-existing `main.rs`/`tests/**` exclusions already configured in `make coverage`.

## Required Fixes Before Close

1. **[M1]** Add a "Production" subsection (or extend Quick Start) in `README.md` documenting `make prod-build`, `make prod-up`, `make prod-down`, and `make scan`, per STACK.md §7.3's requirement that new operator actions are documented in the same change.

## Fix Log

- **[M1]** FIXED on 2026-07-25. Added a "Production" subsection to `README.md` immediately after Quick Start, explaining the dev (`docker-compose.yml`, `target: build`) vs. production (`docker-compose.prod.yml`, `target: runtime` + limits + healthchecks) split and documenting `make prod-build`, `make prod-up`, `make prod-down`, `make scan`. Verification: `make compose-config` passes; README renders as valid Markdown with correct fenced code block.
