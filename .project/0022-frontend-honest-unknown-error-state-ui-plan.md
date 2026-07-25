# Plan 0022: frontend honest-unknown + error state UI

- **Status**: closed
- **Approved**: 2026-07-25 by agent
- **Implemented**: 2026-07-25 by agent
- **Closed**: 2026-07-25 by agent
- **Review verdict**: approved
- **Branch**: feat/frontend-honest-unknown-error-state-ui
- **Feature ID**: 0022
- **Created**: 2026-07-25
- **Owner**: agent

## Objective

Feature 0021 shipped the chat widget's golden path with an explicitly interim, placeholder rendering for the two non-golden states: when `fell_back=true` it shows a generic "Nessuna informazione trovata" callout, and on any `askChat` failure (network error or a non-2xx `/chat` response) it surfaces the raw thrown message — including backend-internal strings like `"upstream service unavailable"` or `"no active persona configured"` — directly to the citizen. This violates STACK.md §4.5's "honest states, no lying spinners" and PRINCIPLES.md §6.3's rejection of raw technical errors shown to citizens. This plan replaces both interim renderings with deliberately designed, calm, zero-jargon copy: the honest-unknown state (per Constitution §5's Knowledge Base Rule — Spontini must explicitly say when it found nothing, never invent) and the error state (network failure or `/chat` returning 502/503, rendered as an honest "non riesco a rispondere ora" message, never a raw error string or an "Error 500"). In scope: `ChatMessage.vue`'s fallback rendering, `ChatWidget.vue`'s error-catch rendering, and BDD-style coverage for both paths. Out of scope: the golden-path citation rendering (closed, feature 0021), any backend change (the `ChatResponse`/`fell_back` contract and HTTP status codes are consumed as-is), and the full accessibility audit (feature 0023).

## Non-Goals

- Backend changes to `/chat`'s response shape, status codes, or the `RagEngine` fallback logic — all already correct and tested (feature 0003).
- Retrying failed requests automatically — a failure is shown once; the citizen re-asks manually (consistent with "no lying spinners", no hidden retry magic).
- The dedicated accessibility/keyboard/reduced-motion audit (feature 0023) — this plan's new markup reuses the already-audited `DsCallout` roles (`alert`/`status`/`note`) from feature 0019/0020, so no new accessibility surface is introduced, but a fresh full audit pass is explicitly 0023's job.
- Distinguishing between different non-2xx status codes in the copy shown to the citizen (e.g. a different message for 502 vs 503) — STACK.md's roadmap description asks for one honest "non riesco a rispondere ora" state for "502/503", not a status-code-specific taxonomy.

## Phases

### Phase 1: Honest-unknown copy

Goal: When `fell_back=true`, the citizen sees the persona's actual fallback message presented with a calm, non-alarming tone and no citations — not a generic "no information found" wrapper around it.

- [x] **Task 1.1** — Redesign `ChatMessage.vue`'s fallback rendering
  - What: Replace the current `DsCallout` `title="Nessuna informazione trovata"` (which duplicates the message that follows) with a `note`-role presentation that reads as a calm, first-person statement from Spontini — the persona's `response.answer` (already the honest fallback text set by the backend, e.g. "Non ho trovato informazioni su questo argomento nei documenti comunali") is the sole content, with no editorializing wrapper text and no citation affordance rendered alongside it. Keep the DSI `DsCallout` component (for its live-region role) but drop the redundant title and use `variant="primary"` (a calm, neutral tone — `warning`/`danger` reads as alarming for "I don't know", which STACK.md §4.5 explicitly asks to avoid).
  - Deliverables:
    - `frontend/src/components/chat/ChatMessage.vue` (fallback branch reworked)
    - `frontend/src/components/chat/__tests__/ChatMessage.test.ts` updated: the existing fallback test asserts the calm-tone rendering (no alarming title, `variant="primary"`/`note` role) and that no citation list renders
  - Skills to load: spontini-bdd-gherkin, spontini-tdd-rust, spontini-verify-gate
  - Verification: `npm run test -- ChatMessage` passes in `frontend/`.

### Phase 2: Error state copy

Goal: When `askChat` fails — network failure or `/chat` returning a non-2xx status — the citizen sees one honest, calm "non riesco a rispondere ora" message, never the raw thrown error text.

- [x] **Task 2.1** — Add an honest error-copy constant and use it in `ChatWidget.vue`
  - What: Replace `ChatWidget.vue`'s current `catch` block, which surfaces `e.message` verbatim for `ChatApiError` (leaking backend-internal strings like `"upstream service unavailable"`), with a single fixed citizen-facing message — "Non riesco a rispondere ora. Riprova tra qualche minuto." — used for both `ChatApiError` and any other thrown error, regardless of status code or message content. The raw error is no longer interpolated into the UI at all.
  - Deliverables:
    - `frontend/src/components/chat/ChatWidget.vue` (`catch` block simplified to a single constant message for every failure path)
    - `frontend/src/components/chat/__tests__/ChatWidget.test.ts` updated: the existing rejection test asserts the fixed citizen-facing copy renders (not the raw `ChatApiError` message), plus a new case for a non-`ChatApiError` rejection (e.g. a thrown network `TypeError`) asserting the same fixed copy renders — closing the untested-branch gap noted as finding m1 in `.project/0021-frontend-chat-widget-with-citation-rendering-review.md`
  - Skills to load: spontini-tdd-rust, spontini-verify-gate
  - Verification: `npm run test -- ChatWidget.test` passes in `frontend/`.

### Phase 3: BDD scenarios

Goal: Executable, Given/When/Then-structured proof of both honest states, following the established no-cucumber-runner precedent (Plan 0018, reaffirmed in Plan 0021).

- [x] **Task 3.1** — Add honest-unknown and 502/503 BDD scenarios
  - What: Extend `ChatWidget.bdd.test.ts` with two additional Given/When/Then-commented scenarios: (1) a citizen asks a question the knowledge base has no document for (`askChat` resolves with `fell_back=true`) and sees the calm honest-unknown message with no citations; (2) a citizen asks a question while `/chat` is returning 502/503 (`askChat` rejects with a `ChatApiError`) and sees the fixed "non riesco a rispondere ora" message, never the raw backend error text.
  - Deliverables:
    - `frontend/src/components/chat/__tests__/ChatWidget.bdd.test.ts` (two new scenarios appended to the existing golden-path one)
  - Skills to load: spontini-bdd-gherkin, spontini-verify-gate
  - Verification: `npm run test -- ChatWidget.bdd` passes in `frontend/`.

## Acceptance Criteria

- When `/chat` returns `fell_back=true`, the citizen sees the persona's fallback message in a calm, non-alarming presentation with no citations and no invented detail.
- When `/chat` is unreachable or returns a non-2xx status, the citizen sees a single fixed, honest "non riesco a rispondere ora" message — never a raw error string, a stack trace, or an "Error 500".
- `ChatWidget.bdd.test.ts`'s honest-unknown and error scenarios are green, alongside the existing golden-path scenario.
- `ChatMessage.test.ts` and `ChatWidget.test.ts` cover both branches of every fallback/error conditional (closing review 0021's m1 finding).
- `make verify`-equivalent gates (build, test, clippy, fmt, vue-tsc, docker compose config) pass with no regression to the golden path shipped in feature 0021.

## Risks

- **Copy bikeshedding** — mitigation: the exact wording is fixed by this plan's tasks (not left to implementation-time judgment) and is deliberately short, matching STACK.md §4.5's zero-jargon rule.
- **Regressing the golden path while touching shared files** (`ChatMessage.vue`, `ChatWidget.vue`) — mitigation: run the full existing `ChatMessage.test.ts`/`ChatWidget.test.ts`/`ChatWidget.bdd.test.ts` suites (not just the new cases) after each change.

## Out-of-Scope

- Backend changes (feature 0003's `RagEngine`/`/chat` contract is consumed as-is).
- Automatic retry on failure.
- Full accessibility/keyboard/reduced-motion audit (feature 0023).
- Status-code-specific error copy.
