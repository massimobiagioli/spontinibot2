# Review 0017: admin-ui Bot imprinting section

- **Plan**: [0017-admin-ui-bot-imprinting-section-plan.md](./0017-admin-ui-bot-imprinting-section-plan.md)
- **Branch**: feat/admin-ui-bot-imprinting-section
- **Reviewed**: 2026-07-24
- **Reviewer**: agent
- **Verdict**: approved

## Summary

The implementation matches the plan closely: `adminApi.ts` gains four typed persona functions and three new interfaces (`PersonaResponse`, `CreatePersonaRequest`, `StatusResponse`) that verified field-for-field against `backend/src/admin/mod.rs` and route paths against `backend/src/lib.rs` (`GET/POST /admin/api/persona`, `POST /admin/api/persona/:id/activate`, `POST /admin/api/persona/reload`) — no mismatch found. `ImprintingView.vue` composes `ReloadPersonaButton`, `PersonaEditor`, and `VersionHistory`, all wired through `adminApi.ts` as the sole HTTP boundary (no stray `fetch()` calls found). Every honest-state and destructive-confirmation requirement (loading → error/empty/data, `DsConfirmDialog` gating persona activation exactly like feature 0016's `SectionList.vue`) is implemented and tested. `npm run test` (79/79 tests, including the new BDD-style `ImprintingView.integration.test.ts` and the `/imprinting` axe-core test) and `npm run lint` (`vue-tsc --noEmit`) both pass cleanly on the branch as-is. A handful of untested minor branches and one plan-inherited design risk (free-text `name` field on first save) are noted below but do not block.

## Findings

### Blockers

None.

### Major

None.

### Minor

- **[m1]** `admin-ui/src/services/__tests__/adminApi.test.ts:222-299` — Task 1.1's deliverable text asks for "one test per new function covering the success path **and** the non-2xx `AdminApiError` path." Only success-path tests exist for the four new functions (`getPersonaVersions`, `createPersona`, `activatePersona`, `reloadPersona`); the error path is exercised only generically via the pre-existing `getIngestConfig` in the shared `throws AdminApiError...` test. Functionally this is not a coverage gap — `request()`/`jsonRequest()` are the sole, non-branching error-handling path shared by every function in the file, so the `!response.ok` branch is fully exercised by *some* test and would show 100% covered by any line/branch tool. It is a literal deviation from the plan's stated deliverable wording, not a behavioral gap. Suggested fix (optional, cheap): add one `mockRejectedValue`/`AdminApiError` assertion per new function for literal conformance.
- **[m2]** `admin-ui/src/components/imprinting/PersonaEditor.vue:19,31,73-78` — The `name` field is free-text and editable whenever `hasAnyVersion` is false (first-ever save), exactly as Task 2.2 specifies. Because `ImprintingView.vue:14,28` always queries `getPersonaVersions(personaName)` with the fixed `VITE_PERSONA_NAME` build-time constant, an operator who edits the pre-filled name on that first save (typo or otherwise) creates a persona row under a different name. The backend's `idx_persona_active` unique index is global (not scoped per name — see `docs/STACK.md` §3.5), so an `activate=true` save under a mistyped name would still become the live `/chat` persona, yet the admin-ui would show "Nessuna persona configurata" forever after (querying the fixed name finds nothing) — an operator could lose visibility of the live persona while `/chat` keeps serving it. This is inherited directly from the plan's own Task 2.2 wording, not an implementation deviation, so it does not fault the implementer, but it is a real latent risk worth a defensive follow-up (e.g. make the field genuinely read-only always and pre-filled from `personaName`, since the plan's non-goal already forbids multi-persona name management).
- **[m3]** `admin-ui/src/components/imprinting/__tests__/VersionHistory.test.ts` — no test exercises `version.created_by === null` (`VersionHistory.vue:55`'s `v-if="version.created_by"` false branch) — the backend's `created_by: Option<String>` can be `None`. Low risk (display-only conditional), but an untested branch.
- **[m4]** `admin-ui/src/views/__tests__/ImprintingView.test.ts` — no test covers `versions.length > 0` with no version having `is_active: true` (the `activeVersion ? ... : 'nessuna'` false branch at `ImprintingView.vue:72`). Very low real-world likelihood given the backend's global unique-active constraint, but technically reachable if `list_versions` and the DB ever disagree transiently.

### Nits

- **[n1]** `VersionHistory.vue:59` — the truncated `system_prompt.slice(0, 120)` preview has no ellipsis or other truncation indicator, so an operator may not realize the text is cut off.
- **[n2]** `VersionHistory.vue:56` (`class="badge badge-success"`) is the first use of Bootstrap Italia's `.badge`/`.badge-success` classes anywhere in `admin-ui` — verified against `node_modules/bootstrap-italia/src/scss/components/_badge.scss` (`&.badge-#{$color}` generates exactly this class), so it is valid DSI markup, just a newly-introduced pattern worth knowing about for future consistency.
- **[n3]** None of the new persona fields (`tone`, `system_prompt`, `fallback_message`) carry an inline `?` hint tooltip, per `docs/STACK.md` §4.5's "inline help on every field" ambition. This mirrors the pre-existing gap in feature 0016's `SectionList.vue`/`UploadDropzone.vue` (only `ScheduleEditor.vue` uses `hint`), so it is not a regression introduced by this feature, just an unaddressed prior gap.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | pass | Each component has one responsibility (`PersonaEditor` edits+saves, `VersionHistory` lists+activates, `ReloadPersonaButton` reloads, `ImprintingView` orchestrates+fetches). No direct `fetch()` calls in any Vue file — `adminApi.ts` is the sole HTTP boundary, confirmed by grep. |
| Truthfulness & RAG | n/a | This feature manages persona CRUD/versioning only; it does not touch citation rendering, retrieval, or answer generation. No RAG invariant is at risk. |
| Ingest correctness | n/a | Not touched — confirmed no ingest files appear in the diff. |
| Tests (coverage + TDD + BDD) | pass | Every new exported function/component has a behavioral (not tautological) test; every `try/catch` in the new Vue components has both a success- and an `AdminApiError`-path test (an improvement over feature 0016's initial gap, per its own review's M1). `ImprintingView.integration.test.ts` is a genuine Given/When/Then scenario exercising save→history→activate→reload end to end against mocked `adminApi`. Minor untested branches noted in m1/m3/m4 — none rise to a functional coverage hole. |
| Clean Code | pass | No `any`/unjustified `as` casts introduced (the few `as` casts in `adminApi.ts`/tests are pre-existing patterns, not new). No magic numbers beyond the 120-char preview truncation (self-explanatory). No dead code. Names are clear and Italian-language UI strings are consistent with the rest of `admin-ui`. |
| Clean Design (UI/UX) | pass | Honest loading → error/empty/data states mirror `IngestView.vue`'s established pattern exactly; activation is gated behind `DsConfirmDialog` using the identical contract as `SectionList.vue` (feature 0016) — cancel path verified to never call `activatePersona`; success/error callouts are distinct and non-lying. |
| Plan conformance | pass | All 4 phases / 7 tasks' deliverables exist and match; verification commands (`npm run test`, `npm run lint`) pass as run during this review. No scope creep beyond the plan (no Training/Ingest/auth code touched). The one unrelated file, `frontend/.prettierignore`, is an explicitly-flagged, justified one-line infra fix mirroring `admin-ui/.prettierignore`, not scope creep. |

## Coverage Report

- Line coverage on changed files: not measured — no coverage tool is wired for `admin-ui` (pre-existing gap since feature 0015/0016, deferred to feature 0024's CI pipeline per prior reviews). Manually verified every new exported function and every new component's methods/branches have at least one direct test, with the specific gaps listed in m1/m3/m4.
- Branch coverage on changed files: not measured by tooling for the same reason; `if (!response.ok)` in the shared `request()` helper is exercised (via a pre-existing caller), and every `try/catch` added in this PR's Vue components has both branches tested except the three narrow cases in m1/m3/m4.
- Excluded files: none (no `coverage-exclusions.txt` entry needed — no coverage tool runs against `admin-ui` yet).

## Required Fixes Before Close

None — verdict is approved with zero blockers and zero majors. The minors (m1-m4) and nits (n1-n3) are optional follow-ups; none block closing this plan.
