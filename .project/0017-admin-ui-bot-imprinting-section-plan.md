# Plan 0017: admin-ui Bot imprinting section

- **Status**: review
- **Approved**: 2026-07-24 by agent
- **Implemented**: 2026-07-24 by agent
- **Branch**: feat/admin-ui-bot-imprinting-section
- **Feature ID**: 0017
- **Created**: 2026-07-24
- **Owner**: agent

## Objective

Milestone 3's operator console gets its second business section. This feature builds the **Bot imprinting** section of `admin-ui` — the screen where an operator shapes Spontini's identity (per the [Constitution](../docs/CONSTITUTION.md) §2: Spontini embodies Gaspare Spontini, and speaks with a voice defined by `system_prompt`/`tone`/`fallback_message`). The section lets the operator view the currently active persona, edit its fields in a form, save the edit as a new immutable version (the backend never updates in place — feature 0008's `POST /admin/api/persona` always inserts), browse the version history for the persona, activate any prior version (making it the one `/chat` and training sessions use), and force a cache reload so a freshly-activated persona takes effect immediately. Every call goes through a new typed function set added to the existing `admin-ui/src/services/adminApi.ts` client (built in feature 0016), talking to the already-existing `/admin/api/persona`, `/admin/api/persona/:id/activate`, and `/admin/api/persona/reload` endpoints (feature 0008) — no backend changes. Because `GET /admin/api/persona` is scoped by a `name` query parameter and the persona table has no "list distinct names" or "get active regardless of name" endpoint, and because Spontini is a single-persona system by design (Constitution §2, §3 Simplicity), the UI targets one well-known persona name read from a build-time `VITE_PERSONA_NAME` env var (default `gaspare`, matching the identity established in the Constitution and used consistently across the existing backend test fixtures). In scope: the Imprinting route, the active-persona edit form, save-as-new-version, version history with per-version activate (behind a confirmation dialog, since activating changes what citizens see live), the reload action, and the left-rail `Imprinting` link activation. Out of scope: the Ingest section (closed, feature 0016) and the Training section (feature 0018), operator authentication (feature 0027 — same `VITE_ADMIN_API_KEY` placeholder as feature 0016), deleting a persona version (no backend endpoint exists — the roadmap's "delete a draft" wording does not map to any real capability since every insert is an immutable version, not a draft), and the full cross-app accessibility audit (feature 0019).

## Non-Goals

- No Ingest or Training screens — only the Imprinting section is built.
- No operator login/session UI — reuses the `VITE_ADMIN_API_KEY` build-time env var and `X-Admin-Key` header pattern from feature 0016. Feature 0027 replaces this with real auth.
- No backend changes — `/admin/api/persona`, `/admin/api/persona/:id/activate`, and `/admin/api/persona/reload` already exist (feature 0008) and are consumed as-is.
- No delete/discard of a persona version — the backend has no delete endpoint (`insert_persona` is append-only, per feature 0008's design). The UI offers save-new-version and activate, never delete.
- No multi-persona name switcher — the UI manages exactly one persona name (`VITE_PERSONA_NAME`, default `gaspare`), consistent with the Constitution's single-bot-identity scope. A future feature can add a name picker if the system ever needs multiple personas.
- No dedicated Gherkin `.feature` file — `admin-ui` has no cucumber runner (established in feature 0015/0016); the roadmap's "BDD scenario" is satisfied by a Vitest integration test with explicit Given/When/Then structure.

## Phases

### Phase 1: Admin API client extension

Goal: `admin-ui/src/services/adminApi.ts` can call every `/admin/api/persona*` endpoint.

- [x] **Task 1.1** — Extend the typed admin API client with persona functions
  - What: Add `getPersonaVersions(name: string)`, `createPersona(payload)`, `activatePersona(id: number)`, `reloadPersona()` to `admin-ui/src/services/adminApi.ts`, with TypeScript interfaces (`PersonaResponse`, `CreatePersonaRequest`) mirroring `backend/src/admin/mod.rs`'s `PersonaResponse`/`CreatePersonaRequest` verbatim; reuse the existing `AdminApiError` and `X-Admin-Key` request helper already in the file.
  - Deliverables:
    - `admin-ui/src/services/adminApi.ts` updated
    - `admin-ui/src/services/__tests__/adminApi.test.ts` updated (one test per new function covering the success path and the non-2xx `AdminApiError` path)
  - Skills to load: (none)
  - Verification: `npm run test` passes; every new exported function has a covering test.

### Phase 2: Imprinting route and active-persona view

Goal: The `/imprinting` route renders the active persona (or an honest empty state on first run) and the left-rail `Imprinting` link is activated.

- [x] **Task 2.1** — Scaffold the `/imprinting` route and activate the left-rail link
  - What: Add `admin-ui/src/views/ImprintingView.vue` (fetches `getPersonaVersions(personaName)` on mount, where `personaName` comes from `import.meta.env.VITE_PERSONA_NAME ?? 'gaspare'`, and renders a loading state then either the persona editor or an empty-state callout when the list is empty), register it at `/imprinting` in `admin-ui/src/router/index.ts`, and change `admin-ui/src/App.vue`'s `businessLinks` so the `Imprinting` entry has `to: '/imprinting'` instead of being a disabled placeholder.
  - Deliverables:
    - `admin-ui/src/views/ImprintingView.vue`
    - `admin-ui/src/router/index.ts` updated
    - `admin-ui/src/App.vue` updated (`Imprinting` link active; `Training` remains a placeholder)
    - `admin-ui/.env.example` updated with `VITE_PERSONA_NAME` (default documented as `gaspare`)
  - Skills to load: (none)
  - Verification: `npm run test` — a mounting test for `ImprintingView.vue` (mocking `adminApi.getPersonaVersions`) asserts the loading state renders then resolves to either the editor or the empty state; `npm run dev`, navigate via the left rail to `/imprinting`, confirm it loads.

- [x] **Task 2.2** — Build the persona edit form with save-as-new-version
  - What: Implement `admin-ui/src/components/imprinting/PersonaEditor.vue` — a form pre-filled from the current active version (or blank fields when none exists yet) with `<DsInput>`/textarea fields for `name` (editable only when creating the very first version; read-only afterwards, since the UI manages a single fixed persona name), `system_prompt`, `tone`, `fallback_message`, and an `activate` checkbox defaulted to checked; a `<DsButton>` "Salva nuova versione" calls `adminApi.createPersona` and, on success, refreshes the version list and shows a `<DsCallout>` success message.
  - Deliverables:
    - `admin-ui/src/components/imprinting/PersonaEditor.vue`
    - `admin-ui/src/components/imprinting/__tests__/PersonaEditor.test.ts` (renders prefilled from the active version; empty-state renders blank editable fields; submit calls `createPersona` with the right payload and shows the success callout; error path shows the honest error message from `AdminApiError`)
    - `admin-ui/src/views/ImprintingView.vue` updated to render `PersonaEditor`
  - Skills to load: (none)
  - Verification: `npm run test` passes.

### Phase 3: Version history with activate, and reload

Goal: The operator can browse every persona version and activate a prior one behind confirmation, and force a cache reload.

- [x] **Task 3.1** — Build the version history list with per-version activate
  - What: Implement `admin-ui/src/components/imprinting/VersionHistory.vue` rendering every version returned by `getPersonaVersions` (version number, `created_at`, `created_by`, truncated `system_prompt` preview, an "Attiva" badge on the currently active row) with an "Attiva questa versione" button on every non-active row, gated behind the existing `DsConfirmDialog` (from feature 0016's `admin-ui/src/components/ds/`) since activating an old version immediately changes what `/chat` and training sessions serve to end users; on confirm, calls `adminApi.activatePersona(id)` and refreshes the list.
  - Deliverables:
    - `admin-ui/src/components/imprinting/VersionHistory.vue`
    - `admin-ui/src/components/imprinting/__tests__/VersionHistory.test.ts` (renders all versions with the active one badged; activate button opens the confirm dialog then calls `activatePersona` on confirm, not before; list refresh after activation)
    - `admin-ui/src/views/ImprintingView.vue` updated to render `VersionHistory`
  - Skills to load: (none)
  - Verification: `npm run test` passes.

- [x] **Task 3.2** — Build the reload-active-persona action
  - What: Implement `admin-ui/src/components/imprinting/ReloadPersonaButton.vue` — a `<DsButton>` "Ricarica persona attiva" calling `adminApi.reloadPersona()` and showing a `<DsCallout>` confirming the cache was cleared (or the honest error message on failure); compose it at the top of `ImprintingView.vue`.
  - Deliverables:
    - `admin-ui/src/components/imprinting/ReloadPersonaButton.vue`
    - `admin-ui/src/components/imprinting/__tests__/ReloadPersonaButton.test.ts` (click calls `reloadPersona`; success and error states render distinct callouts)
    - `admin-ui/src/views/ImprintingView.vue` updated to render `ReloadPersonaButton`
  - Skills to load: (none)
  - Verification: `npm run test` passes.

### Phase 4: Integration scenario and accessibility gate

Goal: The full "save a new version, see it in history, activate a previous version, reload" flow is proven end to end against a mocked API, and `/imprinting` is zero-violation on both automated gates.

- [x] **Task 4.1** — Write the Given/When/Then integration scenario for `ImprintingView`
  - What: Implement `admin-ui/src/views/__tests__/ImprintingView.integration.test.ts`, mounting `ImprintingView.vue` with `adminApi` mocked end-to-end and structured as explicit Given/When/Then comment blocks: Given an existing active persona version 1, When the operator edits the form and saves as a new version 2, Then version 2 appears in history and is badged active; When the operator activates version 1 (confirming the dialog), Then version 1 is badged active and version 2 is not; When the operator clicks reload, Then a success callout renders.
  - Deliverables:
    - `admin-ui/src/views/__tests__/ImprintingView.integration.test.ts`
  - Skills to load: (none)
  - Verification: `npm run test` passes, including this scenario.

- [x] **Task 4.2** — Extend the accessibility gates to `/imprinting`
  - What: Add `/imprinting` to the axe-core test in `admin-ui/src/__tests__/accessibility.test.ts` (mounting `ImprintingView` with a populated mocked version list so the real DOM — not just the loading state — is audited) and to the `ROUTES` array in `admin-ui/scripts/run-a11y.mjs`.
  - Deliverables:
    - `admin-ui/src/__tests__/accessibility.test.ts` updated
    - `admin-ui/scripts/run-a11y.mjs` updated (`ROUTES` includes `/imprinting`)
  - Skills to load: spontini-verify-gate
  - Verification: `npm run test` (axe assertions pass for `/imprinting`) and `npm run build && npm run a11y` report zero errors on `/`, `/dev`, `/ingest`, and `/imprinting`; `make verify` still passes end-to-end.

## Acceptance Criteria

- Navigating to `/imprinting` (via the now-active left-rail `Imprinting` link) renders the reload action, the active-persona edit form (or an honest empty state on first run), and the version history list.
- An operator can: edit the persona fields and save as a new version (which becomes active when the activate checkbox is checked); see the new version in the history list, badged active; activate a previous version behind an explicit confirmation dialog and see the badge move; force a reload and see a confirming callout.
- `npm run test` is green, including the extended `adminApi` client tests, the per-component tests, and the `ImprintingView` Given/When/Then integration scenario.
- `npm run build && npm run a11y` reports zero errors on `/`, `/dev`, `/ingest`, and `/imprinting`.
- `make verify` passes end-to-end with the extended admin-ui gates.
- No direct DOM manipulation of DSI markup outside the `ds/` wrapper components; no hard-coded hex/px values in new SCSS (STACK.md §4.3).

## Risks

- **No backend endpoint to fetch "the active persona regardless of name"** — mitigation: the UI fixes on one persona name via `VITE_PERSONA_NAME` (default `gaspare`), consistent with the Constitution's single-bot-identity scope; documented in the Objective and Non-Goals so it isn't mistaken for an oversight.
- **First-run state: the persona table is empty** (no seed data exists in any migration) — mitigation: `PersonaEditor` renders an editable blank form in this case (Task 2.2) and `ImprintingView` shows an honest empty-state callout instead of an error (Task 2.1).
- **Activating a version is effectively destructive to the live citizen-facing experience** even though no data is deleted — mitigation: gated behind `DsConfirmDialog` exactly like the delete flows in feature 0016 (Task 3.1).
- **`npm run a11y` runs against the built static preview with no live `backend`**, so `/imprinting` will hit a real fetch failure — mitigation: `ImprintingView` must render an honest error state (not an infinite spinner or unhandled rejection) on a failed initial `getPersonaVersions`, which is itself accessible and covered by Task 4.2.
- **Admin key embedded in client-side JS is inherently visible to anyone with dev tools** — accepted as a known placeholder per feature 0008/0016's own scope note; feature 0027 replaces it with real operator auth. Not re-litigated here.

## Out-of-Scope

- Ingest section (closed, feature 0016) and Training section (feature 0018).
- Operator authentication / session handling (feature 0027).
- Deleting a persona version (no backend endpoint — every insert is an immutable version, not a draft).
- Managing more than one persona name (no name picker; `VITE_PERSONA_NAME` is fixed at build time).
- The full cross-app WCAG audit (feature 0019).
- Any backend change — `/admin/api/persona*` is consumed exactly as built in feature 0008.
