# ADR 0009: Bootstrap Italia 3.x Beta with Patched Splide Exports

- **Status**: accepted
- **Date**: 2026-07-24
- **Deciders**: agent
- **Related**: 0015

## Context

STACK.md §4.1 mandates Design System Italia (DSI) for both `admin-ui` and `frontend`, consumed via `bootstrap-italia` + `design-tokens-italia`, integrated as a Dart Sass `@use`/`@forward` pipeline (STACK.md §4.3 forbids the legacy `@import` syntax). STACK.md §4.1 already flags a known risk: "Bootstrap Italia 2.x has a known issue resolving `@splidejs/splide/src/css/core/index` under Vite," and recommends Bootstrap Italia 3.x's modular architecture as the fix.

At implementation time, `bootstrap-italia`'s only published 3.x releases are pre-release (`3.0.0-alpha.0` through `3.0.0-beta.2`; no stable 3.0.0 exists yet). Worse, the flagged Splide issue is **not actually fixed** in 3.0.0-beta.2: `components/_carousel.scss` still does `@use '@splidejs/splide/src/css/core/index'; // XXX TO DO: To verify @use migration`, and `@splidejs/splide`'s own `package.json` `exports` map has no entry for that deep `src/css/core/index` path (it only exposes the prebuilt `./css/core` bundle), so `vite build` fails with `Missing "./src/css/core/index" specifier in "@splidejs/splide" package` regardless of which bootstrap-italia major version is used.

`admin-ui` needs the Sass pipeline (not just the prebuilt CSS bundle) so `design-tokens-italia`'s SCSS variables and bootstrap-italia's own token-driven CSS custom properties are available to project-authored SCSS, per STACK.md §4.3 ("Design tokens come first ... use `var(--color-...)` or the SCSS token map"). `admin-ui` does not use the carousel component today, so there is no functional need to fully resolve Splide's CSS — only to stop it from breaking the build of unrelated components forwarded by the same aggregator file.

## Decision

We will pin `bootstrap-italia` to the exact pre-release `3.0.0-beta.2` (not a caret range) and patch `@splidejs/splide`'s `package.json` `exports` map via `patch-package` to add the two missing entries (`./src/css/core/index` and `./src/css/core/`) that Dart Sass needs to resolve the carousel component's `@use`. The patch is committed to `admin-ui/patches/` and applied automatically by a `postinstall: patch-package` script, so `npm ci` (including inside the Docker build) is self-healing with no manual step.

## Rationale

This satisfies Constitution §6 in order: (1) it unblocks the DSI-mandated Sass pipeline, directly serving the mission-critical UX bar (STACK.md §4); (2) it keeps the entire fix local to `admin-ui`'s own `node_modules`/`patches/` — no fork of `bootstrap-italia`, no vendored copy, no upstream PR dependency; (3) it reduces complexity versus the alternative of hand-copying bootstrap-italia's ~140-line forward list to exclude `carousel` (which would silently drift from every upstream update); (4) `patch-package` is a well-established, low-risk pattern specifically for this class of problem (a transitive dependency's `exports` map blocking a legitimate deep import) and is reversible the moment either `@splidejs/splide` fixes its `exports` map upstream or `bootstrap-italia` ships a stable 3.0.0 that no longer needs the deep import.

## Consequences

### Positive

- The DSI Sass pipeline works end-to-end (`design-tokens-italia` variables + bootstrap-italia's full component set, verified: build succeeds, `--bs-*` custom properties present in the compiled CSS).
- The fix survives `npm ci` unattended (`postinstall` hook), including inside the Docker build, with no manual post-install step for future contributors.
- No fork or vendored copy of either `bootstrap-italia` or `@splidejs/splide` — upgrading either package is a normal `npm install` plus (if needed) a patch regeneration via `npx patch-package @splidejs/splide`.

### Negative

- `bootstrap-italia` is pinned to a pre-release (`3.0.0-beta.2`), not a stable release, which STACK.md's general versioning policy (§3.8) discourages ("latest stable major, never legacy"). This must be revisited once a stable 3.0.0 ships.
- The patch is a workaround for someone else's bug (`@splidejs/splide`'s incomplete `exports` map), not a fix we control upstream — if `@splidejs/splide` changes its internal file layout in a future version, the patch could silently stop applying cleanly and `patch-package` will fail loudly on `npm install`, which is the intended fail-safe.
- The compiled CSS bundle is large (~507 KB unminified-equivalent gzip ~80 KB) because the full bootstrap-italia aggregator (100+ components) is pulled in for four wrapper components; tree-shaking to only the forwarded partials actually used is deferred as a future optimization, not attempted here to avoid the same fork-and-drift risk described above.

### Neutral

- `admin-ui` does not use the carousel component; the patch only unblocks the build, it does not make the carousel itself functional. If a future feature needs a working carousel, the Splide integration will need a fresh look regardless of this ADR.

## Alternatives Considered

### Alternative A: Stay on Bootstrap Italia 2.x with the prebuilt CSS bundle

STACK.md's own documented fallback (`bootstrap-italia/dist/css/bootstrap-italia.min.css`) avoids the Sass pipeline entirely. Rejected because it would prevent `design-tokens-italia`'s SCSS variables and any token-driven custom SCSS (STACK.md §4.3) from working, which the plan's Sass architecture requires.

### Alternative B: Fork bootstrap-italia's aggregator file to exclude `carousel`

Copy `bootstrap-italia.scss`'s ~140-line forward list into `admin-ui`, dropping `@forward 'components/carousel'`. Rejected: silently drifts from every upstream bootstrap-italia update (new components added upstream would need manual re-syncing), and duplicates a file we don't own.

### Alternative C: Wait for a stable bootstrap-italia 3.0.0 and/or an upstream Splide fix

Blocks the entire feature indefinitely on an external, uncontrolled timeline. Rejected as incompatible with shipping Milestone 3.

## Compliance

Enforced by `admin-ui/package.json`'s `postinstall: patch-package` script and the committed `admin-ui/patches/@splidejs+splide+4.1.4.patch` — any `npm install`/`npm ci` (including the Docker build) re-applies the patch automatically, and `patch-package` fails the install loudly if the patch no longer applies cleanly (e.g., after an unreviewed `@splidejs/splide` upgrade), forcing a conscious re-evaluation of this ADR rather than a silent break.
