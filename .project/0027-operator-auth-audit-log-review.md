# Review 0027: Operator auth + audit log

- **Plan**: [0027-operator-auth-audit-log-plan.md](./0027-operator-auth-audit-log-plan.md)
- **Branch**: feat/operator-auth-audit-log
- **Reviewed**: 2026-07-25
- **Reviewer**: Sisyphus (Claude Code)
- **Verdict**: changes-requested

## Summary

A clean, idiomatic replacement of the `X-Admin-Key` placeholder with real single-operator auth: argon2-hashed credential file, `SessionStore` (a direct structural copy of the proven `PreviewStore` TTL pattern), an `OperatorSession` axum extractor wired into every admin handler, and a best-effort `AuditLogPort` recording every write. Verified live end-to-end — not just via automated tests — through both the backend directly and the `admin-ui`/`frontend` nginx proxies, including a real `audit_log` row queried from `kb.db`. The one gap is that two binding docs (`CONSTITUTION.md`, `STACK.md`) were not updated to reflect the new auth surface this feature deliberately and correctly introduces.

## Findings

### Blockers

(none)

### Major

- **[M1]** `docs/CONSTITUTION.md:17,24` — The Simplicity principle states "No authentication, no user management... This is a concept/prototype," and §4 lists "User authentication" under Out of Scope. This feature — approved via the roadmap and Milestone 5's explicit "shippable to the Comune" goal — directly and correctly contradicts that text. The Constitution was accurate when written (Milestone 0) but is now stale for the operator-facing surface. Expected: the binding doc reflects reality. Actual: it still says the opposite of what's shipped. Suggested fix: amend §3 to scope "no authentication" to the citizen-facing `/chat` surface specifically (which remains and should remain unauthenticated, per Constitution §4's accessibility goal), and note in §4 that operator authentication was added by feature 0027 to close the admin-surface security gap left open since feature 0008.
- **[M2]** `docs/STACK.md:60-67` — The documented `/admin/api/*` endpoint list (§3.1) does not include the two new routes this feature adds: `/admin/api/auth/login` and `/admin/api/auth/logout`. Expected: STACK.md is the authoritative surface-area reference for the admin API, per its own stated purpose. Actual: an operator or future contributor reading STACK.md would not know these routes exist. Suggested fix: add both routes to the list at STACK.md:60-67, e.g. `/admin/api/auth/login` — operator login, issues a session cookie; `/admin/api/auth/logout` — invalidates the session.

### Minor

- **[m1]** `backend/src/auth/handlers.rs:46` (`login`) — `req.username != credential.username` is a plain (non-constant-time) string comparison; only the password check (via argon2/`password-hash`, which uses constant-time comparison internally) is timing-safe. In this single-operator, on-premises deployment the username isn't treated as a secret (it's fixed/known — "operator" — and login rate-limiting is already an explicit Non-Goal), so the practical risk is negligible, but it's worth a one-line note (or a `subtle`-crate constant-time comparison) for defense-in-depth completeness. Not required before close given the documented threat model.
- **[m2]** `backend/src/bin/set-operator-credential.rs:56-64` — The written credential JSON file (containing the argon2 hash) is created with default (umask-derived) permissions, not explicitly restricted (e.g. `0600`). The container's single non-root user makes the practical exposure low, but setting `Permissions::from_mode(0o600)` after the write would be a cheap hardening step consistent with the file's sensitivity. Not required before close.
- **[m3]** Per-handler audit wiring — 13 of the 14 write handlers (all but `create_persona`, which has a dedicated BDD scenario asserting the actual `audit_log` row) are verified only by visual/pattern consistency with the one proven example, not by an automated assertion that each handler calls `record_best_effort` with the correct `action`/`target` string. The underlying mechanism (`AuditLogPort`, `record_best_effort`) is thoroughly unit-tested in isolation, and manual live verification (see plan's Implementation Notes) additionally confirmed the pattern end-to-end for `create_persona`. Residual risk is a copy-paste string typo in one of the other 13 `action`/`target` calls going uncaught. Not required before close given the mechanical consistency and live verification already performed, but worth a follow-up if this pattern is extended again.

### Nits

- **[n1]** `backend/src/auth/session_store.rs` — a clean, direct structural mirror of `preview_store.rs`; good reuse of a proven pattern rather than inventing a new one.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | pass | New `auth`/`audit` modules follow the existing port/adapter file layout (`mod.rs` port + DTOs, `adapter.rs` KbStore-backed impl) used throughout `admin/*`. `OperatorSession` is a thin axum extractor wrapping pure, directly-testable functions (`authorize`, `extract_session_token`) — framework coupling isolated at the edge. |
| Truthfulness & RAG | n/a | No change to `/chat`, retrieval, prompt assembly, or persona. Live-verified `/chat` unaffected. |
| Ingest correctness | n/a | No change to embedding, chunking, or ingest adapters. |
| Tests (coverage + TDD + BDD) | pass (see m3) | 303 workspace tests pass; 50 BDD scenarios / 225 steps pass, including 5 new auth/audit scenarios exercising the real login/logout/audit-record flow. `cargo tarpaulin` numbers could not be produced — confirmed pre-existing gap (missing from the build image, unrelated to this branch, same as noted in feature 0026's review), substituted with live end-to-end verification. |
| Clean Code | pass | No unjustified `unwrap()`/`expect()` in production code — the two `.expect()` calls in `auth/handlers.rs` parse self-constructed, provably-valid header strings (hex token + numeric TTL), a legitimate "impossible to fail" invariant. Names are intention-revealing throughout. |
| Clean Design (UI/UX) | pass | `LoginView.vue` follows existing DSI component conventions exactly (`DsInput`/`DsButton`/`DsCallout`), one primary action, honest loading state ("Accesso in corso…", no fake delay), zero new axe violations (verified: `/login` route added to the accessibility test suite, 7/7 passing). |
| Plan conformance | pass | Every task's listed deliverables exist and verification passed. Task 6.1 used session-seeding instead of a full HTTP login round-trip for the *existing* scenario conversions (disclosed and justified in the plan's Implementation Notes) — the dedicated new login scenarios in Task 6.2 do exercise the real HTTP login handshake, so the plan's actual intent (prove the real auth mechanism end-to-end) is fully met. |

## Coverage Report

- Line coverage on changed files: not measured — `cargo-tarpaulin` absent from the build image, confirmed pre-existing and unrelated to this branch (same gap noted in feature 0026's review).
- Branch coverage on changed files: not measured, same reason.
- Substitute verification: full workspace `cargo test` (303 tests), full BDD suite (50 scenarios / 225 steps), plus live manual end-to-end verification against the running containers (login, wrong-password rejection, unauthenticated-write rejection, authenticated write with correct `created_by`, a real `audit_log` row, logout, stale-cookie rejection, `/chat` unaffected) — see the plan's Implementation Notes for the exact commands run.
- Excluded files: none beyond the pre-existing `main.rs`/`tests/**` exclusions already configured in `make coverage`.

## Required Fixes Before Close

1. **[M1]** Update `docs/CONSTITUTION.md` §3 (Simplicity) and §4 (Scope) to scope "no authentication" to the citizen-facing surface and note that operator authentication is in scope, added by feature 0027.
2. **[M2]** Add `/admin/api/auth/login` and `/admin/api/auth/logout` to the documented admin API route list in `docs/STACK.md` §3.1.

## Fix Log

- **[M1]** FIXED on 2026-07-25. Reworded `docs/CONSTITUTION.md` §3's Simplicity principle to scope "no authentication" to the citizen-facing `/chat` surface specifically, and noted the operator admin surface requires single-operator authentication (feature 0027). Reworded §4's Out of Scope line from "User authentication" to "Citizen-facing authentication," with a parenthetical pointing to feature 0027 for the now-in-scope operator auth. Kept "production deployment" and the other pre-existing (already-stale, out of this fix's scope) entries untouched. Verification: diff reviewed, no code touched, no gate affected.
- **[M2]** FIXED on 2026-07-25. Added `/admin/api/auth/login` and `/admin/api/auth/logout` to the documented Admin surface route list in `docs/STACK.md` §3.1, with a one-line description of each matching the existing list's style. Verification: diff reviewed, no code touched, no gate affected.
