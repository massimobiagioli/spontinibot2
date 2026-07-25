# ADR 0010: Production Compose Overlay: Non-Root Runtime Images with Resource Limits

- **Status**: accepted
- **Date**: 2026-07-25
- **Deciders**: Sisyphus (Claude Code)
- **Related**: Feature 0026

## Context

Feature 0026 hardens the runtime for the eventual Comune di Maiolati Spontini deployment (Milestone 5 — "the system is shippable"): every owned container must run as a non-root user, every one of the 6 services needs a healthcheck (including the two `llama.cpp` inference containers and the `ingest` daemon, which has no HTTP server), and memory/CPU limits must be sized for the target Mac Intel i7 / 16 GB RAM host.

[ADR 0002](./0002-multi-stage-docker-compose-target.md) already established that `docker-compose.yml` targets the `build` stage for `backend`/`ingest` by default — a deliberate dev-first choice so `docker compose run --rm <svc> cargo ...` works without installing Rust on the host, per [STACK.md §7.3](../docs/STACK.md#73-makefile--container-first-operator-entry-point) ("the host machine needs only Docker + Docker Compose + make"). That `build`-stage image is ~2 GB, runs (at the time) as root, and is never what should ship.

The same tension exists, undocumented until now, for `frontend`/`admin-ui`: their single-stage `node:22-alpine` image bundles the full npm devDependency tree (`vite`, `vitest`, `esbuild`) and a Chromium install, needed only for `docker compose run --rm <svc> npm run test|lint|a11y`. Running `make scan` (trivy, added by this feature) against that image surfaced real, fixable HIGH/CRITICAL CVEs (`brace-expansion`, `tar`, `sigstore`, `picomatch` — all deep dependencies of `npm` itself, not application code) that never actually ship, since `nginx` only serves the static `dist/` output.

Naively adding `mem_limit`/`cpus` directly to the base `docker-compose.yml`'s `backend`/`ingest` services would also constrain `docker compose run --rm backend cargo test|tarpaulin|clippy` — compile-heavy operations that can need more RAM than a production runtime footprint, breaking `make verify`. Naively adding a `runtime` stage without also renaming the built image would silently overwrite the `:latest` tag that dev commands rely on, since `docker compose build` tags a service's image by service name regardless of which stage was built (confirmed by reproducing this exact break during implementation: `docker compose run --rm backend cargo test` failed with `cargo: executable file not found in $PATH` after a runtime-stage build overwrote the dev image).

## Decision

We will add `docker-compose.prod.yml` as a Compose override file, applied via `docker compose -f docker-compose.yml -f docker-compose.prod.yml <cmd>` (wired as `make prod-build` / `make prod-up` / `make prod-down`). For the four services we own (`backend`, `ingest`, `frontend`, `admin-ui`) it sets `build.target: runtime` and an explicit, distinct `image:` tag (`spontini-bot-2-<svc>:prod`) so the production image can never collide with the dev image `docker compose run` depends on. It applies `mem_limit`/`cpus` to all 6 services, sized for the Mac Intel i7 / 16 GB RAM target, and adds healthchecks for the 3 services that lack one in the base file (`ingest` via a new heartbeat-file mechanism, since it has no HTTP server; `llama-embed`/`llama-generate` via `curl .../health`).

Every Dockerfile we own (`backend`, `ingest`, `frontend`, `admin-ui`) creates and switches to a fixed-UID (`10001`) non-root user in every stage. `frontend`/`admin-ui` gained a genuine second `runtime` stage — plain `alpine:3.24` + `nginx` + the built static assets only, no Node.js/npm at all — eliminating the devDependency and Chromium CVE surface entirely (runtime image size: 1.83 GB → 11.1 MB). The base `docker-compose.yml` now pins `target: build` explicitly for `frontend`/`admin-ui` too (previously implicit, since the Dockerfile was single-stage), needed once a second stage exists so the dev default doesn't silently become the last-defined stage.

The upstream `ghcr.io/ggml-org/llama.cpp:server` image is explicitly excluded from both the non-root requirement and the `make scan` zero-CVE gate: we don't own its Dockerfile, it has real HIGH/CRITICAL Go-stdlib CVEs we cannot patch, and gating our build on an image we don't control would make hardening permanently, unactionably red.

## Rationale

This decision is evaluated against the [Constitution §6 criteria](../docs/CONSTITUTION.md#6-decision-making), in order:

1. **Serves the mission?** Yes — a shippable, hardened production path is Milestone 5's explicit goal.
2. **Keeps the stack local?** Yes — no change to the local-only inference/storage architecture.
3. **Reduces complexity?** Mostly yes, with one deliberate trade-off: a second compose file and a second Dockerfile stage for 4 services is more moving parts than a single file, but the alternative — one image serving both dev and prod — is precisely what ADR 0002 already rejected for `backend`/`ingest`, and this decision applies that same, already-accepted pattern consistently rather than inventing a new one. `make prod-build`/`make prod-up` are thin one-line `docker compose -f ... -f ...` delegations (STACK.md §7.3 Rule 7), so the Makefile surface stays simple even though the compose topology grew.
4. **Improves UX?** Yes for the operator who deploys (a real hardened path exists and is documented); the dev UX (`make build`/`test`/`lint`/`coverage`) is provably unaffected — verified live, not assumed.

## Consequences

### Positive

- Every container we own runs non-root in both dev and prod images.
- The images that actually ship (`:prod` tags) are dramatically smaller (backend 3.59 GB → 104 MB, ingest 2.99 GB → 94.7 MB, frontend/admin-ui 1.83 GB → 11.1 MB) and have zero HIGH/CRITICAL CVEs, verified live via `make scan`.
- `docker compose run --rm <svc> cargo|npm ...` (the entire container-first dev workflow) is unaffected — confirmed by running the full `make verify` gate (build, test, lint, fmt-check, compose-config, a11y — all pass) plus `make scan` (0 findings) after the change.
- The `ingest` daemon (no HTTP server) now has a real liveness signal via the heartbeat-file healthcheck, closing a gap that existed since feature 0006.
- All 6 services report `healthy` under the production overlay, and a live `/chat` request through the hardened stack returns a correctly cited answer — verified end-to-end, not just claimed.

### Negative

- Two Dockerfiles per frontend/admin-ui stage means two `npm ci`/`chromium` install paths to keep in sync if the build stage's dependencies ever change in a way the runtime stage needs to know about (in practice, `runtime` only copies the built `dist/`, so this risk is low).
- `docker-compose.prod.yml` duplicates the memory/CPU budget as static numbers; if the target hardware changes, these need manual re-tuning (no autoscaling — acceptable per Constitution §3 "Simplicity").
- Operators must remember there are now two ways to bring up the stack (`make up` vs `make prod-up`) — mitigated by the README's new "Production" section and the compose file's own header comment explaining the split.

### Neutral

- The upstream `llama.cpp` image's user and CVE posture remain outside this project's control by design — documented here rather than silently ignored.

## Alternatives Considered

### Alternative A: Apply resource limits and `target: runtime` directly in `docker-compose.yml`

Rejected: `docker compose run` reuses the service's own resource limits and `build.target`, so this would break `make test`/`make lint`/`make coverage` (verified this exact failure mode occurred once during implementation before the fix). ADR 0002 already made the "dev-first default, disjoint from the runtime artifact" call for `backend`/`ingest`; reversing it here would be inconsistent and regressive.

### Alternative B: Separate `Dockerfile.prod` files instead of a second stage in the same Dockerfile

Rejected by [ADR 0002's Alternative C](./0002-multi-stage-docker-compose-target.md#alternative-c-dev-and-production-dockerfiles) for the same reason it applies here: dev/prod Dockerfile pairs duplicate configuration and drift. A second named stage in the existing Dockerfile keeps one file, one source of truth for the base image and copied artifacts.

### Alternative C: Include the upstream `llama.cpp` image in the non-root requirement and the `make scan` gate

Rejected: we don't own that Dockerfile. Forcing non-root would require forking and maintaining a patched image (explicitly out of scope per the feature 0026 plan's Non-Goals); including it in the CVE gate would make `make scan` permanently red on CVEs we cannot fix, defeating the gate's purpose as an actionable signal.

## Compliance

- `docker-compose.prod.yml`'s own header comment documents the `:prod` tag rationale and the `target: build` vs `target: runtime` split, so future edits see the reasoning inline, not just in this ADR.
- `bin/scan.sh` enforces the zero-HIGH/CRITICAL-CVE gate on the 4 owned `:prod` images via `trivy image --exit-code 1`, wired as `make scan`.
- `docker compose -f docker-compose.yml -f docker-compose.prod.yml config -q` (part of `make prod-build`'s underlying compose invocation) validates the overlay merges cleanly; a future CI gate could add this explicitly alongside the existing `compose-config` gate (out of scope for this ADR — feature 0026 did not wire `make scan` or `make prod-build` into CI, only into local operator use).
- Any new owned container added in the future should follow this same pattern: a `runtime` stage in its Dockerfile, a `:prod` image tag and `target: runtime` override in `docker-compose.prod.yml`, and inclusion in `bin/scan.sh`'s `IMAGES` array — enforced by review checklist (Plan conformance dimension), not by an automated check.
