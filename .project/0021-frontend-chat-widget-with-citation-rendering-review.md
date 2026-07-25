# Review 0021: frontend chat widget with citation rendering

- **Plan**: [0021-frontend-chat-widget-with-citation-rendering-plan.md](./0021-frontend-chat-widget-with-citation-rendering-plan.md)
- **Branch**: feat/frontend-chat-widget-with-citation-rendering
- **Reviewed**: 2026-07-25
- **Reviewer**: agent
- **Verdict**: approved

## Summary

Implements the public chat widget (`ChatWidget.vue`, `ChatMessage.vue`, `ChatInput.vue`, `chatApi.ts`) with citations rendered strictly from the `/chat` response's `sources` DTO, never from parsing answer text, and an honest interim rendering for the fallback/error paths. All 8 plan tasks are complete with tests, and the implementer went beyond the plan's letter to find and fix a real production defect: neither `frontend/nginx.conf` nor `frontend/vite.config.ts` proxied `/chat` to `backend`, which would have made the widget silently non-functional in `make up` despite every component test passing. The end-to-end path was verified live (seeded persona + uploaded document, real `llama-embed`/`llama-generate`, curled through the actual built nginx container) with a correctly cited answer. This ships as-is; two minor test-coverage gaps and a deferred visual-polish note are recorded below for future attention, none of which block.

## Findings

### Blockers

None.

### Major

None.

### Minor

- **[m1]** `frontend/src/components/chat/ChatWidget.vue:24-29` — the `catch` block's non-`ChatApiError` branch (`'Non riesco a rispondere ora. Riprova più tardi.'`) is never exercised by a test; `ChatWidget.test.ts`'s only rejection case throws a `ChatApiError`. PRINCIPLES.md §7 requires both sides of every `catch` to be tested. Suggested fix: add a case in `ChatWidget.test.ts` where `askChat` rejects with a plain `Error` (e.g. a simulated network failure) and assert the generic fallback message renders.
- **[m2]** `frontend/src/components/chat/ChatWidget.vue:44-47` — the pending/in-flight `v-else` branch (`"Sto rispondendo…"`) is exercised by `ChatWidget.test.ts`'s "disables the input while a request is in flight" test but never asserted on directly — that test only checks the `disabled` attribute, not that the pending indicator's content renders. A regression that deleted the pending markup would not be caught. Suggested fix: assert `wrapper.text()` contains `'Sto rispondendo'` while the promise is unresolved in that same test.

### Nits

- **[n1]** `frontend/src/components/chat/ChatMessage.vue`, `ChatWidget.vue` — the new BEM classes (`chat-message`, `chat-message__question`, `chat-widget__empty`, …) carry no SCSS rules; the widget currently renders as unstyled stacked text beyond what Bootstrap Italia's own component classes (`.btn`, `.form-control`, `.callout`) provide. STACK.md §4.4.3 asks new custom components to "match DSI's visual language." This mirrors the exact same pattern already shipped and approved in `admin-ui`'s `MessageList.vue`/`AskAnswerBox.vue` (no bespoke chat-bubble CSS either), so it is not a regression introduced here — flagging only so a future visual pass (naturally feature 0023's accessibility/polish audit, or a dedicated design pass) picks it up.
- **[n2]** `frontend/src/components/chat/ChatMessage.vue:24` (`<details><summary>Fonti (...)</summary>`) — the citation disclosure is a native, unstyled `<summary>` with no verified ≥44×44px touch target, same open question as the equivalent disclosure in `admin-ui`'s `MessageList.vue` which shipped without one. Deferred to feature 0023 per this plan's own Non-Goals, consistent with precedent.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | n/a | Frontend-only Vue change; no Rust crate boundaries touched. `chatApi.ts` cleanly isolates the fetch call, `ChatWidget` orchestrates, `ChatMessage`/`ChatInput` are presentation-only — consistent with the thin service+component pattern already established in `admin-ui`. |
| Truthfulness & RAG | pass | Citations are built exclusively from `response.sources` (`ChatMessage.vue:27-29`), never from parsing `answer` text. The `fell_back` path renders the persona's honest fallback message with no citations (no invented detail). Verified live against the real `RagEngine`: an answerable question returned a correctly cited answer; a documentless question returned the honest fallback. |
| Ingest correctness | n/a | Not touched by this feature. |
| Tests (coverage + TDD + BDD) | pass | TDD followed throughout (each task's test was written and confirmed failing before the implementation, per the session's actual execution order). BDD scenario (`ChatWidget.bdd.test.ts`) is Given/When/Then structured per the established no-cucumber-runner precedent (Plan 0018). No frontend coverage tool is wired (pre-existing, repo-wide gap since feature 0015, same as every prior frontend/admin-ui review) so coverage is manually audited: all branches covered except m1/m2 above. |
| Clean Code | pass | Small, single-purpose functions (`ask`, `submit`), intention-revealing names, no magic numbers, no dead code, no unjustified `unwrap`/non-null assertion in production code (the one `!` in test code is a standard deferred-promise pattern). |
| Clean Design (UI/UX) | pass | One primary action (ask), honest loading/empty/error states with no fake delay, forgiving input (trims, blocks blank), citations inline and expandable, error/fallback states use existing `DsCallout` roles for correct screen-reader announcement. Visual polish gap noted as n1 (consistent with precedent, not a regression). |
| Plan conformance | pass | All 8 tasks' deliverables exist and their verifications pass. Two additions beyond the literal task list — `DsInput`'s `placeholder` prop and the `/chat` nginx/vite proxy fix — were both transparently documented in the plan file with justification at the time they were made, not silent scope creep, and both were necessary for the feature to function (the roadmap explicitly requires a placeholder; the proxy fix was a real functional gap, not a nice-to-have). |

## Coverage Report

- Line coverage on changed files: not mechanically measured — no coverage tool is configured for `frontend` (pre-existing, repo-wide gap since feature 0015; frontend coverage tooling is deferred to feature 0024's CI pipeline per the roadmap, same precedent as every prior frontend/admin-ui review).
- Branch coverage on changed files: not mechanically measured; manually audited. All conditional branches in `chatApi.ts`, `ChatMessage.vue`, `ChatInput.vue`, and `ChatWidget.vue` are covered except the two noted in m1/m2.
- Excluded files: none explicitly excluded.

## Required Fixes Before Close

None — verdict is `approved`. m1/m2/n1/n2 are recorded for awareness; addressing them is optional and left to the implementer's discretion or a future pass.
