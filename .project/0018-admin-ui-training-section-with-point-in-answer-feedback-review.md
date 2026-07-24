# Review 0018: admin-ui Training section with point-in-answer feedback

- **Plan**: [0018-admin-ui-training-section-with-point-in-answer-feedback-plan.md](./0018-admin-ui-training-section-with-point-in-answer-feedback-plan.md)
- **Branch**: feat/admin-ui-training-section-with-point-in-answer-feedback
- **Reviewed**: 2026-07-24
- **Reviewer**: agent
- **Verdict**: changes-requested

## Summary

The Training section is implemented cleanly and is faithful to the plan: `adminApi.ts` gains eight typed functions whose DTOs and routes were checked line-by-line against `backend/src/admin/training_{sessions,messages,feedback}/{mod,handlers}.rs` and match exactly, including the `chunk_id` omission behavior (verified empirically that serde deserializes a missing `Option<i64>` key as `None`, matching the client's conditional-spread payload). The Truthfulness/RAG dimension — the primary concern for this feature — is correctly implemented: `MessageList.vue` renders citations strictly from the `sources` DTO array, never by parsing `answer` text, and when `fell_back` is `true` no `<details>`/citations block renders at all (verified by both code and a passing test asserting `details` does not exist), only the honest-unknown callout. The sentence-segmentation span-feedback design deviation is implemented soundly: clicking a segment selects exactly that segment's text as `answer_span`, clicking again deselects it, and `chunk_id` is never sent as a literal payload key. The closed-session UX hides `AskAnswerBox` via `v-if` (not just CSS) while `SpanFeedback` remains available — a reasonable, sound judgment call. All 124 admin-ui tests pass and `vue-tsc --noEmit` is clean; the diff touches only `admin-ui/` files plus the plan file, as required. The one real gap is test coverage: two newly introduced error/catch branches (`SessionList.vue`'s create-session failure path, `SpanFeedback.vue`'s feedback-list-fetch failure path) have zero test coverage, which is inconsistent with this feature's own precedent (the equivalent close-session error path is tested) and with the already-merged 0016 precedent (`SectionList.vue`/`SourceList.vue` both test their equivalent create-error paths). This is a narrow, cheaply fixable gap, not a structural or truthfulness problem.

## Findings

### Blockers

(none)

### Major

- **[M1]** `admin-ui/src/components/training/SessionList.vue:29-34` (catch block of `addSession`) — The create-session failure path (`addError` ternary, both the `AdminApiError` branch and the generic fallback branch) has zero test coverage. `admin-ui/src/components/training/__tests__/SessionList.test.ts` tests the close-session error path (`'shows the honest error message from AdminApiError on close failure'`) but has no equivalent test for `createSession` rejecting. This is inconsistent with the sibling create-flows in the already-merged feature 0016 (`SectionList.test.ts` and `SourceList.test.ts` both cover their create-error branch) and violates PRINCIPLES.md §7 ("No untested branches ... no exceptions"). Fix: add a test that mocks `adminApi.createSession` to reject with an `AdminApiError`, submits the add-session form, and asserts the honest error message renders.
- **[M2]** `admin-ui/src/components/training/SpanFeedback.vue:27-34` (`loadFeedback`) — The `catch { … }` block that silently starts the feedback list empty on a failed initial `listTrainingFeedback` fetch is entirely untested; every `SpanFeedback.test.ts` case mocks `listTrainingFeedback` to resolve. This is a newly introduced silent-catch pattern (no equivalent exists elsewhere in the codebase) with no test proving the documented fallback behavior actually holds (i.e., that the component still mounts and functions rather than throwing an unhandled rejection). Fix: add a test that mocks `adminApi.listTrainingFeedback` to reject on mount and asserts the component renders normally with an empty persisted-feedback list.

### Minor

- **[m1]** Across all new components (`AskAnswerBox.vue`, `SessionList.vue`'s close path, `SpanFeedback.vue`'s submit path, `TrainingSessionsView.vue`, `TrainingSessionView.vue`), only the `e instanceof AdminApiError` branch of the `error.value = e instanceof AdminApiError ? e.message : '<generic message>'` ternary is tested; the generic-fallback branch (a non-`AdminApiError` rejection, e.g. a network `TypeError`) is never exercised. This mirrors a pre-existing, repo-wide pattern already present in features 0016/0017 (`ScheduleEditor.vue`, `UploadDropzone.vue`, `PersonaEditor.vue`, etc. have the same untested fallback branch), so it is not a regression introduced by 0018 — flagged for awareness only, not required to fix here.
- **[m2]** `admin-ui/src/views/DevCatalog.vue` (`/dev` route) was not updated to list the four new business components (`SessionList`, `AskAnswerBox`, `MessageList`, `SpanFeedback`) per STACK.md §4.4.3 ("documented in a component catalog"). This mirrors established precedent: features 0016 and 0017 also introduced multiple new business components (`SectionList`, `SourceList`, `UploadDropzone`, `PersonaEditor`, `VersionHistory`, `ReloadPersonaButton`, `DsConfirmDialog`) without adding them to `DevCatalog.vue` (its only commit is from feature 0015). Pre-existing, repo-wide gap, not unique to this feature.

### Nits

- **[n1]** `admin-ui/src/components/training/SpanFeedback.vue:87` — the segment `v-for` uses the array index as `:key`. Harmless here since the segment list is static per mount and never reordered, but index-as-key is a general Vue anti-pattern; a stable key such as `` `${index}-${segment}` `` would be marginally more defensive against future changes.
- **[n2]** `admin-ui/src/components/training/MessageList.vue:28-35` — if a non-fallback (`fell_back: false`) message legitimately had zero sources, the UI would render "Fonti (0)", which is exactly the pattern Constitution §5 warns against for the fallback case. In the current backend, `fell_back: false` always implies at least one retrieved source, so this is not reachable in practice — flagged only as a latent coupling to a backend invariant that the UI does not itself assert.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | pass | `adminApi.ts` is the sole HTTP boundary — grepped the whole diff for stray `fetch(` calls, found none outside `adminApi.ts`. Each component has one responsibility: `SessionList` (list/create/close), `AskAnswerBox` (ask), `MessageList` (render + compose `SpanFeedback`), `SpanFeedback` (span selection + feedback CRUD-lite). Views orchestrate fetch/compose only, no business logic. |
| Truthfulness & RAG | pass | Citations sourced strictly from `message.sources`, never from parsing `message.answer` (grepped for regex/text-parsing citation extraction in `MessageList.vue` — none exists). `fell_back: true` renders zero citations (verified via `<details v-else>` gating and a passing test asserting `details` does not exist) plus a calm honest-unknown `DsCallout`. `chunk_id` omission verified against real serde behavior (empirically confirmed via an isolated `serde_json` test that a missing `Option<i64>` JSON key deserializes to `None`, matching the backend's `CreateFeedbackRequest.chunk_id: Option<i64>`). `listTrainingFeedback(messageId)` is correctly scoped per-message, not per-session. |
| Ingest correctness | n/a | No ingest files touched — confirmed via `git diff --stat`; only `admin-ui/` files and the plan file are in the diff. |
| Tests (coverage + TDD + BDD) | fail | 124/124 tests pass, every new `adminApi.ts` function has a success + `AdminApiError` test, and Task 5.1's `TrainingSessionView.integration.test.ts` is a real, behavioral Given/When/Then scenario (ask → cited answer → span-click → negative feedback → persisted). However two catch branches introduced by this feature (M1, M2) have zero coverage, violating PRINCIPLES.md §7's "no untested branches, no exceptions" gate. |
| Clean Code | pass | No magic numbers/strings of concern (the sentence-split regex `/(?<=[.!?])\s+/` is a self-documenting domain rule, not a magic literal); no `any`/`as any`/`@ts-ignore` in any new file; no dead code; functions stay small and single-purpose; names reveal intent (`toggleSegment`, `submitFeedback`, `loadFeedback`). |
| Clean Design (UI/UX) | pass | Honest loading ("Caricamento…"), error (`DsCallout variant="danger"` with the real `AdminApiError` message), empty ("Nessuna sessione"), and not-found (404 message surfaced verbatim) states are all present and tested. Closing a session is gated behind `DsConfirmDialog`, structurally identical to the 0016 `SectionList.vue` delete-confirmation contract (`requestClose`/`confirmClose`/`cancelClose`, `variant="danger" outline` button, dialog only opened not the mutation). `AskAnswerBox` is hidden for a closed session via `v-if="!session.closed_at"` (a real conditional render, not CSS), while `SpanFeedback` remains available — verified by two dedicated tests. No lying spinners, no fake delays. |
| Plan conformance | pass | All 5 phases / 8 tasks' declared deliverables exist and match; verification commands (`npm run test`, `npm run lint`) both pass as claimed. Diff contains only `admin-ui/` files plus the plan file — no `frontend/` or `backend/` changes, and (correctly) no equivalent to 0017's one-line `frontend/.prettierignore` fix was needed or added. No unrequested scope creep found. |

## Coverage Report

- Line coverage on changed files: not measured — no coverage tool is configured for admin-ui (`npm run test` = `vitest run`, no `--coverage` flag, no `vitest.config.ts` coverage block; this is a pre-existing, repo-wide gap, not introduced by 0018).
- Branch coverage on changed files: not measured with tooling; manually audited every `if`/`catch`/ternary in the new files. Two catch branches are untested — see M1, M2. All other branches (loading/error/empty/not-found states, `fell_back` true/false, segment select/deselect, dialog confirm/cancel, `created_by` present/absent) have direct tests.
- Excluded files: none.

## Required Fixes Before Close

1. Add a test to `admin-ui/src/components/training/__tests__/SessionList.test.ts` that mocks `adminApi.createSession` to reject with an `AdminApiError`, submits the add-session form, and asserts the honest error message renders — mirroring the existing close-session error test in the same file and the precedent in `SectionList.test.ts`/`SourceList.test.ts`. Fixes **M1**.
2. Add a test to `admin-ui/src/components/training/__tests__/SpanFeedback.test.ts` that mocks `adminApi.listTrainingFeedback` to reject on mount and asserts the component still renders correctly with an empty persisted-feedback list (no unhandled rejection). Fixes **M2**.

## Fix Log

- **[M1]** FIXED on 2026-07-24. Added `'shows the honest error message from AdminApiError on create failure'` to `SessionList.test.ts`, mocking `createSession` to reject and asserting the error message renders with no `changed` emission. Verification: `npm run test` 126/126 passing, `npm run lint` clean, `npm run format:check` clean.
- **[M2]** FIXED on 2026-07-24. Added `'renders normally with an empty feedback list when the initial fetch fails'` to `SpanFeedback.test.ts`, mocking `listTrainingFeedback` to reject and asserting the component still renders its segments with no persisted-feedback list and no leaked error text. Verification: `npm run test` 126/126 passing, `npm run lint` clean, `npm run format:check` clean.
