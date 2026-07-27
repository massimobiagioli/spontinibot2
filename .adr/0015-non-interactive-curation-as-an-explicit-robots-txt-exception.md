# ADR 0015: Non-Interactive Curation as an Explicit, Config-Driven robots.txt Exception

- **Status**: accepted
- **Date**: 2026-07-27
- **Deciders**: massimobiagioli, Claude Sonnet 5
- **Related**: Feature 0030

## Context

`ingest-core`'s `ScraperAdapter` honors `robots.txt` unconditionally for every source — a deliberate, load-bearing guarantee (feature 0029 re-confirmed it live: a manual-ingest request against `https://www.halleyweb.com/.../delibere` correctly returns `IngestError::RobotsTxt`, never silently bypassing it). But the Comune's own official `delibere`/`determine`/Giunta records live exclusively on `halleyweb.com`, whose `robots.txt` disallows scraping entirely. Feature 0028 already worked around this once, by hand: an operator manually downloaded and uploaded each delibera individually through the admin-ui's existing preview/confirm upload flow (feature 0009), with the project owner's full, explicit knowledge and direction that these are the Comune's own public records and this specific site's content is authorized for this specific purpose. That manual process doesn't scale to routine, recurring curation of a five-month backlog, let alone ongoing upkeep.

The project owner was emphatic and explicit that no domain name may be hardcoded into the binary for this: *"ATTENZIONE!!! NON VOGLIO VEDERE HALLEYWEB ED ALTRE FONTI HARD-CODATE NEL CODICE!!!! TUTTO DEV ESSERE PARAMETRICO!!!"* — any such exception must be pure deployment configuration, not source code, so it can never be silently generalized to an unauthorized domain by a future contributor who doesn't know the history above.

## Decision

We add a second, explicitly-scoped ingestion pathway — `HalleyCurationAdapter` — that does not honor `robots.txt`, reserved for a small, named, operator-authorized allow-list of first-party civic-data domains (`CURATION_ALLOWED_HOSTS`, a comma-separated env var, **empty by default**; `docker-compose.yml` is the only place `halleyweb.com` is ever named, and only for this deployment). A new `CuratingIngestManualAdapter` dispatches `POST /admin/api/ingest/manual` requests by the target URL's host: an allow-listed host goes to `HalleyCurationAdapter`; every other host goes, unmodified, to the existing `PipelineIngestManualAdapter` (feature 0029), which still honors `robots.txt` unconditionally. Both implement the same `IngestManualAdminPort` — this is Open/Closed extension, not a change to the existing scrape path's contract or behavior.

`HalleyCurationAdapter` walks Halley's own listing/detail HTML (parsed by a pure, fixture-tested `halley::parser` module, tested against real captured markup, not invented structure) and feeds each act's PDF/RTF attachment through the **existing** `CompositeExtractor` and `UploadPort::ingest_uploaded` (feature 0009) — the identical call the human preview/confirm flow already makes, just invoked directly instead of waiting on an operator's confirm click. A new `ingest_bookmark` table (`section_id`, `source_url`, `last_item_ref`, `last_item_date`) records the most-recently-curated act per section+source, checkpointed after **every** successfully curated item (not batched to the end of a run — see Consequences), so a later run only fetches what's newer instead of re-walking the whole window.

## Rationale

Evaluated against [Constitution §6](../docs/CONSTITUTION.md#6-decision-making):

1. **Serves the mission?** Yes — citizens asking about `delibere`/Giunta decisions get answers grounded in the Comune's actual official records, which is the entire point of the KB, and which was previously only reachable through unsustainable manual labor.
2. **Keeps the stack local?** Unaffected — no new external service; this fetches public HTML/PDF pages over plain HTTP, same as the existing scraper's transport.
3. **Reduces complexity?** Yes relative to the alternative of duplicating the upload pipeline (see Alternatives) — reuses `CompositeExtractor` and `UploadPort` verbatim; the only new logic is the Halley-specific HTML parsing and the bookmark checkpoint.
4. **Improves UX?** Yes for the operator (`bin/ingest` becomes a one-line recurring command instead of manual per-document uploads) — with zero change to the citizen-facing chat UX, since ingested content is indistinguishable at query time from any other manually-uploaded document.

## Consequences

### Positive

- `robots.txt` enforcement for `ingest-core::ScraperAdapter` remains absolute and untouched for every source not on the explicit allow-list — this ADR does not weaken that guarantee anywhere else.
- No domain name is hardcoded in the binary. `Config::curation_allowed_hosts` defaults to `Vec::new()` (curation effectively off) unless a deployment's own environment explicitly opts a host in — verified by `parse_curation_hosts`'s own unit tests (`should_be_empty_when_no_env_var_set`).
- The bookmark makes curation runs incremental and resumable in principle, bounding both third-party traffic and re-processing on repeat runs.

### Negative

- This is, in substance, an automated fetcher for a site whose `robots.txt` disallows automated access. It is deliberately scoped and documented specifically so it is never mistaken for a general-purpose bypass; any future addition to `CURATION_ALLOWED_HOSTS` must carry the same explicit, project-owner-authorized justification this one did, not be added casually because it's now easy to.
- **A real gap was found and fixed during the first live run**: `HalleyCurationAdapter::ingest()` originally wrote `ingest_bookmark` only once, after the entire batch succeeded (or, on an explicit `curate_one` error, to the last successfully-processed row). A live run against `halleyweb.com` was cut short by a client-side timeout after 43 documents had already been durably chunked and embedded — but because the whole `ingest()` future was dropped mid-loop rather than returning an explicit `Err`, none of that progress was checkpointed, and a naive retry would have re-ingested (and duplicated, since `insert_document` has no dedup-by-`source_ref`) all 43 items. Fixed by writing the bookmark immediately after each item succeeds, proven with a regression test that reproduces true async-cancellation semantics (`tokio::select!` + dropping the in-flight future), verified red before the fix and green after. The residual risk — at most the single in-flight item being duplicated if cancellation lands between that item's `ingest_uploaded` call and its bookmark write — is accepted as proportionate for a low-volume, operator-triggered job; true cross-step atomicity would require transactional coordination `UploadPort`'s interface doesn't expose today.
- Halley's markup is a third-party CMS outside this project's control and could change without notice. The parser fails closed (`HalleyParseError`, not a panic or silently-wrong data) if the expected structure isn't found.

### Neutral

- Curated documents are recorded with `source = 'manual'`, identical to a human-uploaded PDF — there is no separate `DocumentSource::Curated` variant. This is intentional (Plan 0030 explicitly reuses feature 0009's upload path rather than duplicating it), but means the `documents` table alone cannot distinguish "an operator uploaded this by hand" from "curation fetched this automatically." The `ingest_bookmark` table is the only durable record of which sections have an active, config-driven curation source, and it is not currently surfaced in admin-ui.

## Alternatives Considered

### Alternative A: Loosen `ingest-core::ScraperAdapter`'s robots.txt enforcement with a per-source override flag

Add an `ignore_robots: bool` field to `IngestSource` instead of a separate adapter. Rejected: this would put the exception in the same code path every other source shares, making it one config toggle away from being flipped on for an unauthorized domain by mistake, and would blur `ingest-core`'s existing, simple guarantee ("this component never bypasses robots.txt, full stop") that other contributors currently rely on without needing to read every source's flags.

### Alternative B: A fully separate, duplicated upload/ingestion pipeline for curated content

Build a standalone code path from HTML parsing through embedding and storage, independent of feature 0009's `UploadPort`. Rejected: pure duplication of already-correct, already-tested logic (chunking, tagging, embedding-model selection) for no benefit — the only genuinely new logic this feature needs is "parse Halley's HTML" and "remember where we left off," both of which are the actual deliverables (`halley::parser`, `ingest_bookmark`).

## Compliance

- `backend/src/config.rs::parse_curation_hosts` and its tests are the enforcement point that no host is curation-eligible unless explicitly configured.
- `docker-compose.yml`'s `CURATION_ALLOWED_HOSTS=${CURATION_ALLOWED_HOSTS:-halleyweb.com}` line, with its accompanying comment, is the only place this deployment names an eligible domain — any reviewer auditing "does this bypass robots.txt anywhere" should look there first, not in `backend/src/admin/ingest_manual/halley/`.
- `backend/src/admin/ingest_manual/composite_adapter.rs`'s dispatch tests (`should_dispatch_allowlisted_host_to_curation`, `should_dispatch_other_hosts_to_scrape`) are the executable proof that non-allow-listed hosts are unaffected.
