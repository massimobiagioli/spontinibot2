# Architecture Decision Records

This directory holds the ADRs for the Spontini Bot 2 project. Each ADR records a binding architectural decision with its context, rationale, and consequences.

| ID | Title | Status | Date |
|---|---|---|---|---|
| [0001](./0001-generation-model-3b.md) | Generation model — Qwen2.5-3B-Instruct instead of 7B | Superseded by 0013 | 2026-07-09 |
| [0002](./0002-multi-stage-docker-compose-target.md) | Multi-stage Docker Builds as Compose Default Target | accepted | 2026-07-09 |
| [0003](./0003-rag-engine-ports-adapters.md) | RAG Engine as Backend Module with Ports/Adapters Architecture | accepted | 2026-07-09 |
| [0004](./0004-libsql-storage-layer.md) | libSQL as Storage Layer with Vector Search and Versioned Persona | accepted | 2026-07-09 |
| [0005](./0005-ingest-configuration-data-model.md) | Ingest Configuration Data Model in kb-store | accepted | 2026-07-09 |
| [0006](./0006-ingest-pipeline-trait.md) | Ingest Pipeline Trait and Composition Pattern | accepted | 2026-07-09 |
| [0007](./0007-cron-based-ingest-scheduler.md) | Cron-Based Ingest Scheduler with Config Polling | accepted | 2026-07-09 |
| [0008](./0008-preview-confirm-upload-workflow.md) | Preview/Confirm Upload Workflow with In-Memory TTL Token Store | accepted | 2026-07-24 |
| [0009](./0009-bootstrap-italia-3x-beta-with-patched-splide-exports.md) | Bootstrap Italia 3.x Beta with Patched Splide Exports | accepted | 2026-07-24 |
| [0010](./0010-production-compose-overlay-non-root-runtime-images-with-resource-limits.md) | Production Compose Overlay: Non-Root Runtime Images with Resource Limits | accepted | 2026-07-25 |
| [0011](./0011-session-cookie-operator-authentication-with-best-effort-audit-log.md) | Session-Cookie Operator Authentication with Best-Effort Audit Log | accepted | 2026-07-25 |
| [0012](./0012-categorical-refusal-rules-and-standard-fallback-text.md) | Categorical Refusal Rules and Standard Fallback Text | accepted | 2026-07-26 |
| [0013](./0013-generation-model-1-5b-and-reduced-rag-top-k-for-latency.md) | Generation Model 1.5B and Reduced RAG_TOP_K for Latency | accepted | 2026-07-26 |
| [0014](./0014-instant-identity-imprinting-answers-bypass-rag.md) | Instant Identity/Imprinting Answers Bypass RAG Retrieval and Generation | accepted | 2026-07-26 |

## How to add a new ADR

1. Use the [`/create-adr`](../.opencode/commands/create-adr.md) command.
2. The command assigns the next 4-digit ID, writes `.adr/<ID>-<slug>.md`, and appends a row to this index.
3. ADRs are immutable once Accepted — supersede with a new ADR that references the old one.
