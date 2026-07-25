# ADR 0011: Session-Cookie Operator Authentication with Best-Effort Audit Log

- **Status**: accepted
- **Date**: 2026-07-25
- **Deciders**: Sisyphus (Claude Code)
- **Related**: Feature 0027

## Context

Feature 0008 introduced `/admin/api/persona` behind a placeholder: a static `X-Admin-Key` header compared against `ADMIN_API_KEY`, which defaulted to the literal string `"dev-key"` when unset. Every admin feature since (0009–0014) reused that same placeholder, and the `admin-ui` SPA sent it on every request. The roadmap called this out directly: "This closes the security gap left open by the admin surface plans." Feature 0027 replaces it with the project's **first real authentication mechanism** — a decision the original [Constitution](../docs/CONSTITUTION.md) §3 didn't anticipate ("No authentication... This is a concept/prototype"), since amended (this ADR, together with feature 0027, is the reason).

The constraints, per Constitution §3 (Simplicity) and the roadmap's own scoping: **single operator** (no accounts, no roles), **on-premises** (no external identity provider — would violate Locality), and no unnecessary persistence (the existing `PreviewStore` — feature 0009, [ADR 0008](./0008-preview-confirm-upload-workflow.md) — already established the pattern of an in-memory, TTL-based store for exactly this kind of short-lived, operator-facing state, rather than a database table). Every `/admin/api/*` write also needed an audit trail (`actor`, `action`, `target`, `payload`, `at`) recording who did what — a new requirement with no existing pattern in this codebase to reuse.

## Decision

We will authenticate the operator via a password (argon2-hashed, stored in an env-loaded JSON credential file — never plaintext, never in an environment variable directly) checked by a `POST /admin/api/auth/login` endpoint, which on success issues a random 256-bit token held in an in-memory `SessionStore` (a direct structural copy of `PreviewStore`'s TTL-map pattern) and returned as an `HttpOnly`, `SameSite=Strict` session cookie. A custom axum extractor, `OperatorSession`, validates that cookie on every `/admin/api/*` route (except `/admin/api/auth/login` itself) and supplies the authenticated actor's identity to the handler — replacing the per-handler `check_admin_key(&headers, &state.config)?` call (and its duplicate copy in `admin/upload/handlers.rs`) with a single, type-safe, framework-level guard.

Every write handler records an audit entry via a shared `AuditLogPort`/`KbStoreAuditLogAdapter` (an `audit_log` table, V6 migration), using the `OperatorSession`'s real authenticated actor instead of the hardcoded `"admin"` placeholder `create_persona` shipped with. The write and the audit record are **not transactional** with each other: the write commits first, and the audit record is attempted best-effort afterward (`record_best_effort`, logging via `tracing::error!` on failure rather than failing the request) — the operator's action succeeding is the primary guarantee; the audit trail is a secondary, non-blocking observability concern.

## Rationale

This decision is evaluated against the [Constitution §6 criteria](../docs/CONSTITUTION.md#6-decision-making), in order:

1. **Serves the mission?** Yes — closing a real security gap on the operator surface that configures everything Spontini answers from.
2. **Keeps the stack local?** Yes — no external identity provider, no SaaS auth service; the credential file and session store are entirely local.
3. **Reduces complexity?** Yes, relative to the alternatives considered below: reusing the proven `PreviewStore` pattern for sessions avoids inventing a new persistence mechanism, and a single `OperatorSession` extractor replacing ~14 duplicated `check_admin_key` call sites is a net reduction in code, not an increase.
4. **Improves UX?** Yes for the operator (a real login instead of a shared secret baked into a `VITE_ADMIN_API_KEY` env var) and for whoever audits the system later (a real, queryable record of who changed what).

The non-transactional audit design specifically satisfies criterion 3 (reduces complexity): making the write and the audit record atomic would require either a shared database transaction spanning two different concerns (the domain write and the audit trail) or a two-phase-commit-style protocol, neither of which this single-operator, single-instance deployment needs — the failure mode being protected against (an audit write failing right after a domain write succeeded) is rare and, per the roadmap's own scoping, acceptable to lose with a logged error rather than roll back an otherwise-successful operator action.

## Consequences

### Positive

- A single `OperatorSession` extractor is the one place `/admin/api/*` auth is enforced — eliminates the duplicated `check_admin_key` implementation that had drifted into two copies (`admin/mod.rs` and `admin/upload/handlers.rs`).
- Every admin write now has a real, queryable record of who did what — verified live: `SELECT * FROM audit_log` after an authenticated `create_persona` call shows the exact `actor`, `action`, and `target`.
- `created_by` fields (e.g. on `Persona`) now carry the real authenticated operator identity instead of a hardcoded `"admin"` string.
- Session validation and audit recording are both fully local — no new external dependency beyond the already-common `argon2` (RustCrypto) crate for password hashing.

### Negative

- The in-memory `SessionStore` means a backend restart logs every operator out — accepted explicitly (see Alternatives).
- The audit trail is not a strict guarantee — a crash or DB error between the write committing and the audit record being written loses that one audit entry (logged, not silently swallowed, but not retried).
- Single-operator design: no accounts, no roles, no per-action authorization beyond "authenticated or not." Adding real multi-operator support later is a new decision, not a mechanical extension of this one.

### Neutral

- The upstream `llama.cpp` inference containers and the citizen-facing `/chat` endpoint are entirely unaffected — this decision is scoped to `/admin/api/*` only, consistent with Constitution §3's amended Simplicity principle (citizen-facing stays anonymous by design).

## Alternatives Considered

### Alternative A: JWT-based stateless auth

Issue a signed JWT instead of an opaque session token, avoiding server-side session state entirely. Rejected: for a single operator, the added complexity (key management, expiry/revocation semantics, a new dependency) buys nothing — revocation (logout) is actually *harder* with stateless JWTs (typically requires a denylist, which is itself server-side state) than with the simple in-memory map this ADR chose, where `SessionStore::remove` is a real, immediate revocation.

### Alternative B: Persistent (database-backed) session table

Store sessions in `kb.db` instead of in memory, surviving backend restarts. Rejected: sessions are short-lived (default 30 minutes) and single-operator; the operator logging in again after a restart is a trivial cost, and a new DB table for this would be exactly the kind of persistence layer Constitution §3 says to avoid "beyond what is strictly needed." Mirrors the same call already made for `PreviewStore` (ADR 0008) — this ADR extends that precedent rather than reversing it.

### Alternative C: Transactional audit log (write and audit record in one DB transaction)

Wrap the domain write and the `audit_log` insert in a single libSQL transaction so they succeed or fail together. Rejected: the domain write and the audit write are logically two different concerns going through two different ports (e.g. `PersonaAdminPort` and `AuditLogPort`), and forcing them into one transaction would couple every write handler's port implementation to a shared transaction context — a real architectural complexity increase for a single-operator, low-write-volume admin surface where losing one audit entry to a rare crash is an acceptable, logged trade-off.

## Compliance

- Every `/admin/api/*` handler (except `/admin/api/auth/login`) takes `session: OperatorSession` as a parameter — enforced structurally by the axum router (a handler that omits it would need no session validation at all, which is visible in code review, not hidden behind a runtime check).
- `bin/scan.sh` / `make scan` (feature 0026) covers the `backend` image, which now includes the `argon2` dependency — no new CVE-scanning gap introduced.
- `features/admin_auth_audit.feature` (5 BDD scenarios) proves the login, logout, unauthenticated-rejection, and audit-recording behavior end-to-end against the real `OperatorSession` extractor and `SessionStore` — not mocks.
- Any new write endpoint added to `/admin/api/*` should follow the same pattern: `session: OperatorSession` parameter, a call to `record_best_effort` after the write succeeds — enforced by review checklist (Plan conformance dimension), not by an automated check.
