# Review 0020: frontend Design System Italia integration + /dev catalog

- **Plan**: [0020-frontend-design-system-italia-integration-dev-catalog-plan.md](./0020-frontend-design-system-italia-integration-dev-catalog-plan.md)
- **Branch**: feat/frontend-design-system-italia-integration-dev-catalog
- **Reviewed**: 2026-07-24
- **Reviewer**: agent
- **Verdict**: approved

## Summary

This feature mirrors feature 0015's already-approved `admin-ui` DSI integration almost exactly, adapted for the citizen-facing `frontend` app (a simpler header+main shell instead of a left-rail nav, no `DsNav`/`DsConfirmDialog`). It wires `bootstrap-italia` 3.0.0-beta.2 + `design-tokens-italia` through Dart Sass `@use`/`@forward` (reusing the exact Splide `exports`-map patch from ADR 0009), adds `vue-router` with a `/` shell route and a `/dev` component catalog, ships three thin wrapper components (`DsButton`, `DsInput`, `DsCallout`) each with unit tests, and wires `axe-core` (unit-test signal) + `pa11y` (real-browser gate) into `npm run a11y` and `make verify`. Every task's deliverables exist, every task's own verification was independently re-run and passes, and the containerized `make build`, `make test` (208 tests total across backend/frontend/admin-ui), `make lint`, `make fmt-check`, `make compose-config`, and `make a11y` all pass cleanly. The only failing gate, `make coverage`, is the same pre-existing `cargo-tarpaulin`-missing-from-the-backend-image gap documented in every feature since 0009 (0011, 0012, 0013, 0014, 0015, 0016, 0018, 0019) — confirmed unrelated here since `backend/`, `Cargo.toml`, and the Rust toolchain are untouched by this branch. Ships as-is.

## Findings

### Blockers

(none)

### Major

(none)

### Minor

- **[m1]** `frontend/src/components/ds/DsButton.vue:12,28` — the `size` prop (`sm`/`lg`) has no test asserting `btn-sm`/`btn-lg` is applied; only the `variant`/`outline`/`disabled`/click branches are covered in `DsButton.test.ts`. This is the identical gap flagged as `[m2]` in the 0015 review of `admin-ui`'s `DsButton` (never fixed there either — `admin-ui/src/components/ds/__tests__/DsButton.test.ts` still has no size assertion today). Suggested fix: add `expect(mount(DsButton, { props: { size: 'sm' } }).classes()).toContain('btn-sm')` (and `lg`).
- **[m2]** `frontend/src/components/ds/DsInput.vue:10-11,34-35` — the `disabled` and `required` prop branches are wired (`:disabled="disabled"`, `:required="required"`) but untested; `DsInput.test.ts` only covers label/id association, `update:modelValue`, and the hint `aria-describedby` link. Same pre-existing gap as `admin-ui`'s `DsInput` (also never fixed). Suggested fix: add `expect(wrapper.get('input').attributes('disabled')).toBeDefined()` and the `required` equivalent.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | n/a | Pure frontend build-tooling/UI-scaffolding feature; no ports, adapters, or domain code touched. |
| Truthfulness & RAG | n/a | No `/chat`, prompt, retrieval, or persona code touched — this feature only builds the design-system foundation ahead of feature 0021's actual chat widget. |
| Ingest correctness | n/a | No ingest code touched. |
| Tests (coverage + TDD + BDD) | pass | 17 new/updated frontend tests, all behavioral (assert DOM classes/attrs/emitted events/roles, not tautological). Two untested branches noted as m1/m2, consistent with the accepted precedent in the reference `admin-ui` implementation and not blocking (frontend coverage tooling is explicitly deferred to feature 0024 per the roadmap and prior reviews). BDD not required — matches feature 0015's precedent (no Gherkin scenario for design-system scaffolding, only unit + axe + pa11y gates). |
| Clean Code | pass | Small, single-purpose SFCs; intent-revealing names; no magic numbers, no dead code, no unjustified `unwrap`/`any`. One pre-existing-pattern nit: `frontend/Dockerfile:5`'s comment says "used by pa11y — make lint" but pa11y actually runs under `make a11y` — copied verbatim from `admin-ui/Dockerfile`'s identical (also-inaccurate) comment, so not a regression introduced here, just worth a follow-up cleanup in both files. |
| Clean Design (UI/UX) | n/a | The `/dev` catalog is an internal developer tool, not the citizen-facing surface (the Jobs-aesthetic chat widget is feature 0021's scope, explicitly out of scope here per the plan's Non-Goals). The app shell (`header` + `main`, 44px touch target on the dev-catalog link) is minimal and semantic, matching the equivalent unaudited shell in the already-approved 0015. |
| Plan conformance | pass | All 9 tasks across 4 phases have their exact listed deliverables present and their listed verification independently re-confirmed. Two files not explicitly named in the plan's deliverable lists (`frontend/patches/@splidejs+splide+4.1.4.patch`, `frontend/src/views/HomeView.vue`, and the Dockerfile Chromium/Puppeteer additions) are necessary consequences of the declared deliverables (bootstrap-italia 3.x needs the ADR-0009 patch to build at all; the `/` route needs a component to render; pa11y needs a headless browser inside the container) — not unrequested scope creep. |

## Coverage Report

- Line coverage on changed files: not mechanically measured — `cargo tarpaulin` is not installed in the `backend` container image (`error: no such command: tarpaulin`), the same pre-existing infra gap noted in every review since feature 0011 and out of scope for this frontend-only feature (no `backend`/Rust files are touched by this diff). No coverage tool is wired for `frontend`/`admin-ui` either — the Makefile's `coverage` target is Rust-workspace-only, and frontend coverage tooling is explicitly deferred to feature 0024's CI pipeline per the roadmap (same precedent as the 0015/0016 reviews).
- Branch coverage on changed files: not mechanically measured (same reason). Manually enumerated: all branches are covered except the two noted in m1/m2 (`DsButton` size variants, `DsInput` disabled/required attribute forwarding), which mirror the same unresolved minor gaps already accepted in the shipped `admin-ui` reference implementation.
- Excluded files: none explicitly excluded; `frontend/package-lock.json` is generated and not reviewed line-by-line (verified via `npm install` producing a clean, reproducible lockfile).

## Required Fixes Before Close

None — verdict is `approved` (zero blockers, zero majors). `/fix-review 0020` may close the plan without additional changes; m1/m2 are optional cheap follow-ups (~10 min) consistent with the existing, already-accepted gap in `admin-ui`'s equivalent components.
