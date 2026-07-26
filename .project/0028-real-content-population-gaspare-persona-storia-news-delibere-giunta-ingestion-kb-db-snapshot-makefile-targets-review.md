# Review 0028: Real content population (Gaspare persona, storia/news/delibere/Giunta ingestion) + kb.db snapshot Makefile targets

- **Plan**: [0028-real-content-population-gaspare-persona-storia-news-delibere-giunta-ingestion-kb-db-snapshot-makefile-targets-plan.md](./0028-real-content-population-gaspare-persona-storia-news-delibere-giunta-ingestion-kb-db-snapshot-makefile-targets-plan.md)
- **Branch**: feat/real-content-population-gaspare-persona-storia-news-delibere-giunta-ingestion-kb-db-snapshot-makefile-targets
- **Reviewed**: 2026-07-26
- **Reviewer**: Claude Code (autonomous /next-steps run)
- **Verdict**: changes-requested

## Summary

The code diff is small and focused: two new `Makefile` targets (`eject-data`/`use-data`) for kb-data volume snapshots, a real bug fix in `ingest-core`'s chunker (UTF-8 char-boundary panic, found live ingesting a real determina PDF, TDD'd with two regression tests), and a `.gitignore` entry. The bulk of the plan's work — persona re-imprinting, 12 news items, 10 delibere/determine, a new giunta section — is live KB data populated through the existing, already-reviewed admin API, correctly producing no source diff, and is thoroughly documented with real URLs, dates, and smoke-test evidence in `0028-content-ingestion-log.md`. The chunking fix is correct, minimal, and well-tested. Two real issues in the Makefile targets need fixing before close: an unnecessary root-privilege escalation, and a lack of coordination with the live `backend`/`ingest` services that the implementer had to work around manually during verification.

## Findings

### Blockers

None.

### Major

- **[M1]** `Makefile:167,176` — `eject-data`/`use-data` run `docker compose run --user root` to get write access to the host-bind-mounted `.data/` directory. This escalates the one-off maintenance container to root, a real deviation from the project's established non-root discipline (the `backend` Dockerfile sets `USER spontini` — uid 10001 — in both build and runtime stages, unconditionally, not just under the ADR 0010 production overlay). Expected: achieve host-writable bind-mount access without an in-container privilege escalation. Actual: `--user root` grants the transient container full root inside its namespace for the duration of the tar operation. Suggested fix: use `--user "$$(id -u):$$(id -g)"` instead of `--user root` — this makes the container process run as the host-invoking user's uid:gid, which already owns `.data/` (created by the preceding `mkdir -p .data` on the host), giving write access without any root escalation.
- **[M2]** `Makefile:167-182` — Neither target coordinates with the live `backend`/`ingest` containers, which keep the same `kb-data` volume mounted and actively open while `eject-data`/`use-data` read/replace the underlying `kb.db` file out from under them. This is not theoretical: the plan's own verification (`0028-content-ingestion-log.md`, "Round-trip tooling verification") shows the implementer had to manually run `docker compose restart backend ingest` after `use-data` to get consistent behavior — a step the Makefile target itself doesn't perform or even mention. Expected: `use-data` (and ideally `eject-data`, to avoid snapshotting a `kb.db` mid-write) either stops/restarts the dependent services itself, or the target's `## use-data:` help line and a printed warning make the required manual restart explicit so an operator doesn't discover it by trial and error. Actual: silent — a user who runs `make use-data DATA_FILE=...` without restarting `backend`/`ingest` afterward can be left serving stale in-memory state or, in the worst case, reading a `kb.db` mid-swap. Suggested fix: have both targets `docker compose stop backend ingest` before the tar operation and `docker compose start backend ingest` after, or at minimum add a `@echo` warning and a Makefile comment documenting that a restart is required.

### Minor

- **[m1]** `ingest-core/src/chunking.rs:126-135` (`floor_char_boundary`) — The `if index >= s.len() { return s.len(); }` early-return branch is not exercised by either new regression test: both call sites that invoke `floor_char_boundary` only ever pass an `index` that is already `< s.len()` by construction (the overlap-window call subtracts from `joined.len()`; the `split_long_paragraph` call is only reached in the `else` branch of a guard that already excludes `start + chunk_chars >= paragraph.len()`). Per PRINCIPLES.md §7 ("No untested branches — every `if`... has a test for both sides"), this is a real, if minor, gap — the branch appears currently unreachable from any caller, which is itself worth a one-line comment (or a direct unit test on `floor_char_boundary` itself, calling it with `index >= s.len()`, rather than only indirectly through `chunk()`).

### Nits

- **[n1]** `0028-content-ingestion-log.md` — The RAG_MIN_SCORE permissiveness finding (retrieval never truly falls back at the current corpus size, even for fully unrelated questions) is honestly documented and correctly out of scope per the plan's Non-Goals, but is a significant enough finding (it affects every citizen-facing honest-refusal guarantee ADR 0012 promises) that it's worth flagging to the user as a candidate follow-up plan, not just a buried log entry.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | pass | `floor_char_boundary` is a small, pure, private helper in the correct module (`ingest-core::chunking`); no framework types, no new dependencies, no layer violation. Makefile changes are infra glue, not application code. |
| Truthfulness & RAG | n/a | No `RagEngine`/persona/prompt code was touched — the persona and content changes are live data via the existing, already-reviewed admin API (feature 0008/0009), correctly producing no source diff. The RAG_MIN_SCORE finding (n1) is a real observation but is data/config-driven, not a code change in this diff. |
| Ingest correctness | pass | The chunking fix touches only chunk-boundary slicing safety; embedding model, adapter boundaries, and `ingest-cli`/`admin-ui` logic are all untouched. |
| Tests (coverage + TDD + BDD) | pass (see m1) | TDD followed: two new regression tests written for the real panic, both pass against the fixed code; no `#[ignore]`, no deleted tests, no hardcoded assertions. No new user-visible behavior requiring a Gherkin scenario. `cargo tarpaulin` coverage could not be mechanically measured — confirmed pre-existing gap (missing from the `backend` image since feature 0009), documented and treated identically in every plan since (0011 through 0027); not a defect of this branch. One untested branch found by manual read (m1). |
| Clean Code | pass | `floor_char_boundary` is well-named and has a comment explaining the non-obvious real-world trigger (PDF curly quotes), consistent with this project's "only comment the non-obvious WHY" convention. No magic numbers, no dead code, no unjustified `unwrap()`. |
| Clean Design (UI/UX) | n/a | No UI touched. |
| Plan conformance | pass (see M1, M2) | Every task's checkbox, deliverable, and verification is accounted for in the plan file and `0028-content-ingestion-log.md`. Content-population claims (document IDs, smoke tests) are documented in detail but not independently re-verified against the live KB in this review pass (this review is code-diff-focused per its own scope; the log is itself a reviewed, tracked file). No unrequested scope creep — the chunking fix was a genuine blocker for Task 6.2's deliverable, not gratuitous, and follows the exact precedent set by `TEST-INGESTION-0001`'s own mid-session bug fixes. |

## Coverage Report

- Line coverage on changed files: not mechanically measured — `cargo-tarpaulin` is absent from the `backend` build-stage image (`error: no such command: tarpaulin`), the same pre-existing, repeatedly-documented infra gap noted in every review since feature 0009 (0011, 0012, 0013, 0014, 0015, 0018, 0020, 0026, 0027).
- Branch coverage on changed files: not mechanically measured, same reason. Manually enumerated: `floor_char_boundary`'s early-return branch is untested (m1); every other new/changed line in `chunking.rs` and `scheduler.rs` is exercised by the existing or new test suite.
- Excluded files: none new: `Makefile` and `.gitignore` are configuration, not measured by `cargo tarpaulin`; `.project/*.md` are documentation.

## Required Fixes Before Close

1. **[M1]** Change `Makefile`'s `eject-data` and `use-data` targets from `--user root` to `--user "$$(id -u):$$(id -g)"`.
2. **[M2]** Make `eject-data`/`use-data` coordinate with the live `backend`/`ingest` containers — either stop/start them around the volume operation, or add an explicit printed warning/comment that a manual restart is required afterward.
3. **[m1]** (optional but recommended) Add a direct unit test on `floor_char_boundary` covering the `index >= s.len()` branch, or a comment noting it is currently unreachable from any caller and why it's kept as a defensive guard.

## Fix Log

- **[M1]** FIXED on 2026-07-26. Investigated before applying the literally-suggested fix: `docker run --rm -v spontini-bot-2_kb-data:/data alpine ls -lan /data` showed `/data`'s contents are owned by uid 10001 (the image's own default non-root `spontini` user, set unconditionally by the Dockerfile), so switching to the *host*-matching `$(id -u):$(id -g)` would have broken write access to `/data` instead — the suggested fix was wrong for this repo's actual ownership layout. Applied the correct equivalent instead: dropped `--user` entirely (the container now runs as its normal non-root default, which already owns `/data`) and made the host `.data/` bind-mount directory world-writable (`chmod 777 .data`) so the same non-root user can write the snapshot there too. No root escalation anywhere. Verification: live-tested both `make eject-data` and `make use-data` end-to-end against the running stack — snapshot created and restored correctly, document count round-tripped exactly (127 → +1 marker → 128 → restore → 127), backend healthy throughout.
- **[M2]** FIXED on 2026-07-26. Both targets now `docker compose stop backend ingest` before the tar operation and `docker compose start backend ingest` after, so the volume is never read/written while the live services hold it open, and the operator never needs to remember a manual restart. Verification: same live round-trip test as M1 — `backend`/`ingest` observed stopping and starting cleanly around both operations, `/health` returned `{"status":"ok"}` immediately after.
- **[m1]** FIXED on 2026-07-26. Added two direct unit tests on `floor_char_boundary` (`should_clamp_to_string_length_when_index_out_of_bounds`, `should_walk_back_to_nearest_char_boundary_when_index_is_mid_character`) exercising both the early-return branch and the walk-back loop directly, closing the coverage gap the review found. Verification: `cargo test -p ingest-core --lib` — 45 passed, 0 failed (up from 43 pre-fix).

**Full gate re-run after all fixes**: `make verify` — build/test/lint/fmt-check all pass (same 157 backend unit tests + full BDD suite + now 45 `ingest-core` tests, all green); `coverage` still fails on the same pre-existing `cargo-tarpaulin`-missing-from-image gap (unrelated, documented since feature 0009); `compose-config` and `a11y` run directly (gate stops at first failure) — both pass, `a11y` confirming 0 accessibility errors across every `frontend`/`admin-ui` route.
