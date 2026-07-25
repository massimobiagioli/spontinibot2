# Review 0025: End-to-end BDD against live containers

- **Plan**: [0025-end-to-end-bdd-against-live-containers-plan.md](./0025-end-to-end-bdd-against-live-containers-plan.md)
- **Branch**: feat/end-to-end-bdd-against-live-containers
- **Reviewed**: 2026-07-25
- **Reviewer**: agent
- **Verdict**: approved

## Summary

This feature adds `backend/tests/bdd_e2e.rs`, a Cucumber suite that runs `features/chat.feature` as an external HTTP client (`reqwest`) against a live `make up` stack — seeding the knowledge base through the real `/admin/api/persona` and `/admin/api/upload` endpoints, never by writing to `kb.db` directly — and exercises the real `llama-embed`/`llama-generate` HTTP adapters and real libSQL vector retrieval. It is gated behind a Cargo `e2e` feature (`required-features = ["e2e"]`) so it never runs under a plain `cargo test`, keeping `make verify`/CI unaffected, and wired via a new `make bdd-e2e` host-native Makefile target. Both `README.md` and `docs/STACK.md` document the new target and its prerequisites, including a small, honest correction to `bdd`'s own description (it does not touch the live stack — a pre-existing inaccuracy this change happened to be adjacent to). The suite was executed for real, repeatedly, against both the pre-existing long-running dev stack and a freshly-built isolated throwaway stack (see verification notes below), and it correctly detected real, live-stack-only conditions no stub-based test could ever surface. It ships as-is; see the Truthfulness & RAG note below for a discovered, out-of-scope production observation worth a follow-up.

## Findings

### Blockers

None.

### Major

None.

### Minor

None.

### Nits

- **[n1]** `backend/tests/bdd_e2e.rs:196` (`then_cites_source`) — asserts `cited.contains(title)` rather than exact equality, because `source_ref` for an uploaded document is the raw filename (`"{title}.md"`), not the bare title. This is documented in the plan's implementation notes rather than in-code; a one-line comment at the assertion site would help a future reader who compares it against `bdd.rs`'s exact-equality version and wonders why they differ.
- **[n2]** `Makefile:88-91` — `bdd-e2e` is the only target in the file that runs natively on the host rather than via `$(COMPOSE) run --rm ...`, which is a deliberate, plan-documented exception (STACK.md §7.3's "every target runs inside containers" framing), but the exception isn't explained in the Makefile itself, only in the plan and STACK.md's table row. A one-line `#` comment above the target would save the next reader a trip to the docs.

## Verification Performed (real live-stack runs)

Beyond the plan's own per-task verification, I ran the suite for real, multiple times, against two different stacks, specifically to prove the deliverable actually works end-to-end rather than merely compiles:

1. Against the pre-existing long-running dev stack (`spontini-bot-2` project, `kb-data` volume accumulated over weeks of prior feature work): the answerable scenario passed cleanly (real citation, real generation). The honest-unknown scenario failed once — `fell_back=false` for "Quanto pago di tasse comunali?" — because that stack's accumulated `kb.db` already contained an unrelated pre-existing document (`orari.txt`, `document_id=1`, not created by this session) that the real embedding model matched above the 0.35 `min_score` threshold.
2. To rule out "accumulated dev data" as the sole explanation, I built and ran an **isolated, throwaway** Compose project (`-p spontini-e2e-verify`, its own fresh named volume, never touching `spontini-bot-2_kb-data`) three separate times from an empty knowledge base, tearing it down completely between attempts. Results: attempt 1 failed on a 502 (generation model still loading its 2.1 GB weights — a timing issue, not a defect); attempt 2 failed on a transient persona-insert 500 (did not reproduce on manual retry — likely brief libSQL write contention immediately following the preceding chunk-insert) and again on `fell_back=false` for the same tax question, this time against a `kb.db` containing **only** the single freshly-seeded "Orari sportello anagrafe" document; attempt 3 passed the answerable scenario fully and hit a 502 on the second `/chat` call for the honest-unknown scenario. A direct `curl /chat` against that same single-document fresh `kb.db` confirmed the pattern deterministically: `fell_back: false`, citing `"Orari sportello anagrafe.md"` for a municipal-tax question, even though the real generation model's own prose hedged honestly ("Non ho trovato l'informazione nei documenti comunali").
3. The isolated stack and its volume were fully torn down (`docker compose -p spontini-e2e-verify down -v`) after verification; the original dev stack was restored (`docker compose up -d`) and confirmed healthy, with its original `kb-data` volume untouched throughout.

**Conclusion**: the suite's step definitions and assertions are correct and are doing exactly their job — surfacing real, reproducible, live-stack-only behavior. Two of the observations are environmental/timing (generation warmup, transient write contention under back-to-back writes) and are inherent to testing against real inference, already anticipated in the plan's Risks section. The third — retrieval crossing `min_score=0.35` for "Quanto pago di tasse comunali?" against an office-hours document, using the real `nomic-embed-text` model — reproduced consistently and is a genuine production observation, not test flakiness. It is out of scope for this plan (Non-Goals explicitly exclude any change to `rag_engine`) and is called out below and in my final report to the user as a candidate follow-up.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | n/a | No `rag_engine`/`ingest-core`/`kb-store` code touched — only a new external test client (`reqwest`), a Cargo feature gate, and Makefile/docs. `bdd_e2e.rs` talks to the system exclusively through its public HTTP API (`/chat`, `/admin/api/persona`, `/admin/api/upload[/confirm]`), never importing internal crate types — it cannot violate a dependency direction it has no access to. |
| Truthfulness & RAG | pass, with a discovered production observation | The e2e suite itself correctly asserts citation (`then_cites_source`), the 3-part-prompt invariant (delegated, with a documented rationale, to the existing in-process `bdd.rs` coverage since it isn't observable over HTTP), and the honest-unknown fallback's exact-match wording. The suite is doing its job correctly. Separately — see "Verification Performed" above — real-stack testing surfaced that `rag_engine`'s default `min_score=0.35` lets a municipal-tax query retrieve an unrelated office-hours document with the real embedding model, which is a Constitution §5-adjacent concern for the *production* threshold, not a defect in this plan's deliverable. Explicitly out of scope here (plan Non-Goals: no `rag_engine` changes); flagged for a follow-up, not a blocker on this review. |
| Ingest correctness | pass | The document-seeding step goes through the real `/admin/api/upload` → `/admin/api/upload/confirm/:token` flow (real `ingest-core` chunk/embed/insert), never writing to `kb.db` directly, matching `spontini-ingest-flow`'s rule that `Given` steps must go through a real port, not a direct DB write. |
| Tests (coverage + TDD + BDD) | pass | `backend/tests/bdd_e2e.rs` is a test file, correctly excluded from the `cargo tarpaulin --exclude-files '**/tests/**'` coverage gate (consistent with the existing `bdd.rs`) — no coverage regression. The Cargo `[features]`/`[[test]]` additions are configuration, not logic, and don't introduce an uncovered branch. Scenarios are BDD-first by construction (reusing the pre-existing, already-reviewed `features/chat.feature` verbatim, per the plan's explicit design — no new scenario text was authored, only new step implementations bound to a real HTTP client). No `#[ignore]`, no deleted tests, no hardcoded assertions — assertions are deliberately loosened only where real-model non-determinism makes exact-match assertions structurally wrong (documented rationale in both the plan and the code comments). |
| Clean Code | pass | Step definitions are small, one behavior each; the manually-encoded multipart helper is a direct, commented port of the existing `bdd.rs` pattern rather than a new abstraction; env-var configuration (`E2E_BASE_URL`, `E2E_ADMIN_API_KEY`) is documented in the file's module doc comment. No magic numbers beyond the two clearly-named env-var defaults. |
| Clean Design (UI/UX) | n/a | No UI touched. |
| Plan conformance | pass | All 3 tasks across both phases produced their exact listed deliverables and passed their listed verifications (`cargo check`/`clippy`/`fmt` clean; `make help` lists `bdd-e2e`; both docs updated). One implementation-time correction was made beyond the plan's literal text: the plan's Task 1.2 "What" did not anticipate that registering `bdd_e2e` as a plain `[[test]]` would make it run under `make verify`/`make test`/`make coverage` by default (violating the plan's own Non-Goals and Acceptance Criteria) — this was caught during the mandated `spontini-verify-gate` pass between tasks and fixed with a `required-features = ["e2e"]` gate plus a matching `[features] e2e = []` addition to `backend/Cargo.toml`, re-verified by confirming `cargo test --workspace --all-targets --no-run` no longer builds `bdd_e2e` without the flag. This is a necessary correction within Task 1.1/1.2's own scope (making the stated Non-Goal actually hold), not scope creep. |

## Coverage Report

- Line coverage on changed files: n/a for `backend/tests/bdd_e2e.rs` (excluded from the coverage gate, same as `backend/tests/bdd.rs`, per `Makefile`'s `coverage` target: `--exclude-files '**/main.rs' '**/tests/**'`).
- Branch coverage on changed files: n/a, same reason.
- Excluded files: `backend/tests/bdd_e2e.rs` (test file, pre-existing exclusion pattern — no new entry needed in `coverage-exclusions.txt`). `backend/Cargo.toml`, `Makefile`, `README.md`, `docs/STACK.md` are configuration/docs, not measured by `cargo tarpaulin`.

## Required Fixes Before Close

None — verdict is `approved`. Proceed directly to `/fix-review 0025` (or close manually) with no required changes. The two nits above are non-blocking polish, not required for close.

## Fix Log

No required fixes (verdict was `approved`). Plan closed directly.
