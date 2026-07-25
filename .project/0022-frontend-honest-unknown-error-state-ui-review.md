# Review 0022: frontend honest-unknown + error state UI

- **Plan**: [0022-frontend-honest-unknown-error-state-ui-plan.md](./0022-frontend-honest-unknown-error-state-ui-plan.md)
- **Branch**: feat/frontend-honest-unknown-error-state-ui
- **Reviewed**: 2026-07-25
- **Reviewer**: agent
- **Verdict**: approved

## Summary

Closes the two interim placeholders left by feature 0021: `ChatMessage.vue`'s fallback rendering no longer duplicates the honest-unknown message under an alarming "Nessuna informazione trovata" title (now a calm `variant="primary"` callout showing only the persona's own fallback text), and `ChatWidget.vue`'s catch path no longer leaks raw backend error strings (e.g. `"upstream service unavailable"`, `"no active persona configured"`) to the citizen — every failure, `ChatApiError` or otherwise, now renders one fixed honest message. Both of review 0021's minor findings (m1, m2 — untested error branches) are closed as a side effect of this plan's own test additions. Small, correct, well-tested diff; ships as-is.

## Findings

### Blockers

None.

### Major

None.

### Minor

None.

### Nits

- **[n1]** `frontend/src/components/chat/ChatWidget.vue:45-52` — the pending/in-flight branch's content (`"Sto rispondendo…"`) is still exercised but not directly asserted by any test (review 0021's m2 was about the error branch's content, which Task 2.1's new tests now do assert; this is a distinct, still-open, cosmetic gap). Low risk: a regression here would only hide the loading text, not leak information or break functionality. Optional future cleanup.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | n/a | Frontend-only Vue change, no crate boundaries touched. The `error: string \| null` → `failed: boolean` refactor in `ChatWidget.vue` is a genuine simplification: the component no longer needs to carry a per-exchange message once the message is a fixed constant, removing a previously-unused-in-practice dimension of state. |
| Truthfulness & RAG | pass | No RAG/backend change. The honest-unknown callout now shows exactly `response.answer` — the persona's own fallback text — with no editorializing wrapper and no citations, satisfying Constitution §5 (explicit "not found," no invented detail). Verified live: `curl /chat` against the real `RagEngine` still returns `fell_back`/`sources` unchanged; only client-side rendering of that DTO changed. |
| Ingest correctness | n/a | Not touched. |
| Tests (coverage + TDD + BDD) | pass | TDD followed: each task's test was written and observed failing against the pre-change component before the fix (confirmed during implementation — e.g. `ChatMessage.test.ts`'s new assertion failed with the old "Nessuna informazione trovata…" text, `ChatWidget.test.ts`'s new assertion failed by finding the raw `"upstream service unavailable"` string). `ChatWidget.bdd.test.ts` gained two new Given/When/Then scenarios (honest-unknown, backend-error) alongside the existing golden-path one, all three green. Review 0021's m1 (untested non-`ChatApiError` catch branch) is now moot — the branch distinction was removed entirely (both paths funnel to the same `failed = true`), and a dedicated test (`TypeError` rejection) still exercises and asserts it. |
| Clean Code | pass | `HONEST_ERROR_MESSAGE` is a named constant, not a repeated literal; `failed: boolean` is simpler and more intention-revealing than the previous `error: string \| null` that was always resolving to the same one or two strings; the `catch (e)` narrowed to a parameterless `catch` since `e` is no longer used — no dead bindings. |
| Clean Design (UI/UX) | pass | Matches STACK.md §4.5 exactly: honest-unknown reads as a calm first-person statement (no alarming title, neutral `primary`/`note` tone), and the error state is one fixed, jargon-free "non riesco a rispondere ora" message — no raw error text, no stack trace, ever reaching the citizen. No fake delay introduced. |
| Plan conformance | pass | All 3 tasks' deliverables exist and verifications pass. No scope creep — no backend files touched, no unrelated component changed. |

## Coverage Report

- Line coverage on changed files: not mechanically measured — no coverage tool is configured for `frontend` (pre-existing, repo-wide gap since feature 0015, same as every prior frontend/admin-ui review, including 0021's).
- Branch coverage on changed files: not mechanically measured; manually audited. `ChatMessage.vue`'s `fell_back` branch and `ChatWidget.vue`'s `try`/`catch` (both `ChatApiError` and generic-`Error` paths) are each covered by a dedicated test. No new untested branch was introduced.
- Excluded files: none.

## Required Fixes Before Close

None — verdict is `approved`.
