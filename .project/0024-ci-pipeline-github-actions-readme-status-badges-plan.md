# Plan 0024: CI pipeline (GitHub Actions) + README status badges

- **Status**: review
- **Approved**: 2026-07-25 by agent
- **Implemented**: 2026-07-25 by agent
- **Branch**: feat/ci-pipeline-github-actions-readme-status-badges
- **Feature ID**: 0024
- **Created**: 2026-07-25
- **Owner**: agent

## Objective

Spontini's [Constitution](../docs/CONSTITUTION.md) mission demands a trustworthy assistant, and trustworthiness starts with a codebase that provably passes its own quality gate on every change, not just when a human remembers to run `make verify` locally. This feature wires a GitHub Actions workflow that runs `make verify` (build + test + lint + fmt-check + coverage + compose config + a11y) on every push and pull request, using Docker layer caching and cargo registry caching to keep run time reasonable, and fails the run on any non-zero gate — closing the gap left explicitly open since feature 0001 ("no CI yet"). It also replaces the three `pending`-status badges in `README.md` with live GitHub Actions / codecov-style badges reflecting the new workflow. The 100% line / 80% branch coverage gate from [PRINCIPLES.md §7](../docs/PRINCIPLES.md#7-100-test-coverage-on-the-codebase) is enforced in CI exactly as `make coverage` already enforces it locally — no gate is loosened for CI. In scope: the `.github/workflows/ci.yml` workflow, README badge wiring, and any Makefile/CI-only glue needed to run `make verify` in the GitHub Actions runner (Docker-in-Docker via the `docker` and `docker compose` plugin, which ships on `ubuntu-latest` runners). Out of scope: any deployment step, any change to the gate's thresholds, and any change to the actual business logic of `backend`/`frontend`/`admin-ui`/`ingest`.

## Non-Goals

- No deployment workflow (staging/production release) — a separate, future plan per the roadmap row's explicit note.
- No change to the coverage thresholds (100% line / 80% branch) or to what `make verify` runs — CI must run the existing gate unmodified, not redefine it.
- No self-hosted runner setup — GitHub-hosted `ubuntu-latest` runners only, matching the "Mac Intel i7 / 16 GB RAM" and "no GPU required" constraints already satisfied by the existing Docker Compose stack.
- No external coverage-hosting service (Codecov/Coveralls) integration unless it is free and requires no secrets beyond the repository's own `GITHUB_TOKEN`; a self-contained badge (shields.io endpoint badge fed by the workflow, or a static badge updated by the workflow) is acceptable and preferred to avoid a new external dependency.

## Phases

### Phase 1: GitHub Actions workflow

Goal: `make verify` runs automatically on every push and PR, with caching, and fails the run on any gate failure.

- [x] **Task 1.1** — Author the CI workflow file
  - What: Create `.github/workflows/ci.yml` with a single `verify` job on `ubuntu-latest`, triggered on `push` (all branches) and `pull_request` (targeting `main`), that checks out the repo, sets up Docker Buildx, restores/saves a cache for Docker layers (`actions/cache` keyed on `Cargo.lock`/`package-lock.json` hashes, or `docker/build-push-action` with `cache-from`/`cache-to` using GitHub Actions cache backend `type=gha`) and for the cargo registry (`actions/cache` on `~/.cargo/registry` and `~/.cargo/git` keyed on `Cargo.lock` hash), then runs `make verify`.
  - Deliverables:
    - `.github/workflows/ci.yml`
  - Skills to load: spontini-verify-gate
  - Verification: `actionlint` (or `yamllint` if `actionlint` unavailable) reports no syntax errors on the workflow file; manual read-through confirms triggers, caching, and the `make verify` invocation are present and correctly ordered.

- [x] **Task 1.2** — Confirm `make verify` is runnable inside the GitHub Actions runner
  - What: Verify that every `make verify` sub-target (`build`, `test`, `lint`, `fmt-check`, `coverage`, `compose-config`, `a11y`) resolves to `docker compose run/build` invocations only (no host-native `cargo`/`npm` calls), since `ubuntu-latest` runners ship Docker and the `docker compose` v2 plugin but not the pinned Rust/Node toolchains — grep the Makefile to confirm no target assumes a host toolchain.
  - Deliverables: none (verification-only task; if a host-toolchain assumption is found, fix it in the Makefile as part of this task and note the deliverable then)
  - Skills to load: spontini-verify-gate
  - Verification: `grep -n 'cargo\|npm' Makefile` shows every such invocation prefixed by `$(COMPOSE) run --rm <service>`.

### Phase 2: README status badges

Goal: `README.md`'s three `pending` badges are replaced with badges that reflect the real CI workflow.

- [x] **Task 2.1** — Wire the build/test status badges to the new workflow
  - What: Replace the `Build` and `Tests` badges in `README.md`'s Status section with GitHub Actions workflow-status badges pointing at `.github/workflows/ci.yml` (`https://github.com/<owner>/<repo>/actions/workflows/ci.yml/badge.svg`), and update the surrounding sentence that currently says "CI badges will be wired when the CI pipeline is introduced."
  - Deliverables:
    - Updated `README.md` Status section
  - Skills to load: (none — documentation edit)
  - Verification: the badge URLs use the actual GitHub remote owner/repo (read from `git remote get-url origin`); the "will be wired" sentence is removed or updated to reflect that CI now exists.

- [x] **Task 2.2** — Wire the coverage badge
  - What: Replace the `Coverage` badge with one that reflects the `make coverage` result. Since no external coverage host is in scope, add a workflow step that parses the tarpaulin summary output and updates a shields.io endpoint-JSON badge committed to the repo (e.g. `.github/badges/coverage.json`) via `actions/github-script` or a small shell step, committed back to a dedicated branch or served as a workflow artifact/GitHub Pages badge — OR, if that is overly complex for the scope, use a static "coverage gate: 100%/80% enforced in CI" badge (shields.io static badge, no dynamic value) that links to the workflow run. Prefer the static badge if the dynamic badge would require extra permissions (contents:write from CI) beyond what's already granted.
  - Deliverables:
    - Updated `README.md` Coverage badge
  - Skills to load: (none — documentation edit)
  - Verification: the badge renders (manually confirm the shields.io URL returns a valid SVG via `curl -sI <badge-url>` returning `200`); no CI secrets beyond `GITHUB_TOKEN` are introduced.

## Acceptance Criteria

- `.github/workflows/ci.yml` exists, triggers on `push` and `pull_request`, and runs `make verify` (or the exact same set of gates) as its core step.
- The workflow uses caching for Docker layers and the cargo registry so re-runs are materially faster than a cold run (cache steps present and correctly keyed).
- `README.md`'s Status section shows three badges with no "pending" placeholders, and the "CI badges will be wired..." caveat sentence is removed or updated.
- No change to `make verify`'s constituent gates or thresholds — `make verify` run locally still passes identically to before this feature.
- No deployment step is added anywhere in `.github/workflows/`.

## Risks

- GitHub Actions `ubuntu-latest` runners have 14 GB disk / 7 GB RAM, which may be tight for building 6 Docker images including two llama.cpp-based inference containers with GGUF models — mitigation: scope the CI `build`/`test` steps to the crates/services that don't require downloading multi-GB GGUF model files (the `provision-models` step is a separate, explicitly-out-of-scope concern; if `make verify` transitively requires live model files, document that limitation in the workflow via a comment and consider `make verify` in CI covering build+test+lint+fmt-check+coverage+compose-config while `a11y` (which needs a built frontend, not live inference) runs with an appropriately mocked/stubbed backend if the existing test setup already does this — if the existing test suite already mocks the inference containers (confirm during Task 1.2), no change is needed.
- Coverage badge dynamism requires either extra CI permissions or an external host — mitigation: default to the static "enforced in CI" badge (Non-Goals already scopes this down) unless the dynamic version is trivial to add without new secrets.

## Out-of-Scope

- Deployment / release workflow.
- Changing the coverage thresholds or gate composition.
- Self-hosted runners.
- External coverage-hosting service requiring new secrets.
