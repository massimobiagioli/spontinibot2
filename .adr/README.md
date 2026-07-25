# Architecture Decision Records

This directory holds the ADRs for the Spontini Bot 2 project. Each ADR records a binding architectural decision with its context, rationale, and consequences.

| ID | Title | Status | Date |
|---|---|---|---|---|
| [0001](./0001-generation-model-3b.md) | Generation model — Qwen2.5-3B-Instruct instead of 7B | Accepted | 2026-07-09 |
| [0002](./0002-multi-stage-docker-compose-target.md) | Multi-stage Docker Builds as Compose Default Target | accepted | 2026-07-09 |
| [0003](./0003-rag-engine-ports-adapters.md) | RAG Engine as Backend Module with Ports/Adapters Architecture | accepted | 2026-07-09 |
| [0004](./0004-libsql-storage-layer.md) | libSQL as Storage Layer with Vector Search and Versioned Persona | accepted | 2026-07-09 |
| [0005](./0005-ingest-configuration-data-model.md) | Ingest Configuration Data Model in kb-store | accepted | 2026-07-09 |
| [0006](./0006-ingest-pipeline-trait.md) | Ingest Pipeline Trait and Composition Pattern | accepted | 2026-07-09 |
| [0007](./0007-cron-based-ingest-scheduler.md) | Cron-Based Ingest Scheduler with Config Polling | accepted | 2026-07-09 |
| [0008](./0008-preview-confirm-upload-workflow.md) | Preview/Confirm Upload Workflow with In-Memory TTL Token Store | accepted | 2026-07-24 |
| [0009](./0009-bootstrap-italia-3x-beta-with-patched-splide-exports.md) | Bootstrap Italia 3.x Beta with Patched Splide Exports | accepted | 2026-07-24 |

## How to add a new ADR

1. Use the [`/create-adr`](../.opencode/commands/create-adr.md) command.
2. The command assigns the next 4-digit ID, writes `.adr/<ID>-<slug>.md`, and appends a row to this index.
3. ADRs are immutable once Accepted — supersede with a new ADR that references the old one.
