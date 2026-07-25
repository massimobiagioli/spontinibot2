# Plan 0021: frontend chat widget with citation rendering

- **Status**: closed
- **Approved**: 2026-07-25 by agent
- **Implemented**: 2026-07-25 by agent
- **Closed**: 2026-07-25 by agent
- **Review verdict**: approved
- **Branch**: feat/frontend-chat-widget-with-citation-rendering
- **Feature ID**: 0021
- **Created**: 2026-07-25
- **Owner**: agent

## Objective

Build the citizen-facing chat widget that is the entire reason Spontini exists: a single primary action (send a message), a conversation view, a forgiving input ("Scrivi la tua domanda…"), and answers rendered with inline expandable source citations built strictly from the `sources` array of the `POST /chat` response DTO (`ChatResponse` in `backend/src/routes.rs`) — never by parsing the answer text, consistent with the Constitution's citation-integrity requirement. This closes the loop opened by feature 0020 (DS Italia integration + `/dev` catalog): that plan built the wrapper components and the catalog route but shipped no chat UI. This plan is scoped to the **happy path and the loading/empty states** of the widget — touch targets ≥ 44×44 px, an empty conversation state, and a loading state while awaiting `/chat`. The **honest-unknown (`fell_back=true`) and HTTP error (502/503) states** are explicitly out of scope for this plan and are the entire subject of feature 0022, which will extend this widget's rendering branches. A minimal, calm placeholder is acceptable here for those branches (e.g. rendering `fell_back` answers with no citations, using the existing `DsCallout` warning pattern already proven in `admin-ui`'s `MessageList.vue`), but polishing that tone and covering the error-response path is 0022's job, not this plan's.

## Non-Goals

- Honest-unknown tone design and BDD coverage (feature 0022).
- 502/503 error-state design and BDD coverage (feature 0022).
- Accessibility audit pass with `axe-core`/`pa11y` gate wiring beyond what already exists from feature 0020 (feature 0023).
- Point-in-answer feedback (that is an operator/training-only feature, 0014/0018 — the public chat has no feedback mechanism).
- Streaming/typing-indicator responses — `/chat` is a single request/response round trip.
- Persisting conversation history across page reloads (in-memory only for this plan).

## Phases

### Phase 1: Chat API client

Goal: A typed, tested client for `POST /chat` that the widget component can consume, following the same `request`/`jsonRequest` pattern already used in `admin-ui/src/services/adminApi.ts`.

- [x] **Task 1.1** — Add `frontend/src/services/chatApi.ts`
  - What: Implement `askChat(question: string): Promise<ChatResponse>` that POSTs `{ question }` (JSON) to `/chat`, mirroring `backend/src/routes.rs`'s `ChatRequest`/`ChatResponse`/`ChatSource` shapes (`answer: string`, `sources: { document_id: number; source_ref: string }[]`, `fell_back: boolean`), and a `ChatApiError` class carrying the HTTP status and message (mirrors `AdminApiError` in `admin-ui`). No admin-key header — `/chat` is the public, unauthenticated endpoint.
  - Deliverables:
    - `frontend/src/services/chatApi.ts` (exports `askChat`, `ChatApiError`, `ChatResponse`, `ChatSource` types)
    - `frontend/src/services/__tests__/chatApi.test.ts` covering: success response shape, non-2xx response throws `ChatApiError` with parsed `{error}` body, network/JSON-parse failure falls back to a generic message
  - Skills to load: spontini-tdd-rust (test-first discipline applies project-wide, including TS), spontini-verify-gate
  - Verification: `npm run test -- chatApi` passes in `frontend/`; `askChat` return type matches `ChatResponse` structurally.

### Phase 2: Chat widget components

Goal: A conversation UI composed of small, focused components under `frontend/src/components/chat/`, wired into the home route.

- [x] **Task 2.1** — Build `ChatMessage.vue` (single exchange renderer)
  - What: Render one question/answer exchange: the citizen's question, the bot's answer text, and — when `fell_back` is `false` and `sources.length > 0` — an expandable `<details>` list of citations built from `source.source_ref` (one list item per source, keyed by `document_id`), reusing the DSI expandable-details pattern already proven in `admin-ui/src/components/training/MessageList.vue`. When `fell_back` is `true`, render a `DsCallout` (variant `warning`) instead of citations (placeholder tone; 0022 refines wording). No text-parsing of `answer` to invent citations.
  - Deliverables:
    - `frontend/src/components/chat/ChatMessage.vue`
    - `frontend/src/components/chat/__tests__/ChatMessage.test.ts` covering: renders question + answer text; renders one `<li>` per source when not fallen back; renders `DsCallout` and no citation list when `fell_back=true`; citation labels come from `source_ref`, not from scanning `answer`
  - Skills to load: spontini-tdd-rust, spontini-verify-gate
  - Verification: component tests pass; visual check confirms citations render as an expandable disclosure, not inline unlabeled text.

- [x] **Task 2.2** — Build `ChatInput.vue` (question composer)
  - What: A form wrapping `DsInput` (label "La tua domanda", placeholder "Scrivi la tua domanda…") and a `DsButton` submit action ("Invia" / disabled + "Invio…" while awaiting response), sized so the submit control meets the ≥44×44px touch-target rule (reuse the CSS approach from the feature-0019 admin-ui touch-target fixes). Emits `ask` with the trimmed question string; clears the input and refuses to emit on empty/whitespace-only input.
  - Deliverables:
    - `frontend/src/components/chat/ChatInput.vue`
    - `frontend/src/components/chat/__tests__/ChatInput.test.ts` covering: emits `ask` with trimmed text on submit; does not emit on blank input; disables the button while `busy` prop is true
    - `frontend/src/components/ds/DsInput.vue` extended with a `placeholder` prop passed through to the native input (it previously only supported a below-field `hint`), plus a covering test in `frontend/src/components/ds/__tests__/DsInput.test.ts` — a minimal necessary extension to satisfy the roadmap's explicit "forgiving placeholder" requirement
  - Skills to load: spontini-tdd-rust, spontini-verify-gate
  - Verification: component tests pass.

- [x] **Task 2.3** — Build `ChatWidget.vue` (conversation orchestrator) and wire into `HomeView.vue`
  - What: Owns the conversation array (each entry: `{ question: string; response: ChatResponse | null; error: string | null }`), an empty-conversation state (calm placeholder copy inviting the first question — no citations, no error chrome), a loading state (disables `ChatInput` and shows a busy indicator on the pending exchange while `askChat` is in flight), calls `askChat` on `ChatInput`'s `ask` event, appends the resulting `ChatMessage` entries in order, and — for this plan only — on a caught `ChatApiError`/network failure appends a minimal `DsCallout` (variant `danger`) with the caught message as an interim placeholder (0022 replaces this with the designed honest "non riesco a rispondere ora" copy). Replace `HomeView.vue`'s placeholder `<h1>Spontini</h1>` with the widget mounted inside the existing `app-shell__main` region from `App.vue`.
  - Deliverables:
    - `frontend/src/components/chat/ChatWidget.vue`
    - `frontend/src/views/HomeView.vue` (updated to mount `ChatWidget`)
    - `frontend/src/components/chat/__tests__/ChatWidget.test.ts` covering: empty state renders before any question is asked; asking a question appends a `ChatMessage` with the mocked `askChat` response; a rejected `askChat` call appends an error callout instead of crashing; input is disabled while a request is in flight
    - `frontend/vite.config.ts` and `frontend/nginx.conf` extended to proxy `/chat` to the `backend` service — discovered during manual sanity (Gate 10) that neither the dev server nor the production nginx config routed `/chat` anywhere, which would have made the widget non-functional end-to-end despite all component tests passing (mirrors the existing `/admin/api` proxy pattern in `admin-ui/vite.config.ts`)
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard, spontini-verify-gate
  - Verification: component tests pass; `npm run dev` in `frontend/` and manually asking a question against a running `backend` returns a rendered answer with expandable sources.

### Phase 3: BDD scenario and DS catalog entry

Goal: Executable acceptance proof and consistency with the `/dev` catalog convention established in feature 0020.

- [x] **Task 3.1** — Add BDD scenario for ask → answer → expand citations
  - What: `frontend` has no cucumber runner — same established precedent as `admin-ui` (Plan 0018 Non-Goals: "no dedicated Gherkin `.feature` file — `admin-ui` has no cucumber runner ... the roadmap's BDD scenario is satisfied by a Vitest integration test with explicit Given/When/Then structure"). Follow that precedent: add a Vitest integration test on `ChatWidget.vue` structured with explicit `// Given` / `// When` / `// Then` comments exercising the golden path — a citizen opens the chat, asks a question the mocked `chatApi` can answer, sees the answer rendered, and expands the citation list to see the cited source(s) — against a mocked `askChat`.
  - Deliverables:
    - `frontend/src/components/chat/__tests__/ChatWidget.bdd.test.ts` (Given/When/Then-structured Vitest test, separate from `ChatWidget.test.ts`'s unit-level cases)
  - Skills to load: spontini-bdd-gherkin, spontini-verify-gate
  - Verification: `npm run test -- ChatWidget.bdd` passes in `frontend/`.

- [x] **Task 3.2** — Register chat components in the `/dev` catalog
  - What: Add `ChatMessage` and `ChatInput` (not the full stateful `ChatWidget`, which depends on live network calls) to `DevCatalog.vue` with representative fixture props (an answered exchange, a fallen-back exchange), following the existing catalog entry pattern for `DsButton`/`DsInput`/`DsCallout`.
  - Deliverables:
    - `frontend/src/views/DevCatalog.vue` (updated with two new catalog sections)
    - Updated `frontend/src/views/__tests__/DevCatalog.test.ts` assertions for the new sections
  - Skills to load: spontini-verify-gate
  - Verification: `/dev` route renders the new sections; catalog test passes.

## Acceptance Criteria

- A citizen can type a question into the chat widget on `/` and receive a rendered answer.
- When the answer has cited sources, they render as an expandable, labeled disclosure built from the `sources` DTO field — never from parsing `answer` text.
- The submit control and any interactive citation toggle meet the ≥44×44 px touch-target minimum.
- An empty-conversation state and a loading (in-flight request) state are visibly distinct and intentionally designed, not blank/frozen UI.
- `frontend/src/components/chat/__tests__/ChatWidget.bdd.test.ts`'s ask → answer → expand-citations scenario is green.
- All new components appear in the `/dev` catalog.
- `make verify` passes with the existing coverage gate maintained.

## Risks

- **Scope creep into honest-unknown/error polish** (0022's territory) — mitigation: the interim `fell_back`/error rendering in Tasks 2.1/2.3 is explicitly a placeholder using an existing DSI pattern (`DsCallout`), not new copy design; 0022 is scoped to replace it, not to build it from scratch.
- **BDD harness location/tooling for `frontend` may differ from what feature 0018 established in `admin-ui`** (different test runner wiring) — mitigation: inspect `admin-ui`'s BDD setup during Task 3.1 and mirror it exactly rather than inventing a new harness.

## Out-of-Scope

- Honest-unknown and 502/503 error UI polish (feature 0022).
- Accessibility audit and zero-violation gate (feature 0023).
- Conversation persistence across reloads.
- Streaming responses.
