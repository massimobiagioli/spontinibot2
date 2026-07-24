# Review 0016: admin-ui Ingest configuration section

- **Plan**: [0016-admin-ui-ingest-configuration-section-plan.md](./0016-admin-ui-ingest-configuration-section-plan.md)
- **Branch**: feat/admin-ui-ingest-configuration-section
- **Reviewed**: 2026-07-24
- **Reviewer**: agent
- **Verdict**: changes-requested

## Summary

The feature implements the `/ingest` route end to end: a typed `adminApi` HTTP client, a schedule editor, section/source list management with a reusable confirm dialog, a per-section manual-upload dropzone, and a trigger-run status poller. The implementation is architecturally clean (one responsibility per component, no duplicated ingest logic, correct use of the existing feature 0009-0011 endpoints), matches the plan's scope exactly, and the honest-error/honest-loading states are well handled where tested. The blocking issue is test coverage: five `catch` branches across `SectionList.vue`, `SourceList.vue`, and `UploadDropzone.vue` (the add-section, delete-section, add-source, delete-source, and confirm-upload error paths) have no covering test, which falls short of the plan's own stated coverage bar and PRINCIPLES.md §7's 100%-line/80%-branch gate on changed production code.

## Findings

### Blockers

None.

### Major

- **[M1]** `admin-ui/src/components/ingest/SectionList.vue:32-38` (addSection catch), `:60-67` (confirmDelete catch); `admin-ui/src/components/ingest/SourceList.vue:29-35` (addSource catch), `:53-60` (confirmDelete catch); `admin-ui/src/components/ingest/UploadDropzone.vue:76-83` (confirm catch) — five error branches have no covering test. `SectionList.test.ts` and `SourceList.test.ts` only exercise the happy path and the cancel path for add/delete; `UploadDropzone.test.ts` covers the `uploadDocument` failure but not the `confirmUpload` failure. Every sibling component that follows the same try/catch pattern (`ScheduleEditor.vue`, `RunTrigger.vue`, `IngestView.vue`, `adminApi.ts`) does have a dedicated error-path test, so this is an inconsistency within the same PR, not a stylistic choice. Expected: 100% line / 80% branch coverage on changed production code (PRINCIPLES.md §7, restated by the verify gate). Actual: these five `catch` blocks are dead code as far as the test suite is concerned — a regression here (e.g. accidentally swallowing the error, or a wrong fallback string) would not be caught. Fix: add one test per component mocking the corresponding `adminApi` function to reject with `AdminApiError` and asserting the honest error message renders (mirroring the existing pattern already used in `ScheduleEditor.test.ts` / `RunTrigger.test.ts` / `UploadDropzone.test.ts`'s upload-failure case).

### Minor

- **[m1]** `admin-ui/src/components/ds/DsConfirmDialog.vue:30-38` — the `showModal()`/`close()` branches of the visibility watcher are unreachable in the jsdom test environment (jsdom 29 has no `HTMLDialogElement.showModal`/`close`), so `DsConfirmDialog.test.ts` can only ever exercise the `:open` attribute fallback path, never the native-modal call itself. This is inherent to the jsdom limitation (not a defect), and the real behavior was confirmed manually via `npm run a11y` (pa11y/headless Chromium) reporting zero errors. Worth a one-line comment in the test file noting the gap is environmental, so a future reader doesn't mistake it for an oversight.
- **[m2]** `admin-ui/src/components/ingest/UploadDropzone.vue:113` — the file `<input>` has `accept=".pdf,.docx,.md,.txt"` but no client-side size check against `upload_max_bytes` before submitting; an oversized file is only caught by the backend's 413 response (surfaced correctly as an honest `AdminApiError` message). This is acceptable — the backend is the authority — but a client-side pre-check would save the operator a round trip for an obviously-too-large file. Not required by the plan; noted for a future polish pass.
- **[m3]** No frontend line/branch coverage tool is wired into `admin-ui` (no `vitest --coverage`, no npm coverage script, and the Makefile's `coverage` target runs `tarpaulin` against the Rust backend only). This predates this feature (plan 0015 didn't introduce one either), so it isn't a regression, but it means the M1 gap above was only caught by manual grep, not a CI gate. Worth flagging for a future infra plan (e.g. as part of feature 0024's CI pipeline work) rather than blocking this PR.

### Nits

- **[n1]** `admin-ui/.env.example` documents `VITE_ADMIN_API_KEY=dev-key`, matching the backend's `Config::from_env` default — good, but worth double-checking at deploy time that a real deployment overrides this before going live (feature 0027 is the real fix; this is just a reminder for whoever wires the first non-dev environment).
- **[n2]** `admin-ui/src/components/ingest/ScheduleEditor.vue:56-62` — the cron-expression field has no client-side format validation (any string is accepted and submitted as-is). This matches the backend's own lack of validation (`UpsertScheduleRequest.cron_expr: String`, unchecked), so it's not a regression, just an easy follow-up for a later polish pass.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | pass | `adminApi.ts` is a pure HTTP service layer with no UI logic; each Vue component has a single responsibility (schedule, sections, sources, upload, run-trigger); no ingest/embedding logic duplicated client-side — the upload flow delegates entirely to the existing `/admin/api/upload` endpoints per `spontini-ingest-flow`'s "no parallel embedding logic" rule. |
| Truthfulness & RAG | n/a | This feature touches no RAG/chat code path. |
| Ingest correctness | pass | Verified the UI's "deleting a section also removes its sources" copy (`SectionList.vue`) is factually true: `ingest_source.section_id` has `ON DELETE CASCADE` (`kb-store/src/migrations/V2__ingest_config.sql:17`) and `kb-store` enables `PRAGMA foreign_keys = ON` (`kb-store/src/lib.rs:1458`). Preview-before-index is preserved (upload → preview → confirm, no shortcut). No new source type or embedding logic introduced. |
| Tests (coverage + TDD + BDD) | fail | See M1 — five error branches uncovered. Otherwise TDD is followed with behavioral (not tautological) assertions, and the Given/When/Then integration scenario (`IngestView.integration.test.ts`) genuinely exercises add-section → add-source → trigger-run → poll-to-done end to end, matching the roadmap's required scenario. |
| Clean Code | pass | Names are intention-revealing (`POLL_INTERVAL_MS`, `TERMINAL_STATUSES`, `pendingDeleteId`), functions are small and single-purpose, no magic numbers, no dead code, no unjustified `unwrap`/`as any`/`@ts-ignore`. |
| Clean Design (UI/UX) | pass | Destructive actions (delete section/source) go through an explicit `DsConfirmDialog`; loading/error states are honest (no fake spinners, real error messages surfaced from `AdminApiError`); heading hierarchy was corrected during implementation (h4→h3) to fix a real axe `heading-order` violation, and `npm run build && npm run a11y` reports zero errors on `/`, `/dev`, `/ingest`. |
| Plan conformance | pass | All 11 tasks across 5 phases match their stated deliverables; no unrequested scope creep (no edit affordance was added for sections/sources, consistent with the plan's explicit non-goal). |

## Coverage Report

- Line coverage on changed files: not measured by tooling (no coverage runner wired into `admin-ui` — see m3); manually verified all new production files have at least one passing test, except the five branches in M1.
- Branch coverage on changed files: not measured by tooling; the five `catch` branches in M1 are confirmed uncovered by direct inspection of the test files (no `mockRejectedValue`/`AdminApiError` usage found in `SectionList.test.ts` or `SourceList.test.ts`, and no `confirmUpload` rejection case in `UploadDropzone.test.ts`).
- Excluded files: none.

## Required Fixes Before Close

1. Add a test to `SectionList.test.ts` covering `addSection`'s error path (mock `createSection` to reject with `AdminApiError`, assert the honest message renders) — addresses M1.
2. Add a test to `SectionList.test.ts` covering `confirmDelete`'s error path (mock `deleteSection` to reject, assert `deleteError` renders and the dialog still closes) — addresses M1.
3. Add a test to `SourceList.test.ts` covering `addSource`'s error path (mock `createSource` to reject, assert the honest message renders) — addresses M1.
4. Add a test to `SourceList.test.ts` covering `confirmDelete`'s error path (mock `deleteSource` to reject, assert `deleteError` renders) — addresses M1.
5. Add a test to `UploadDropzone.test.ts` covering `confirm`'s error path (mock `confirmUpload` to reject, assert the error renders and `phase` falls back to `'preview'` rather than getting stuck on `'confirming'`) — addresses M1.

## Fix Log

- **[M1]** FIXED on 2026-07-24. Added five tests covering the previously-uncovered `catch` branches: `SectionList.test.ts` gained "shows an honest error message when adding a section fails" and "...when deleting a section fails"; `SourceList.test.ts` gained the equivalent pair for add/delete source; `UploadDropzone.test.ts` gained "shows an honest error message when confirming the upload fails" (asserting the honest error renders and `phase` falls back to `'preview'`, confirmed by the confirm/cancel buttons still being present rather than the flow getting stuck). All five fail against a reverted fix and pass against the current code. Verification: `npm run test` — 61/61 passed (up from 56); `npm run lint` (vue-tsc) — clean; `npm run format:check` — clean; `npm run build && npm run a11y` — 0 errors on `/`, `/dev`, `/ingest`.
