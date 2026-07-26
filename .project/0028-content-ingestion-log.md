# Plan 0028 — Content Ingestion Log

Tracks every real, sourced document ingested by Plan 0028 (styled after `TEST-INGESTION-0001.md` §5.3). All `news`/`delibere`/`giunta` content is manually uploaded (feature 0009, preview→confirm) — `halleyweb.com` disallows scraping entirely via `robots.txt` (confirmed `TEST-INGESTION-0001.md` §5.2).

## Persona

| Date | Change | Verified |
|---|---|---|
| 2026-07-26 | Created and activated persona `gaspare` (id 6, version 2), replacing active `spontini-bot` v1. `fallback_message` set to the exact ADR 0012 string, closing a pre-existing compliance gap (the outgoing `spontini-bot` persona's fallback text did not match). | ☑ smoke-tested: grounded question ("Quando è nato Gaspare Spontini?") answered correctly, cited `document_id` 22/4; ungrounded question ("Chi sei?") correctly falls back with the exact ADR 0012 string (Constitution §5 — no self-description without a KB source) instead of citing unrelated stray documents, as the pre-existing `spontini-bot` persona incorrectly did. |

## storia (verified, ingested pre-plan by `TEST-INGESTION-0001`)

| # | Title/subject | URL or file | Method | Verified in KB (this plan) |
|---|---|---|---|---|
| 1 | Storia del Comune (official page) | comune.maiolatispontini.an.it/.../storia-comune | manual upload | ☑ |
| 2 | Maiolati Spontini (Wikipedia) | it.wikipedia.org/wiki/Maiolati_Spontini | scrape | ☑ |
| 3 | Gaspare Spontini (Wikipedia) | it.wikipedia.org/wiki/Gaspare_Spontini | scrape | ☑ |
| 4 | Organi politico-amministrativi (roster, pre-plan snapshot) | halleyweb.com/.../organi-politico-amministrativo/... | manual upload | ☑ |

See `TEST-INGESTION-0001.md` §5.3 for the original ingestion record.

## news (Feb–Jul 2026)

| # | Title | Date | Source URL | document_id(s) | Verified |
|---|---|---|---|---|---|
| 1 | L'associazione Auser Media Vallesina taglia il traguardo dei vent'anni di attività | 24/02/2026 | halleyweb.com/.../mostra_news.php?id=1113 | 31, 32 | ☑ |
| 2 | ASSEGNO DI MATERNITA' ANNO 2026 | 12/02/2026 | halleyweb.com/.../mostra_news.php?id=1108 | 33 | ☑ |
| 3 | Il Comune punta a dotarsi di defibrillatori | 17/03/2026 | halleyweb.com/.../mostra_news.php?id=1120 | 34, 35 | ☑ smoke-tested, correctly cited |
| 4 | Servizio mensa e trasporto scolastico A.S. 2026-2027 - modulistica | 24/03/2026 | halleyweb.com/.../mostra_news.php?id=1125 | 36 | ☑ |
| 5 | Giornata della Terra: nuovo logo del progetto Piedibus | 22/04/2026 | halleyweb.com/.../mostra_news.php?id=1141 | 37, 38 | ☑ |
| 6 | Il Consiglio approva la messa a norma degli stadi; fotovoltaico ex discarica | 28/04/2026 | halleyweb.com/.../mostra_news.php?id=1143 | 39, 40 | ☑ |
| 7 | Personale del Comune, pensionamenti e nuovi arrivi | 05/05/2026 | halleyweb.com/.../mostra_news.php?id=1145 | 41 | ☑ |
| 8 | Bando Servizi Digitali Integrati - DigitalizziAMO Maiolati Spontini | 11/05/2026 | halleyweb.com/.../mostra_news.php?id=1150 | 42 | ☑ |
| 9 | Insediamento del Consiglio e nomina della Giunta comunale (Romagnoli vicesindaco) | 06/06/2026 | halleyweb.com/.../mostra_news.php?id=1155 | 43, 44 | ☑ |
| 10 | Sindaco e assessori illustrano le priorità del mandato amministrativo | 13/06/2026 | halleyweb.com/.../mostra_news.php?id=1159 | 45, 46 | ☑ smoke-tested, correctly cited (vicesindaco question) |
| 11 | Finanziamento 20.000€ bando "Città che legge 2025" | 02/07/2026 | halleyweb.com/.../mostra_news.php?id=1169 | 47 | ☑ |
| 12 | Il patrimonio di Gaspare Spontini conquista due nuovi riconoscimenti | 10/07/2026 | halleyweb.com/.../mostra_news.php?id=1175 | 48, 49 | ☑ |

## delibere (Apr–Jul 2026)

| # | Type | Number | Title | Date | Source URL | document_id(s) | Verified |
|---|---|---|---|---|---|---|---|
| 1 | Delibera di Giunta | 74 | Modifica disposizione posteggi area Fiera Sant'Anna 2026 | 13/07/2026 | halleyweb.com/.../delibere/dettaglio/atto/GTlRFekE9RT0-H (PDF: delibera copia uso amministrativo.pdf) | 50-55 | ☑ smoke-tested, correctly cited |
| 2 | Delibera di Giunta | 73 | Campi di calcio "M. Pierucci" e "Grande Torino" - approvazione linee di indirizzo per affidamento gestione | 07/07/2026 | halleyweb.com/.../delibere/dettaglio/atto/GTlRFekE1Zz0-H | 56-66 | ☑ |
| 3 | Delibera di Giunta | 72 | Campo di calcio "G. Scirea" Maiolati Spontini - approvazione linee di indirizzo per affidamento gestione | 07/07/2026 | halleyweb.com/.../delibere/dettaglio/atto/GTlRFekE1Yz0-H | 67-76 | ☑ |
| 4 | Delibera di Giunta | 71 | Approvazione stato attuazione dei programmi esercizio 2026 - schema DUP 2027-2029 | 07/07/2026 | halleyweb.com/.../delibere/dettaglio/atto/GTlRFekU1TT0-H | 77-81 | ☑ |
| 5 | Delibera di Giunta | 70 | Bilancio consolidato esercizio 2025 - individuazione componenti del "Gruppo Comune di Maiolati Spontini" | 07/07/2026 | halleyweb.com/.../delibere/dettaglio/atto/GTlRFekU1ST0-H | 82-90 | ☑ |
| 6 | Determina | Reg. Gen. 455 | Servizi di garanzia 36 mesi per nuovo ponte radio | 24/07/2026 | halleyweb.com/.../determine/dettaglio/atto/GTlRFMEE5TT0-H | 91-99 | ☑ smoke-tested, correctly cited |
| 7 | Determina | Reg. Gen. 454 | Lavori di tinteggiatura interna e sistemazione infissi | 23/07/2026 | halleyweb.com/.../determine/dettaglio/atto/GTlRFME61Yz0-H | 100-107 | ☑ |
| 8 | Determina | Reg. Gen. 453 | Sistema culturale integrato - progetto Un fiume di cultura | 22/07/2026 | halleyweb.com/.../determine/dettaglio/atto/GTlRFME61UT0-H | 108-114 | ☑ |
| 9 | Determina | Reg. Gen. 452 | Realizzazione dossi rallentatori di velocità in conglomerato | 22/07/2026 | halleyweb.com/.../determine/dettaglio/atto/GTlRFME61ST0-H | 115-117 | ☑ |
| 10 | Determina | Reg. Gen. 443 | Affidamento del servizio di manutenzione per i moduli Backoffice SUE | 21/07/2026 | halleyweb.com/.../determine/dettaglio/atto/GTlRFMEU1Zz0-H | 119-127 | ☑ ingested after the chunking.rs UTF-8 fix (see below) |

## giunta (new section)

| # | Title | Source URL | document_id(s) | Verified |
|---|---|---|---|---|
| 1 | Giunta e Consiglio Comunale di Maiolati Spontini (composizione, deleghe) | halleyweb.com/.../organi-politico-amministrativo/index/index/categoria/78 | 118 | ☑ smoke-tested — retrieved and cited correctly (names/deleghe grounded, no hallucination; minor generation-phrasing imprecision noted, out of scope per plan Non-Goals — no RagEngine/generation change) |

## Real bug found and fixed during Task 6.2

`ingest-core/src/chunking.rs` panicked (`start byte index N is not a char boundary`) when a chunk's overlap-window byte offset landed inside a multi-byte UTF-8 character (curly quotes `"` `"`, common in real PDF-extracted text) — reproduced live uploading determina 443 (Reg. Gen. 443/2026), crashed the backend's confirm-upload request (`curl: (52) Empty reply from server`), backend process itself stayed up. Root cause: raw byte-index string slicing in both the inter-chunk overlap logic and `split_long_paragraph`'s long-paragraph splitter, neither char-boundary-safe. Fixed with a `floor_char_boundary` helper (walks back to the nearest valid UTF-8 boundary) applied at both slice sites, TDD'd with two new regression tests (`should_not_panic_when_overlap_boundary_falls_inside_multi_byte_char`, `should_not_panic_when_long_paragraph_split_boundary_falls_inside_multi_byte_char`). `ingest-core/src/chunking.rs`. No ADR — a straightforward bug fix to already-tested code, same precedent as `TEST-INGESTION-0001`'s scheduler/config-reload fixes (commit `17077ad`).

## Real finding: RAG_MIN_SCORE permissiveness now empirically confirmed at the larger corpus size (out of scope to fix, Plan 0028 Non-Goals)

`TEST-INGESTION-0001.md` Appendix C flagged "retrieval threshold known to be potentially permissive" as a risk, unresolved at that time for lack of data. With the KB now at 119 real documents (post-population), three deliberate categorical-refusal probes were run live:

- "Chi sarà il prossimo sindaco di Maiolati Spontini nel 2031?" (future prediction)
- "Che tempo farà domani a Maiolati Spontini?" (weather)
- "Qual è la ricetta ufficiale della paella valenciana?" (fully unrelated control)

All three retrieved 2 chunks (`RAG_TOP_K=2`) above `RAG_MIN_SCORE=0.6` and returned `fell_back:false` with citations, even the fully-unrelated control question. In all three cases the model still declined to answer, in its own honest words ("Non ho trovato l'informazione...") — **no hallucination occurred in any case**, so Constitution §5's core no-hallucination guarantee held. But none matched ADR 0012's exact mandated fallback string, and none went through the true zero-chunk fallback path, because at this corpus size top-2-nearest-neighbor retrieval apparently always clears 0.6 regardless of query relevance. This is a real, reproducible confirmation of the exact risk ADR 0012's "Consequences — Negative" section already anticipated ("the retrieval-fallback mechanism... does not itself understand this is a future-prediction question"). Fixing `RagEngine`/`RAG_MIN_SCORE`/`RAG_TOP_K` is explicitly out of scope for Plan 0028 (Non-Goals) — flagged here as a candidate follow-up plan, not fixed.

## Final gate (`make verify`)

Run 2026-07-26 against the fully populated KB (persona `gaspare`, storia, 12 news, 10 delibere/determine, giunta):

- `build`: pass (all 6 service images build cleanly).
- `test`: pass — 157 backend unit tests + full BDD suite (all scenarios/steps) + `ingest-core` regression tests for the chunking fix, all green.
- `lint`: pass (clippy, `vue-tsc --noEmit` for frontend and admin-ui).
- `fmt-check`: pass, after fixing 2 real `cargo fmt` violations (the new chunking.rs code, and one pre-existing unformatted line in `ingest/src/scheduler.rs` unrelated to this plan) directly on the host — `docker compose run --rm` containers have no bind mount back to host source, so a containerized `cargo fmt` write is a no-op on disk; fixes must be applied via the host filesystem directly.
- `coverage`: fails — `error: no such command: tarpaulin`. Confirmed pre-existing, long-documented infra gap: `cargo-tarpaulin` has been missing from the `backend` build-stage image since feature 0009, and every single feature plan since (0011, 0012, 0013, 0014, 0015, 0018, 0020, 0026, 0027 — see their review files) has hit this identical failure, confirmed it predates their branch, and proceeded with coverage verified by other means instead of fixing the Dockerfile. Consistent with that established precedent, not fixed here — out of scope for a content-population plan.
- `compose-config`: pass (run directly, since `make verify`'s prerequisite chain stops at the first failure).
- `a11y`: pass (run directly) — 0 accessibility errors across every `frontend` and `admin-ui` route.

## Round-trip tooling verification

| Date | Action | Result |
|---|---|---|
| 2026-07-26 | `make eject-data` → uploaded 1 throwaway test document (count 30→31) → `make use-data DATA_FILE=.data/data-2026-07-26.bin` | Count restored to 30 exactly; backend restarted cleanly against the restored volume; operator credential still valid post-restore. |
