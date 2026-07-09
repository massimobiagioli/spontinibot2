# ADR 0005: Ingest Configuration Data Model in kb-store

- **Status**: accepted
- **Date**: 2026-07-09
- **Deciders**: Sisyphus (opencode)
- **Related**: Feature 0004

## Context

The ingest service ([Feature 0006](../.project/0006-ingest-service-long-running-scheduler-plan.md)) needs to read its configuration (schedule, sections, sources) from `kb.db`. The admin surface ([Features 0010, 0011](../.project/ROADMAP.md)) needs to write this configuration via HTTP endpoints. The configuration must be stored in the same `kb.db` file that `kb-store` already manages, following the established migration pattern from ADR [0004](./0004-libsql-storage-layer.md).

The design must handle: a singleton schedule (at most one cron expression), hierarchical sections with ordering, per-section sources with type discrimination (scrape vs. api), on-demand run requests that the ingest service consumes, and the constraint that `api` source type is stored but never wired to an adapter.

## Decision

We will extend `kb-store` with four new tables via a `V2__ingest_config.sql` migration: `ingest_schedule` (singleton row at `id=1`), `ingest_section` (name + ordering), `ingest_source` (section_id FK with CASCADE delete, source_type CHECK constraint), and `ingest_run_request` (flag-row table with status progression). The `api` source type is stored but always returned with `enabled=false` and `coming_soon: true` at the application layer.

## Rationale

The singleton schedule row avoids the complexity of multiple schedule configurations while matching the real-world constraint (one ingest service, one schedule). The flag-row pattern for run requests is simple, works with the existing `KbStore` CRUD API, and avoids introducing a message queue. CASCADE delete ensures source cleanup when a section is removed. The `api` source reservation allows the admin UI to display the option without requiring adapter implementation.

## Consequences

### Positive

- Configuration lives in the same `kb.db` as documents and persona — no separate config store
- The ingest service picks up changes on its next config poll — no restart required
- CASCADE delete prevents orphaned source rows
- The flag-row run request pattern is simple and debuggable

### Negative

- The singleton schedule row is enforced at the application layer, not via a database CHECK constraint
- Config polling introduces up to N-second delay for configuration changes
- The `api` source type is stored but not functional — operators see a disabled option

### Neutral

- The `V2` migration follows the same linear pattern as `V1` (check version, apply if absent, record)

## Alternatives Considered

### Alternative A: Separate config file (TOML/YAML)

Store ingest configuration in a file instead of `kb.db`. Rejected because it splits configuration across two stores, requires file-watching for the ingest service, and doesn't integrate with the admin API's CRUD endpoints.

### Alternative B: JSON column in a single config row

Store all ingest config as a single JSON blob. Rejected because it prevents partial updates (changing one source requires rewriting the entire config) and makes SQL-level queries impossible.

## Compliance

The `spontini-ingest-flow` skill enforces that both entry points (scheduler and admin-ui) access configuration through `kb-store`'s public API. The `spontini-clean-arch-guard` skill ensures no framework types leak into the configuration domain types. Unit tests validate the singleton schedule behavior, CASCADE delete, and the flag-row consumption pattern.
