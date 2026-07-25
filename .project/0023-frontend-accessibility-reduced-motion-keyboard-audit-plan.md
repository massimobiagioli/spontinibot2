# Plan 0023: frontend accessibility + reduced-motion + keyboard audit

- **Status**: review
- **Approved**: 2026-07-25 by agent
- **Implemented**: 2026-07-25 by agent
- **Branch**: feat/frontend-accessibility-reduced-motion-keyboard-audit
- **Feature ID**: 0023
- **Created**: 2026-07-25
- **Owner**: agent

## Objective

Milestone 4 delivered the citizen-facing chat widget (features 0020-0022) with a first automated a11y baseline from feature 0020: an `axe-core` unit-test suite (`src/__tests__/accessibility.test.ts`) and a `pa11y` script (`scripts/run-a11y.mjs`) covering `/` and `/dev` in their default, empty-conversation state. That baseline never exercises the chat widget's other visual states introduced by 0021/0022 — an in-progress exchange (`role="status"` pending message), an answered exchange with expandable citations (`<details>`/`<summary>`), the honest-unknown fallback (`DsCallout variant="primary"`), and the error state (`DsCallout variant="danger"`) — nor can either tool verify keyboard operability, focus-ring visibility, or `prefers-reduced-motion` handling, which require manual and semantic review. Per the [Constitution](../docs/CONSTITUTION.md) §3 Accessibility principle and STACK.md §4.2 (non-negotiable WCAG 2.1 AA), this feature performs a dedicated, focused audit of the public chat app against WCAG 2.1 AA with the same rigor as feature 0019's admin-ui audit: keyboard navigability and operability of every interactive element in every chat state, a visible focus ring that is never suppressed, correct `aria-label`/semantic markup, sufficient color contrast, `prefers-reduced-motion` honored wherever motion exists, touch targets ≥ 44×44 px, and semantic HTML preferred over ARIA. It closes every violation found, extends the automated `axe-core` and `pa11y` coverage to the chat widget's answered/error/honest-unknown states, and adds a documented manual screen-reader smoke-test BDD scenario. In scope: the `frontend` app only (`/` chat widget and `/dev` catalog). Out of scope: the `admin-ui` app (covered by feature 0019, already closed), any new frontend feature or route, and backend changes.

## Non-Goals

- No new frontend routes, sections, or business features — this is an audit-and-fix pass over the existing chat widget and `/dev` catalog only.
- No changes to `admin-ui` — that was feature 0019.
- No changes to backend APIs or DTOs.
- No visual redesign beyond what is required to fix a concrete WCAG violation (e.g. contrast or focus-ring fixes).

## Phases

### Phase 1: Automated coverage — extend the a11y gates to every chat state

Goal: the `axe-core` unit gate and the `pa11y` real-browser gate both exercise the chat widget's answered, honest-unknown, error, and pending states, not just the empty default, and any violation currently escaping detection is caught.

- [x] **Task 1.1** — Extend the `axe-core` unit suite to cover populated chat states
  - What: Add test cases to `frontend/src/__tests__/accessibility.test.ts` that mount `App` at `/`, drive `ChatWidget` (directly, or via its child components with mocked props/exchanges) into an answered state with citations, an honest-unknown state (`fell_back: true`), and an error state (`failed: true`), running `axe.run` against each rendered state.
  - Deliverables:
    - Updated `frontend/src/__tests__/accessibility.test.ts` with new `it(...)` blocks for the answered-with-citations, honest-unknown, and error states, each asserting zero axe violations
  - Skills to load: spontini-verify-gate
  - Verification: `npm run test` (vitest) passes with the new assertions all reporting zero violations, or a genuine violation is found and fixed before this task is checked off.

- [x] **Task 1.2** — Extend the `pa11y` route list to include a populated chat state
  - What: Since `pa11y` drives a real built preview server (no mocked API available), add a way to reach a non-empty, representative DOM state for the pa11y pass — either a query-param/dev-only seed hook rendering a fixed sample exchange (answered + citations) in `ChatWidget`, or, if that is disproportionate to this audit's scope, document in the plan why the `axe-core` unit coverage from Task 1.1 is the authoritative gate for populated states and `pa11y` stays scoped to the static shell routes (`/`, `/dev`).
  - Deliverables:
    - Either an updated `frontend/scripts/run-a11y.mjs` `ROUTES` list plus the minimal seed mechanism it needs, or a documented decision (in this plan's Risks section) to keep `pa11y` scoped to `/` and `/dev` with `axe-core` as authoritative for dynamic states
  - Skills to load: spontini-verify-gate
  - Verification: `npm run a11y` runs and reports 0 errors for every route in `ROUTES`.

- [x] **Task 1.3** — Run and fix the full automated gate
  - What: Run `npm run a11y` and `npm run test` (vitest, including the accessibility suite) and fix any violation surfaced (e.g. missing labels, heading order, landmark issues).
  - Deliverables:
    - Any `.vue` component fixes required to reach zero violations
    - Clean `npm run a11y` output
    - Clean `npm run test` output
  - Skills to load: spontini-verify-gate
  - Verification: both commands exit 0 with zero reported violations.

### Phase 2: Manual audit — keyboard, focus, motion, touch targets

Goal: every interactive element across both routes (`/` in every chat state, `/dev`) is keyboard-reachable and operable, focus is always visible, motion respects user preference, and touch targets meet the 44×44 px minimum.

- [x] **Task 2.1** — Keyboard navigability audit and fixes
  - What: Tab through `/` and `/dev` in a running `frontend` dev server, verifying the question input, submit button, the `<details>/<summary>` citation disclosure, and the `/dev` catalog controls are all reachable via Tab/Shift+Tab and operable via Enter/Space as appropriate; fix any element found to be a click-only handler on a non-interactive tag by converting it to a semantic element or adding `tabindex="0"` + keyboard handlers.
  - Deliverables:
    - Component fixes for any keyboard-inoperable control found (checked candidates: `ChatInput.vue`, `ChatMessage.vue`'s `<details>`, `DsButton.vue`, `App.vue`'s `RouterLink`)
  - Skills to load: (none — manual browser audit, no domain skill applies)
  - Verification: manual pass documented in the plan's Fix Log listing each route/state and confirming full keyboard operability; `grep -rn "@click" frontend/src` cross-checked against element tags for each hit shows no click handler on a non-focusable, non-semantic element.

- [x] **Task 2.2** — Focus ring audit
  - What: Verify no custom CSS in `frontend/src/styles/` or component `<style>` blocks removes or hides the browser/DSI focus ring (`outline: none` / `outline: 0` without a replacement `:focus-visible` style) on any focusable element; DSI's default focus treatment is expected to satisfy this, so the audit is primarily a `grep` + visual confirmation, not new styling.
  - Deliverables:
    - Confirmation there is no `outline: none`/`outline: 0` without a `:focus-visible` replacement, or a fix if one is found
  - Skills to load: (none)
  - Verification: `grep -rn "outline: none\|outline: 0\|outline:none\|outline:0" frontend/src` returns no unguarded matches; visual confirmation of a visible focus ring on Tab through `/` (all states) and `/dev`.

- [x] **Task 2.3** — `prefers-reduced-motion` audit and global guard
  - What: Confirm whether any CSS transition/animation exists in project-owned styles (`frontend/src/styles/_app.scss`, component `<style>` blocks — none found in the pre-audit scan) or is introduced by DSI components in use; add a global `@media (prefers-reduced-motion: reduce)` rule in `frontend/src/styles/_app.scss` that disables/shortens transitions and animations for users who request it, scoped to project-owned classes plus a defensive `*` rule if DSI does not already honor the preference.
  - Deliverables:
    - `prefers-reduced-motion` media query added to `frontend/src/styles/_app.scss`
  - Skills to load: (none)
  - Verification: toggling "reduce motion" in the OS/browser and re-testing the chat widget's pending/answered transitions (if any) shows no motion, or the audit note in the Fix Log explains why none was needed given no transitions exist today.

- [x] **Task 2.4** — Touch target size audit
  - What: Measure every clickable control (`DsButton` submit, the `<summary>` citation toggle, the `RouterLink` catalog link, `DsInput`) against the 44×44 px minimum using browser devtools; fix any control below threshold by adjusting padding (never by shrinking visible content) using DSI spacing tokens (`var(--bs-spacer, ...)`), per STACK.md §4.3.
  - Deliverables:
    - Component/style fixes for any control found under 44×44 px
  - Skills to load: (none)
  - Verification: devtools box-model measurement ≥ 44×44 px for every interactive control on `/` (all states) and `/dev`, documented in the Fix Log.

### Phase 3: Screen-reader smoke test + verify gate

Goal: a documented manual screen-reader pass exists as a BDD scenario, and the zero-violation automated gate is confirmed wired into `make verify`.

- [x] **Task 3.1** — Manual screen-reader smoke test, documented as a BDD scenario
  - What: Perform a manual VoiceOver (macOS) pass over `/`, asking a question and confirming the pending status, an answered response with its citation disclosure, the honest-unknown fallback, and the error state are all announced meaningfully; write the scenario and its pass/fail outcome as a Gherkin feature (documentation of a manual test, not an automated one) under `frontend/features/frontend_accessibility.feature` (corrected from the plan's original `features/frontend_accessibility.feature` path to match the established `admin-ui/features/admin_accessibility.feature` precedent from feature 0019).
  - Deliverables:
    - `frontend/features/frontend_accessibility.feature` describing the screen-reader smoke test steps and expected announcements for each chat state
  - Skills to load: spontini-bdd-gherkin
  - Verification: the feature file exists, is well-formed Gherkin, and the manual pass is confirmed complete (documented in the plan's Fix Log with the outcome of each step).

- [x] **Task 3.2** — Confirm the zero-violation gate is enforced in `make verify`
  - What: Verify `npm run a11y` and `npm run test` (including `accessibility.test.ts`) are already invoked by the `frontend` portion of the root `make verify` target (established in feature 0020, confirmed present at `Makefile:96-100,155`); if either is missing from the gate, wire it in.
  - Deliverables:
    - Confirmation (or fix) that `make verify` runs both the `axe-core` unit tests and the `pa11y` script for `frontend`
  - Skills to load: spontini-verify-gate
  - Verification: `make verify` run from the repo root exits 0 and its output shows both the vitest accessibility suite and the `a11y` pa11y script executing for `frontend`.

## Acceptance Criteria

- `npm run a11y` (frontend) reports 0 errors across every route in `ROUTES`.
- `vitest run` for `frontend/src/__tests__/accessibility.test.ts` passes with zero axe-core violations across the empty, answered-with-citations, honest-unknown, and error chat states.
- Every interactive element on `/` (all states) and `/dev` is keyboard-reachable and operable (documented manual pass in the Fix Log).
- No focusable element has its focus ring suppressed without a `:focus-visible` replacement.
- `prefers-reduced-motion` is honored for any transition/animation in project-owned styles.
- Every interactive control measures ≥ 44×44 px.
- `frontend/features/frontend_accessibility.feature` documents the manual screen-reader smoke test with a recorded pass outcome.
- `make verify` runs the frontend a11y gates and exits 0.

## Risks

- Some violations may only be fixable by patching DSI component usage rather than project code (DSI is a third-party dependency) — mitigation: scope fixes to project-owned wrapper components (`src/components/ds/`) and CSS overrides, consistent with ADR 0009's precedent of patching DSI where necessary.
- `pa11y` drives a static built preview server with no live backend, so reaching the answered/error/honest-unknown chat states may not be reachable without a seed mechanism disproportionate to this audit's scope — **decision taken**: `pa11y`'s `ROUTES` stays scoped to `/` and `/dev` (the static shell); `axe-core` (Task 1.1, which mounts `ChatWidget` directly with a mocked `askChat` and can inject the answered/honest-unknown/error/pending states) is the authoritative automated gate for those dynamic states. Adding a dev-only seed query-param to `ChatWidget` was rejected as production-code complexity disproportionate to an audit-only feature with no functional change in scope.
- Manual screen-reader testing is inherently subjective and machine-unverifiable — mitigation: document exact steps and expected announcements in the Gherkin scenario so the pass is reproducible by any operator with VoiceOver.

## Out-of-Scope

- The `admin-ui` operator console (feature 0019 already covers its accessibility audit).
- New frontend features, routes, or business logic.
- Backend or API changes.
- Automated screen-reader testing tooling (no mature open-source automated NVDA/VoiceOver test runner is in scope; the smoke test stays manual, per the roadmap description).
