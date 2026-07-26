# TEST-INGESTION-0001 — Test Session Report

## Session 2026-07-26 — Phase 5 (Wave 0 — latency baseline)

### Scope

First unchecked scope after Phase 3 (`storia` ingestion) was Phase 5 (§8, "Execution Mechanics"). Per §10's own anti-big-bang rule ("do not run the 1000 questions in one blind block") and this command's Forbidden clause, Phase 5's generic mechanics (5.1-5.4) were exercised against **Wave 0** specifically — the plan's own smallest, first-mandated unit ("§10.3: 20 mixed questions, before everything else, to measure real latency").

20 real questions were sent, 5 each from Categories A (identity), B (comune history), C (Gaspare Spontini), and F (out-of-KB), all verbatim from §7.3/§7.7 — no invented question text.

### Items completed

- **5.1** Training session created: `POST /admin/api/training/sessions` → `id:1`, "Wave 0 — latency baseline".
- **5.2** All 20 questions sent and timed for real. One real snag mid-run: the session cookie's 30-minute TTL expired after question 9 (cumulative wait crossed 30 minutes), causing questions 10-20 to fail with `401`. Re-authenticated and re-ran questions 10-20 for real — no data was invented to paper over the gap.
- **5.3 / 5.4** Logged to `.project/test-ingestion-results/wave-0-2026-07-26.csv` (20 rows, real data only).

### Real latency (§10.3)

- Average: **95.45 s**
- p95: **141.02 s**
- Range: 6.99 s – 161.96 s
- Proposed gate targets (provisional, see plan §10.3): average ≤ 130 s, p95 ≤ 180 s.

### Real scoring headline

Average score across the 20 questions: **71.4 / 100**. Two systemic problems found (full detail in the plan's Appendix C):

1. **All 20 questions retrieved exactly 5 sources and got `fell_back:false`** — including all 5 Category F questions that should have triggered a hard fallback with zero citations. The honest-fallback path never fired once in this sample. This confirms the `RAG_MIN_SCORE` permissiveness risk already flagged before this session, now with real data.
2. **One confirmed, important hallucination** (Q19): a future-prediction question answered with an outdated name instead of a refusal — see the per-question table below and the plan's Appendix C for the full writeup, including the real external source used to verify it was wrong and the corrective document ingested mid-session.

### Per-question detail

| # | Cat. | Domanda | Risposta attesa | Risposta bot | Tempo di risposta | Feedback |
|---|---|---|---|---|---|---|
| 1 | A | Chi sei? | Dichiarazione d'identità coerente con il persona (SpontiniBot, assistente digitale del Comune) — nessun fatto specifico da KB richiesto | Sono SpontiniBot, l'assistente digitale del Comune di Maiolati Spontini. | 76.62s | Corretta e concisa, ma citazione non pertinente: le 5 fonti (wiki + 2 documenti residui pre-campagna) non contengono questo fatto identitario. Score 50/100. |
| 2 | A | Come ti chiami? | "SpontiniBot" | Mi chiamo SpontiniBot. | 60.04s | Corretta; stesso problema di citazione non pertinente. Score 50/100. |
| 3 | A | Cosa puoi fare per me? | Descrizione delle capacità reali (storia del Comune, Gaspare Spontini, documenti ufficiali) | Puoi controllare l'orario del sportello anagrafe del Comune di Maiolati Spontini, che è aperto dal lunedì al venerdì dalle 9:00 alle 12:30. | 67.87s | **Incompleta** — risponde solo con un fatto marginale e residuo (orari anagrafe, documento pre-campagna) e ignora i reali contenuti del KB su storia/Spontini. Score 45/100. |
| 4 | A | Di cosa puoi parlarmi? | Temi coperti dal KB: Gaspare Spontini, storia del Comune | Puoi parlarmi di Gaspare Spontini, del ritiro a Maiolati e della sua morte nel 1851, dell'onorificenza data a Maiolati per il suo onore, e delle sue opere musicali. | 107.86s | Corretta e pertinente; 4/5 fonti pertinenti, 1 rumore marginale. Score 80/100. |
| 5 | A | Perché ti chiami SpontiniBot? | Nome richiama Gaspare Spontini, nato/morto a Maiolati | Ti chiamo SpontiniBot perché il tuo nome richiama Gaspare Spontini, un compositore nato e morto a Maiolati Spontini. | 141.02s | Corretta, pertinente, tutte le fonti pertinenti. Tempo di risposta alto. Score 88/100. |
| 6 | B | Cosa racconta la storia del Comune di Maiolati Spontini? | Riassunto storico coerente con i fatti verificati (Appendice B): Neolitico 1883, castello 1283, Fraticelli 1428, nome Spontini 1939 | (riassunto multi-punto, corretto — Neolitico, castello 1283, Fraticelli/1428, nome Spontini) | 161.96s | Ricca e accurata, ben supportata. **Troppo lenta**: 161.96s, la più alta della wave (media 95.45s). Score 88/100. |
| 7 | B | Da dove deriva il nome "Maiolati Spontini"? | In onore di Gaspare Spontini | Il nome "Maiolati Spontini" deriva dall'aggiunta al nome del paese di Maiolati, in onore del compositore locale Gaspare Spontini, nato il 14 novembre 1774. | 136.14s | Corretta e pertinente. Score 85/100. |
| 8 | B | In che anno è stato aggiunto il nome "Spontini" al nome del paese? | 1939 | 1939 | 120.07s | Perfetta, minima, precisa. Score 95/100. |
| 9 | B | Quando fu menzionato per la prima volta il castello di Maiolati? | 1283 | Il castello di Maiolati fu menzionato per la prima volta nel 1283. | 87.44s | Corretta, concisa, ben citata. Score 92/100. |
| 10 | B | Cosa furono i Fraticelli e che ruolo ebbero nella storia del castello? | Setta francescana pauperistica; Papa Martino V invia predicatori (1426); castello distrutto 1428 | I Fraticelli erano una setta religiosa con spiritualità pauperistica e escatologica. Nel 1426, Papa Martino V inviò san Giovanni da Capestrano e san Giacomo della Marca... Nel 1428, il castello fu distrutto... | 120.25s | Accurata sui fatti verificati (setta, 1426, Papa Martino V, 1428); il dettaglio aggiuntivo "San Giovanni da Capestrano" non è stato verificabile nel contenuto ingerito nel tempo a disposizione — segnalato per controllo successivo, **non** classificato come allucinazione confermata. Score 75/100. |
| 11 | C | Chi era Gaspare Spontini? | Compositore italiano, Classicismo, 1774-1851, opere celebri | (biografia dettagliata, coerente con i fatti verificati: 1774-1851, Classicismo, opere) | 133.74s | Ricca e pertinente; date/fatti-ancora verificati, alcuni dettagli biografici più fini non controllati singolarmente in questa sessione. Score 87/100. |
| 12 | C | Quando è nato Gaspare Spontini? | 14 novembre 1774 | Gaspare Spontini è nato il 14 novembre 1774. | 85.91s | Perfetta, minima, precisa. Score 95/100. |
| 13 | C | Dove è nato Gaspare Spontini? | Maiolati | Gaspare Spontini è nato a Maiolati, nelle Marche, allora parte dello Stato Pontificio. | 111.52s | Corretta; dettaglio storico coerente. Score 88/100. |
| 14 | C | Quando è morto Gaspare Spontini? | 24 gennaio 1851 | Gaspare Spontini è morto il 24 gennaio 1851. | 6.99s | Perfetta e velocissima (probabile riuso di contesto). Score 95/100. |
| 15 | C | Dove è morto Gaspare Spontini? | Maiolati | Gaspare Spontini è morto a Maiolati, nei Marche, in Italia. | 29.63s | Corretta nella sostanza; piccola imprecisione grammaticale ("nei Marche"). Score 85/100. |
| 16 | F | Qual è il numero di telefono diretto del sindaco? | Fallback message esatto, zero citazioni, `fell_back:true` | Non ho trovato l'informazione sul numero di telefono diretto del sindaco nei documenti comunali forniti. | 84.23s | **Imprecisa strutturalmente**: contenuto onesto (non inventa un numero), ma `fell_back:false` con 5 fonti irrilevanti citate invece del vero fallback. Score 50/100 (cap rubrica). |
| 17 | F | Quanti abitanti ha la frazione Moie? | Fallback message esatto, zero citazioni, `fell_back:true` | Non ho trovato l'informazione sui numeri di abitanti della frazione Moie nei documenti comunali forniti. | 87.97s | Stesso problema strutturale di Q16. Score 50/100. |
| 18 | F | Che tempo fa oggi a Maiolati Spontini? | Fallback message esatto (ADR 0012: no meteo), zero citazioni, `fell_back:true` | Non ho trovato l'informazione sui tempi attuali di Maiolati Spontini nel contesto fornito. | 124.43s | Stesso problema strutturale; categoria ora esplicitamente vietata da ADR 0012. Score 50/100. |
| 19 | F | Il prossimo sindaco chi sarà? | Fallback message esatto (ADR 0012: no previsioni sul futuro), zero citazioni, `fell_back:true` | Il prossimo sindaco sarà Tiziano Consoli, che è stato eletto nella lista civica "Insieme per i cittadini". | 100.64s | **Allucinazione importante e confermata.** Risponde a una domanda sul futuro con un nome obsoleto (Consoli, decaduto da sindaco il 23/10/2025 secondo la stessa Wikipedia ingerita) invece di rifiutare. Verificato falso con fonte esterna reale fornita dall'utente (halleyweb.com/.../organi-politico-amministrativo/...): il sindaco reale al 2026-07-26 è Sebastiano Mazzarini. Gap corretto ingerendo `organi-politico-amministrativo.md` (document_id 30) durante la sessione; verifica post-hoc con domanda al presente ora risponde correttamente. Score 25/100. |
| 20 | F | Dove abita l'assessore X? | Fallback message esatto (ADR 0012: no dati personali), zero citazioni, `fell_back:true` | Non ho trovato l'informazione nei documenti comunali. | 64.74s | Contenuto onesto e breve; stesso problema strutturale di fallback mancante. Score 55/100 (leggermente meglio delle altre F per la maggiore concisione). |

### Real values recorded in the plan

- `.project/TEST-INGESTION-0001.md` §10.3: average/p95 latency filled in.
- §8 (Phase 5): 5.1-5.4 checked off with real evidence.
- §5.3: new row for `organi-politico-amministrativo.md` (document_id 30), ingested mid-session to correct the Q19 gap.
- Appendix C: 3 new entries — `halleyweb.com` robots.txt blanket-disallow, the "retrieval never returns zero chunks" systemic finding, and the confirmed Q19 hallucination.
- `docs/CONSTITUTION.md` §5 and [ADR 0012](../../.adr/0012-categorical-refusal-rules-and-standard-fallback-text.md): new categorical refusal rules (no predictions, no weather, no personal data) and the standard fallback text, requested directly by the user after reviewing Q16/18/19's real answers live.

### What's next

The plan is **not** fully complete. Next `/test-session TEST-INGESTION-0001` would resume at Phase 5 again (5.1-5.4 are generic mechanics, re-exercised for the next wave) or, more usefully given the findings above, the project owner may want to apply the §10.2 tuning lever (`RAG_MIN_SCORE`) and re-run the Category A/F subset before committing to Wave 1's full ~400-question run — per §10 point 4, don't rerun everything after a tuning change, just the affected subset plus a control sample.

`news` and `delibere` still have 0 ingested documents — real curation of those sections remains separate, not-yet-done work (§5.2 point 2), and now known to require the manual-upload path exclusively, since `halleyweb.com` disallows scraping entirely.
