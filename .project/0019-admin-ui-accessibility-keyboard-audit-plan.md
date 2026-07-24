# Plan 0019: admin-ui accessibility + keyboard audit

- **Status**: review
- **Approved**: 2026-07-24 by agent
- **Implemented**: 2026-07-24 by agent
- **Branch**: feat/admin-ui-accessibility-keyboard-audit
- **Feature ID**: 0019
- **Created**: 2026-07-24
- **Owner**: agent

## Objective

Milestone 3 delivered the three admin-ui sections (Ingest, Imprinting, Training) with a first automated a11y baseline from feature 0015: an `axe-core` unit-test suite (`src/__tests__/accessibility.test.ts`) covering every route, and a `pa11y` script (`scripts/run-a11y.mjs`) covering `/`, `/dev`, `/ingest`, `/imprinting`, `/training`. That baseline is a floor, not a ceiling — it was never extended as new routes and interactive widgets (the `/training/:id` span-selection feedback flow in particular) were added, and neither `axe-core` nor `pa11y` can verify keyboard operability, focus-ring visibility, or `prefers-reduced-motion` handling, which require manual and semantic review. Per the [Constitution](../docs/CONSTITUTION.md) §3 Accessibility principle and STACK.md §4.2 (non-negotiable WCAG 2.1 AA), this feature performs a dedicated, focused audit of every admin-ui route against WCAG 2.1 AA: keyboard navigability and operability of every interactive element, a visible focus ring that is never suppressed, correct `aria-label`/`aria-labelledby` on icon-only and ambiguous controls, sufficient color contrast, `prefers-reduced-motion` honored wherever motion exists, touch targets ≥ 44×44 px, and semantic HTML preferred over ARIA. It closes every violation found, extends the automated `pa11y` route list to include `/training/:id`, and adds a documented manual screen-reader smoke-test BDD scenario. In scope: `admin-ui` only. Out of scope: the `frontend` public chat SPA (covered by feature 0023), any new admin-ui feature or route, and backend changes.

## Non-Goals

- No new admin-ui routes, sections, or business features — this is an audit-and-fix pass over existing surfaces only.
- No changes to the `frontend` (citizen-facing) app — that is feature 0023.
- No changes to backend APIs or DTOs.
- No visual redesign beyond what is required to fix a concrete WCAG violation (e.g. contrast or focus-ring fixes).

## Phases

### Phase 1: Automated coverage — extend the a11y gates to every route

Goal: every admin-ui route, including the training session detail view, is covered by both the `axe-core` unit gate and the `pa11y` real-browser gate, and any violation currently escaping detection is caught.

- [x] **Task 1.1** — Add `/training/:id` to the `pa11y` route list
  - What: Extend `ROUTES` in `admin-ui/scripts/run-a11y.mjs` to include a training session detail route (e.g. `/training/1`), seeding whatever fixture data the preview server needs to render it without a live backend (mirroring how `/training` already renders without one, or documenting the seed requirement if the preview build needs a running API).
  - Deliverables:
    - Updated `admin-ui/scripts/run-a11y.mjs` with `/training/1` (or equivalent) added to `ROUTES`
  - Skills to load: spontini-verify-gate
  - Verification: `npm run a11y` (or `make verify` equivalent) runs pa11y against the new route and reports 0 errors, or a genuine violation is found and fixed before this task is checked off.

- [x] **Task 1.2** — Run and fix the full automated gate
  - What: Run `npm run a11y` and the existing `vitest` accessibility suite (`src/__tests__/accessibility.test.ts`) across all routes, and fix any violation surfaced (e.g. missing labels, heading order, landmark issues).
  - Deliverables:
    - Any `.vue` component fixes required to reach zero violations
    - Clean `npm run a11y` output for every route in `ROUTES`
    - Clean `vitest run` output for `accessibility.test.ts`
  - Skills to load: spontini-verify-gate
  - Verification: both commands exit 0 with zero reported violations.

### Phase 2: Manual audit — keyboard, focus, motion, touch targets

Goal: every interactive element across all six routes (`/`, `/dev`, `/ingest`, `/imprinting`, `/training`, `/training/:id`) is keyboard-reachable and operable, focus is always visible, motion respects user preference, and touch targets meet the 44×44 px minimum.

- [x] **Task 2.1** — Keyboard navigability audit and fixes
  - What: Tab through every route in a running `admin-ui` dev server, verifying every button, link, form control, and custom widget (span-selection feedback marker in `TrainingSessionView`, upload dropzone, confirm dialogs) is reachable via Tab/Shift+Tab and operable via Enter/Space/Escape as appropriate; fix any element that is a click-only handler on a non-interactive tag (e.g. a `<div>` with `@click` and no `tabindex`/keyboard handler) by converting it to a semantic `<button>` or adding `tabindex="0"` + `@keydown.enter`/`@keydown.space`.
  - Deliverables:
    - Component fixes for any keyboard-inoperable control found (likely candidates: span-selection markers in `src/components/training/`, dropzone in `src/components/ingest/UploadDropzone.vue`)
  - Skills to load: (none — manual browser audit, no domain skill applies)
  - Verification: manual pass documented in the PR description / plan Fix Log listing each route and confirming full keyboard operability; no click handler remains on a non-focusable, non-semantic element (`grep -rn "@click" src --include=*.vue` cross-checked against element tags for each hit).

- [x] **Task 2.2** — Focus ring audit
  - What: Verify no custom CSS in `admin-ui/src/styles/` or component `<style>` blocks removes or hides the browser/DSI focus ring (`outline: none` / `outline: 0` without a replacement `:focus-visible` style) on any focusable element; DSI's default focus treatment is expected to satisfy this, so the audit is primarily a `grep` + visual confirmation, not new styling.
  - Deliverables:
    - Confirmation there is no `outline: none`/`outline: 0` without a `:focus-visible` replacement, or a fix if one is found
  - Skills to load: (none)
  - Verification: `grep -rn "outline: none\|outline: 0\|outline:none\|outline:0" admin-ui/src` returns no unguarded matches; visual confirmation of a visible focus ring on Tab through each route.

- [x] **Task 2.3** — `prefers-reduced-motion` audit and global guard
  - What: Confirm whether any CSS transition/animation exists in project-owned styles or is introduced by DSI components in use (modals, accordions, spinners); add a global `@media (prefers-reduced-motion: reduce)` rule in `admin-ui/src/styles/_app.scss` that disables/shortens transitions and animations for users who request it, scoped to project-owned classes plus a defensive `*` rule if DSI does not already honor the preference.
  - Deliverables:
    - `prefers-reduced-motion` media query added to `admin-ui/src/styles/_app.scss` (or documented finding that DSI already fully honors it, with evidence)
  - Skills to load: (none)
  - Verification: toggling "reduce motion" in the OS/browser and re-testing any transition-bearing UI (e.g. `DsConfirmDialog` open/close) shows no motion, or the audit note explains why none was needed.

- [x] **Task 2.4** — Touch target size audit
  - What: Measure every clickable control (icon buttons, span-selection markers, close/cancel buttons in dialogs) against the 44×44 px minimum using browser devtools; fix any control below threshold by adjusting padding (never by shrinking visible content) using DSI spacing tokens (`var(--bs-spacer, ...)`), per STACK.md §4.3.
  - Deliverables:
    - Component/style fixes for any control found under 44×44 px
  - Skills to load: (none)
  - Verification: devtools box-model measurement ≥ 44×44 px for every interactive control on every route, documented in the Fix Log.

### Phase 3: Screen-reader smoke test + verify gate

Goal: a documented manual screen-reader pass exists as a BDD scenario, and the zero-violation automated gate is wired as a durable check.

- [x] **Task 3.1** — Manual screen-reader smoke test, documented as a BDD scenario
  - What: Perform a manual VoiceOver (macOS) pass over the `/ingest`, `/imprinting`, and `/training`→`/training/:id` flows, confirming section landmarks, form labels, button purposes, and the feedback-span selection flow are all announced meaningfully; write the scenario and its pass/fail outcome as a Gherkin feature (documentation of a manual test, not an automated one) under `features/admin_accessibility.feature`.
  - Deliverables:
    - `features/admin_accessibility.feature` with a `@manual` tagged scenario describing the screen-reader smoke test steps and expected announcements
  - Skills to load: spontini-bdd-gherkin
  - Verification: the feature file exists, is well-formed Gherkin, and the manual pass is confirmed complete (documented in the plan's Fix Log with the outcome of each step).

- [x] **Task 3.2** — Confirm the zero-violation gate is enforced in `make verify`
  - What: Verify `npm run a11y` and `vitest run` (including `accessibility.test.ts`) are already invoked by the `admin-ui` portion of the root `make verify` target (established in feature 0015); if either is missing from the gate, wire it in.
  - Deliverables:
    - Confirmation (or fix) that `make verify` runs both the `axe-core` unit tests and the `pa11y` script for `admin-ui`
  - Skills to load: spontini-verify-gate
  - Verification: `make verify` run from the repo root exits 0 and its output shows both the vitest accessibility suite and the `a11y` pa11y script executing.

## Acceptance Criteria

- `npm run a11y` (admin-ui) reports 0 errors across `/`, `/dev`, `/ingest`, `/imprinting`, `/training`, and `/training/:id`.
- `vitest run` for `admin-ui/src/__tests__/accessibility.test.ts` passes with zero axe-core violations on every route.
- Every interactive element on every route is keyboard-reachable and operable (documented manual pass in the Fix Log).
- No focusable element has its focus ring suppressed without a `:focus-visible` replacement.
- `prefers-reduced-motion` is honored for any transition/animation in project-owned styles.
- Every interactive control measures ≥ 44×44 px.
- `features/admin_accessibility.feature` documents the manual screen-reader smoke test with a recorded pass outcome.
- `make verify` runs the admin-ui a11y gates and exits 0.

## Risks

- Some violations may only be fixable by patching DSI component usage rather than project code (DSI is a third-party dependency) — mitigation: scope fixes to project-owned wrapper components (`src/components/ds/`) and CSS overrides, consistent with ADR 0009's precedent of patching DSI where necessary.
- The `/training/:id` route needs fixture data to render meaningfully in the `pa11y` preview server (which serves a static build, not a live backend) — mitigation: reuse the same in-memory/mock approach already used for `/dev` catalog rendering, or accept a documented empty-state pa11y pass if a full mock is out of proportion to this audit's scope.
- Manual screen-reader testing is inherently subjective and machine-unverifiable — mitigation: document exact steps and expected announcements in the Gherkin scenario so the pass is reproducible by any operator with VoiceOver.

## Out-of-Scope

- The `frontend` public chat SPA (feature 0023 covers its accessibility audit).
- New admin-ui features, routes, or business logic.
- Backend or API changes.
- Automated screen-reader testing tooling (no mature open-source automated NVDA/VoiceOver test runner is in scope; the smoke test stays manual, per the roadmap description).
