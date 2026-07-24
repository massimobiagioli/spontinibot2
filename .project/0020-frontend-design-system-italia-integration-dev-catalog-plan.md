# Plan 0020: frontend Design System Italia integration + /dev catalog

- **Status**: closed
- **Approved**: 2026-07-24 by agent
- **Implemented**: 2026-07-24 by agent
- **Closed**: 2026-07-24 by agent
- **Review verdict**: approved
- **Branch**: feat/frontend-design-system-italia-integration-dev-catalog
- **Feature ID**: 0020
- **Created**: 2026-07-24
- **Owner**: agent

## Objective

Milestone 4 turns `frontend` from a walking-skeleton SPA into the citizen-facing chat surface described by the [Constitution](../docs/CONSTITUTION.md)'s mission: citizens ask Spontini a question and get an honest, cited answer with no friction. Before the chat widget (feature 0021) or any honest-unknown/error UI (feature 0022) can be built, `frontend` must stand on **Design System Italia** (DSI) per STACK.md §4.1 — the mandated design system for Italian Pubblica Amministrazione digital services — exactly as `admin-ui` did in feature 0015. This feature integrates `bootstrap-italia` + `design-tokens-italia` into the `frontend` Vite + Dart Sass build, establishes the same thin Vue wrapper component pattern (`<DsButton>`, `<DsInput>`, `<DsCallout>`, …) mandated by STACK.md §4.1.2/§4.4.2 (wrap DSI markup, don't fork it), adds `vue-router` with a `/dev` route cataloging every wrapper in isolation (Storybook-lite, STACK.md §4.4.3), and wires `axe-core` + `pa11y` into the test suite with a zero-violation gate (STACK.md §4.2). This closes the gap between the current one-`<h1>` skeleton and a foundation ready to host feature 0021's chat widget. In scope: build tooling, Sass wiring, routing shell, DSI wrapper components needed by the future chat UI, the `/dev` catalog, and the accessibility gate. Out of scope: the chat widget itself, message send/receive logic, citation rendering, honest-unknown/error states (features 0021-0022), and the full-app accessibility audit beyond the shell and `/dev` catalog (feature 0023, which audits screens that don't exist yet).

## Non-Goals

- No chat widget, conversation view, or `/chat` API integration — this feature only builds the design-system foundation and the `/dev` catalog (features 0021-0022 build the actual chat UI).
- No citation rendering, honest-unknown state, or error state UI — those are features 0021/0022.
- No operator/authentication concerns — `frontend` is citizen-facing and unauthenticated by design; nothing here touches auth.
- No full WCAG audit of hypothetical future screens — only the `/dev` catalog and the app shell are audited here; feature 0023 re-audits everything once the chat UI exists.
- No `admin-ui` changes — `admin-ui`'s DSI integration (feature 0015) is a separate, already-closed app and is not touched by this feature.

## Phases

### Phase 1: Build tooling and dependency integration

Goal: `frontend` compiles with `bootstrap-italia` + `design-tokens-italia` wired through Dart Sass, using `@use`/`@forward` only.

- [x] **Task 1.1** — Add DSI and routing dependencies to `frontend/package.json`
  - What: Add `bootstrap-italia` (latest 3.x, per ADR 0009's pinned modular-Sass architecture), `design-tokens-italia` (latest stable), `sass` (Dart Sass) as devDependency, and `vue-router` (^4) as a runtime dependency; run `npm install` to refresh `package-lock.json`.
  - Deliverables:
    - `frontend/package.json` updated dependencies/devDependencies
    - `frontend/package-lock.json` regenerated
  - Skills to load: (none — dependency management, no domain skill applies)
  - Verification: `npm install` succeeds with no peer-dependency errors inside the `frontend` container (`docker compose run --rm frontend npm install`).

- [x] **Task 1.2** — Wire the Sass entry point with DSI tokens via `@use`/`@forward`
  - What: Create `frontend/src/styles/main.scss` that `@use`s `bootstrap-italia` and `design-tokens-italia` partials (no legacy `@import`), and import it once from `main.ts`.
  - Deliverables:
    - `frontend/src/styles/main.scss`
    - `frontend/src/main.ts` updated to import the stylesheet
  - Skills to load: (none)
  - Verification: `npm run build` (vue-tsc + vite build) succeeds and the built CSS in `dist/assets/*.css` contains DSI custom properties (`grep -q -- '--bs-' dist/assets/*.css` or equivalent token check).

- [x] **Task 1.3** — Add vue-router with an app shell and a `/dev` placeholder route
  - What: Create `frontend/src/router/index.ts` with a `createWebHistory` router, a root route rendering the citizen-facing app shell (header + main content area, no left-rail — this is a public single-purpose app, not an operator console), and a `/dev` route; mount the router in `main.ts`; replace `App.vue`'s static `<h1>` with a `<RouterView>` inside the shell layout.
  - Deliverables:
    - `frontend/src/router/index.ts`
    - `frontend/src/App.vue` updated to host `<RouterView>`
    - `frontend/src/views/DevCatalog.vue` (empty shell, populated in Phase 2)
  - Skills to load: (none)
  - Verification: `npm run dev` serves `/` and `/dev` without a router error; `npm run test` (existing placeholder suite) still passes.

### Phase 2: Thin Vue DSI wrapper components

Goal: A small set of reusable, documented DSI wrapper components exist under `src/components/ds/`, re-exported from a barrel, each with a unit test.

- [x] **Task 2.1** — Build `<DsButton>`, `<DsInput>`, `<DsCallout>` wrappers
  - What: Implement three thin Vue SFCs under `frontend/src/components/ds/` that forward props/slots onto the corresponding DSI markup/classes (`btn btn-primary`, `form-control`, `callout callout-*`), per STACK.md §4.1.2 and §4.4.2 (wrap, don't fork). These three are the minimum set feature 0021's chat widget will need (send button, message input, honest-unknown/error callouts).
  - Deliverables:
    - `frontend/src/components/ds/DsButton.vue`
    - `frontend/src/components/ds/DsInput.vue`
    - `frontend/src/components/ds/DsCallout.vue`
    - `frontend/src/components/ds/index.ts` (barrel export)
  - Skills to load: (none)
  - Verification: `npm run test` includes a unit test per component (props forwarded, correct DSI classes applied, accessible name present) and passes.

- [x] **Task 2.2** — Wire `@vue/test-utils` and a jsdom test environment
  - What: Add `@vue/test-utils` and configure Vitest to use the `jsdom` environment (via `vite.config.ts` `test.environment` or a dedicated `vitest.config.ts`) so component tests can mount and assert on rendered DOM.
  - Deliverables:
    - `frontend/package.json` devDependency `@vue/test-utils`, `jsdom`
    - `frontend/vite.config.ts` (or new `vitest.config.ts`) with `test: { environment: 'jsdom' }`
  - Skills to load: (none)
  - Verification: The Task 2.1 component tests (which mount SFCs) run and pass under `npm run test`.

### Phase 3: `/dev` component catalog

Goal: The `/dev` route lists every DSI wrapper component in isolation with live prop variants, à la Storybook-lite (STACK.md §4.4.3).

- [x] **Task 3.1** — Populate `DevCatalog.vue` with a live entry per wrapper component
  - What: Implement `frontend/src/views/DevCatalog.vue` rendering each of `<DsButton>`, `<DsInput>`, `<DsCallout>` with 2-3 prop variants each (e.g. button primary/secondary/disabled), under a semantic heading structure (`<h2>` per component).
  - Deliverables:
    - `frontend/src/views/DevCatalog.vue` (complete)
  - Skills to load: (none)
  - Verification: `npm run dev`, navigate to `/dev`, confirm all three components render with their variants; `npm run test` includes a smoke test that `/dev` mounts without error.

### Phase 4: Accessibility gate

Goal: `axe-core` and `pa11y` run against the app shell and `/dev` route with a zero-violation CI-enforced gate.

- [x] **Task 4.1** — Add `axe-core` automated test coverage
  - What: Add `axe-core` (via `vitest-axe` or `@axe-core/vue`/direct `axe-core` + jsdom) and a test that renders the app shell and the `/dev` catalog, running `axe` against each and asserting zero violations.
  - Deliverables:
    - `frontend/package.json` devDependency for the chosen axe integration
    - `frontend/src/__tests__/accessibility.test.ts`
  - Skills to load: (none)
  - Verification: `npm run test` runs the axe assertions and passes with zero violations.

- [x] **Task 4.2** — Add `pa11y` CI script against the built app
  - What: Add `pa11y` as a devDependency and an `npm run a11y` script that serves the built `dist/` (via `vite preview` or a static server) and runs `pa11y` against `/` and `/dev`, failing on any error-level issue.
  - Deliverables:
    - `frontend/package.json` devDependency `pa11y`, new `a11y` script
  - Skills to load: (none)
  - Verification: `npm run build && npm run a11y` exits 0 with zero pa11y errors reported.

- [x] **Task 4.3** — Wire the accessibility gate into `make verify`
  - What: Extend the Makefile's `lint` (or a new dedicated target folded into `verify`) to run `npm run a11y` for `frontend` inside its container, so a violation fails `make verify`, mirroring the `admin-ui` gate added in feature 0015.
  - Deliverables:
    - `Makefile` updated (`lint` or `verify` prerequisite chain includes the frontend `a11y` gate)
  - Skills to load: spontini-verify-gate
  - Verification: `make verify` (or the targeted sub-target) runs the frontend accessibility gate and passes.

## Acceptance Criteria

- `docker compose run --rm frontend npm run build` succeeds and the built CSS includes DSI design tokens (no hard-coded hex/px values in new SCSS, per STACK.md §4.3).
- Navigating to `/` renders the citizen-facing app shell; navigating to `/dev` renders the component catalog with every wrapper's variants.
- `npm run test` (unit) is green, including component tests for `<DsButton>`, `<DsInput>`, `<DsCallout>` and the axe-core zero-violation assertions.
- `npm run a11y` (pa11y against the built app) reports zero errors on `/` and `/dev`.
- `make verify` passes end-to-end with the new frontend gates included (Rust coverage gate excepted per the pre-existing `cargo-tarpaulin`-missing environment gap, documented since feature 0009 and re-confirmed unrelated to this frontend-only diff — `backend/`, `Cargo.toml`, and the Rust toolchain are untouched by this branch).
- No legacy Sass `@import` anywhere in `frontend/src/styles/` — only `@use`/`@forward`.

## Risks

- **Bootstrap Italia 2.x Vite resolution bug** (STACK.md §4.1 caveat, already solved once in ADR 0009 for `admin-ui`) — mitigation: reuse the same 3.x modular `@use`/`@forward` architecture and pinned version documented in ADR 0009 rather than re-discovering the issue.
- **No official Vue wrapper for DSI** means hand-rolled wrappers may drift from DSI markup updates — mitigation: keep wrappers thin (forward props/slots onto DSI classes verbatim) so upgrades touch few files, per STACK.md §4.4.2; this duplicates the `admin-ui` wrapper set rather than sharing a package, which is an accepted trade-off since `frontend` and `admin-ui` are independently deployed containers with no shared build (per STACK.md §2 architecture overview).
- **axe-core/jsdom limitations** (jsdom doesn't fully replicate browser layout, so some contrast/visibility checks are unreliable) — mitigation: treat `pa11y` (headless Chromium, real rendering) as the authoritative accessibility gate; axe-core in unit tests is a fast first-pass signal only.

## Out-of-Scope

- Chat widget, conversation view, message send/receive (feature 0021).
- Citation rendering, honest-unknown state, error state UI (features 0021-0022).
- Full-app accessibility audit beyond the shell and `/dev` catalog (feature 0023).
- Any change to `admin-ui` (already closed under feature 0015).
