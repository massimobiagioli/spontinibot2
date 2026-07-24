# ADR 0008: Preview/Confirm Upload Workflow with In-Memory TTL Token Store

- **Status**: proposed
- **Date**: 2026-07-24
- **Deciders**: Sisyphus
- **Related**: Feature 0009

## Context

Feature 0009 adds a manual-upload surface (`/admin/api/upload`) so an operator can index municipal documents that are not reachable by the automated scraper (feature 0005). Unlike the scraper path — which the Constitution already trusts to index whatever it fetches — a manual upload is fed directly by an operator's file, and [Constitution](../docs/CONSTITUTION.md) §5 (truthfulness and source citation) requires that the operator never index content they have not actually seen: extraction quality varies across PDF/DOCX/Markdown/plain-text, and a bad extraction (garbled text, truncated tables) silently indexed would let Spontini cite a citizen-facing "source" that misrepresents the original document.

This forces a two-step interaction: extract and show the operator what will be indexed, then index only on explicit confirmation. The state produced by the first step (the extracted text, its section, and its metadata) has to survive between the two HTTP requests. Two mechanisms were viable: persist it in `kb.db` via `kb-store`, or hold it in server memory. `kb-store` is the system's only persistence boundary today (Constitution — single source of truth for indexed content) and every other admin table is a durable, audited record; a preview that the operator abandons is neither durable nor a record — it is scratch state with no value once it expires or is confirmed.

## Decision

We will hold preview state in an in-process `PreviewStore` (a `DashMap<String, PreviewEntry>` keyed by a random 32-character token) with a 15-minute TTL and a background eviction task, instead of writing preview rows to `kb.db`. `POST /admin/api/upload` extracts text via a format-agnostic `TextExtractor` port (`PdfExtractor`/`DocxExtractor`/`MarkdownExtractor`/`PlainTextExtractor` behind a `CompositeExtractor` dispatcher) and returns a token; `GET /admin/api/upload/preview/:token` reads it back for operator review; `POST /admin/api/upload/confirm/:token` consumes the token and is the only path that writes to `kb.db`, via the existing `ingest-core` pipeline through a new `UploadPort`.

## Rationale

An in-memory store keeps unconfirmed, potentially-garbled extractions out of the durable knowledge base entirely — there is no code path by which a preview row could accidentally become searchable, which is the strongest possible enforcement of "never index unseen content" (Constitution §5). It also avoids adding a `V*__upload_preview.sql` migration and a cleanup job for state that is deliberately short-lived and disposable, keeping `kb-store`'s schema reserved for durable, audited data (Constitution §6 — prefer the simplest mechanism that satisfies the constraint). The single-operator, single-process deployment target (STACK.md — Mac Intel i7 / 16 GB RAM, one `backend` container) means the in-memory store's one real cost — state lost on restart — is an acceptable trade for a 15-minute-lived token the operator is actively waiting on.

## Consequences

### Positive

- Unconfirmed extractions can never leak into `kb.db` — there is no write path from `PreviewStore` other than through `confirm`, which re-runs the full ingest pipeline.
- No schema migration, no audited row, no cleanup query for state that has no value after 15 minutes.
- `TextExtractor` is a clean, testable port independent of the token-store mechanism — swapping formats or extraction libraries never touches Phase 2.

### Negative

- A `backend` restart (deploy, crash, `SIGTERM`) silently drops all in-flight previews; an operator mid-upload must re-upload with no server-side trace of the loss.
- The store does not scale beyond a single `backend` process — a future multi-instance deployment would need a shared store (Redis, or a `kb.db` table with TTL cleanup) to keep tokens valid across instances.
- The 15-minute TTL is an in-code constant exercised only by unit tests on `PreviewStore` directly; the review for this feature ([0009 review](../.project/0009-admin-api-upload-review.md), m1) notes the end-to-end BDD scenario for TTL expiration was skipped as impractical without time manipulation.

### Neutral

- `upload_max_bytes` (default 10MB, `Config`-driven) bounds the in-memory footprint per preview entry; this is a capacity control, not a correctness one.

## Alternatives Considered

### Alternative A: Persist preview rows in `kb.db`

Add a `document_preview` table (`kb-store` V-migration) holding extracted text, metadata, and an expiry timestamp, with confirm deleting the row and expiry cleaned up by a periodic query. Rejected: it would be the only durable table in the schema whose entire purpose is to be deleted unconfirmed, mixing scratch state into the system's single source of truth and requiring a migration + cleanup job for data with a 15-minute useful life.

### Alternative B: Stateless confirm (client resubmits the extracted text)

Skip server-side preview storage; `POST /admin/api/upload` returns the extracted text directly, and the client re-sends the (possibly operator-edited) text to `confirm`. Rejected: it re-opens exactly the risk this feature exists to close — the server would index whatever text the client sends, with no guarantee it matches what was actually extracted from the uploaded file, weakening the "never index unseen content" guarantee to "never index unseen content, unless the client lies."

## Compliance

The `spontini-clean-arch-guard` skill enforces that `PreviewStore` and `TextExtractor` stay behind the `UploadPort` boundary — `backend`'s upload module depends outward on `ingest-core` only through the port, never on `ingest-core` internals directly. `PreviewStore` unit tests ([0009 plan](../.project/0009-admin-api-upload-plan.md) Task 2.1) cover insert/get/remove/TTL-eviction directly; the confirm handler is proven to be the only `kb.db` write path by the BDD upload-preview-confirm-searchable scenario in `backend/tests/bdd.rs`.
