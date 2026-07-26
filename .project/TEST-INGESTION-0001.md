# Ultra-Detailed Test Plan — Ingestion and Answer Quality

- **Status**: draft — ready for execution
- **Created**: 2026-07-25
- **Owner**: TBD (operator + agent)
- **Scope**: the public knowledge base of Spontini Bot (delibere/determine, news, comune history, bot imprinting) and the end-to-end quality of `/chat` answers.
- **This is not** a Feature Plan under the `/create-plan` lifecycle (it doesn't introduce new code by itself); it's an operational playbook. If execution surfaces architectural gaps (see Appendix C), those should become separate features in `ROADMAP.md`.

## 0. Guiding Principle

> "Don't invent anything. Don't hallucinate."

This applies at two levels, both binding for this plan:

1. **The bot must never answer with facts not anchored in the knowledge base** — this is the [Knowledge Base Rule](../docs/CONSTITUTION.md) (§5): *"Spontini MUST only answer from documents stored in the KB. If the answer is not found in any document, Spontini MUST explicitly say so. No hallucination, no extrapolation."* This is a constitutional constraint, not a quality goal: tolerance for hallucination is **zero**, not "as low as possible".
2. **Whoever writes this plan and whoever executes the tests must not invent facts, URLs, or unverified document numbers.** Every source URL cited in this document was verified via web fetch on 2026-07-25 (see Appendix B). Every "concrete" question in the test set is either (a) anchored to a fact verified below, or (b) explicitly marked as a **TEMPLATE** to be filled in after ingestion with the real title/content of the actually-indexed document — never with imagined content.

## 1. Relevant Architecture (for whoever runs the tests)

Minimal summary, verified against the real code — do not treat this section as self-updating: if the code changes, update this summary too.

- **`/chat` flow**: `POST /chat {"question": "..."}` → `RagEngine` (`backend/src/rag_engine`) → embeds the question (`llama-embed`, `nomic-embed-text` model, 768 dims) → cosine-similarity retrieval against `kb.db` (libSQL) with `top_k=5`, `min_score=0.35` (defaults, overridable via env `RAG_TOP_K` / `RAG_MIN_SCORE`, see `backend/src/config.rs`) → **if zero chunks are retrieved above threshold → honest fallback, the generation model is never even called** (Constitution §5, guaranteed at the code level, not just by the prompt) → otherwise a 3-part prompt with separate sections (persona / retrieved context / question) → `llama-generate` (Qwen2.5-3B-Instruct, Q4_K_M, ADR 0001) → answer.
- **Response shape**: `{ answer: string, sources: [{document_id, source_ref}], fell_back: boolean }`. When `fell_back=true`, `sources` is always empty and `answer` is the active persona's `fallback_message`, verbatim.
- **No deterministic extractors.** Unlike the "parent" project (`/Users/massimobiagioli/github/massimobiagioli/spontini`, see §2), **every** answer here goes through the 3B model: there is no regex/lookup layer that answers point facts (numbers, names, amounts, dates) with a guaranteed shortcut. This has two direct consequences for the test plan:
  - **latency is uniform and non-trivial across all 1000 questions**, not just on a "hard" subset;
  - **hallucination risk is uniform**: even a trivial question like "how many residents does the comune have?" depends entirely on (a) the right chunk being retrieved and (b) the 3B model reporting it faithfully, with no safety shortcut.
- **Imprinting (persona)**: `backend/src/admin` exposes `POST /admin/api/persona` (creates a new version: `name`, `system_prompt`, `tone`, `fallback_message`, `activate`), `POST /admin/api/persona/:id/activate`, `POST /admin/api/persona/reload`. The final prompt keeps persona/context/question as three separate parts (never concatenated into one free-form block).
- **Ingestion**: sections (`ingest_section`: name + ordering) and sources (`ingest_source`: `section_id`, `source_type` `scrape`|`api`, `url`, `enabled`). Only `scrape` is actually wired: **a single HTTP GET on one URL + visible-text extraction**, no crawler, no pagination, no link-following (`ingest-core/src/scraper.rs`, feature 0005). The `api` adapter exists only as a stub, always `enabled=false`. Chunking: 512 tokens / 64 overlap (`ingest-core/src/chunking.rs`). As an alternative to scheduled scraping there's manual per-section upload (`POST /admin/api/upload` → preview → `POST /admin/api/upload/confirm/:token`, feature 0009), which accepts pdf/docx/md/txt with a mandatory preview before indexing.
- **Auth**: since feature 0027, every write to `/admin/api/*` requires a session (cookie `session=...`, obtained via `POST /admin/api/auth/login {"username","password"}`), and is recorded in `audit_log`. See Appendix A for commands.
- **Training session** (features 0012-0014): `POST /admin/api/training/sessions {"title","created_by"}` creates a session; `POST /admin/api/training/sessions/:id/messages {"question"}` makes exactly the same call as `/chat` (same `RagEngine::answer`, same DTO) but **persists** the whole exchange (`question`, `answer`, `sources` JSON, `fell_back`, `created_at`) in `kb.db`. This is the mechanism we'll use to run and record the 1000 questions (§8), instead of an ad-hoc script that only talks to `/chat` and leaves no persistent trace.
- **Point-in-answer feedback** (features 0014/0018): `POST /admin/api/training/feedback {"message_id","chunk_id","answer_span","sentiment":"positive"|"negative","comment"}` — useful to annotate specific spans of a problematic answer, in addition to the holistic score we'll keep in the external log (§9).

## 2. Critical Difference vs. the "Parent" Project (`spontini`)

The project used as inspiration for the questions (`/Users/massimobiagioli/github/massimobiagioli/spontini`) has a **different, not directly comparable** architecture:

| | `spontini` (parent) | `spontini-bot-2` (this project) |
|---|---|---|
| Point facts (names, amounts, dates, delegations) | Deterministic extractors (regex + lookup), bypass the LLM | Always generic RAG + 3B LLM, no shortcut |
| Typical latency for an "easy" question | 0.1–0.3 s (extractor) | On the order of tens of seconds (hardware-dependent — to be measured, §10.3) |
| Typical latency for an "open"/narrative question | 20–58 s (3B LLM, llama3.2) | Same order of magnitude expected (Qwen2.5-3B, different hardware — don't assume it's the same, measure it) |
| Hallucination risk | Concentrated on questions not covered by an extractor | Uniform across all questions |
| Answer rendering | `v-html` of raw markdown/HTML generated by the LLM | Structured DTO `{answer, sources[], fell_back}`, citations are never parsed out of markdown/HTML (Milestone 4 constraint, feature 0021) |

**Implication for this plan**: the parent project's report (`reports.md`, 29 edge cases + 113 regression questions, 0-100 scoring) is a great source of **inspiration for questions and for the scoring methodology**, but its **baseline numbers (timings, pass rates) don't transfer**: they must be re-measured from scratch here (§10.3). The **specific facts** that appear in the parent report (e.g. council composition, assessore names, amounts from specific determine) also **must not be reused as "expected answer"** in this plan: that KB contained different documents than the ones we'll ingest here. Every question in this plan with a concrete expected answer is either anchored to a fact verified in Appendix B, or explicitly marked `[TO BE COMPLETED POST-INGESTION]`.

## 3. Phase 0 — Environment Prerequisites

- [x] **0.1** Stack up: `make up` (all 6 containers `healthy`, verify with `docker compose ps`). Verified 2026-07-26: all 6 containers up (`backend`, `admin-ui`, `frontend`, `llama-embed`, `llama-generate` reporting `(healthy)`; `ingest` has no container-level healthcheck defined in `docker-compose.yml` — confirmed `running`, consistent with the known gap noted in Appendix C).
- [x] **0.2** Models provisioned: `make provision-models` (nomic-embed-text + Qwen2.5-3B-Instruct Q4_K_M downloaded into `./models/`). Ran 2026-07-26 — both files already present (`models/embed/nomic-embed-text-q4.gguf`, `models/generate/qwen2.5-3b-instruct-q4_k_m.gguf`), script reported "already present" for both.
- [x] **0.3** Operator credential set: `make set-operator-credential USERNAME=operator` (interactive password prompt — **do not put the password in plain text in this file or in logged commands**). Ran 2026-07-26 via `docker compose run --rm -i backend cargo run --bin set-operator-credential -- --username operator --output /data/operator-credential.json` (password generated with `openssl rand -base64 24`, piped via stdin, kept only in a scratchpad file outside the repo, never logged). This overwrote a pre-existing `operator-credential.json` from an earlier, unrelated session (see 0.5) — its password was unknown, so a fresh credential was required to authenticate.
- [x] **0.4** Login and save the session for later commands:
  ```bash
  curl -sS -c /tmp/spontini-session.txt -X POST http://localhost:8080/admin/api/auth/login \
    -H 'Content-Type: application/json' \
    -d '{"username":"operator","password":"<password chosen in step 0.3>"}'
  # every subsequent admin call: curl -sS -b /tmp/spontini-session.txt ...
  ```
  Verified 2026-07-26: login returned `{"status":"logged_in"}` (HTTP 200), cookie saved to `/tmp/spontini-session.txt`, and an authenticated call (`GET /admin/api/persona?name=gaspare`) succeeded (HTTP 200).
- [x] **0.5** Verify `kb.db` is empty or in a known state before starting (avoid mixing prior test data with this campaign). If it isn't empty, record the starting state here.

  **Starting state recorded 2026-07-26** (`kb.db`, via `docker run --rm -v spontini-bot-2_kb-data:/data alpine sh -c 'sqlite3 ...'`) — **not empty**, contains leftover data from unrelated prior work, none of it belonging to this campaign:
  - `documents`: 2 rows, `source='manual'` — office-hours content ("Lo sportello anagrafe è aperto dal..."), unrelated to storia/news/delibere.
  - `persona`: 4 rows — id 1 `spontini` (inactive), id 2/3 `gaspare-e2e` (inactive, from the BDD e2e test suite), id 4 `gaspare` (**active**, `system_prompt: "Sei Gaspare Spontini, compositore."`, created 2026-07-25 11:35:59). None of these is the `spontini-bot` persona Phase 1 will create.
  - `ingest_section`: 0 rows. `ingest_source`: 0 rows. `training_session`: 0 rows. — clean for this campaign's purposes.
  - `audit_log`: 1 row (`create_persona` for persona id 4, by `operator`, 2026-07-25 11:35:59).
  - **Risk flagged for Phase 2/3**: the 2 stray `manual` documents remain retrievable by `RagEngine` and were not cleared (out of this session's scope — Phase 0 only records state, it doesn't mandate cleanup). Whoever runs Phase 2/3 should decide whether to delete them first, since they could surface as an unexpected citation on an unrelated question (relevant to Category F/G scoring, §9).

## 4. Phase 1 — Bot Imprinting (persona derived from Gaspare Spontini)

**Verified facts** (Italian Wikipedia, fetched 2026-07-25 — see Appendix B):

- Gaspare Luigi Pacifico Spontini, born in Maiolati on **14 November 1774**, died in Maiolati on **24 January 1851**.
- Italian composer and conductor, representative of Classicism.
- The name "Spontini" was added to the comune's name (then "Maiolati") in **1939**, in his honor.
- The comune has **5,916 residents** (figure reported in the Wikipedia article's opening line — to be reconfirmed at ingestion time, it may be outdated).

Other biographical facts (Conservatorio della Pietà dei Turchini in Naples, career in Paris and Berlin, relationship with Napoleon and Friedrich Wilhelm III of Prussia) are **plausible and well-documented historically** (they also appear in the parent project's question set), but must be **reconfirmed against the sources actually ingested in Phase 2** before being treated as certain facts in the `system_prompt` — the persona's `system_prompt` is not a KB document, but it shapes the bot's tone and identity, so it must not introduce facts the KB can't later back up with a citation.

- [x] **1.1** Create the initial persona (note: `system_prompt`, `tone`, and `fallback_message` are runtime strings shown to citizens, so they stay in Italian — the surrounding curl command and comments stay in English):
  ```bash
  curl -sS -b /tmp/spontini-session.txt -X POST http://localhost:8080/admin/api/persona \
    -H 'Content-Type: application/json' \
    -d '{
      "name": "spontini-bot",
      "system_prompt": "Sei SpontiniBot, l'\''assistente digitale del Comune di Maiolati Spontini. Il tuo nome richiama Gaspare Spontini (1774-1851), compositore nato e morto a Maiolati, in cui onore il paese aggiunse il suo nome nel 1939. Rispondi ai cittadini in italiano, in modo cordiale, conciso e sempre onesto: usa esclusivamente le informazioni contenute nei documenti ufficiali del Comune che ti vengono forniti come contesto. Se un'\''informazione non è presente nel contesto fornito, dillo chiaramente invece di inventare o supporre.",
      "tone": "cordiale, istituzionale ma accessibile, mai burocratico",
      "fallback_message": "Non ho trovato questa informazione nei documenti ufficiali del Comune che conosco al momento. Ti consiglio di contattare direttamente gli uffici comunali per una risposta certa.",
      "activate": true
    }'
  ```
  *(note: the apostrophes inside the Italian strings need correct bash escaping — validate the JSON with `jq` before sending, or build it in a file and use `-d @persona.json` to avoid quoting mistakes.)*

  Ran 2026-07-26: built `persona.json` in the scratchpad (validated with `python3 -m json.tool`) and posted it with `-d @persona.json`. Response: `{"id":5,"version":1,"name":"spontini-bot", ... ,"is_active":true,"created_at":"2026-07-26 06:54:12","created_by":"operator"}`, HTTP 201.
- [x] **1.2** Verify activation: `GET /admin/api/persona?name=spontini-bot` → version 1 must have `is_active=true`. Verified 2026-07-26: `GET /admin/api/persona?name=spontini-bot` (HTTP 200) returned exactly one row, `id:5, version:1, is_active:true`.
- [x] **1.3** Run one identity smoke-test question (see Category A, §7) to confirm the tone behaves as expected, **before** proceeding with bulk ingestion.

  Ran 2026-07-26: `POST /chat {"question":"Chi sei?"}` (public endpoint, no ingested storia/news/delibere content yet) → HTTP 200 in 22.65s:
  ```json
  {"answer":"Sei SpontiniBot, l'assistente digitale del Comune di Maiolati Spontini.","sources":[{"document_id":2,"source_ref":"Orari sportello anagrafe.md"},{"document_id":1,"source_ref":"orari.txt"}],"fell_back":false}
  ```
  Tone confirmed as expected (first-person, cordial, concise, matches the persona's `system_prompt`). **Real anomaly observed and recorded, not fixed in this session (out of Phase 1 scope)**: `fell_back:false` with two `sources` cited — but those sources are exactly the two stray leftover `manual` documents flagged as a risk in §0.5 (office-hours content, `id:1`/`id:2`), not anything that grounds the identity claim actually made. This is a real, reproducible instance of the "irrelevant citation" failure mode the scoring rubric penalizes (§9, "Citation correctness") and the exact risk anticipated in Appendix C ("Retrieval threshold known to be potentially permissive", `RAG_MIN_SCORE`). Should be addressed — via the §10.2 `RAG_MIN_SCORE` lever and/or clearing the stray documents — before Phase 3.4's smoke test and before Wave 1 scoring, or it will depress Category A's citation-correctness score from question one.

## 5. Phase 2 — Ingestion Preparation

### 5.1 Sections

Create the three required sections, using the conventional names already used in project documentation (`delibere`, `news`, `storia` — see `ROADMAP.md` feature 0016):

```bash
curl -sS -b /tmp/spontini-session.txt -X POST http://localhost:8080/admin/api/ingest/config/sections \
  -H 'Content-Type: application/json' -d '{"name":"storia","ordering":10}'
curl -sS -b /tmp/spontini-session.txt -X POST http://localhost:8080/admin/api/ingest/config/sections \
  -H 'Content-Type: application/json' -d '{"name":"news","ordering":20}'
curl -sS -b /tmp/spontini-session.txt -X POST http://localhost:8080/admin/api/ingest/config/sections \
  -H 'Content-Type: application/json' -d '{"name":"delibere","ordering":30}'
```

Ran 2026-07-26: all 3 sections created successfully — `storia` (`id:1`), `news` (`id:2`), `delibere` (`id:3`), all HTTP 201.

### 5.2 Architectural Gap to Know Before Configuring Sources

The scraper (`ingest-core`) does **a single GET on a single URL** and extracts its visible text — it is not a crawler, it doesn't paginate, it doesn't follow links. The real pages on the comune's site (verified in Appendix B) have different structures:

- The **"Storia del Comune"** page (`https://www.comune.maiolatispontini.an.it/c042023/zf/index.php/storia-comune`) is **a single static page with full narrative text** → a perfect fit for a single `scrape source`.
- The **news list** page (`https://www.halleyweb.com/c042023/po/elenco_news.php?area=H`) is **a list of teasers/titles**, not the full text of every article → scraping just that one page isn't enough to ingest "the last 3 months of news" usefully: it would only give short titles/excerpts, not the content needed to answer detail questions.
- The **determine** page (`https://www.halleyweb.com/c042023/zf/index.php/atti-amministrativi/determine`) is a **paginated search engine** (1,401 pages as of 2026-07-25), not a single-shot scrapeable list. Individual delibere/determine do have detail-page permalinks though (observed pattern: `.../atti-amministrativi/delibere/dettaglio/atto/<id>/...`), which **are** single-page-scrapeable.

**Operational decision (to execute, not to invent):**

1. **`storia`**: 1 scrape source on the official "Storia del Comune" page + optionally 1-2 more scrape sources on the verified Wikipedia pages (`https://it.wikipedia.org/wiki/Maiolati_Spontini`, `https://it.wikipedia.org/wiki/Gaspare_Spontini`) — all single pages, a good fit for the scraper as it stands today.
2. **`news`** and **`delibere`**: **do not** rely on scraping the list page automatically. The operator must:
   - browse the real listing (news: the list page above; delibere/determine: the "atti amministrativi" search engine, filtered by date, last 3 months),
   - for each real document found, **either** add its permalink as an individual `scrape source` in the section (if the site exposes a textual detail page), **or**, if the document is a PDF/attachment, download it and use the **manual per-section upload** (`POST /admin/api/upload`, feature 0009 — still goes through preview→confirm, so the operator sees the extracted text before it's indexed).
   - This is slower than just pointing the scraper at one URL, but with the current code it's the only way not to end up ingesting empty teasers/lists instead of the actual document text.
3. If the volume of documents from the last quarter is large (the site normally shows several dozen determine/news per quarter), **ingesting 100% isn't necessary**: covering a representative sample (e.g. 15-25 delibere/determine and 15-25 news items from the last 3 months, chosen for topical variety — contracts, budget, roadworks, registry office, events, grants) is enough for a quality test; the goal is having enough real, varied documents to cover every question category in §7, not ingesting the entire municipal archive.
4. Record here, as you go, the real list of ingested documents (title, URL or filename, section, document date) — this list is the basis for filling in the `[TO BE COMPLETED POST-INGESTION]` placeholders in the question set in §7.

```bash
# example, storia (direct scrape, single page):
curl -sS -b /tmp/spontini-session.txt -X POST http://localhost:8080/admin/api/ingest/config/sources \
  -H 'Content-Type: application/json' \
  -d '{"section_id": <storia section id>, "source_type":"scrape", "url":"https://www.comune.maiolatispontini.an.it/c042023/zf/index.php/storia-comune", "enabled": true}'
```

**Correction found 2026-07-26** (real discrepancy vs. this file's own text and vs. Appendix A, which claims to be "verified against `backend/src/lib.rs`"): `create_source`'s actual handler (`backend/src/admin/ingest_config/handlers.rs:167-172`) takes `section_id` as a **query parameter**, not a JSON body field — the body call above returns `400 Failed to deserialize query string: missing field 'section_id'`. The real, working shape is:
```bash
curl -sS -b /tmp/spontini-session.txt -X POST "http://localhost:8080/admin/api/ingest/config/sources?section_id=1" \
  -H 'Content-Type: application/json' \
  -d '{"source_type":"scrape", "url":"https://www.comune.maiolatispontini.an.it/c042023/zf/index.php/storia-comune", "enabled": true}'
```

**Session findings 2026-07-26 — Phase 3 blocked, do not repeat this without first fixing the code.** All 3 `storia` sources (official page + 2 Wikipedia pages) were created for real using the corrected call above, and immediately surfaced two serious, independently-verified bugs (both added to Appendix C):

1. `ingest/src/scheduler.rs`'s `run_interval.tick()` branch (lines 107-118) calls `runner.run_all(&config.sources)` **unconditionally every `RUN_POLL_SECS` (10s)** the moment any source exists — with no relation to `POST /admin/api/ingest/run` and no backoff on failure. Within seconds of creating the 3 sources, `ingest` started hammering `it.wikipedia.org` and the comune's official site with a fresh GET every ~10 seconds, indefinitely.
2. Deleting all 3 sources (`DELETE /admin/api/ingest/config/sources/:id`, confirmed via direct `kb.db` query that `ingest_source` was empty) **did not stop the loop** — `ingest` kept re-running the pipeline against the deleted sources for several `CONFIG_POLL_SECS` cycles (2.5+ minutes observed), meaning `ConfigWatcher` is not reliably propagating config changes into the running scheduler. Had to `docker compose stop ingest` to guarantee the external hammering actually stopped, then `docker compose start ingest` (clean restart, config reloaded fresh from `kb.db` with 0 sources, no lock error, no further scraping — verified in logs).

On top of the above, the 3 individual pipeline runs that did execute before the container was stopped **all failed anyway**, for reasons unrelated to the scheduler bug — real, reproducible content/config problems (see Appendix C):
- Official "Storia del Comune" page: `robots.txt: path /c042023/zf/index.php/storia-comune is disallowed by robots.txt` — contradicts this section's own "perfect fit for a single scrape source" assumption (§5.2 point 1). The site's robots.txt disallows exactly this path.
- Both Wikipedia pages: `embedding error: HTTP 500 ... input (678 tokens) is too large to process. increase the physical batch size (current batch size: 512)` — `ingest-core`'s chunker is configured for 512-token chunks (`chunk_size=512`), but the real chunks produced from these pages' actual text came out to 678 and 659 tokens by the embed model's own tokenizer, exceeding `llama-embed`'s compiled/configured batch size of 512.

**Net effect (as first observed)**: with the code as it stood, none of `storia`'s 3 candidate sources could be ingested without a code fix first.

**Resolved 2026-07-26, commit `17077ad`**, after explicit user go-ahead to fix the code (not just document the blocker): (a) the scheduler bug fixed — gated behind real run requests instead of an unconditional timer; (b) the config-reload bug fixed — `ConfigLoader` reuses one long-lived connection instead of reopening a new one every poll; (c) `llama-embed`'s batch size raised to 2048; (d) the robots.txt-blocked official page ingested via manual upload instead of scraping. All 4 re-verified live with real traffic — see §5.3 and Appendix C for the fix details and evidence, and Phase 3 (§6) for the successful re-run. No ADR was written for the scheduler/config-reload fixes; these were treated as straightforward bug fixes to already-tested code paths (the DB-layer run-request methods existed and were tested; the scheduler just never called them) rather than a architectural decision reversal — reconsider if the user wants one on reflection.

### 5.3 Ingested Documents Log (to be filled in during execution)

| # | Section | Real title/subject | URL or file | Document date | Method (scrape/upload) | Verified in KB? |
|---|---|---|---|---|---|---|
| 1 | storia | Storia del Comune (official page) | comune.maiolatispontini.an.it/.../storia-comune, uploaded as `storia-comune.md` (real text fetched 2026-07-26, robots.txt blocks scraping it directly) | n/a (static page) | manual upload (feature 0009, preview confirmed real content before confirming) — **ingested 2026-07-26**, 3 chunks, document_ids 27-29 | ☑ |
| 2 | storia | Maiolati Spontini (Wikipedia) | it.wikipedia.org/wiki/Maiolati_Spontini | n/a | scrape — **ingested 2026-07-26** (after the scheduler/config/batch-size fixes, commit `17077ad`), 9 chunks | ☑ |
| 3 | storia | Gaspare Spontini (Wikipedia) | it.wikipedia.org/wiki/Gaspare_Spontini | n/a | scrape — **ingested 2026-07-26** (after the fixes), 15 chunks | ☑ |
| … | news | *(to fill in — real curation not yet done, §5.2 point 2)* | | | | ☐ |
| … | delibere | *(to fill in — real curation not yet done, §5.2 point 2)* | | | | ☐ |

## 6. Phase 3 — Running the Ingestion and Verifying It

> **UNBLOCKED 2026-07-26.** The 3 real bugs found earlier this session (unconditional timer-driven scraping, stale config reads, embedding batch-size mismatch) were fixed in code — `ingest/src/scheduler.rs`, `ingest/src/config.rs`, `ingest/src/main.rs`, `docker-compose.yml` (commit `17077ad`) — and re-verified live before re-attempting this phase: a configured source now sits idle with zero requests until a real run is triggered; a run request correctly goes `pending → running → done`; swapping a source and waiting past `CONFIG_POLL_SECS` picks up the change; embedding no longer errors on real chunk sizes. Full incident + fix writeup in Appendix C. The robots.txt block on the official storia page is a real, separate constraint (not a bug) — worked around via manual upload (feature 0009) per §5.2's own allowance for non-scrapeable documents.
>
> **Scope note**: only `storia` has real content as of this session (2 scrape sources + 1 manual upload, see §5.3). `news` and `delibere` still have 0 sources — that's real curation work (browsing the live listings, picking representative documents) not yet done, tracked separately, not a blocker of Phase 3's mechanics. 3.1-3.4 below are verified against `storia`; the same mechanism now works and will apply to `news`/`delibere` once documents are selected for them.

- [x] **3.1** Trigger: `curl -sS -b /tmp/spontini-session.txt -X POST http://localhost:8080/admin/api/ingest/run` → 202 + run `id`. Ran 2026-07-26 three times for real (once for each of the 2 `storia` scrape sources, individually, while verifying the fix): run `id:1` (Gaspare_Spontini) and run `id:2` (Maiolati_Spontini) both returned `202` with a `pending` run request.
- [x] **3.2** Poll: `curl -sS -b /tmp/spontini-session.txt http://localhost:8080/admin/api/ingest/run/<id>` until `status` is `done` (or `failed` — in that case check `docker compose logs ingest` for the cause: robots.txt, disallowed content-type, timeout, etc.). Verified 2026-07-26: both run `id:1` and `id:2` transitioned `pending → running → done` for real, confirmed via repeated polling and cross-checked against `ingest` logs (`run request N consumed: triggering pipeline for 1 sources`).
- [x] **3.3** Real quantitative verification (there's no dedicated admin endpoint to "count documents" — known gap, Appendix C). Method already verified in this very working session:
  ```bash
  docker run --rm -v spontini-bot-2_kb-data:/data alpine sh -c \
    "apk add --no-cache sqlite >/dev/null && sqlite3 /data/kb.db \
     \"SELECT source, count(*) FROM documents GROUP BY source;\""
  ```
  Ran 2026-07-26, real result: `manual: 5` (2 pre-existing stray documents from before this campaign, §0.5, + 3 real chunks from the `storia-comune.md` manual upload), `scrape: 24` (15 chunks from the Gaspare_Spontini Wikipedia page + 9 from Maiolati_Spontini) — matches the 3 real `storia` sources in §5.3, zero embedding failures.
- [x] **3.4** Minimal functional verification (smoke test) **before** launching the 1000 questions: 1 question per section with a known answer (e.g. "In che anno è morto Gaspare Spontini?" → must answer 1851 **and** cite the storia source). If even this smoke test fails, don't proceed to scale up: diagnose first (were chunk embeddings actually generated? is `min_score` too high for the chunk length? is the persona active?).

  Ran 2026-07-26 for `storia` (the only section with real content — see scope note above): `POST /chat {"question":"In che anno è morto Gaspare Spontini?"}` → HTTP 200 in 146.9s: `{"answer":"Gaspare Spontini è morto l'anno successivo al 1850, quindi nel 1851.","sources":[{"document_id":5,"source_ref":"https://it.wikipedia.org/wiki/Gaspare_Spontini"},{"document_id":22,"source_ref":"https://it.wikipedia.org/wiki/Maiolati_Spontini"},{"document_id":4,"source_ref":"https://it.wikipedia.org/wiki/Gaspare_Spontini"},{"document_id":6,"source_ref":"https://it.wikipedia.org/wiki/Gaspare_Spontini"},{"document_id":3,"source_ref":"https://it.wikipedia.org/wiki/Gaspare_Spontini"}],"fell_back":false}`. **Passes**: states 1851, `fell_back:false`, every cited source is a genuine `storia` document (unlike the Phase 1 identity-question anomaly, which cited unrelated stray documents — this confirms retrieval works correctly once real matching content exists). Note the 146.9s latency — much higher than Phase 1's ~7-22s single-question baseline now that the KB holds 29 documents; a real number for §10.3, not yet the formal Wave 0 measurement (needs 20 mixed questions). `news` and `delibere` smoke tests remain pending until those sections have real content (see scope note).

## 7. Phase 4 — The 1000-Question Set

### 7.1 Methodology

Not all 1000 questions are hand-written one by one in this file: that would be impractical, and for the delibere/news categories it would be **impossible without inventing facts** before the real ingestion (Phase 2-3) is complete. The set is built as follows:

1. A **golden set** of hand-written concrete questions, for categories anchorable to facts already verified today (bot identity, general history, Spontini's biography) — listed in full in §7.3.
2. **Parametric templates** for categories that depend on real ingested content (news, delibere) — the question structure is fixed now, the content (document name, specific fact) is filled in after Phase 3, using **only** facts read in the actually-ingested document (never invented).
3. A **variation matrix** (register, direct/indirect phrasing, single-fact/multi-fact question) applied systematically to every available fact/document, to reach 1000 questions without repeating the same wording.
4. A fixed block of **out-of-KB questions** (honest refusal is mandatory) and one of **edge-case/adversarial questions** (hard phrasings: typos, dialect, multi-part questions, ambiguous questions) — inspired by the "Refusal honesty" section and the observations in the parent project's report (`reports.md` §3.5, §4.2).

### 7.2 Distribution by Category (total 1000)

| Cat. | Name | # Questions | Source |
|---|---|---|---|
| A | Bot identity / imprinting | 60 | Persona (Phase 1) — no ingestion dependency |
| B | Comune history (general) | 140 | `storia` section — official page + comune's Wikipedia page |
| C | Gaspare Spontini (life and works) | 110 | `storia` section — Spontini's Wikipedia page (+ the official page if it covers him) |
| D | News (last 3 months) | 260 | `news` section — real documents from §5.3 |
| E | Delibere/Determine (last 3 months) | 260 | `delibere` section — real documents from §5.3 |
| F | Out-of-KB / honest refusal | 90 | Cross-cutting — no section above covers the requested fact |
| G | Edge-case/adversarial questions | 80 | Cross-cutting — hard rephrasings of A-E |
| **Total** | | **1000** | |

With ~20-25 real documents expected for `news` and for `delibere` (§5.2 point 3), categories D and E work out to **roughly 10-13 questions per document**, produced via the variation matrix (§7.4) — not 260 distinct facts to invent.

### 7.3 Golden Set — Concrete Questions (Categories A, B, C)

Declared source of inspiration: structure and style taken from `spontini/docs/sample-questions.md` and `spontini/docs/edge-cases-questions.md` (parent project), rewritten to use only facts verified for **this** project (Appendix B) instead of the parent's facts.

Note: the actual question text is left in Italian, since these are runtime citizen-facing chat inputs, not documentation prose.

**Category A — Identity (examples, to be expanded to 60)**

1. Chi sei?
2. Come ti chiami?
3. Cosa puoi fare per me?
4. Di cosa puoi parlarmi?
5. Perché ti chiami SpontiniBot?
6. Chi era Spontini, dato che porti il suo nome?
7. Da dove prendi le informazioni che mi dai?
8. Sei un funzionario comunale?
9. In che lingua posso scriverti?
10. Se non sai una risposta, cosa fai?
11. Puoi aiutarmi con una pratica anagrafica?
12. Chi ti ha creato?
13. Sei sempre disponibile?
14. Cosa NON puoi fare?
15. Come faccio a essere sicuro che la tua risposta sia corretta?

*(→ reach 60 by applying the variation matrix in §7.4: tu/lei form, direct/indirect, short/extended, etc. to each of the 15 root questions.)*

**Category B — Comune History (examples, verified roots)**

Verified anchor facts (Appendix B): comune in the province of Ancona, Marche; the name "Spontini" added in 1939; the castle of Maiolati first mentioned as a "castrum" in 1283; Neolithic settlements discovered in 1883; a Fraticelli stronghold until its destruction in 1428; autonomous comune since 1808 (Napoleonic period); a silk mill active 1921-1964; current economy based on agriculture, livestock, viticulture; population 5,916 residents (to be reconfirmed).

1. Cosa racconta la storia del Comune di Maiolati Spontini?
2. Da dove deriva il nome "Maiolati Spontini"?
3. In che anno è stato aggiunto il nome "Spontini" al nome del paese?
4. Quando fu menzionato per la prima volta il castello di Maiolati?
5. Cosa furono i Fraticelli e che ruolo ebbero nella storia del castello?
6. In che anno fu distrutta la roccaforte dei Fraticelli a Maiolati?
7. Quando Maiolati diventò un comune autonomo?
8. Cosa successe durante il periodo napoleonico a Maiolati?
9. Quali reperti sono stati scoperti nel 1883?
10. Cosa producevano la filanda di Maiolati e per quanti anni fu attiva?
11. In quale provincia si trova il Comune di Maiolati Spontini?
12. In quale regione si trova Maiolati Spontini?
13. Quanti abitanti ha il Comune di Maiolati Spontini?
14. Di cosa vive oggi l'economia di Maiolati Spontini?
15. Cosa si intende con "castrum" nella storia di Maiolati?

**Category C — Gaspare Spontini, Life and Works (examples, verified roots)**

Verified anchor facts: born in Maiolati on 14 November 1774; died in Maiolati on 24 January 1851; Italian composer and conductor; representative of Classicism. (Other details — conservatory, Paris, Berlin, specific works — must be verified against the text actually extracted from the ingested Wikipedia page, not assumed upfront: the Italian Wikipedia article consulted during scoping contained these facts, but 512-token chunking may split them across different chunks.)

1. Chi era Gaspare Spontini?
2. Quando è nato Gaspare Spontini?
3. Dove è nato Gaspare Spontini?
4. Quando è morto Gaspare Spontini?
5. Dove è morto Gaspare Spontini?
6. Che professione svolgeva Gaspare Spontini?
7. A quale corrente artistica appartiene Gaspare Spontini?
8. Quanti anni visse Gaspare Spontini?
9. Perché il Comune ha aggiunto il suo nome a quello del paese?
10. Cosa lega Gaspare Spontini a Maiolati?
11. Gaspare Spontini è nato e morto nello stesso paese?
12. In che secolo è vissuto Gaspare Spontini?

*(→ reach 110 across B and C by combining the roots above with the variation matrix in §7.4 and with every additional fact actually present in the ingested text, verified after the fact.)*

### 7.4 Variation Matrix (to scale A/B/C to target and to generate D/E from the templates)

Apply systematically to every root question:

| Axis | Variants |
|---|---|
| Register | formal ("Potrebbe indicarmi…"), direct ("Qual è…"), informal ("Sai dirmi…") |
| Form | direct question, polite imperative ("Dimmi…"), affirmative-with-confirmation ("È vero che…?") |
| Breadth | single-fact question vs. two-fact question in the same sentence (e.g. "Chi era Spontini e quando è nato?") |
| Noise | with a plausible typo, with light dialect/regionalism, all lowercase without punctuation |
| Specificity | with explicit reference to the source ("Cosa dice il sito del Comune su…") vs. without |

### 7.5 Template — Category D (news, `[TO BE COMPLETED POST-INGESTION]`)

For each real document logged in §5.3 under `news`, generate ~10-13 questions from these templates, substituting `<subject>` with the document's real content:

1. Di cosa parla la notizia su `<subject>`?
2. Quando è stata pubblicata la notizia su `<subject>`?
3. Cosa prevede `<subject>` per i cittadini?
4. Entro quando bisogna fare qualcosa, secondo la notizia su `<subject>`? *(only if the document contains a deadline — otherwise this question belongs in Category F for that document, to test the honest refusal when the data isn't there)*
5. Chi è coinvolto in `<subject>`?
6. Dove si svolge `<subject>`? *(if applicable)*
7. Quanto costa `<subject>`? *(only if the document reports a figure — otherwise → Category F)*
8. Riassumi la notizia su `<subject>`.
9. C'è qualche novità recente dal Comune su questo argomento?
10. `<informal paraphrase of the document's real title>`?

### 7.6 Template — Category E (delibere/determine, `[TO BE COMPLETED POST-INGESTION]`)

For each real document logged in §5.3 under `delibere`:

1. Cosa stabilisce la delibera/determina su `<subject>`?
2. Quando è stata approvata?
3. A chi è stato affidato `<subject>`, se si tratta di un appalto/fornitura? *(only if actually present in the text)*
4. Quanto costa `<subject>`, secondo l'atto? *(only if a figure is actually reported)*
5. Qual è il numero/protocollo dell'atto? *(only if visible in the extracted text — the tabular layout of some administrative pages might not survive plain-text extraction: this is itself a test case, see §11)*
6. Quale ufficio/settore ha adottato l'atto?
7. Perché è stato adottato questo provvedimento?
8. Ci sono delibere recenti sul bilancio comunale? *(cross-check across multiple documents in the section)*

### 7.7 Category F — Out-of-KB / Honest Refusal (90 questions, examples)

These are the **most important questions in the whole set**: here the tolerance for hallucination is zero by definition (Constitution §5). Include:

- Questions about plausible-but-not-in-KB facts (e.g. "Qual è il numero di telefono diretto del sindaco?", "Quanti abitanti ha la frazione Moie?" — a frazione-level figure, not comune-level, unless verified to actually be in the KB).
- Questions about topics entirely unrelated to the comune (e.g. "Che tempo fa oggi a Maiolati Spontini?", "Qual è la ricetta della crescia marchigiana?").
- Questions asking for an opinion or a prediction, not a fact ("Il prossimo sindaco chi sarà?", "Conviene investire a Maiolati Spontini?").
- Questions about personal/sensitive data that isn't published ("Dove abita l'assessore X?").
- Questions about documents outside the ingested time window (e.g. a 2015 delibera, if it wasn't ingested).
- Every "only if present" sub-question from templates D/E, for the cases where the real document does **not** contain that data.

For each, the expected answer is **exactly** the active persona's `fallback_message` (§4), **zero citations**, `fell_back=true`.

### 7.8 Category G — Edge-case/Adversarial Questions (80 questions)

Inspired by failures observed in the parent project (`reports.md` §3.5 "Rifiuti onesti", §4.2 "Storia del comune" — the parent's weakest area): open narrative questions ("Raccontami tutto quello che sai su Gaspare Spontini"), questions inviting a long summary (risk of exceeding the expected conciseness), double-negation questions, questions that mix an in-KB and an out-of-KB fact in the same sentence (to verify the bot answers the in-KB part and honestly refuses the other, without blending the two), questions that explicitly ask for the source ("Da quale documento hai preso questa informazione?").

## 8. Phase 5 — Execution Mechanics

- [ ] **5.1** Create one training session per test wave (e.g. one session per 100 questions, or one per category): `POST /admin/api/training/sessions {"title": "Wave 1 — Category A/B/C", "created_by": "test-ingestion"}`.
- [ ] **5.2** For every question: call `POST /admin/api/training/sessions/:id/messages {"question": "..."}`, **timing the response on the client side** (the `training_message`'s `created_at` field does not record latency — it must be timed externally, e.g. `curl -w "%{time_total}"` or a scripted wrapper).
- [ ] **5.3** Log every exchange in a structured external log (CSV or similar), **in addition to** the training session — the training session is the canonical product-side record (reuses `RagEngine`, persists `sources`/`fell_back`), but aggregate scoring needs a log with extra columns:

  | Column | Description |
  |---|---|
  | `wave` | test wave number |
  | `category` | A–G |
  | `question` | exact text sent (Italian, verbatim) |
  | `training_session_id`, `training_message_id` | for traceability to the canonical source |
  | `answer` | returned text |
  | `fell_back` | bool |
  | `sources` | list of returned `source_ref` |
  | `citation_correct` | bool, assessed manually/via cross-check (§9) |
  | `hallucination` | bool |
  | `score` | 0-100 (§9) |
  | `latency_seconds` | measured |
  | `notes` | free-text observations |

- [ ] **5.4** Save the log under `.project/test-ingestion-results/wave-<N>-<date>.csv` (create the folder; don't version any sensitive data that surfaces during testing).

## 9. Phase 6 — Scoring Rubric

Adapted from the parent project's methodology (`spontini/reports.md` §2), extended with an explicit citation criterion because here citation is a first-class structured DTO, not a formatting detail.

| Criterion | Weight | Description |
|---|---|---|
| Factual correctness | 35 | The reported fact is exact and verifiable in the cited source document |
| Citation correctness | 20 | When `fell_back=false`, `sources[]` is non-empty **and** every `source_ref` genuinely corresponds to a document that contains the fact used in the answer (not a retrieved-but-irrelevant document) |
| Absence of hallucination | 20 | No invented or not-present-in-context name, figure, date, or fact |
| Relevance and conciseness | 15 | Answers exactly what was asked, no more, no less (no "echo" of unrequested context) |
| Fallback honesty (when applicable) | 10 | On Category F questions, the fallback always appears, never a partial invented attempt |

**Cap rules** (aligned with the parent project, adapted):

- Confirmed factual hallucination → max score **35**.
- Refusal (fallback) on a fact that was genuinely present in the ingested document → max score **40**.
- Non-fallback answer with no citation at all, or with an irrelevant citation → max score **50**.
- Correct answer that blatantly exceeds the expected conciseness (e.g. returns the entire document instead of the requested fact) → 10-15 point penalty on "Relevance and conciseness", no more.

## 10. Phase 7 — Iteration Loop Toward "Perfection"

**Do not run the 1000 questions in one blind block.** Wave-based procedure:

1. **Wave 0 — latency baseline** (§10.3): 20 mixed questions, before everything else, to measure real latency on this hardware and set a numeric target (see below — it is not pre-fixed in this document, it must be measured).
2. **Wave 1**: the entire golden set (Categories A+B+C, ~310 questions) + Category F (90) — the only ones fully writable today without depending on post-ingestion content. Run, score, fix.
3. **Wave 2**: after filling in the D/E templates with real documents (§5.3, §7.5, §7.6) and Category G — the remaining ~600 questions.
4. **Subsequent waves**: **don't rerun all 1000 after every fix.** After each tuning change (§10.2), rerun only the subset that failed, plus a random 10% control sample of the total (regression check) to make sure the fix didn't break anything else.
5. Reserve a **full 1000-question run** only as the final pre-signoff gate, once the targeted subsets are all green.

### 10.1 Why Not "All At Once": Time Budget

With a 3B model on CPU, the expected order of magnitude per answer is **tens of seconds** (see §2 — the parent project, with different hardware and model, measured 20-58s on questions routed to the 3B model, which here is 100% of cases). Even at an optimistic 15s/question average, 1000 sequential questions take **over 4 hours** of execution alone, before scoring. Plan the execution in waves and in the background (don't block an interactive session on an hours-long run), and measure for real before promising timings.

### 10.2 Tuning Levers (all real, none invented)

| Lever | Where | Effect |
|---|---|---|
| `RAG_MIN_SCORE` (env, default 0.35) | `backend` | Minimum similarity threshold to accept a chunk. **Known project observation**: Feature 0025 observed that, with the real embedding model, the default threshold can sometimes accept a semantically distant document — if irrelevant citations show up (low "Citation correctness" score), raising this threshold is the first thing to try. |
| `RAG_TOP_K` (env, default 5) | `backend` | How many chunks are passed to the prompt. Lowering it reduces noise/echo; raising it helps on multi-fact questions. |
| Chunk size/overlap (512/64, `ingest-core/src/chunking.rs`) | `ingest-core`, requires a code change + **full re-ingestion** | Smaller chunks = more precise citations but higher risk of cutting a fact in half; touch this only if env-level tuning isn't enough, and only with an ADR if it changes the project standard. |
| Persona `system_prompt` / `tone` / `fallback_message` | `POST /admin/api/persona` + `POST /admin/api/persona/reload` | Wording, tone, explicit handling of "I don't know" — iterate here for conciseness/tone issues, not for hallucination issues (those are almost always a retrieval problem, not a prompt problem). |
| Document coverage | Phase 2 | If an entire category fails systematically, the most likely cause is that the right document wasn't ingested, or was ingested incompletely (e.g. a table lost during plain-text extraction) — check this before touching RAG parameters. |
| Generation model (Qwen2.5-3B-Instruct) | ADR 0001 | **Do not change ad hoc.** If, after tuning retrieval and prompt, quality remains structurally insufficient, that's a signal for a new ADR reconsidering the model choice — not a lever of this test plan. |

### 10.3 Latency Target — To Be Measured, Not Invented

Run Wave 0 (20 questions, mixed categories) and record the real average and p95 latency **on this hardware**. Only then, fix the final numeric target here (explicit placeholder until measured):

- Measured average latency (Wave 0): `______ s` *(to fill in)*
- Measured p95 latency (Wave 0): `______ s` *(to fill in)*
- Proposed target for the final gate: average ≤ `______` s, p95 ≤ `______` s *(to set after measuring — a reasonable multiple of the baseline, not a made-up number)*

## 11. Exit Criteria — Measurable Definition of "Perfection"

The campaign is considered complete and the system "ready" when **all** of the following criteria are simultaneously satisfied across the full 1000-question set (or across the latest complete regression wave):

- [ ] **Zero confirmed hallucinations** across all 1000 questions. This is a binary constraint (Constitution §5), not a statistical one: even a single confirmed hallucination on a verifiable fact blocks the gate until it's fixed and re-verified.
- [ ] **100%** of Category F questions (out-of-KB, 90 questions) receive the honest fallback — zero exceptions.
- [ ] **≥ 98%** of non-fallback answers have at least one correct citation ("Citation correctness" criterion, §9).
- [ ] **Weighted average score ≥ 90/100** across the full set.
- [ ] **No category (A-G) with an average below 80/100.**
- [ ] **Average and p95 latency within the target set in §10.3** after real measurement.
- [ ] The document log (§5.3) is complete and every row is marked "Verified in KB".

If a criterion isn't met after an iteration cycle (§10), don't lower the bar to make it pass: **document the gap, apply a relevant tuning lever (§10.2), retest the affected subset.**

## 12. Reporting

After every significant wave, produce `.project/test-ingestion-report-<date>.md` in the style of `spontini/reports.md` (executive summary, per-category table, trend vs. the previous wave, list of worst cases with root cause and applied fix). The final report, after the gate in §11, closes this campaign.

---

## Appendix A — Endpoint Cheat-Sheet (verified against `backend/src/lib.rs`)

| Method | Path | Use |
|---|---|---|
| POST | `/admin/api/auth/login` | `{"username","password"}` → session cookie |
| POST | `/admin/api/auth/logout` | invalidates the session |
| GET/POST | `/admin/api/persona` | list versions / create new version (`{"name","system_prompt","tone","fallback_message","activate"}`) |
| POST | `/admin/api/persona/:id/activate` | activates a specific version |
| POST | `/admin/api/persona/reload` | invalidates the cached active persona |
| POST | `/admin/api/upload` → preview → `/admin/api/upload/confirm/:token` | manual pdf/docx/md/txt upload for a section |
| GET | `/admin/api/ingest/config` | schedule/sections/sources tree |
| PUT | `/admin/api/ingest/config/schedule` | `{"cron_expr","enabled"}` |
| POST/DELETE | `/admin/api/ingest/config/sections[/:id]` | `{"name","ordering"}` |
| POST/DELETE | `/admin/api/ingest/config/sources[/:id]` | **Correction (verified 2026-07-26, this cheat-sheet was stale)**: `section_id` is a query parameter (`?section_id=<id>`), not a body field — body is `{"source_type","url","enabled"}` only. See `backend/src/admin/ingest_config/handlers.rs:167-172`. |
| POST | `/admin/api/ingest/run` | trigger an immediate run → 202 + id |
| GET | `/admin/api/ingest/run/:id` | status (`pending`/`running`/`done`/`failed`) |
| POST/GET | `/admin/api/training/sessions` | create/list sessions (`{"title","created_by"}`) |
| POST | `/admin/api/training/sessions/:id/close` | closes a session |
| POST/GET | `/admin/api/training/sessions/:id/messages` | ask a recorded question / list exchanges (`{"question"}`) |
| POST | `/admin/api/training/feedback` | `{"message_id","chunk_id","answer_span","sentiment","comment"}` |
| GET | `/admin/api/training/messages/:id/feedback` | list feedback for a message |
| POST | `/chat` | public endpoint, `{"question"}` → `{"answer","sources","fell_back"}` |

## Appendix B — Verified Sources (fetched 2026-07-25)

| URL | Verified as | Method |
|---|---|---|
| `https://www.comune.maiolatispontini.an.it/` | Official institutional site of the comune | WebSearch |
| `https://www.comune.maiolatispontini.an.it/c042023/zf/index.php/storia-comune` | "Storia del Comune" page, full narrative text, single static page | WebFetch (content confirmed) |
| `https://www.halleyweb.com/c042023/po/elenco_news.php?area=H` | News/press-release list, up to date (items dated July 2026 observed) | WebFetch (content confirmed) |
| `https://www.halleyweb.com/c042023/zf/index.php/atti-amministrativi/determine` | "Determine" search engine, paginated (1,401 pages as of 2026-07-25) | WebFetch (structure confirmed) |
| `https://it.wikipedia.org/wiki/Maiolati_Spontini` | Wikipedia article on the comune — verified opening line: *"Maiolati Spontini è un comune italiano di 5 916 abitanti della provincia di Ancona nelle Marche. Nel 1939 al nome del paese è stato aggiunto il nome Spontini..."* | WebFetch (verbatim text confirmed) |
| `https://it.wikipedia.org/wiki/Gaspare_Spontini` | Wikipedia article on the composer — verified opening line: *"Gaspare Luigi Pacifico Spontini (Maiolati, 14 novembre 1774 – Maiolati, 24 gennaio 1851) è stato un compositore italiano, esponente del Classicismo."* | WebFetch (verbatim text confirmed) |

All other URLs mentioned in search results (e.g. individual delibera detail pages, `amministrazionicomunali.it`, etc.) **were not verified with a direct fetch** and must be confirmed by the operator before being used as a `scrape source`.

## Appendix C — Known Risks and Architectural Gaps

- **No crawler**: the scraper ingests a single page per configured source (§5.2). Ingesting "the last 3 months" of delibere/news requires manual work to select permalinks or manual uploads — it's not automatable with the current code in a single step. If this becomes a recurring problem, consider a dedicated feature (crawler with pagination/date filtering) in `ROADMAP.md`, with an ADR for the crawling strategy.
- **No document-count endpoint**: post-ingestion quantitative verification requires direct access to the `kb.db` file (Phase 3.3) because there's no `GET /admin/api/documents` or similar. Consider whether to add this as a future feature to make this kind of verification less artisanal.
- **Retrieval threshold known to be potentially permissive**: an observation already recorded when Feature 0025 was closed (`ROADMAP.md`) — the `RAG_MIN_SCORE=0.35` default can, with the real embedding model, accept a semantically distant document. First lever to try if irrelevant citations show up (§10.2).
- **Latency not measured on this hardware**: no real baseline number yet exists for Qwen2.5-3B-Instruct Q4_K_M on this target hardware — must be measured in Wave 0 (§10.3), not assumed from the parent project (different model and hardware).
- **Text extraction from pages with tables/complex layout** (e.g. "atti amministrativi" pages with date/office/subject columns): the scraper extracts "visible text", which on a tabular layout can lose the association between columns. Must be checked empirically on the first documents ingested into `delibere` (Category E, question 5 in §7.6 is designed specifically to stress this case).
- **`kb.db` startup race**: if `backend` and `ingest` start at the exact same instant (e.g. `docker compose restart` or `make up` from a stopped stack), both try to open/migrate the same SQLite/libSQL file and one of them can fail with `database is locked` on startup. Observed in practice on 2026-07-25: `ingest` was automatically restarted by the container's restart policy and recovered on its own a second later, with no data loss. If the stack is restarted during a test campaign, **verify with `docker compose ps` that `ingest` is actually `running` and check its logs** before triggering an ingest run — there's no application-level retry-with-backoff on opening the kb store, it relies on the Docker restart policy.
- **Scheduler ran the full pipeline unconditionally on a timer, not on request** (found 2026-07-26, blocked Phase 3 — **FIXED 2026-07-26, commit `17077ad`**): `ingest/src/scheduler.rs:107-118`, the `run_interval.tick()` branch called `runner.run_all(&config.sources)` every `RUN_POLL_SECS` (default 10s) whenever `config.sources` was non-empty — with **no relation at all** to `POST /admin/api/ingest/run` and no failure backoff. The moment any source was configured, `ingest` started re-scraping every enabled URL every ~10 seconds, forever, regardless of whether anyone ever triggered a run. Observed hammering `it.wikipedia.org` and the comune's official site this way for 10+ minutes before intervention. This contradicted the on-demand model documented in Appendix A (`POST /admin/api/ingest/run` → "trigger an immediate run"). **Fix**: the timer branch now calls `KbStore::consume_run_request()`/`complete_run()` every tick (DB-layer methods that already existed, tested, but were never wired into the scheduler) and only runs the pipeline when a request was actually pending. Verified live: a configured source now sits idle indefinitely — confirmed silent for 35+ seconds with a source present and no request — until a real `POST /admin/api/ingest/run` moves it through `pending → running → done`.
- **Config changes (deletes/disables) didn't reliably propagate to the running scheduler** (found 2026-07-26, blocked Phase 3 — **FIXED 2026-07-26, commit `17077ad`**): after `DELETE /admin/api/ingest/config/sources/:id` for all 3 configured sources (confirmed via direct `kb.db` query that `ingest_source` was empty), the scheduler kept re-running the pipeline against the deleted sources for 2.5+ minutes — well past `CONFIG_POLL_SECS` (default 30s). The only reliable way to stop it at the time was `docker compose stop ingest`. **Fix**: `ConfigLoader` was reopening a brand-new `KbStore` (fresh libsql `Database`) on every single poll instead of reusing one long-lived connection — the one place in the codebase deviating from the pattern every other `KbStore` consumer already uses. Switched to a single connection opened once at startup and reused. Verified live: deleted one source and added a different one, waited 40s (past the 30s poll interval), triggered a run, and confirmed via `ingest` logs that it processed the *new* source, not the deleted one.
- **Embedding batch-size mismatch** (found 2026-07-26 — **FIXED 2026-07-26, commit `17077ad`**): real chunks produced from the two Wikipedia pages (`ingest-core/src/chunking.rs`, configured `chunk_size=512`) came out to 678 and 659 tokens by `llama-embed`'s own tokenizer — both over its compiled/configured physical batch size of 512, so every embedding call failed with `HTTP 500 ... input (N tokens) is too large to process`. **Fix**: raised `llama-embed`'s `--batch-size`/`--ubatch-size` to 2048 in `docker-compose.yml` (comfortable headroom above the observed worst case). The chunker's token accounting still doesn't exactly match the embed model's real tokenizer (that mismatch wasn't fixed, just given headroom) — worth revisiting if a much longer real document ever produces chunks north of ~2000 tokens.
- **The official "Storia del Comune" page is blocked by robots.txt**: contrary to §5.2 point 1's assumption ("perfect fit for a single scrape source"), a real scrape attempt on 2026-07-26 failed with `robots.txt: path /c042023/zf/index.php/storia-comune is disallowed by robots.txt`. This is a real, verified fact, not a code bug — no fix applicable. **Worked around 2026-07-26** per §5.2's own allowance for non-scrapeable documents: fetched the real page text directly, then ingested it via manual upload (feature 0009, preview confirmed before confirming) instead of a scrape source. See §5.3 row 1.
