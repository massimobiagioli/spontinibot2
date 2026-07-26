# Plan 0028: Real content population (Gaspare persona, storia/news/delibere/Giunta ingestion) + kb.db snapshot Makefile targets

- **Status**: review
- **Approved**: 2026-07-26 by Claude Code (autonomous /next-steps run)
- **Implemented**: 2026-07-26 by Claude Code (autonomous /next-steps run)
- **Branch**: feat/real-content-population-gaspare-persona-storia-news-delibere-giunta-ingestion-kb-db-snapshot-makefile-targets
- **Feature ID**: 0028
- **Created**: 2026-07-26
- **Owner**: Claude Code (autonomous /next-steps run)

## Objective

Milestone 6 turns Spontini from an architecturally-complete but sparsely-populated system into one that actually serves the mission stated in [Constitution](../docs/CONSTITUTION.md) §1: answering citizens of the Comune di Maiolati Spontini from real official documents. This plan does two things. First, it (re)populates the live knowledge base with real, sourced content: the bot's persona re-imprinted under the name "Gaspare" per Constitution §2 (the bot embodies Gaspare Spontini and speaks in his voice) and, while doing so, brings the active persona's `fallback_message` into compliance with [ADR 0012](../.adr/0012-categorical-refusal-rules-and-standard-fallback-text.md) (the currently-active `spontini-bot` persona's fallback text does not match the mandated exact string — a real, pre-existing compliance gap this plan closes); the `storia` section (already substantially populated by the `TEST-INGESTION-0001` session — verified, not re-done); a `news` section covering February–July 2026; a `delibere` section (delibere + determine) covering April–July 2026; and a brand-new `Giunta` section sourced from the comune's political-administrative-bodies page. All of `news`/`delibere`/`Giunta` content is sourced from `halleyweb.com`, a domain independently confirmed (`TEST-INGESTION-0001.md` §5.2) to disallow scraping entirely via `robots.txt` — every document in those three sections is therefore fetched and ingested via the manual per-section upload flow (feature 0009, preview→confirm), never a scrape source. Second, it adds two operator-convenience Makefile targets, `eject-data` and `use-data`, that snapshot and restore the `kb-data` Docker volume, so a known-good populated KB state doesn't have to be rebuilt from scratch after every `docker compose down -v` or environment reset.

In scope: persona re-imprinting; representative-sample real content ingestion for `news` (12 items, Feb–Jul 2026) and `delibere` (10 items — 5 delibere + 5 determine — Apr–Jul 2026); creation of the `Giunta` section with its one real source document; verification that `storia`'s existing content is intact; the `eject-data`/`use-data` Makefile targets; confirming operator credential stability (already satisfied — see Phase 1). Out of scope (see Non-Goals): exhaustive ingestion of the full municipal archive, a scraper/crawler capable of paginating `halleyweb.com`'s search engine, any change to `RagEngine`, `RAG_TOP_K`/`RAG_MIN_SCORE`, or the retrieval/generation pipeline itself.

## Non-Goals

- Exhaustive ingestion of every news item, delibera, or determina ever published — a representative, topically-varied real sample is the explicit goal (`TEST-INGESTION-0001.md` §5.2 point 3), not the entire archive.
- Building a paginating crawler for `halleyweb.com`'s search-engine-style `determine`/`delibere` listing (1,401+ pages) — out of scope for `ingest-core`'s single-GET scraper design.
- Any change to `RagEngine`, `RAG_TOP_K`, `RAG_MIN_SCORE`, retrieval, or generation code — this plan is content population and operator tooling only.
- Rotating or changing the operator credential — it is already stable in `secrets.txt` per explicit prior instruction; this plan verifies, never rotates.
- A code-level ingest scheduler/scraper fix — the scheduler and config-reload bugs found during `TEST-INGESTION-0001` were already fixed in commit `17077ad`, prior to this plan.

## Phases

### Phase 1: Operator credential stability (verification only)

Goal: confirm the existing `secrets.txt` operator credential is still valid and stays stable — no rotation.

- [x] **Task 1.1** — Verify the live operator credential
  - What: log in against the running `backend` (`POST /admin/api/auth/login`) using the exact credentials already recorded in `secrets.txt` (repo root, gitignored); confirm `200` + session cookie. Do not regenerate or rotate the credential — `secrets.txt` already documents "DO NOT ROTATE AGAIN" from the prior session.
  - Deliverables:
    - A verified, working session cookie usable for every subsequent admin call in this plan.
    - No file changes (credential already stable; this is a read-only check).
  - Skills to load: none (operational verification, no code change).
  - Verification: `curl -sS -c /tmp/spontini-session.txt -X POST http://localhost:8080/admin/api/auth/login ...` returns HTTP 200 with the credentials in `secrets.txt`.

### Phase 2: kb-data snapshot Makefile tooling

Goal: add `make eject-data` / `make use-data` so a populated `kb.db` state can be snapshotted and restored without re-running ingestion.

- [x] **Task 2.1** — `eject-data` Makefile target
  - What: add a `.PHONY: eject-data` target that runs `docker compose run --rm --no-deps --user root -v $(PWD)/.data:/backup backend sh -c 'tar czf /backup/data-$$(date +%Y-%m-%d).bin -C /data .'` (creates `.data/` first if missing), snapshotting the `kb-data` volume without hardcoding its project-qualified name and without requiring the non-root runtime user to have write access to the host bind mount.
  - Deliverables:
    - `Makefile` — `eject-data` target + `## eject-data:` help line.
    - `.gitignore` — add `.data/` (operator snapshot directory, never committed, mirrors the existing `secrets.txt`/`secrets/` exclusion).
  - Skills to load: none.
  - Verification: `make eject-data` against the live stack produces `.data/data-<yyyy-MM-dd>.bin`, a non-empty tar.gz.

- [x] **Task 2.2** — `use-data` Makefile target
  - What: add a `.PHONY: use-data` target taking `DATA_FILE=<path>`, validating the argument and file existence, then running `docker compose run --rm --no-deps --user root -v $(PWD)/.data:/backup backend sh -c 'rm -rf /data/* && tar xzf /backup/<basename> -C /data'` to restore a previously-ejected snapshot.
  - Deliverables:
    - `Makefile` — `use-data` target + `## use-data:` help line.
  - Skills to load: none.
  - Verification: round-trip test against the live stack — record `select count(*) from documents` before `eject-data`, run `make down && make up` (fresh volume state is NOT wiped by `down` without `-v`, so instead verify via a scratch volume swap or by comparing document IDs before/after `use-data` of the just-ejected file onto the same running volume), confirm document count and a sample document's content match after `use-data` restores the snapshot.

### Phase 3: Bot imprinting — persona "Gaspare"

Goal: replace the active `spontini-bot` persona with a new `gaspare` persona version whose identity matches Constitution §2 (the bot embodies Gaspare Spontini) and whose `fallback_message` complies with ADR 0012's exact mandated text.

- [x] **Task 3.1** — Create and activate the `gaspare` persona version
  - What: `POST /admin/api/persona` with `name: "gaspare"`, a `system_prompt` written in the first-person voice of a proud local figure (grounded strictly in the verified facts already gathered in `TEST-INGESTION-0001.md` §4 — born 14 Nov 1774, died 24 Jan 1851 in Maiolati, name added to the comune in 1939 — no invented biographical detail), explicitly encoding the three ADR 0012 categorical refusals (no future predictions, no weather, no personal data) in the prompt text, `tone` reflecting "cordiale, orgoglioso delle radici del paese, preciso, mai burocratico", `fallback_message` set to exactly `"Mi dispiace ma è un'informazione che non conosco."` (ADR 0012), and `activate: true`.
  - Deliverables:
    - A new active `persona` row (`name: "gaspare"`) in the live `kb.db`, superseding the currently-active `spontini-bot` v1.
    - `.project/0028-content-ingestion-log.md` — new file, styled after `TEST-INGESTION-0001.md` §5.3's ingested-documents table, recording the persona change and every document ingested by this plan (title, section, URL/method, date, verified-in-KB checkbox).
  - Skills to load: none (data population via existing admin API, no code change).
  - Verification: `GET /admin/api/persona?name=gaspare` shows the new version `is_active=true`; `POST /chat {"question":"Chi sei?"}` returns an answer in the new voice; `POST /chat` with an out-of-KB question (e.g. a future-prediction question) returns `fell_back=true` and the exact ADR 0012 fallback string.

### Phase 4: `storia` section — verification (no re-ingestion)

Goal: confirm the `storia` content ingested during `TEST-INGESTION-0001` (official comune page + 2 Wikipedia pages + the political-bodies roster, 28 chunks total) is intact and still counts as this plan's `storia` deliverable.

- [x] **Task 4.1** — Verify existing `storia` documents
  - What: query the live `kb.db` (`documents` joined through chunks tagged `storia`, or via `ingest_section`/source lineage) to confirm the 4 documents logged in `TEST-INGESTION-0001.md` §5.3 (storia-comune, Maiolati Spontini wiki, Gaspare Spontini wiki, organi-politico-amministrativo) are still present and retrievable; run one `/chat` smoke question per document to confirm citation.
  - Deliverables:
    - `.project/0028-content-ingestion-log.md` — a `storia` row block cross-referencing `TEST-INGESTION-0001.md` §5.3 (no new files, this section's content was already ingested pre-plan).
  - Skills to load: none.
  - Verification: 4/4 documents confirmed present in `kb.db`; a smoke question about Gaspare Spontini's birth date returns a cited, correct answer.

### Phase 5: `news` section — real content population (Feb–Jul 2026)

Goal: manually upload 12 real, dated, topically-varied news items from the comune's real news feed (`halleyweb.com/c042023/po/elenco_news.php?area=H`, verified live-fetched 2026-07-26), spanning February through July 2026, via the preview→confirm upload flow (feature 0009) — never scraped, per the confirmed `robots.txt` block.

- [x] **Task 5.1** — Ingest news batch 1 (Feb–Apr 2026, 6 items)
  - What: `WebFetch` the full article text of each of the following 6 real, already-identified news items, save each as a markdown file, and run them through `POST /admin/api/upload` (`section=news`) → `GET preview` → `POST confirm`:
    1. id 1113, 24/02/2026 — "L'associazione Auser Media Vallesina taglia il traguardo dei vent'anni di attività"
    2. id 1108, 12/02/2026 — "ASSEGNO DI MATERNITA' ANNO 2026"
    3. id 1120, 17/03/2026 — "Il Comune punta a dotarsi di defibrillatori da installare nei luoghi più frequentati del territorio"
    4. id 1125, 24/03/2026 — "SERVIZIO MENSA E TRASPORTO SCOLASTICO A.S. 2026-2027 - MODULISTICA"
    5. id 1141, 22/04/2026 — "Giornata della Terra: presentato il nuovo logo del progetto Piedibus"
    6. id 1143, 28/04/2026 — "Il Consiglio approva la messa a norma degli stadi"
    (URLs: `https://www.halleyweb.com/c042023/po/mostra_news.php?id=<id>&area=H`)
  - Deliverables:
    - 6 ingested documents in the `news` section of the live `kb.db`.
    - `.project/0028-content-ingestion-log.md` updated with all 6 rows (title, URL, date, method=manual upload, verified checkbox).
  - Skills to load: none.
  - Verification: each `confirm_upload` call returns `document_ids`; a `/chat` smoke question about one item (e.g. the defibrillators initiative) returns a correct, cited answer.

- [x] **Task 5.2** — Ingest news batch 2 (May–Jul 2026, 6 items)
  - What: same method as Task 5.1, for:
    7. id 1145, 05/05/2026 — "Personale del Comune, pensionamenti e nuovi arrivi"
    8. id 1150, 11/05/2026 — "Bando Servizi Digitali Integrati - DigitalizziAMO Maiolati Spontini"
    9. id 1155, 06/06/2026 — "Insediamento del Consiglio e nomina dei componenti della Giunta comunale"
    10. id 1159, 13/06/2026 — "Sindaco e assessori illustrano le priorità del mandato amministrativo"
    11. id 1169, 02/07/2026 — "Il Comune ottiene un finanziamento di 20 mila euro nel bando 'Città che legge 2025'"
    12. id 1175, 10/07/2026 — "Il patrimonio di Gaspare Spontini conquista due nuovi riconoscimenti"
  - Deliverables:
    - 6 more ingested documents in the `news` section.
    - `.project/0028-content-ingestion-log.md` updated with all 6 rows.
  - Skills to load: none.
  - Verification: same as Task 5.1; total `news` section document count = 12 (or more, if any article produced multiple chunks).

### Phase 6: `delibere` section — real content population (Apr–Jul 2026)

Goal: manually upload 10 real, numbered, dated municipal acts (5 delibere + 5 determine, verified live-fetched 2026-07-26 from `halleyweb.com`'s acts search engine) via the same manual-upload flow.

- [x] **Task 6.1** — Ingest delibere (5 items)
  - What: `WebFetch` each detail page's full text and upload via `section=delibere`:
    1. n. 74, 13/07/2026 — "Modifica disposizione posteggi area Fiera Sant'Anna 2026"
    2. n. 73, 07/07/2026 — "Campi di calcio 'M. Pierucci' e 'Grande Torino' - approvazione linee di indirizzo per affidamento gestione"
    3. n. 72, 07/07/2026 — "Campo di calcio 'G. Scirea' Maiolati Spontini - approvazione linee di indirizzo per affidamento gestione"
    4. n. 71, 07/07/2026 — "Approvazione stato attuazione dei programmi esercizio 2026 - schema DUP 2027-2029"
    5. n. 70, 07/07/2026 — "Bilancio consolidato esercizio 2025 - individuazione componenti del 'Gruppo Comune di Maiolati Spontini'"
    (detail URLs under `https://www.halleyweb.com/c042023/zf/index.php/atti-amministrativi/delibere/dettaglio/atto/...`, permalinks already captured live)
  - Deliverables:
    - 5 ingested documents in the `delibere` section.
    - `.project/0028-content-ingestion-log.md` updated with all 5 rows.
  - Skills to load: none.
  - Verification: each upload confirmed; a smoke question about one delibera returns a cited answer.

- [x] **Task 6.2** — Ingest determine (5 items)
  - What: same method, for:
    6. Reg. Gen. 455, 24/07/2026 — "Servizi di garanzia 36 mesi per nuovo ponte radio"
    7. Reg. Gen. 454, 23/07/2026 — "Lavori di tinteggiatura interna e sistemazione infissi"
    8. Reg. Gen. 453, 22/07/2026 — "Sistema culturale integrato - progetto Un fiume di cultura"
    9. Reg. Gen. 452, 22/07/2026 — "Realizzazione dossi rallentatori di velocità in conglomerato"
    10. Reg. Gen. 443, 21/07/2026 — "Affidamento del servizio di manutenzione per i moduli Backoffice SUE"
  - Deliverables:
    - 5 more ingested documents in the `delibere` section.
    - `.project/0028-content-ingestion-log.md` updated with all 5 rows.
  - Skills to load: none.
  - Verification: same as Task 6.1; total `delibere` section document count = 10 (or more, if any act produced multiple chunks).

### Phase 7: New `Giunta` section

Goal: create the `Giunta` section and ingest its one real source document (the comune's political-administrative-bodies roster page, category 78).

- [x] **Task 7.1** — Create the `Giunta` section and ingest its source
  - What: `POST /admin/api/ingest/config/sections` (`name: "giunta"`, next `ordering`); `WebFetch` `https://www.halleyweb.com/c042023/zf/index.php/organi-politico-amministrativo/index/index/categoria/78` (confirmed live 2026-07-26: Sindaco Sebastiano Mazzarini + 4 Assessori with portfolios, plus the full Consiglio roster — same `halleyweb.com` robots.txt block as `storia`'s roster page, manual-upload-only); upload the extracted text via `section=giunta`.
  - Deliverables:
    - A new `giunta` row in `ingest_section`.
    - 1 ingested document in the `giunta` section.
    - `.project/0028-content-ingestion-log.md` updated with the `Giunta` row.
  - Skills to load: none.
  - Verification: `GET /admin/api/ingest/config` lists the `giunta` section; a `/chat` smoke question ("Chi è il sindaco di Maiolati Spontini?" / "Chi sono gli assessori?") returns a correct, cited answer sourced from the `giunta` document.

### Phase 8: Final verification

Goal: confirm the whole feature works end-to-end and the standard gates still pass.

- [x] **Task 8.1** — Live cross-section smoke test + gate
  - What: run one `/chat` smoke question per section (persona identity, storia, news, delibere, giunta) plus one deliberately out-of-KB question (confirming the ADR 0012 exact fallback string), then run `make verify`.
  - Deliverables:
    - `.project/0028-content-ingestion-log.md` finalized with a summary section (document counts per `ingest_section`, smoke-test results).
  - Skills to load: spontini-verify-gate.
  - Verification: all 6 smoke questions behave as expected (5 cited-correctly, 1 honest fallback with the exact ADR 0012 string); `make verify` passes (build + test + lint + fmt-check + coverage + compose-config + a11y).

## Acceptance Criteria

- The active persona is named `gaspare`, embodies Gaspare Spontini per Constitution §2, and its `fallback_message` is exactly `"Mi dispiace ma è un'informazione che non conosco."` (ADR 0012 compliance).
- `storia` has ≥4 real, sourced documents (verified intact from `TEST-INGESTION-0001`).
- `news` has 12 real, dated documents spanning Feb–Jul 2026.
- `delibere` has 10 real, numbered/dated documents (5 delibere + 5 determine) spanning Apr–Jul 2026.
- A new `giunta` section exists with 1 real, sourced document (current Sindaco + Giunta composition).
- `.project/0028-content-ingestion-log.md` exists and accurately logs every document ingested by this plan.
- `make eject-data` produces a restorable snapshot of the `kb-data` volume; `make use-data DATA_FILE=<snapshot>` restores it, verified by a document-count round-trip.
- `secrets.txt`'s operator credential is unchanged (no rotation) and still authenticates.
- `make verify` passes.

## Risks

- **`halleyweb.com` pagination limits the delibere/determine sample to page 1 (all July 2026 dates)** — mitigation: this is disclosed in the plan (Tasks 6.1/6.2 list only July-dated real acts found on the first, most-recent page); still within the Apr–Jul window and still 100% real, sourced content — the roadmap's own guidance (`TEST-INGESTION-0001.md` §5.2 point 3) accepts a representative sample over exhaustive date coverage.
- **`kb-data` volume name is project-directory-dependent** — mitigation: Task 2.1/2.2 use `docker compose run` (which resolves the compose-scoped volume automatically) instead of hardcoding a `docker volume` name.
- **Non-root runtime container lacks write permission to a host-bind-mounted `.data/` directory** — mitigation: `eject-data`/`use-data` use `--user root` on the one-off `docker compose run` maintenance container, scoped to just that command, not the long-running service.
- **Large real-content ingestion volume could hit `llama-embed`'s batch size again** — mitigation: already fixed project-wide in commit `17077ad` (batch size raised to 2048, `docker-compose.yml`), re-verified per-item in Tasks 5.1/5.2/6.1/6.2/7.1's verification steps.

## Out-of-Scope

- Exhaustive ingestion of the full municipal news/acts archive.
- A paginating crawler for `halleyweb.com`.
- Any `RagEngine`/retrieval/generation code change.
- Operator credential rotation.
