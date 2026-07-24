# Plan 0016: admin-ui Ingest configuration section

- **Status**: review
- **Approved**: 2026-07-24 by agent
- **Implemented**: 2026-07-24 by agent
- **Branch**: feat/admin-ui-ingest-configuration-section
- **Feature ID**: 0016
- **Created**: 2026-07-24
- **Owner**: agent

## Objective

Milestone 3's operator console gets its first business section. This feature builds the **Ingest configuration** section of `admin-ui` as a first-class route, turning the design-system foundation laid by feature 0015 into a working screen an operator can actually use, per the [Constitution](../docs/CONSTITUTION.md)'s mandate that operators drive the whole Spontini system from a browser. The section surfaces the ingest schedule (cron expression, enabled toggle), the configured sections (sport/news/delibere/storia) with their per-section scrape sources (API sources shown greyed-out with a "Coming soon" tooltip per the `coming_soon` flag from feature 0010), a manual "run now" trigger with status polling (feature 0011), and a per-section manual-upload dropzone that drives the preview → confirm workflow from feature 0009. Every call goes through a new typed HTTP client wired to the existing `/admin/api/ingest/config`, `/admin/api/ingest/run`, and `/admin/api/upload` endpoints — no backend changes are needed, this is a pure `admin-ui` feature. In scope: the Ingest route, its sub-views (schedule, sections/sources, run trigger + status, per-section upload), the left-rail `Ingest` link activation, and destructive-action confirmation dialogs. Out of scope: the Imprinting and Training sections (features 0017/0018), operator authentication (feature 0027 — the admin key is read from a build-time env var with the same `dev-key` default the backend uses), and the full cross-app accessibility audit (feature 0019, which re-audits every section once they all exist — this feature only keeps the zero-violation gate green for what it adds).

## Non-Goals

- No Imprinting or Training screens — only the Ingest section is built.
- No operator login/session UI — the admin key is a `VITE_ADMIN_API_KEY` build-time env var (default `dev-key`, matching `backend`'s `Config::from_env` default), attached as `X-Admin-Key` to every request. Feature 0027 replaces this with real auth.
- No backend changes — `/admin/api/ingest/config`, `/admin/api/ingest/run`, and `/admin/api/upload` already exist (features 0009-0011) and are consumed as-is.
- No editing of an existing section's name/ordering or an existing source's URL — the backend ports only support create + delete for sections/sources (per feature 0004/0010's `KbStore` API), so the UI offers add + delete, not in-place edit.
- No dedicated Gherkin `.feature` file — this repo's cucumber-based BDD (`spontini-bdd-gherkin`) covers backend Rust crates only; `admin-ui` has no cucumber runner. The roadmap's "BDD scenario" is satisfied by a Vitest integration test structured with explicit Given/When/Then steps, consistent with how feature 0015 tested `admin-ui`.

## Phases

### Phase 1: Admin API client and dev-mode wiring

Goal: `admin-ui` can call every `/admin/api/ingest/*` and `/admin/api/upload/*` endpoint, both under `npm run dev` (vite dev server) and in the built container (nginx already proxies `/admin/api/` to `backend:8080`, per `admin-ui/nginx.conf`).

- [x] **Task 1.1** — Add a vite dev-server proxy and the admin-key env var
  - What: Add a `server.proxy` entry in `admin-ui/vite.config.ts` forwarding `/admin/api` to `http://localhost:8080` (or `VITE_BACKEND_URL` if set) so `npm run dev` works against a locally running `backend`; add `admin-ui/.env.example` documenting `VITE_ADMIN_API_KEY` (default falls back to `dev-key` in code, matching `backend/src/config.rs`'s `ADMIN_API_KEY` default).
  - Deliverables:
    - `admin-ui/vite.config.ts` updated with `server.proxy`
    - `admin-ui/.env.example`
  - Skills to load: (none)
  - Verification: `npm run dev` with `backend` running on `:8080` successfully proxies a manual `fetch('/admin/api/ingest/config')` (checked via the Task 1.2 client's tests, not manually).

- [x] **Task 1.2** — Build the typed admin API client
  - What: Implement `admin-ui/src/services/adminApi.ts` exporting typed functions over `fetch`: `getIngestConfig`, `upsertSchedule`, `createSection`, `deleteSection`, `createSource`, `deleteSource`, `triggerIngestRun`, `getIngestRun`, `uploadDocument`, `getUploadPreview`, `confirmUpload`. TypeScript interfaces mirror the backend DTOs verbatim (`IngestConfigResponse`, `IngestScheduleResponse`, `IngestSectionResponse`, `IngestSourceResponse`, `IngestRunResponse`, `UploadResponse`, `PreviewResponse`, `ConfirmResponse` from `backend/src/admin/{ingest_config,ingest_run,upload}/{mod,handlers}.rs`). Every call sets `X-Admin-Key` from `import.meta.env.VITE_ADMIN_API_KEY ?? 'dev-key'` and throws a typed `AdminApiError` (with `status` and parsed `error` message) on a non-2xx response.
  - Deliverables:
    - `admin-ui/src/services/adminApi.ts`
    - `admin-ui/src/services/__tests__/adminApi.test.ts` (mocks `global.fetch`, one test per function covering the success path and the non-2xx `AdminApiError` path)
  - Skills to load: (none)
  - Verification: `npm run test` passes; every exported function has a covering test.

### Phase 2: Ingest configuration view — schedule, sections, sources

Goal: The `/ingest` route renders the schedule editor and the section/source tree, wired to the API client, with the left-rail `Ingest` link activated.

- [x] **Task 2.1** — Scaffold the `/ingest` route and activate the left-rail link
  - What: Add `admin-ui/src/views/IngestView.vue` (initially rendering a loading state that fetches `getIngestConfig` on mount), register it at `/ingest` in `admin-ui/src/router/index.ts`, and change `admin-ui/src/App.vue`'s `businessLinks` so the `Ingest` entry has `to: '/ingest'` instead of being a disabled placeholder.
  - Deliverables:
    - `admin-ui/src/views/IngestView.vue`
    - `admin-ui/src/router/index.ts` updated
    - `admin-ui/src/App.vue` updated (`Ingest` link active; `Imprinting`/`Training` remain placeholders)
  - Skills to load: (none)
  - Verification: `npm run test` — a mounting test for `IngestView.vue` (mocking `adminApi.getIngestConfig`) asserts the loading state renders then resolves; `npm run dev`, navigate via the left rail to `/ingest`, confirm it loads.

- [x] **Task 2.2** — Build the schedule editor sub-section
  - What: Implement `admin-ui/src/components/ingest/ScheduleEditor.vue` — a form with a `<DsInput>` for the cron expression, an enabled toggle, and a `<DsButton>` "Salva" that calls `adminApi.upsertSchedule` and re-renders the confirmed value; a `?` inline-help tooltip explains cron syntax (per STACK.md §4.5 progressive disclosure).
  - Deliverables:
    - `admin-ui/src/components/ingest/ScheduleEditor.vue`
    - `admin-ui/src/components/ingest/__tests__/ScheduleEditor.test.ts` (renders existing schedule, submits a change, asserts `upsertSchedule` called with the right payload)
  - Skills to load: (none)
  - Verification: `npm run test` passes.

- [x] **Task 2.3** — Build a reusable destructive-action confirmation dialog
  - What: Implement `admin-ui/src/components/ds/DsConfirmDialog.vue`, a thin wrapper around DSI's modal markup (native `<dialog>` element, DSI modal classes) taking `message`, `confirmLabel` props and emitting `confirm`/`cancel`; export it from `admin-ui/src/components/ds/index.ts`. This is the shared confirmation used by every delete action in this feature (per STACK.md §4.5 "destructive actions behind an explicit confirmation").
  - Deliverables:
    - `admin-ui/src/components/ds/DsConfirmDialog.vue`
    - `admin-ui/src/components/ds/__tests__/DsConfirmDialog.test.ts`
  - Skills to load: (none)
  - Verification: `npm run test` passes (dialog renders when open, emits `confirm`/`cancel`, traps focus while open).

- [x] **Task 2.4** — Build the section list with add/delete
  - What: Implement `admin-ui/src/components/ingest/SectionList.vue` rendering each configured section (name, ordering) with a delete button (behind `DsConfirmDialog`, since deleting a section cascades its sources per the backend) and an "add section" inline form (name + ordering, calls `adminApi.createSection`). Compose it into `IngestView.vue`.
  - Deliverables:
    - `admin-ui/src/components/ingest/SectionList.vue`
    - `admin-ui/src/components/ingest/__tests__/SectionList.test.ts` (add flow calls `createSection`; delete flow opens the confirm dialog then calls `deleteSection` on confirm, not before)
    - `admin-ui/src/views/IngestView.vue` updated to render `SectionList`
  - Skills to load: (none)
  - Verification: `npm run test` passes.

- [x] **Task 2.5** — Build the per-section source list with add/delete
  - What: Implement `admin-ui/src/components/ingest/SourceList.vue`, nested under each section in `SectionList.vue`: lists sources (URL, enabled state), renders `api`-type sources greyed-out with a "Prossimamente" (`title` attribute) tooltip when `coming_soon` is true, an "add scrape source" inline form (URL, calls `adminApi.createSource` with `source_type: 'scrape'`), and a delete button behind `DsConfirmDialog`.
  - Deliverables:
    - `admin-ui/src/components/ingest/SourceList.vue`
    - `admin-ui/src/components/ingest/__tests__/SourceList.test.ts` (add scrape source calls `createSource`; a `coming_soon` source row is disabled/greyed with a tooltip; delete flow requires confirmation)
    - `admin-ui/src/components/ingest/SectionList.vue` updated to render `SourceList` per section
  - Skills to load: (none)
  - Verification: `npm run test` passes.

### Phase 3: Manual per-section upload dropzone

Goal: Each section in the Ingest view has a manual-upload control that drives the feature 0009 upload → preview → confirm flow end to end.

- [x] **Task 3.1** — Build the upload dropzone with preview/confirm
  - What: Implement `admin-ui/src/components/ingest/UploadDropzone.vue`, rendered per section in `SectionList.vue`: a file input (`pdf`/`docx`/`md`/`txt` accept filter) plus a metadata form (category, tags, trust_score), calling `adminApi.uploadDocument` on submit; on success, fetch and render the preview (`getUploadPreview`, showing extracted text truncated + metadata) with "Conferma" / "Annulla" actions; "Conferma" calls `confirmUpload` and shows the resulting chunk count, "Annulla" discards the pending token client-side (the preview entry simply expires server-side, per feature 0009 — no cancel endpoint exists).
  - Deliverables:
    - `admin-ui/src/components/ingest/UploadDropzone.vue`
    - `admin-ui/src/components/ingest/__tests__/UploadDropzone.test.ts` (upload → preview render → confirm calls `confirmUpload` and shows chunk count; error path shows the honest error message from `AdminApiError`)
    - `admin-ui/src/components/ingest/SectionList.vue` updated to render `UploadDropzone` per section
  - Skills to load: (none)
  - Verification: `npm run test` passes.

### Phase 4: Trigger-run and status polling

Goal: The operator can trigger an out-of-schedule ingest run and watch it move from pending to done/failed without leaving the page.

- [x] **Task 4.1** — Build the run trigger and status poller
  - What: Implement `admin-ui/src/components/ingest/RunTrigger.vue` — a `<DsButton>` "Esegui ora" calling `adminApi.triggerIngestRun`; on the returned `id`, poll `adminApi.getIngestRun(id)` on a fixed interval (e.g. every 2s via `setInterval`, cleared on unmount or on reaching `done`/`failed`) and render the current status (`pending` → `running` → `done`/`failed`) with a `<DsCallout>` variant matching the outcome. Compose it at the top of `IngestView.vue`.
  - Deliverables:
    - `admin-ui/src/components/ingest/RunTrigger.vue`
    - `admin-ui/src/components/ingest/__tests__/RunTrigger.test.ts` (using fake timers: trigger → pending render → advance timers → poll called → done render; interval cleared on unmount)
    - `admin-ui/src/views/IngestView.vue` updated to render `RunTrigger`
  - Skills to load: (none)
  - Verification: `npm run test` passes.

### Phase 5: Integration scenario and accessibility gate

Goal: The full "add a section, add a scraper source, trigger a run, see the run status" flow is proven end to end against a mocked API, and the `/ingest` route is zero-violation on both automated gates.

- [x] **Task 5.1** — Write the Given/When/Then integration scenario for `IngestView`
  - What: Implement `admin-ui/src/views/__tests__/IngestView.integration.test.ts`, mounting `IngestView.vue` with `adminApi` mocked end-to-end and structured as explicit Given/When/Then comment blocks: Given an empty ingest configuration, When the operator adds a section "news" and a scrape source, Then they appear in the list; When the operator triggers a run, Then the status renders pending then done (via fake-timer-driven polling).
  - Deliverables:
    - `admin-ui/src/views/__tests__/IngestView.integration.test.ts`
  - Skills to load: (none)
  - Verification: `npm run test` passes, including this scenario.

- [x] **Task 5.2** — Extend the accessibility gates to `/ingest`
  - What: Add `/ingest` to the axe-core test in `admin-ui/src/__tests__/accessibility.test.ts` (mounting `IngestView` with a populated mocked config so the real DOM — not just the loading state — is audited) and to the `ROUTES` array in `admin-ui/scripts/run-a11y.mjs`.
  - Deliverables:
    - `admin-ui/src/__tests__/accessibility.test.ts` updated
    - `admin-ui/scripts/run-a11y.mjs` updated (`ROUTES` includes `/ingest`)
  - Skills to load: spontini-verify-gate
  - Verification: `npm run test` (axe assertions pass for `/ingest`) and `npm run build && npm run a11y` report zero errors on `/`, `/dev`, and `/ingest`; `make verify` still passes end-to-end.

## Acceptance Criteria

- Navigating to `/ingest` (via the now-active left-rail `Ingest` link) renders the schedule editor, the section/source tree, the run trigger, and a per-section upload dropzone.
- An operator can: set the schedule; add a section; add a scrape source to it; see an `api`-type source rendered disabled with a "coming soon" indicator; delete a source and a section (each behind an explicit confirmation dialog); upload a document through preview → confirm and see the resulting chunk count; trigger a run and watch its status move to `done`.
- `npm run test` is green, including the `adminApi` client tests, the per-component tests, and the `IngestView` Given/When/Then integration scenario.
- `npm run build && npm run a11y` reports zero errors on `/`, `/dev`, and `/ingest`.
- `make verify` passes end-to-end with the extended admin-ui gates.
- No direct DOM manipulation of DSI markup outside the `ds/` wrapper components; no hard-coded hex/px values in new SCSS (STACK.md §4.3).

## Risks

- **No cancel/delete endpoint for a pending upload preview token** — mitigation: "Annulla" is a client-side no-op (the token simply isn't confirmed and expires server-side per feature 0009's `PreviewStore`); documented in Task 3.1 so it isn't mistaken for a missing feature.
- **Polling a run to `done` under `npm run test` requires fake timers** — mitigation: use Vitest's `vi.useFakeTimers()` and explicit `vi.advanceTimersByTime()` in Task 4.1/5.1 tests rather than real `setTimeout` delays, keeping the suite fast and deterministic.
- **`npm run a11y` runs against the built static preview with no live `backend`**, so `/ingest` will hit a real fetch failure — mitigation: `IngestView` must render an honest error state (not an infinite spinner or unhandled rejection) on a failed initial `getIngestConfig`, which is itself accessible and covered by Task 5.2.
- **Admin key embedded in client-side JS is inherently visible to anyone with dev tools** — accepted as a known placeholder per feature 0008's own scope note; feature 0027 replaces it with real operator auth. Not re-litigated here.

## Out-of-Scope

- Imprinting and Training sections (features 0017, 0018).
- Operator authentication / session handling (feature 0027).
- Editing an existing section's name/ordering or an existing source's URL (backend has no update endpoint for these).
- The full cross-app WCAG audit (feature 0019).
- Any backend change — `/admin/api/ingest/*` and `/admin/api/upload/*` are consumed exactly as built in features 0009-0011.
