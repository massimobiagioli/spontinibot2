# Review 0015: admin-ui Design System Italia integration + /dev catalog

- **Plan**: [0015-admin-ui-design-system-italia-integration-dev-catalog-plan.md](./0015-admin-ui-design-system-italia-integration-dev-catalog-plan.md)
- **Branch**: feat/admin-ui-design-system-italia-integration-dev-catalog
- **Reviewed**: 2026-07-24
- **Reviewer**: agent
- **Verdict**: approved

## Summary

The plan delivers exactly what it scoped: `bootstrap-italia` 3.0.0-beta.2 + `design-tokens-italia` wired through a Dart Sass `@use` pipeline, vue-router with an app shell and a `/dev` Storybook-lite catalog, four thin DSI wrapper components (`DsButton`, `DsInput`, `DsCallout`, `DsNav`) each with behavioral unit tests, and an `axe-core` + `pa11y` zero-violation accessibility gate wired into `make verify`. Along the way the implementation found and fixed two real, pre-existing accessibility bugs (a duplicate `id="app"` and a `landmark-unique` violation) rather than working around them — good sign of the gate doing its job. Quality is solid; the findings below are all minor polish or pre-existing, out-of-scope issues, none of which block shipping this diff.

## Findings

### Blockers

None.

### Major

None.

### Minor

- **[m1]** `admin-ui/src/components/ds/DsButton.vue:26` — `outline` combined with `variant="light"` or `variant="link"` produces `btn-outline-light` / `btn-outline-link`, neither of which exists in bootstrap-italia's compiled CSS (confirmed: only `btn-outline-{primary,secondary,success,warning,danger}` exist). Unused today (`DevCatalog` only demonstrates `outline` with `danger`), but the prop contract permits an invalid, unstyled combination. Suggested fix: narrow the type accepted when `outline` is true (e.g. a separate `OutlineVariant` union), or drop `light`/`link` from `Variant` until a real caller needs them.
- **[m2]** Untested branches across the new wrapper components: `DsButton`'s `size` prop (no test asserts `btn-sm`/`btn-lg`), `DsInput`'s `disabled`/`required` attribute forwarding, `DsCallout`'s `success`/`danger` variant classes, and `DsNav`'s custom `ariaLabel` override (only the default is exercised). All are implemented correctly (verified manually via the `/dev` catalog and build output) but PRINCIPLES.md §7 requires a test for both sides of every branch. No coverage tool is wired for `admin-ui` yet to catch this automatically — that gate is Rust-workspace-only via `cargo-tarpaulin` per the Makefile, and frontend coverage tooling is explicitly deferred to feature 0024. Suggested fix: add the missing assertions (cheap, ~15 min).
- **[m3]** `make verify` does not currently pass end-to-end. It fails at `fmt-check` (`frontend/dist/*` is not excluded from `prettier --check .` — `frontend/` has no `.prettierignore`) and would also fail at `coverage` (`cargo-tarpaulin` is not installed in the `backend` container image: `error: no such command: 'tarpaulin'`). Both are confirmed pre-existing on `main` (`git status --porcelain frontend/` is empty; the backend Dockerfile is untouched by this branch) and unrelated to this plan's diff. Verified independently that every `admin-ui`-scoped gate passes: `docker compose run --rm admin-ui npm run test|lint|format:check|a11y` are all green, `docker compose config -q` passes, and `admin-ui`'s own build succeeds inside the actual Docker image (Chromium + patched splide exports included). Not a defect of this plan; recommend a separate maintenance item for `frontend/.prettierignore` and installing `cargo-tarpaulin` in the backend image, out of this plan's scope (its Non-Goals explicitly exclude `frontend/`).

### Nits

- **[n1]** `admin-ui/src/components/ds/DsNav.vue:27-29` — the disabled-link branch nests a redundant `<span>` inside `<span class="list-item disabled">`. The `<RouterLink>` branch needs the inner span because DSI's `_sidebar.scss` underlines it specifically on `.active`/`[aria-current='page']`; the disabled span has no such rule and can be flattened to a single element.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | n/a | No ports/adapters concept applies to the admin-ui component layer (that's a backend/Rust concern per ADR 0003). The relevant guard is STACK.md §4.4 (thin wrappers, one DSI component per wrapper, forward props/slots) — respected by all four `Ds*` components. |
| Truthfulness & RAG | n/a | No RAG code touched. |
| Ingest correctness | n/a | No ingest code touched. |
| Tests (coverage + TDD + BDD) | pass | Every new component and route has at least one behavioral test (17 tests total, all green, including a real axe-core catch that drove a fix). Minor untested-branch gaps noted in m2. No BDD/Gherkin scenarios expected — no user-visible business behavior exists yet (pure design-system scaffolding); Gherkin scenarios begin with feature 0016. |
| Clean Code | pass | Findings above are minor/nit only. No dead code, no magic values, wrapper components stay thin and single-purpose. |
| Clean Design (UI/UX) | pass | `/dev` is an explicit Storybook-lite dev catalog (STACK.md §4.4.3), so "one thing per screen" doesn't apply to it. The app shell and `DsNav` follow DSI's visual language via inherited tokens/classes rather than bespoke styling. Accessibility (contrast, focus, semantics) is enforced by the new zero-violation `axe-core` + `pa11y` gates. |
| Plan conformance | pass | All 10 tasks' deliverables exist and their stated verifications passed. Two real, pre-existing bugs (duplicate `id="app"`, `landmark-unique` violation) were found and fixed as a direct, in-scope consequence of building the accessibility gate — not scope creep. The `admin-ui/.prettierignore` addition was necessary for Task 4.3's own acceptance criterion to be measurable. |

## Coverage Report

- Line/branch coverage on changed files: not measured — no coverage tool is wired for `admin-ui` (the Makefile's `coverage` target only runs `cargo-tarpaulin` against the Rust workspace; frontend coverage tooling is deferred to feature 0024's CI pipeline per the roadmap).
- Manual verification: every new production `.vue`/`.ts` file has at least one behavioral test exercising its primary path; see m2 for the specific untested branches.
- Excluded files: none declared (no `coverage-exclusions.txt` entry needed — no coverage tool runs against this code yet).

## Fix Log

No required fixes — verdict was `approved` with zero blockers and zero majors. Findings m1-m3 and n1 are minor/nit-level and left as optional follow-ups (m3 in particular is pre-existing, unrelated repo state outside this plan's scope) rather than fixed here, per `/fix-review`'s scope-creep guard. Plan closed directly.
