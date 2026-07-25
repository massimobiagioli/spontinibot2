# Review 0023: frontend accessibility + reduced-motion + keyboard audit

- **Plan**: [0023-frontend-accessibility-reduced-motion-keyboard-audit-plan.md](./0023-frontend-accessibility-reduced-motion-keyboard-audit-plan.md)
- **Branch**: feat/frontend-accessibility-reduced-motion-keyboard-audit
- **Reviewed**: 2026-07-25
- **Reviewer**: agent
- **Verdict**: approved

## Summary

This is a focused, well-scoped accessibility audit of the `frontend` chat widget, mirroring feature 0019's admin-ui audit. It extends the `axe-core` unit suite to cover the answered/honest-unknown/error/pending chat states that the original 0020 baseline never exercised, closes a real WCAG 4.1.3 gap (the honest-unknown fallback used a static `role="note"` that is never auto-announced), and fixes two genuine sub-44px touch targets (`.form-control` at 40px, and a bare `<summary>` with no sizing at all) plus one unguarded CSS transition, all backed by concrete evidence from the compiled DSI CSS rather than assumption. Scope stayed inside the plan's Non-Goals — no visual redesign, no new routes, no backend changes. Ships as-is.

## Findings

### Blockers

None.

### Major

None.

### Minor

None.

### Nits

- **[n1]** `frontend/src/styles/_app.scss:32-39` — the defensive `@media (prefers-reduced-motion: reduce)` block targets `*, *::before, *::after` globally rather than only DSI's unguarded `.btn` transition. This is the standard, widely-used pattern and is explicitly permitted by the plan's Task 2.3 wording ("a defensive `*` rule if DSI does not already honor the preference"), so no fix is required — noting it only because a narrower `.btn:hover` override would have been marginally more surgical.
- **[n2]** The plan's Task 1.2 and Task 3.1 originally specified `features/frontend_accessibility.feature` (repo root); the implementation correctly deviated to `frontend/features/frontend_accessibility.feature` to match the actual `admin-ui/features/admin_accessibility.feature` precedent from feature 0019, and the plan file itself was updated in-place to document the correction. This is the right call, not a defect.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | pass | No domain/application/port code touched. `DsCallout`'s new `role` prop is additive and backward-compatible (defaults preserve every existing consumer's rendered role, verified by grep of all `DsCallout` call sites); SRP intact — the component still owns only its own rendering, the role-override decision is made by the caller (`ChatMessage.vue`) that knows the semantic context. |
| Truthfulness & RAG | n/a | No change to prompt assembly, retrieval, citation logic, or answer content. Only the ARIA announcement metadata of an already-existing, already-tested honest-unknown message changed. |
| Ingest correctness | n/a | Not touched. |
| Tests (coverage + TDD + BDD) | pass | Every new/changed branch has a dedicated test: `DsCallout`'s `role`-override branch (new test), `ChatMessage`'s `role="status"` rendering (new test, replacing the now-stale `role="note"` assertion), and the four new chat-widget-state `axe-core` cases (answered+citations, honest-unknown, error, pending). All 44 tests pass; `vue-tsc --noEmit` is clean; `npm run a11y` (real headless Chromium via pa11y) reports 0 errors on `/` and `/dev` post-fix. `frontend/features/frontend_accessibility.feature` documents the manual screen-reader scenario per the BDD skill's "document a manual test, not wire it to the cucumber runner" precedent from 0019. No coverage tool is configured for `frontend` (pre-existing, repo-wide gap since feature 0015, same as every prior frontend review including 0021 and 0022) — branches were manually audited instead, as in those reviews. |
| Clean Code | pass | Names reveal intent (`computedRole`, `CalloutRole`). Comments explain non-obvious WHY (the `note`-role gap, the DSI token evidence for the 40px input and the unguarded `.btn` transition) rather than restating WHAT. No magic numbers — `44px` matches the project's existing convention (`app-shell__dev-link`) and STACK.md §4.2/PRINCIPLES.md §6.2. No dead code, no scope creep beyond the plan's stated deliverables. |
| Clean Design (UI/UX) | pass | No visual redesign: the `.form-control` and `<summary>` fixes only adjust box size via `min-height`/padding, never shrinking visible content, exactly as the plan's Task 2.4 required. The reduced-motion guard and role fix are behavioral, not visual. Honest states (pending/error/honest-unknown) were not altered in content, only in whether they are announced. |
| Plan conformance | pass | Every task (1.1-3.2) is checked off with its stated deliverables present and verification passing. Task 1.2's decision to keep `pa11y` scoped to `/` and `/dev` (documented in the plan's Risks section) is a legitimate, plan-sanctioned alternative to a disproportionate seed-mechanism build-out. No unrequested scope creep: the `DsCallout` `role`-prop fix is squarely inside "closes every violation found" (Objective) and Task 3.1's screen-reader-focused review, not an added feature. |

## Coverage Report

- Line coverage on changed files: not mechanically measured — no coverage tool is configured for `frontend` (pre-existing, repo-wide gap since feature 0015, same as every prior frontend/admin-ui review, including 0021's and 0022's).
- Branch coverage on changed files: not mechanically measured; manually audited. `DsCallout.vue`'s new `if (props.role) return props.role` branch is covered by a dedicated test; its three pre-existing branches (`danger`/`success`/default) remain covered by pre-existing tests. `ChatMessage.vue` introduced no new branch (a static prop value, not a condition). No new untested branch was introduced.
- Excluded files: none.
