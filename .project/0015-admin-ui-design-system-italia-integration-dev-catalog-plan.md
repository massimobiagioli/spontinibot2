# Plan 0015: admin-ui Design System Italia integration + /dev catalog

- **Status**: review
- **Approved**: 2026-07-24 by agent
- **Implemented**: 2026-07-24 by agent
- **Branch**: feat/admin-ui-design-system-italia-integration-dev-catalog
- **Feature ID**: 0015
- **Created**: 2026-07-24
- **Owner**: agent

## Objective

Milestone 3 turns `admin-ui` from a walking-skeleton SPA into the operator console that drives the whole Spontini system. Before any business section (Ingest, Imprinting, Training) can be built, `admin-ui` must be built on **Design System Italia** (DSI) per STACK.md §4.1 — the Italian Pubblica Amministrazione's mandated design system — because every subsequent screen composes DSI primitives, and retrofitting a design system after screens exist is expensive and error-prone. This feature integrates `bootstrap-italia` + `design-tokens-italia` into the Vite + Dart Sass build, establishes the thin Vue wrapper component pattern (`<DsButton>`, `<DsInput>`, `<DsCallout>`, `<DsNav>`, …) mandated by STACK.md §4.1/§4.4, adds vue-router with a `/dev` route that catalogs every wrapper in isolation (Storybook-lite, per STACK.md §4.4.3), and wires `axe-core` + `pa11y` into the test suite with a zero-violation gate (STACK.md §4.2). This closes the gap between the current one-`<h1>` skeleton and a system ready to host feature 0016's Ingest section. Out of scope: any business section (Ingest/Imprinting/Training — features 0016-0018), authenticated routing/auth guards (feature 0027), and the full accessibility audit pass across business screens (feature 0019, which audits screens that don't exist yet).

## Non-Goals

- No Ingest, Imprinting, or Training screens — this feature only builds the design-system foundation and the `/dev` catalog.
- No operator authentication/session handling — the static shared-secret placeholder from feature 0008 is unrelated to this frontend feature and is not touched.
- No left-rail navigation wiring to real business routes (the nav shell is scaffolded but the three business links are placeholders/disabled until 0016-0018 land).
- No full WCAG audit of hypothetical future screens — only the `/dev` catalog and the app shell are audited here; feature 0019 re-audits everything once all sections exist.

## Phases

### Phase 1: Build tooling and dependency integration

Goal: `admin-ui` compiles with `bootstrap-italia` + `design-tokens-italia` wired through Dart Sass, using `@use`/`@forward` only.

- [x] **Task 1.1** — Add DSI and routing dependencies to `admin-ui/package.json`
  - What: Add `bootstrap-italia` (latest 3.x, modular Sass architecture per STACK.md §4.1 caveat), `design-tokens-italia` (latest stable), `sass` (Dart Sass) as devDependency, and `vue-router` (^4) as a runtime dependency; run `npm install` to refresh `package-lock.json`.
  - Deliverables:
    - `admin-ui/package.json` updated dependencies/devDependencies
    - `admin-ui/package-lock.json` regenerated
  - Skills to load: (none — dependency management, no domain skill applies)
  - Verification: `npm install` succeeds with no peer-dependency errors inside the `admin-ui` container (`docker compose run --rm admin-ui npm install`).

- [x] **Task 1.2** — Wire the Sass entry point with DSI tokens via `@use`/`@forward`
  - What: Create `admin-ui/src/styles/main.scss` that `@use`s `bootstrap-italia` and `design-tokens-italia` partials (no legacy `@import`), and import it once from `main.ts`.
  - Deliverables:
    - `admin-ui/src/styles/main.scss`
    - `admin-ui/src/main.ts` updated to import the stylesheet
  - Skills to load: (none)
  - Verification: `npm run build` (vue-tsc + vite build) succeeds and the built CSS in `dist/assets/*.css` contains DSI custom properties (`grep -q -- '--bs-' dist/assets/*.css` or equivalent token check).

- [x] **Task 1.3** — Add vue-router with an app shell and a `/dev` placeholder route
  - What: Create `admin-ui/src/router/index.ts` with a `createWebHistory` router, a root route rendering the left-rail app shell, and a `/dev` route; mount the router in `main.ts`; replace `App.vue`'s static `<h1>` with a `<RouterView>` inside the shell layout.
  - Deliverables:
    - `admin-ui/src/router/index.ts`
    - `admin-ui/src/App.vue` updated to host `<RouterView>`
    - `admin-ui/src/views/DevCatalog.vue` (empty shell, populated in Phase 2)
  - Skills to load: (none)
  - Verification: `npm run dev` serves `/` and `/dev` without a router error; `npm run test` (existing placeholder suite) still passes.

### Phase 2: Thin Vue DSI wrapper components

Goal: A small set of reusable, documented DSI wrapper components exist under `src/components/ds/`, re-exported from a barrel, each with a unit test.

- [x] **Task 2.1** — Build `<DsButton>`, `<DsInput>`, `<DsCallout>` wrappers
  - What: Implement three thin Vue SFCs under `admin-ui/src/components/ds/` that forward props/slots onto the corresponding DSI markup/classes (`btn btn-primary`, `form-control`, `callout callout-*`), per STACK.md §4.1.2 and §4.4.2 (wrap, don't fork).
  - Deliverables:
    - `admin-ui/src/components/ds/DsButton.vue`
    - `admin-ui/src/components/ds/DsInput.vue`
    - `admin-ui/src/components/ds/DsCallout.vue`
    - `admin-ui/src/components/ds/index.ts` (barrel export)
  - Skills to load: (none)
  - Verification: `npm run test` includes a unit test per component (props forwarded, correct DSI classes applied, accessible name present) and passes.

- [x] **Task 2.2** — Build `<DsNav>` (left-rail navigation shell)
  - What: Implement a `<DsNav>` wrapper rendering the DSI sidebar/nav markup with three placeholder links (Ingest · Imprinting · Training per STACK.md §4.5), semantic `<nav>` + `aria-label`, and a visible active-route indicator using `vue-router`'s `RouterLink`.
  - Deliverables:
    - `admin-ui/src/components/ds/DsNav.vue`
    - Unit test asserting the three links render and the active link gets `aria-current="page"`
  - Skills to load: (none)
  - Verification: `npm run test` passes; manual `npm run dev` check confirms keyboard tab order reaches all three links.

- [x] **Task 2.3** — Wire `@vue/test-utils` and a jsdom test environment
  - What: Add `@vue/test-utils` and configure Vitest to use the `jsdom` environment (via `vite.config.ts` `test.environment` or a dedicated `vitest.config.ts`) so component tests can mount and assert on rendered DOM.
  - Deliverables:
    - `admin-ui/package.json` devDependency `@vue/test-utils`, `jsdom`
    - `admin-ui/vite.config.ts` (or new `vitest.config.ts`) with `test: { environment: 'jsdom' }`
  - Skills to load: (none)
  - Verification: The Task 2.1/2.2 component tests (which mount SFCs) run and pass under `npm run test`.

### Phase 3: `/dev` component catalog

Goal: The `/dev` route lists every DSI wrapper component in isolation with live prop variants, à la Storybook-lite (STACK.md §4.4.3).

- [x] **Task 3.1** — Populate `DevCatalog.vue` with a live entry per wrapper component
  - What: Implement `admin-ui/src/views/DevCatalog.vue` rendering each of `<DsButton>`, `<DsInput>`, `<DsCallout>`, `<DsNav>` with 2-3 prop variants each (e.g. button primary/secondary/disabled), under a semantic heading structure (`<h2>` per component).
  - Deliverables:
    - `admin-ui/src/views/DevCatalog.vue` (complete)
  - Skills to load: (none)
  - Verification: `npm run dev`, navigate to `/dev`, confirm all four components render with their variants; `npm run test` includes a smoke test that `/dev` mounts without error.

### Phase 4: Accessibility gate

Goal: `axe-core` and `pa11y` run against the app shell and `/dev` route with a zero-violation CI-enforced gate.

- [x] **Task 4.1** — Add `axe-core` automated test coverage
  - What: Add `axe-core` (via `vitest-axe` or `@axe-core/vue`/direct `axe-core` + jsdom) and a test that renders the app shell and the `/dev` catalog, running `axe` against each and asserting zero violations.
  - Deliverables:
    - `admin-ui/package.json` devDependency for the chosen axe integration
    - `admin-ui/src/__tests__/accessibility.test.ts`
  - Skills to load: (none)
  - Verification: `npm run test` runs the axe assertions and passes with zero violations.

- [x] **Task 4.2** — Add `pa11y` CI script against the built app
  - What: Add `pa11y` as a devDependency and an `npm run a11y` script that serves the built `dist/` (via `vite preview` or a static server) and runs `pa11y` against `/` and `/dev`, failing on any error-level issue.
  - Deliverables:
    - `admin-ui/package.json` devDependency `pa11y`, new `a11y` script
  - Skills to load: (none)
  - Verification: `npm run build && npm run a11y` exits 0 with zero pa11y errors reported.

- [x] **Task 4.3** — Wire the accessibility gate into `make verify`
  - What: Extend the Makefile's `lint` (or a new dedicated target folded into `verify`) to run `npm run a11y` for `admin-ui` inside its container, so a violation fails `make verify`.
  - Deliverables:
    - `Makefile` updated (`lint` or `verify` prerequisite chain includes the admin-ui `a11y` gate)
  - Skills to load: spontini-verify-gate
  - Verification: `make verify` (or the targeted sub-target) runs the admin-ui accessibility gate and passes.

## Acceptance Criteria

- `docker compose run --rm admin-ui npm run build` succeeds and the built CSS includes DSI design tokens (no hard-coded hex/px values in new SCSS, per STACK.md §4.3).
- Navigating to `/` renders the app shell with the `<DsNav>` left rail (Ingest · Imprinting · Training placeholders); navigating to `/dev` renders the component catalog with every wrapper's variants.
- `npm run test` (unit) is green, including component tests for `<DsButton>`, `<DsInput>`, `<DsCallout>`, `<DsNav>` and the axe-core zero-violation assertions.
- `npm run a11y` (pa11y against the built app) reports zero errors on `/` and `/dev`.
- `make verify` passes end-to-end with the new admin-ui gates included.
- No legacy Sass `@import` anywhere in `admin-ui/src/styles/` — only `@use`/`@forward`.

## Risks

- **Bootstrap Italia 2.x Vite resolution bug** (STACK.md §4.1 caveat) — mitigation: pin to Bootstrap Italia 3.x's modular `@use`/`@forward` architecture, which is Dart Sass compatible and avoids the `@splidejs/splide` resolution issue.
- **No official Vue wrapper for DSI** means hand-rolled wrappers may drift from DSI markup updates — mitigation: keep wrappers thin (forward props/slots onto DSI classes verbatim) so upgrades touch few files, per STACK.md §4.4.2.
- **axe-core/jsdom limitations** (jsdom doesn't fully replicate browser layout, so some contrast/visibility checks are unreliable) — mitigation: treat `pa11y` (headless Chromium, real rendering) as the authoritative accessibility gate; axe-core in unit tests is a fast first-pass signal only.

## Out-of-Scope

- Ingest, Imprinting, Training business sections (features 0016, 0017, 0018).
- Operator authentication (feature 0027).
- Full-app accessibility audit beyond the shell and `/dev` catalog (feature 0019).
- `frontend` (citizen-facing) DSI integration — that is feature 0020, a separate app.
