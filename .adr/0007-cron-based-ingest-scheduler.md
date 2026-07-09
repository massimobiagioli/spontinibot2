# ADR 0007: Cron-Based Ingest Scheduler with Config Polling

- **Status**: accepted
- **Date**: 2026-07-09
- **Deciders**: Sisyphus (opencode)
- **Related**: Feature 0006

## Context

Spontini needs an always-on ingest service that runs the pipeline ([ADR 0006](./0006-ingest-pipeline-trait.md)) on a schedule defined by the operator. The service must read its configuration from `kb.db` (schedule, sections, sources), pick up configuration changes without restart, consume on-demand run requests written by the admin surface, and shut down cleanly on `SIGTERM`/`SIGINT`.

The ingest service is one of the five runtime containers defined in [STACK.md §3.3](../docs/STACK.md#33-runtime-services). It shares `kb.db` with `backend` and `ingest-core`.

## Decision

We will implement a `CronScheduler` that uses the `cron` crate to parse cron expressions and compute next-tick times. A `ConfigWatcher` polls `kb.db` every N seconds (configurable via `CONFIG_POLL_SECS`, default 30) and broadcasts configuration changes via a `tokio::sync::watch` channel. The scheduler's `tokio::select!` loop handles three events: config change (reparse cron), cron tick (run pipeline for all enabled sources), and run-request poll (consume flag-row, trigger immediate run). Graceful shutdown uses `tokio_util::sync::CancellationToken` with `SIGTERM`/`SIGINT` signal handling.

## Rationale

Cron expressions are a well-understood scheduling primitive that operators can configure via the admin UI. Config polling avoids the complexity of a pub/sub mechanism (e.g., SQLite triggers, file-watching) while providing eventual consistency — configuration changes are applied within the poll interval. The flag-row pattern for run requests is simple, works with the existing `KbStore` CRUD API, and avoids introducing a message queue. The `CancellationToken` pattern is the standard tokio approach for graceful shutdown.

## Consequences

### Positive

- Simple, well-understood scheduling model (cron expressions)
- Configuration changes applied without restart (within poll interval)
- Clean shutdown with in-flight run completion (CancellationToken)
- Run requests consumed atomically (flag-row with status progression)
- Single binary, no external dependencies beyond `kb.db`

### Negative

- Config polling introduces up to N-second delay for configuration changes
- Cron parsing may have edge cases with complex expressions
- Flag-row pattern doesn't provide persistent run history (only current status)
- Each source URL is fetched independently — no batch optimization

### Neutral

- The `CronScheduler` runs in a single tokio task — no thread pool or work-stealing

## Alternatives Considered

### Alternative A: File-watching (inotify)

Watch `kb.db` for changes using filesystem notifications. Rejected because it doesn't work reliably across Docker containers and adds platform-specific complexity.

### Alternative B: Message queue (Redis/RabbitMQ)

Use a message queue for run requests and configuration updates. Rejected because it violates the Constitution's local-stack requirement and adds operational complexity for a single-operator system.

### Alternative C: Fixed-interval polling (no cron)

Run the pipeline at a fixed interval (e.g., every 6 hours). Rejected because cron expressions provide more expressive scheduling (e.g., "every weekday at 2 AM") without additional complexity.

## Compliance

The `spontini-ingest-flow` skill enforces the two-entry-point rule: the scheduler consumes `ingest-core::Pipeline` and reads configuration from `kb-store`. The `spontini-clean-arch-guard` skill ensures the `ingest` crate depends only on `ingest-core`, `kb-store`, and standard tokio utilities. Unit tests validate cron parsing, config change detection, and run-request consumption.
