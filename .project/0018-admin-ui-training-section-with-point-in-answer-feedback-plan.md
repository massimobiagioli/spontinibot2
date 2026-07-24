# Plan 0018: admin-ui Training section with point-in-answer feedback

- **Status**: review
- **Approved**: 2026-07-24 by agent
- **Implemented**: 2026-07-24 by agent
- **Branch**: feat/admin-ui-training-section-with-point-in-answer-feedback
- **Feature ID**: 0018
- **Created**: 2026-07-24
- **Owner**: agent

## Objective

Milestone 3's operator console gets its third and final first-class business section. This feature builds the **Training** section of `admin-ui`, giving an operator a place to exercise Spontini's `RagEngine` directly and record structured feedback on its answers — the same truthfulness discipline the citizen-facing `/chat` honors (Constitution §3 Truthfulness, §5 Knowledge Base Rule), made inspectable and correctable by a human. The section lets the operator create and browse training sessions (feature 0012), ask a question inside a session and see the answer with its cited sources rendered from the structured `sources` DTO — never by parsing the answer text, preserving the same citation honesty the roadmap has enforced since feature 0003 (feature 0013), and leave point-in-answer feedback: select a portion of the answer, mark it positive or negative, add an optional comment, and persist it (feature 0014). Every call goes through new typed functions added to the existing `admin-ui/src/services/adminApi.ts` client (built in feature 0016, extended in feature 0017), talking to the already-existing `/admin/api/training/sessions*`, `/admin/api/training/sessions/:id/messages`, and `/admin/api/training/feedback` / `/admin/api/training/messages/:id/feedback` endpoints — no backend changes. In scope: the Training route (a session list plus a per-session detail route), session creation and closing, the ask/answer flow reusing `RagEngine::answer` through the training endpoint, expandable source citations, the honest-unknown fallback rendering (`fell_back=true`), point-in-answer feedback capture and its persisted display, and the left-rail `Training` link activation. Out of scope: the Ingest and Imprinting sections (closed, features 0016/0017), operator authentication (feature 0027, same `VITE_ADMIN_API_KEY` placeholder), and the full cross-app accessibility audit (feature 0019).

## Non-Goals

- No Ingest or Imprinting screens — only the Training section is built.
- No operator login/session UI — reuses the `VITE_ADMIN_API_KEY` build-time env var and `X-Admin-Key` header pattern from features 0016/0017. Feature 0027 replaces this with real auth.
- No backend changes — `/admin/api/training/sessions*`, `/admin/api/training/sessions/:id/messages`, `/admin/api/training/feedback`, and `/admin/api/training/messages/:id/feedback` already exist (features 0012-0014) and are consumed as-is.
- No arbitrary character-range text selection for point-in-answer feedback. The backend's `answer_span` is a free-text column with no offset/length pair, and the browser `Selection`/`Range` API is notoriously brittle to drive deterministically in `jsdom` component tests. Instead, an answer is split into sentence-level segments (on `.`/`!`/`?` followed by whitespace); the operator clicks a segment to select it as the span before leaving feedback. This keeps the "select a span of the answer, mark positive/negative" requirement intact at sentence granularity, with fully deterministic click-based tests instead of simulated text selection.
- No linking feedback to a specific cited chunk — `chunk_id` is nullable on the backend and the roadmap does not require a chunk-picker UI; feedback is submitted with `chunk_id: null`. A future refinement could let the operator tap a citation to attach `chunk_id`, but it is not required here.
- No editing or deleting a session, a message, or a feedback entry — the backend has no update/delete endpoints for any of the three (sessions support create/list/get/close only). The UI offers exactly the backend's surface.
- No dedicated Gherkin `.feature` file — `admin-ui` has no cucumber runner (established in feature 0015/0016/0017); the roadmap's "BDD scenario" is satisfied by a Vitest integration test with explicit Given/When/Then structure.

## Phases

### Phase 1: Admin API client extension

Goal: `admin-ui/src/services/adminApi.ts` can call every `/admin/api/training/*` endpoint.

- [x] **Task 1.1** — Extend the typed admin API client with training functions
  - What: Add `createSession(title, createdBy?)`, `listSessions()`, `getSession(id)`, `closeSession(id)`, `askTrainingMessage(sessionId, question)`, `listTrainingMessages(sessionId)`, `createTrainingFeedback(payload)`, `listTrainingFeedback(messageId)` to `admin-ui/src/services/adminApi.ts`, with TypeScript interfaces (`TrainingSessionResponse`, `CreateSessionRequest`, `ClosedResponse`, `TrainingMessageSource`, `TrainingMessageResponse`, `TrainingFeedbackResponse`, `CreateFeedbackRequest`) mirroring `backend/src/admin/training_sessions/mod.rs`, `backend/src/admin/training_sessions/handlers.rs`, `backend/src/admin/training_messages/mod.rs`, `backend/src/admin/training_messages/handlers.rs`, `backend/src/admin/training_feedback/mod.rs`, and `backend/src/admin/training_feedback/handlers.rs` verbatim. Reuse the existing `AdminApiError`, `request`, and `jsonRequest` helpers already in the file.
  - Deliverables:
    - `admin-ui/src/services/adminApi.ts` updated
    - `admin-ui/src/services/__tests__/adminApi.test.ts` updated (one success-path test and one `AdminApiError` non-2xx test per new function)
  - Skills to load: (none)
  - Verification: `npm run test` passes; every new exported function has a covering test.

### Phase 2: Training route and session list

Goal: The `/training` route lists sessions, lets the operator create a new one and close an existing one, and the left-rail `Training` link is activated.

- [x] **Task 2.1** — Scaffold the `/training` route and activate the left-rail link
  - What: Add `admin-ui/src/views/TrainingSessionsView.vue` (fetches `listSessions()` on mount, renders loading/error/empty states), register it at `/training` in `admin-ui/src/router/index.ts`, and change `admin-ui/src/App.vue`'s `businessLinks` so the `Training` entry has `to: '/training'` instead of being a disabled placeholder.
  - Deliverables:
    - `admin-ui/src/views/TrainingSessionsView.vue`
    - `admin-ui/src/router/index.ts` updated
    - `admin-ui/src/App.vue` updated (`Training` link active)
  - Skills to load: (none)
  - Verification: `npm run test` — a mounting test for `TrainingSessionsView.vue` (mocking `adminApi.listSessions`) asserts the loading state renders then resolves; `npm run dev`, navigate via the left rail to `/training`, confirm it loads.

- [x] **Task 2.2** — Build the session list with create and close
  - What: Implement `admin-ui/src/components/training/SessionList.vue` — renders each session (title, `created_at`, a "Chiusa" badge when `closed_at` is set, a `RouterLink` to `/training/:id`), an "add session" inline form (title, calls `adminApi.createSession`), and a "Chiudi sessione" button per open session gated behind the existing `DsConfirmDialog` (from feature 0016) since closing is irreversible (no reopen endpoint). Compose it into `TrainingSessionsView.vue`.
  - Deliverables:
    - `admin-ui/src/components/training/SessionList.vue`
    - `admin-ui/src/components/training/__tests__/SessionList.test.ts` (create flow calls `createSession`; close flow opens the confirm dialog then calls `closeSession` on confirm, not before; closed sessions render the badge and no close button)
    - `admin-ui/src/views/TrainingSessionsView.vue` updated to render `SessionList`
  - Skills to load: (none)
  - Verification: `npm run test` passes.

- [x] **Task 2.3** — Scaffold the `/training/:id` session detail route
  - What: Add `admin-ui/src/views/TrainingSessionView.vue` (reads the `id` route param, fetches `getSession(id)` and `listTrainingMessages(id)` on mount, renders loading/error/honest-not-found states), register it at `/training/:id` in `admin-ui/src/router/index.ts`.
  - Deliverables:
    - `admin-ui/src/views/TrainingSessionView.vue`
    - `admin-ui/src/router/index.ts` updated
    - `admin-ui/src/views/__tests__/TrainingSessionView.test.ts` (mounts with a mocked route param, asserts loading then resolved state; a 404 `AdminApiError` from `getSession` renders an honest "sessione non trovata" state)
  - Skills to load: (none)
  - Verification: `npm run test` passes.

### Phase 3: Ask/answer flow with citations

Goal: Inside a session, the operator can ask a question and see the recorded exchange with its cited sources or the honest-unknown fallback state.

- [x] **Task 3.1** — Build the ask/answer box
  - What: Implement `admin-ui/src/components/training/AskAnswerBox.vue` — a form with a `<DsInput>` for the question and a `<DsButton>` "Chiedi" calling `adminApi.askTrainingMessage(sessionId, question)`; on success, clears the input and emits `asked` with the new `TrainingMessageResponse` so the parent can prepend it to the message list; on failure (e.g. a 502 from a downstream generation error) shows the honest `AdminApiError` message via `<DsCallout>`, never a raw error code.
  - Deliverables:
    - `admin-ui/src/components/training/AskAnswerBox.vue`
    - `admin-ui/src/components/training/__tests__/AskAnswerBox.test.ts` (submit calls `askTrainingMessage` with the right args and emits `asked`; error path shows the honest message)
  - Skills to load: (none)
  - Verification: `npm run test` passes.

- [x] **Task 3.2** — Build the message list with expandable citations and honest-unknown rendering
  - What: Implement `admin-ui/src/components/training/MessageList.vue`, rendering each `TrainingMessageResponse` newest-first: the question, the answer text, and — built strictly from the `sources` array, never by parsing `answer` — a collapsible `<details>` "Fonti (`N`)" listing each `TrainingMessageSource`'s `source_ref`; when `fell_back` is `true`, render no citations and a calm `<DsCallout variant="warning">` honest-unknown notice instead (mirroring the citizen-facing tone from Constitution §5), never an empty "Fonti (0)" that implies a lookup happened. Compose it into `TrainingSessionView.vue` alongside `AskAnswerBox`.
  - Deliverables:
    - `admin-ui/src/components/training/MessageList.vue`
    - `admin-ui/src/components/training/__tests__/MessageList.test.ts` (renders sources in a collapsible details block; a `fell_back: true` message renders the honest-unknown callout and zero citation entries)
    - `admin-ui/src/views/TrainingSessionView.vue` updated to render `AskAnswerBox` and `MessageList`
  - Skills to load: spontini-rag-build
  - Verification: `npm run test` passes.

### Phase 4: Point-in-answer feedback

Goal: The operator can select a sentence-level span of an answer, mark it positive or negative, add a comment, submit it, and see it persisted alongside the message.

- [x] **Task 4.1** — Build the span-feedback control
  - What: Implement `admin-ui/src/components/training/SpanFeedback.vue`, rendered per message inside `MessageList.vue` in place of the raw answer text: split `message.answer` into sentence segments (regex on `.`/`!`/`?` followed by whitespace), render each as a `<button type="button" aria-pressed>` toggle; clicking a segment selects it (only one selected at a time) and reveals a feedback form (positive/negative `<DsButton>` pair, an optional comment `<textarea>`, a submit button) that calls `adminApi.createTrainingFeedback({ message_id, chunk_id: null, answer_span: <segment text>, sentiment, comment })`; on success, clears the selection and appends the new `TrainingFeedbackResponse` to a "Feedback registrato" list rendered beneath the answer (fetched once via `adminApi.listTrainingFeedback(message.id)` on mount, then updated locally on submit rather than re-fetched, to keep the flow fast).
  - Deliverables:
    - `admin-ui/src/components/training/SpanFeedback.vue`
    - `admin-ui/src/components/training/__tests__/SpanFeedback.test.ts` (clicking a segment reveals the form; submitting positive/negative with a comment calls `createTrainingFeedback` with the exact segment text as `answer_span`; the persisted feedback list renders the new entry after submit; the honest error message renders on an `AdminApiError`)
    - `admin-ui/src/components/training/MessageList.vue` updated to render `SpanFeedback` in place of the plain answer paragraph
  - Skills to load: (none)
  - Verification: `npm run test` passes.

### Phase 5: Integration scenario and accessibility gate

Goal: The full "ask a question, see cited sources, leave negative feedback on a span, see the feedback persisted" flow is proven end to end against a mocked API, and `/training` and `/training/:id` are zero-violation on both automated gates.

- [x] **Task 5.1** — Write the Given/When/Then integration scenario for the Training flow
  - What: Implement `admin-ui/src/views/__tests__/TrainingSessionView.integration.test.ts`, mounting `TrainingSessionView.vue` with `adminApi` mocked end-to-end and structured as explicit Given/When/Then comment blocks: Given an existing open session with no messages, When the operator asks a question, Then the answer renders with its cited sources; When the operator clicks a sentence segment of the answer and submits negative feedback with a comment, Then the feedback appears in the persisted feedback list for that message.
  - Deliverables:
    - `admin-ui/src/views/__tests__/TrainingSessionView.integration.test.ts`
  - Skills to load: (none)
  - Verification: `npm run test` passes, including this scenario.

- [x] **Task 5.2** — Extend the accessibility gates to `/training` and `/training/:id`
  - What: Add both routes to the axe-core test in `admin-ui/src/__tests__/accessibility.test.ts` (mounting `TrainingSessionsView` with a populated mocked session list, and `TrainingSessionView` with a populated mocked session + message + feedback, so the real DOM — not just the loading state — is audited) and add `/training` to the `ROUTES` array in `admin-ui/scripts/run-a11y.mjs` (`/training/:id` is skipped from the static pa11y crawl since it requires a live session id and a live backend, exactly as no dynamic-id route is crawled anywhere else in this repo; its accessibility is covered by the axe-core test instead).
  - Deliverables:
    - `admin-ui/src/__tests__/accessibility.test.ts` updated
    - `admin-ui/scripts/run-a11y.mjs` updated (`ROUTES` includes `/training`)
  - Skills to load: spontini-verify-gate
  - Verification: `npm run test` (axe assertions pass for `/training` and `/training/:id`) and `npm run build && npm run a11y` report zero errors on `/`, `/dev`, `/ingest`, `/imprinting`, and `/training`; `make verify` still passes end-to-end (Rust coverage gate excepted — see Risks, a pre-existing environment gap unrelated to this feature).

## Acceptance Criteria

- Navigating to `/training` (via the now-active left-rail `Training` link) renders the session list with create and close actions.
- Navigating to `/training/:id` for an open session renders the ask/answer box and the message history.
- An operator can: create a session; ask a question and see the answer with its cited sources rendered from the `sources` DTO; ask a question that falls back and see the honest-unknown callout with no citations; select a sentence-level span of an answer and submit positive or negative feedback with an optional comment; see the feedback persisted in the message's feedback list; close a session behind an explicit confirmation dialog.
- `npm run test` is green, including the extended `adminApi` client tests, the per-component tests, and the `TrainingSessionView` Given/When/Then integration scenario.
- `npm run build && npm run a11y` reports zero errors on `/`, `/dev`, `/ingest`, `/imprinting`, and `/training`.
- `make verify` passes end-to-end with the extended admin-ui gates (Rust coverage gate excepted per the pre-existing `cargo-tarpaulin`-missing environment gap noted in feature 0017's verification).
- No direct DOM manipulation of DSI markup outside the `ds/` wrapper components; no hard-coded hex/px values in new SCSS (STACK.md §4.3).

## Risks

- **Sentence-segment span granularity is coarser than true point-in-answer character selection** — mitigation: documented as an explicit Non-Goal with rationale (jsdom `Selection`/`Range` simulation is brittle); the backend's `answer_span` column is free text with no offset/length pair, so nothing downstream assumes finer granularity.
- **`cargo-tarpaulin` is missing from the `backend` Docker image**, a pre-existing environment gap discovered during feature 0017's `make verify` run, unrelated to this admin-ui-only feature — mitigation: noted here and in the final verification report as a pre-existing, unrelated failure; out of scope to fix in a UI feature plan.
- **`npm run a11y` runs against the built static preview with no live `backend`**, so `/training` will hit a real fetch failure — mitigation: `TrainingSessionsView` must render an honest error state (not an infinite spinner or unhandled rejection) on a failed initial `listSessions`, which is itself accessible and covered by Task 5.2.
- **Closing a session is irreversible** (no reopen endpoint) — mitigation: gated behind `DsConfirmDialog` exactly like the delete flows in feature 0016 and the activate flow in feature 0017 (Task 2.2).
- **Admin key embedded in client-side JS is inherently visible to anyone with dev tools** — accepted as a known placeholder per features 0008/0016/0017's own scope note; feature 0027 replaces it with real operator auth. Not re-litigated here.

## Out-of-Scope

- Ingest and Imprinting sections (closed, features 0016, 0017).
- Operator authentication / session handling (feature 0027).
- Editing or deleting a session, message, or feedback entry (no backend endpoints).
- Character-precise text-range selection for feedback (sentence-segment granularity instead).
- Linking feedback to a specific cited chunk (`chunk_id` always submitted as `null`).
- The full cross-app WCAG audit (feature 0019).
- Any backend change — `/admin/api/training/*` is consumed exactly as built in features 0012-0014.
