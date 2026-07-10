# Plan 0008: `/admin/api/persona` — bot imprinting CRUD + reload

- **Status**: closed
- **Approved**: 2026-07-09 by Sisyphus
- **Implemented**: 2026-07-10 by Sisyphus
- **Reviewed**: 2026-07-10 by Sisyphus
- **Closed**: 2026-07-10 by Sisyphus
- **Branch**: feat/admin-api-persona-bot-imprinting-crud-reload
- **Feature ID**: 0008
- **Created**: 2026-07-09
- **Owner**: Sisyphus

## Objective

Add the admin persona surface to `backend`: endpoints to list persona versions, insert new versions with optional activation, activate a specific version, and reload the cached active persona so the next `/chat` request picks up changes immediately. This enables an operator to manage bot imprinting (tone, system prompt, fallback message) via the API, laying the foundation for the admin-ui SPA (Milestone 3). The existing `PersonaPort` is extended with an in-memory cache so the `/reload` endpoint has semantic meaning — clearing the cache forces the next RAG query to re-read from `kb.db`. All admin routes are protected by a static shared-secret header lifted from `Config::admin_api_key`. Per the [Constitution](../docs/CONSTITUTION.md) §3 (Simplicity), auth is a single shared secret — no user management, no sessions. BDD scenarios validate the full lifecycle: insert with activation, reload, version increment, and auth rejection.

## Non-Goals

- No user management or session-based auth (separate feature 0027).
- No admin-ui SPA changes (separate feature 0017).
- No changes to `kb-store` persona schema (it already supports versioned insert + activate).
- No `/admin/api/persona/:id` GET endpoint for a single persona (not needed until the admin-ui version history view).
- No `PUT` or `DELETE` on personas — the data model is versioned insert-only; deactivation is implicit via activation of another version.
- No changes to the `/chat` endpoint or the RAG flow beyond the cache addition.

## Phases

### Phase 1: Persona caching and reload support

Goal: Add an in-memory cache to `PersonaAdapter` so `reload` has semantic meaning, and extend the `PersonaPort` trait with a `reload_persona` method.

- [x] **Task 1.1** — Add `reload_persona` to `PersonaPort` trait
  - What: Add `async fn reload_persona(&self) -> Result<(), RagError>` to the `PersonaPort` trait in `backend/src/rag_engine/ports.rs`. All existing impls (test stubs + PersonaAdapter) must compile. Test stubs can implement it as a no-op.
  - Deliverables:
    - Updated `PersonaPort` trait with `reload_persona` method
    - Updated `TestPersona` test stub with no-op `reload_persona`
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo build -p backend` compiles; existing tests pass.

- [x] **Task 1.2** — Add in-memory cache to `PersonaAdapter`
  - What: Add a `tokio::sync::RwLock<Option<Persona>>` field to `PersonaAdapter`. On `active_persona()`, check the cache first; if empty, query `kb-store`, store in cache, return. On `reload_persona()`, clear the cache (set to `None`). This means the first `/chat` request after a reload re-fetches from the database.
  - Deliverables:
    - Updated `PersonaAdapter` with cache field
    - Updated `active_persona` implementation to use cache
    - `reload_persona` implementation that clears cache
  - Skills to load: spontini-tdd-rust
  - Verification: `cargo test -p backend` passes; new unit tests for cache-hit, cache-miss, and reload behaviors.

### Phase 2: `PersonaAdminPort` trait and `kb-store` adapter

Goal: Define an admin port for persona CRUD and implement it against `kb-store`.

- [x] **Task 2.1** — Define `PersonaAdminPort` trait
  - What: In `backend/src/rag_engine/ports.rs`, define a new `#[async_trait] pub trait PersonaAdminPort: Send + Sync` with methods: `async fn list_versions(&self, name: &str) -> Result<Vec<Persona>, RagError>`, `async fn insert_persona(&self, persona: NewPersona, activate: bool) -> Result<Persona, RagError>`, `async fn activate_persona(&self, id: i64) -> Result<(), RagError>`. These delegate to the existing `kb-store` methods, converting `KbStoreError` to `RagError`.
  - Deliverables:
    - `PersonaAdminPort` trait definition in `ports.rs`
    - Public types `Persona` and `NewPersona` re-exported from `rag_engine` (or accessible via `kb-store`)
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo build -p backend` compiles.

- [x] **Task 2.2** — Implement `PersonaAdminAdapter`
  - What: Create `backend/src/rag_engine/persona_admin.rs` with a `PersonaAdminAdapter` struct holding `Arc<KbStore>`. Implement `PersonaAdminPort` for it, delegating to `kb-store` methods. The adapter also holds an `Arc<dyn PersonaPort>` so that `insert_persona` with `activate=true` can call `reload_persona()` to flush the cache, ensuring a subsequent `/chat` sees the newly-activated persona.
  - Deliverables:
    - `backend/src/rag_engine/persona_admin.rs` module with `PersonaAdminAdapter`
    - `PersonaAdminPort` implementation
    - Unit tests using a temp `kb.db` covering: insert with activate triggers reload, list versions returns versioned rows, activate switches active persona
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo test -p backend` passes with admin adapter tests.

### Phase 3: Axum routes and integration wiring

Goal: Expose the admin persona endpoints via axum under `/admin/api/persona/`, wired into `AppState`.

- [x] **Task 3.1** — Add admin persona route handlers
  - What: In `backend/src/routes.rs` (or a new `admin.rs` module), add handlers: `GET /admin/api/persona?name=<name>` → list versions, `POST /admin/api/persona` → insert (body: `{ name, system_prompt, tone?, fallback_message?, activate: bool }`), `POST /admin/api/persona/:id/activate`, `POST /admin/api/persona/reload`. All handlers check for an `X-Admin-Key` header matching `Config::admin_api_key`; if missing or wrong, return 401.
  - Deliverables:
    - Route handlers for all 4 endpoints
    - Auth check helper (header validation against config)
    - Request/response DTOs with serde Serialize/Deserialize
  - Skills to load: spontini-tdd-rust, spontini-bdd-gherkin
  - Verification: `cargo build -p backend` compiles; unit tests for auth rejection, valid insert, activate, reload.

- [x] **Task 3.2** — Wire admin routes into the router and `AppState`
  - What: In `backend/src/lib.rs`, add `persona_admin: Arc<dyn PersonaAdminPort>` to `AppState`. In `router()`, construct `PersonaAdminAdapter` and pass it. Add axum `route()` calls for the 4 admin persona endpoints. The `Config` struct gains an `admin_api_key: String` field (loaded from `ADMIN_API_KEY` env var, default `"admin"` for development).
  - Deliverables:
    - Updated `AppState` with `persona_admin` field
    - Updated `Config` with `admin_api_key`
    - Updated `router()` wiring
  - Skills to load: spontini-tdd-rust, spontini-clean-arch-guard
  - Verification: `cargo build -p backend` compiles; `cargo test -p backend` passes.

### Phase 4: BDD scenarios

Goal: Add Gherkin scenarios for the admin persona lifecycle.

- [x] **Task 4.1** — Write BDD steps and scenarios for persona admin
  - What: Add BDD scenarios to the existing `backend/tests/bdd.rs`: (1) insert deactivates previous active when `activate=true`; (2) reload picks up a newly-activated persona (`ChatWorld` inserts a persona without activating, calls reload, then verifies chat falls back to the newly-activated persona); (3) version increments within a name. Each scenario uses the `ChatWorld` pattern already established, extended with admin persona endpoints via `reqwest` calls to the test server.
  - Deliverables:
    - BDD scenarios for insert-with-activate, reload, version-increment
    - Wired step definitions reusing existing `ChatWorld` infrastructure
  - Skills to load: spontini-tdd-rust, spontini-bdd-gherkin
  - Verification: `cargo test -p backend --test bdd -- --nocapture` passes with new scenarios green.

## Acceptance Criteria

- `GET /admin/api/persona?name=<name>` returns a JSON array of persona versions ordered by version descending.
- `POST /admin/api/persona` with `{ name, system_prompt, activate: true }` inserts a new persona version, deactivates any previously active persona, and flushes the cache so the next `/chat` sees it.
- `POST /admin/api/persona/:id/activate` activates the given persona version and deactivates all others.
- `POST /admin/api/persona/reload` clears the in-memory cache.
- All endpoints return 401 when `X-Admin-Key` header is missing or wrong.
- All existing tests in the workspace (`cargo test --workspace`) remain green.
- BDD scenarios cover insert-with-activate, reload, and version-increment.

## Risks

- **Cache staleness** — The in-memory cache means a reload is required for the operator's changes to take effect. Mitigation: the reload endpoint is explicitly part of the feature; the admin-ui (future) will call it automatically after every persona write.
- **No persistent sessions** — The shared-secret auth is minimal. Mitigation: documented as temporary; feature 0027 will add proper operator auth.
- **PersonaAdminPort owns a reference to PersonaPort** — The admin adapter must know about the persona port to call `reload_persona()` on insert/activate. This creates a circular dependency risk. Mitigation: `PersonaAdminAdapter` takes `Arc<dyn PersonaPort>` and `Arc<KbStore>` separately; the persona port is shared via `Arc` from the outer wiring, not duplicated.

## Out-of-Scope

- No admin-ui SPA changes.
- No user management or sessions.
- No PUT/DELETE on personas.
- No GET single-persona endpoint (list only).
- No changes to `kb-store` schema.
