# Review 0019: admin-ui accessibility + keyboard audit

- **Plan**: [0019-admin-ui-accessibility-keyboard-audit-plan.md](./0019-admin-ui-accessibility-keyboard-audit-plan.md)
- **Branch**: feat/admin-ui-accessibility-keyboard-audit
- **Reviewed**: 2026-07-24
- **Reviewer**: agent
- **Verdict**: approved

## Summary

This is a focused, well-executed audit that goes beyond a checklist pass: it found and fixed three real WCAG 2.1 AA gaps (a static `role="note"` on dynamically-appearing error/success callouts that never gets announced by assistive tech — WCAG 4.1.3; a cron-syntax help tooltip reachable only by mouse hover; and three controls under the 44×44px touch-target minimum), backed each fix with real measurements (Chromium's accessibility tree and real `getBoundingClientRect()` readings via Puppeteer, not guesswork), and extended the automated `pa11y` gate to the one route it was missing. All gates pass except a pre-existing, previously-documented `cargo-tarpaulin` environment gap unrelated to this admin-ui-only change. Ships as-is.

## Findings

### Blockers

None.

### Major

None.

### Minor

- **[m1]** `admin-ui/src/styles/_app.scss:9,26,29` — the `44px` touch-target minimum is repeated as a literal in three separate rules with no named constant, which is exactly the "magic number" pattern PRINCIPLES.md §1 asks to avoid, and makes the shared WCAG contract (44×44px, referenced from STACK.md §4.2) implicit rather than named. Suggested fix: extract a Sass variable (e.g. `$touch-target-min: 44px;`) at the top of `_app.scss` and reference it in all three rules.
- **[m2]** No automated regression test guards the three touch-target fixes. They were verified correct during implementation with a throwaway Puppeteer script (`getBoundingClientRect()` against the built app), which was then deleted per the plan's file hygiene — a future edit to `_app.scss`, `DsNav.vue`, or DSI's own sidebar tokens could silently regress any of the three fixed elements under 44px with nothing in CI to catch it. `pa11y`/`axe-core` do not check target size (WCAG 2.5.5/2.5.8 are outside their default rulesets). Suggested fix (optional, not required to close): add a small assertion to `scripts/run-a11y.mjs` (or a new script wired into `make verify`) that measures `.app-shell__nav .list-item`, `.app-shell__dev-link`, and `.span-feedback__segment` via the already-open `pa11y`/Puppeteer browser instance and fails if any dimension is under 44px.
- **[m3]** Task 3.1's deliverable path was changed during implementation from the plan's literal `features/admin_accessibility.feature` to `admin-ui/features/admin_accessibility.feature`. This is a well-justified change — the root `features/` directory is swept unconditionally by `backend/tests/bdd.rs`'s `cucumber().fail_on_skipped().run_and_exit(...)`, which has no tag-filtering; a manual, step-definition-free UI scenario placed there would break `cargo test -p backend --test bdd` (Gate 8 of `spontini-verify-gate`) for every future contributor. Recording this explicitly here for the audit trail, since the plan's task text was not updated to match (per `/implement-plan`'s rule against editing plan deliverables) — the `.feature` file's own header comment already documents the reasoning.
- **[m4]** The `admin-ui/features/admin_accessibility.feature` header is honest that the audit was performed via Chromium's accessibility-tree API rather than a literal live VoiceOver session (no macOS GUI access in the implementing environment), and explicitly recommends a human VoiceOver pass as a follow-up. This is the right call on truthfulness grounds, but it means the plan's acceptance criterion "documents the manual screen-reader smoke test with a recorded pass outcome" is met in letter (a scenario file exists, findings are recorded, three real bugs were fixed as a result) but not in the strictest spirit (no human ever ran VoiceOver). Flagging for visibility, not blocking — the mitigation was already anticipated in the plan's own Risks section ("Manual screen-reader testing is inherently subjective and machine-unverifiable").

### Nits

- **[n1]** `admin-ui/src/styles/_app.scss:29-34` — `.span-feedback__segment` uses `min-width: 44px` in addition to `min-height`, which is correct for short single-word sentence segments (e.g. "Sì.") but means longer sentence segments now render as a slightly taller inline-flex box instead of plain inline text; this is a deliberate, necessary trade-off for the touch-target fix and was already scoped as in-bounds by the plan, not a new design decision — noted only for the next visual QA pass.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | n/a | UI-only accessibility fixes; no port/adapter, crate, or layering surface touched. `DsCallout`'s new `role` computed property keeps a single, stated responsibility (variant → ARIA role mapping). |
| Truthfulness & RAG | n/a | Not touched. |
| Ingest correctness | n/a | Not touched. |
| Tests (coverage + TDD + BDD) | pass | New behavior (`DsCallout` role mapping) has 3 new tests covering all 3 branches (danger/success/default). No coverage tool is configured for admin-ui (`npm run test` = `vitest run`, no `--coverage` flag) — a pre-existing, repo-wide gap noted identically in the 0015–0018 reviews, not introduced here. Touch-target fixes lack a permanent regression test — see m2. |
| Clean Code | pass | Comments explain *why* (WCAG citations), not *what*. No magic numbers except the repeated `44px` literal — see m1. No dead code; `cronHelpId` was correctly removed alongside the tooltip it labeled. |
| Clean Design (UI/UX) | pass | Touch-target fixes directly implement PRINCIPLES.md §6.2 ("Touch targets are at least 44×44 px"); no visual redesign beyond what each concrete violation required, consistent with the plan's Non-Goals. |
| Plan conformance | pass | Every task's deliverables exist and verifications passed (build/test/a11y gates re-run and green after every fix). One documented, justified path deviation on Task 3.1 — see m3. |

## Coverage Report

- Line coverage on changed files: not measured — no coverage tool is configured for admin-ui (pre-existing, repo-wide gap; same as noted in the 0015–0018 reviews).
- Branch coverage on changed files: not measured with tooling; manually audited. `DsCallout`'s new 3-way `role` branch is fully covered by 3 new tests (danger/success/primary+warning). All other changed files (`App.vue`, `ScheduleEditor.vue`, `_app.scss`, `run-a11y.mjs`) introduce no new branches.
- Excluded files: none.

## Required Fixes Before Close

None — verdict is `approved`. The minor findings (m1–m4) are recorded for the audit trail and are optional follow-ups; `/fix-review 0019` may address m1 (trivial) at the implementer's discretion but is not required to close the plan.
