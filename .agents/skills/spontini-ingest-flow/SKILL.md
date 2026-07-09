---
name: spontini-ingest-flow
description: Build or modify the Spontini ingest pipeline. Use WHEN working on ingest-core, ingest-cli, source adapters (scraper/api/db/folder), admin-ui upload, chunking, or embedding writes. Enforces the two-entry-point rule, shared kb.db, and the embed-model-must-match constraint.
---

# Spontini Ingest Flow

You are touching the ingestion pipeline: a source adapter, chunking, embedding writes, `ingest-core`, `ingest-cli`, or the `admin-ui` upload route. Load this skill.

## The Two Entry Points

Ingest has shared logic in `ingest-core`, exposed by two independent entry points. They never communicate directly; they only share the `kb.db` file.

```
ingest-cli (automated, one-shot/cron):
  scraper  → ingest-core → embed → kb.db
  api-client → ...
  db-connector → ...
  folder-reader (incl. Obsidian vaults with frontmatter/wikilinks) → ...

admin-ui (manual, operator-driven):
  /admin/upload (drag&drop pdf/docx/md/txt)
    → preview extracted text
    → operator fills metadata (category, tags, priority)
    → ingest-core → embed → kb.db
```

Both entry points call the SAME `ingest-core` functions. Never duplicate ingest logic in `admin-ui` or `ingest-cli`.

## Source Adapters

Each source is a port implementation with a single responsibility: produce a stream of raw documents.

| Adapter        | Reads from                                   | Output                                  |
|----------------|----------------------------------------------|-----------------------------------------|
| `scraper`      | Web pages                                    | Raw HTML/text + URL                     |
| `api-client`   | External REST/JSON APIs                      | Raw JSON/text + endpoint ref           |
| `db-connector` | Third-party databases                        | Rows mapped to text + row id           |
| `folder-reader`| Filesystem folders, incl. Obsidian vaults    | File content + frontmatter + path      |
| `manual`       | `admin-ui` upload                            | Extracted text + operator metadata     |

Rules:

- One adapter per source type. No `if source == "scraper"` branching inside `ingest-core`.
- Adapters return a `RawDocument` (text + `source` + `source_ref` + optional metadata). They do NOT embed, do NOT write to `kb.db`.
- A new source is added by implementing the `DocumentSource` trait, never by editing `ingest-core` internals (Open/Closed, SOLID).

## Chunking

- Chunking happens in `ingest-core`, after the adapter yields `RawDocument` and before embedding.
- Chunk size and overlap are configurable, not hardcoded.
- Each chunk retains the parent document's `id`, `source`, `source_ref`, and `metadata`.

## Embedding Writes

- `ingest-core` calls the `llama-embed` container's `/embedding` endpoint (same endpoint the rag-engine uses at query time).
- The returned vector is written to `documents.embedding` (column type `F32_BLOB(768)` for `nomic-embed-text`; dimension matches the model).
- The embedding model and dimension are defined ONCE in shared config. Both ingest and query read the same value.

## Embedding Model Constraint (Critical)

Changing the embedding model invalidates every vector already in `kb.db`. The workflow for a model change is:

1. Stop ingest jobs.
2. Swap the GGUF model in the `llama-embed` container.
3. Truncate or recreate the `documents` table (vectors are now meaningless).
4. Re-ingest every source from scratch.
5. Confirm the rag-engine reads the same new model endpoint.

Never partially swap. A `kb.db` with mixed-model embeddings is corrupt.

## kb.db Access Rules

- Only `kb-store` talks to `libsql` directly. `ingest-core` calls `kb-store` through the `DocumentPort`.
- Writes are transactional. A document with N chunks writes all N rows or none.
- `source` column values: `'scrape' | 'api' | 'db' | 'folder' | 'manual'`.
- `metadata` column is a JSON string: tags, category, priority/trust_score. Validated before write.

## ingest-cli Is Not a Service

`ingest-cli` runs as a one-shot job, not an always-on container:

```bash
docker compose run --rm ingest-cli --source scraper --config /configs/scraper.toml
```

Scheduled via host cron or an external orchestrator. Do not add a health endpoint, do not add a long-running loop, do not add a web server.

## admin-ui Upload Rules

- `/admin/upload` is protected (operator-only). Authentication is out of scope for v1 per Constitution §4, but the route must be clearly marked as admin and isolated from `/chat`.
- Preview the extracted text BEFORE indexing. The operator confirms.
- Metadata form fields: category, tags, priority/trust_score. Optional but encouraged.
- The upload path goes through `ingest-core` just like `ingest-cli` — no parallel embedding logic.

## Workflow

1. Identify the entry point: `ingest-cli` adapter, or `admin-ui` upload.
2. Identify the layer: adapter (new source), `ingest-core` (shared logic), or `kb-store` (storage).
3. If adding a source: implement `DocumentSource`, do not modify `ingest-core` internals.
4. If touching embedding: confirm ingest and query sides use the same model endpoint.
5. If touching chunking: confirm chunk size/overlap are config-driven.
6. Load `spontini-tdd-rust` for code changes.
7. Load `spontini-verify-gate` before claiming done.

## Forbidden

- Duplicating ingest logic between `ingest-cli` and `admin-ui`.
- An adapter that embeds or writes to `kb.db` directly (violates Single Responsibility).
- A new source implemented by branching inside `ingest-core`.
- Hardcoding the embedding model or dimension in the adapter.
- `UPDATE` on `documents` rows to change their embedding in-place — re-ingest instead.
- Making `ingest-cli` a long-running service.
