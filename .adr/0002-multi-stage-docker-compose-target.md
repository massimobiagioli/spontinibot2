# ADR 0002: Multi-stage Docker Builds as Compose Default Target

- **Status**: proposed
- **Date**: 2026-07-09
- **Deciders**: Sisyphus (opencode)
- **Related**: Plan 0001

## Context

Plan 0001 (bootstrap-infra) introduced Dockerfiles for the 4 application containers (`backend`, `ingest`, `frontend`, `admin-ui`) and a `docker-compose.yml` wiring all 6 services. The Makefile must run all developer commands (test, lint, fmt, check, coverage) **inside** containers, per [STACK.md §7.3 Rule 3](../docs/STACK.md#73-makefile--container-first-operator-entry-point).

Two forces conflict:

1. The production image should be as small as possible — no `cargo`, no `npm`, no `rustup` components, no source code.
2. The development image must have `cargo`, `npm`, `clippy`, `rustfmt`, and all source dependencies available for `docker compose run --rm <service> <cmd>` to work.

A single-stage image would either bloat the production image with dev tooling, or strip dev tooling and make `docker compose run` impossible.

## Decision

Every Rust application Dockerfile (`backend`, `ingest`) is a **multi-stage build** with:

- `build` stage — `rust:1.96.1` base, includes `clippy` and `rustfmt` via `rustup component add`, copies the full workspace source, runs `cargo build --release -p <crate>`. Also serves as the target for `docker compose run` dev commands.
- `runtime` stage — `debian:bookworm-slim` base, copies only the release binary from the build stage.

The `docker-compose.yml` **targets the build stage** (`target: build`) by default, so that `docker compose run --rm backend cargo test` and similar commands work. This produces larger local images (Rust toolchain ~2 GB) but enables the container-first Makefile without requiring the host to install Rust.

Every frontend Dockerfile (`frontend`, `admin-ui`) is a **single-stage** `node:22-alpine` + `nginx` image. The `node` base image keeps `npm` available for `docker compose run` dev commands while nginx serves the built `dist/` at runtime. A multi-stage split (node build → distroless nginx runtime) is not used because:

- The `node:22-alpine` base is already small (~130 MB).
- A distroless nginx runtime would lose `npm`, breaking `docker compose run --rm frontend npm run test`.
- Node images are never deployed to production separately (the whole stack is Dockerized and disposable).

## Rationale

This decision satisfies the [Constitution §6 decision criteria](../docs/CONSTITUTION.md#6-decision-making) in order:

1. **Serves the mission?** Yes. A single `make` command (no host tooling beyond Docker) makes the project reproducible for any operator, including the Comune staff.
2. **Keeps the stack local?** Yes. Everything runs in local containers.
3. **Reduces complexity?** Yes. One Dockerfile per service, two stages, a clear pattern. No need for separate dev/production compose files, no mount-hack for host tooling.
4. **Improves UX?** Yes (operator UX). `make test` works without the operator thinking about whether Rust is installed.

## Consequences

### Positive

- `make test`, `make lint`, `make fmt`, `make check` all work via `docker compose run --rm backend cargo ...` without Rust on the host.
- Single Dockerfile per service (no dev/production split).
- Production runtime images are slim (~120 MB for backend, ~80 MB for ingest).
- The pattern is consistent: every new Rust service follows the same multi-stage template.

### Negative

- `docker compose build` is slower than a single-stage image (extra toolchain layer, full release builds).
- Local `docker compose up` images are large (~2 GB for backend with Rust toolchain). The runtime stage is only used if someone builds with `--target runtime` manually or overrides in `docker-compose.override.yml`.
- Docker layer caching is coarse: all crate sources are COPY'd in one block, so any source change invalidates the entire build cache.

### Neutral

- The `target: build` in compose means `docker compose run --rm` runs against the build image, which has `ENTRYPOINT`/`CMD` from the build stage. The CMD is overridden by the `docker compose run <cmd>` argument, so there is no conflict.

## Alternatives Considered

### Alternative A: Host-based development, Docker only for deployment

Use `cargo` and `npm` directly on the host for development, and Docker only for production builds. Rejected because it violates [STACK.md §7.3 Rule 3](../docs/STACK.md#73-makefile--container-first-operator-entry-point): "The host machine needs only Docker + Docker Compose + make — nothing else."

### Alternative B: Distroless / scratch runtime images

Use `gcr.io/distroless/cc-debian12` or `scratch` as the runtime base for Rust containers. Rejected because debugging (docker compose exec) becomes painful — no shell, no curl, no tools. `debian:bookworm-slim` adds ~40 MB and provides a working debugging environment.

### Alternative C: Dev and production Dockerfiles

Maintain separate `Dockerfile.dev` and `Dockerfile` (or `docker-compose.override.yml`). Rejected because it duplicates configuration and creates drift between dev and prod images. A single multi-stage Dockerfile is simpler.

### Alternative D: Volume-mount sources for development

Mount the host source directory into the container instead of COPYing it. Rejected because `docker compose run --rm backend cargo test` needs to run in the build stage context, and volume-mount permissions are platform-dependent (especially on macOS with bind mounts and the `rust:1.96.1` image user).

## Compliance

This decision is enforced by:

1. **Review checklist** (Plan 0001 review, dimension "Plan conformance"): every new Rust container must use a multi-stage Dockerfile with `build` and `runtime` stages. Every new frontend container must use the `node:22-alpine` + nginx single-stage pattern.
2. **`docker compose config -q`** validates the compose file syntax. A misconfigured `target` value would fail here.
3. **The `spontini-verify-gate` skill** Gate 7 (Docker Compose Config) verifies compose file validity. A future CI gate can add an explicit check that every `build:` block with a `dockerfile:` pointing to a Rust service uses `target: build`.
