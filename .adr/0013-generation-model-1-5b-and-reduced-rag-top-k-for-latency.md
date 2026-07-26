# ADR 0013: Generation Model 1.5B and Reduced RAG_TOP_K for Latency

- **Status**: accepted
- **Date**: 2026-07-26
- **Deciders**: massimobiagioli, Sisyphus (Claude Code)
- **Related**: supersedes [ADR-0001](./0001-generation-model-3b.md); informed by `TEST-INGESTION-0001` Wave 0

## Context

`TEST-INGESTION-0001`'s Wave 0 measured real end-to-end `/chat` latency for the first time on the actual target hardware (a ~9-year-old Intel i7-7820HQ Mac, no GPU, confirmed via `sysctl`): **95.45s average, 141.02s p95** across 20 real questions. The project owner reviewed this and asked for responses under 5 seconds.

Diagnosing for real (not assuming) via `llama-generate`'s own per-request timing logs: the dominant cost is **prompt processing** of the RAG-retrieved context, not generation. A typical request processes ~2,500–3,600 context tokens at ~30 tokens/sec (60–120s), while generation itself is only ~15–35 output tokens at ~6 tokens/sec (2.5–6s). `docker stats` during a live request showed the `llama-generate` container at 1009% CPU — already using essentially all 8 Docker-allocated cores, so this is not an under-threaded config, it is a genuine hardware ceiling for this CPU running a 3B Q4_K_M model with no GPU acceleration.

ADR 0001 (2026-07-09) considered and explicitly rejected Qwen2.5-1.5B ("quality drops noticeably... 3B is the sweet spot") — but that was a *projection*, made before any model in this project had been benchmarked against real RAG-sized context on the real target hardware. This ADR supersedes that call with real, measured data instead.

## Decision

Replace the generation model with **Qwen2.5-1.5B-Instruct (Q4_K_M)**, and lower `RAG_TOP_K` from the default 5 to **2**.

This was chosen after live A/B benchmarking on the real hardware, using real KB content (not synthetic filler) as the RAG context, comparing Qwen2.5-3B (current), Qwen2.5-1.5B, and Qwen2.5-0.5B:

| Config | Latency | Quality (real answers observed) |
|---|---|---|
| 3B + top_k=5 (previous) | ~95s avg | Good (Wave 0 avg score 71.4/100) |
| 1.5B + top_k=5 (full context) | 93s | Barely faster — not worth it alone |
| **1.5B + top_k=2** | **~30–45s** (29.8s isolated benchmark, 45.2s/17.6s live-stack samples) | Correct and complete on every real question tested |
| 1.5B + top_k=1 | 41.7s (noisy) | Degraded — fabricated a location and an opera title not in context |
| 1.5B, near-zero context | 7.5s (hardware floor) | Correct but trivial |
| 0.5B (any context) | 2.3–37s | **Rejected** — missed a date sitting in plain, unambiguous text in its context, and once answered "Mi chiamo Gaspare Spontini" (claiming to *be* the 1774–1851 historical figure, not an assistant referencing him) |

**A sub-5-second target is not achievable on this hardware at any tested model size without an unacceptable quality regression.** Even 1.5B's absolute floor (near-zero context) is 7.5s. 0.5B can hit low single-digit seconds but is not reliable enough for a citizen-facing government assistant bound by Constitution §5's zero-hallucination-tolerance rule.

## Rationale

Evaluated against [Constitution §6](../docs/CONSTITUTION.md#6-decision-making):

1. **Serves the mission?** Yes — ~95s per answer is not a usable citizen-facing service; ~30-45s, while still slow, is a real ~2-3x improvement toward availability, without trading away the truthfulness the 0.5B option would have cost.
2. **Keeps the stack local?** Yes — still a local GGUF model on `llama.cpp`, no external API, no GPU dependency introduced.
3. **Reduces complexity?** Yes — smaller model file (~1.1GB vs ~2.1GB), faster to provision; `RAG_TOP_K=2` is a one-line env change, not new code.
4. **Improves UX?** Yes, materially, even though the original 5s ask isn't met — 30-45s is a real, measured improvement citizens will notice, versus promising an unreachable number or shipping a model that answers fast but sometimes wrong.

## Consequences

### Positive

- Real, measured ~2-3x latency improvement (~95s → ~30-45s average) with no quality loss observed in this session's testing — if anything, the 1.5B model gave a *more complete* answer than the 3B model did in the one head-to-head question tested (both birth and death dates, correctly, concisely).
- Smaller model and context both reduce compute cost per query, leaving more headroom for concurrent requests (4 slots configured).

### Negative

- **Still far from the requested <5s target.** This ADR does not claim to solve that; it documents why it's not reachable on this hardware and picks the best real tradeoff found.
- **Some observed reliability regression on marginal/refusal-shaped answers**: a live-stack test of "Che tempo fa oggi a Maiolati Spontini?" produced a more confused, repetitive non-answer from 1.5B than the 3B model gave to the equivalent Wave 0 question. Smaller models appear to be less consistent specifically on awkward/refusal-shaped generations, even when factual retrieval-grounded answers are fine. Worth watching in Wave 1 scoring, not just assumed away.
- **`RAG_TOP_K=2` trades retrieval recall for speed**: multi-fact questions needing more than 2 chunks' worth of context may now get an incomplete answer where `top_k=5` would have found the second fact. Not observed as a problem in this session's testing, but not exhaustively tested either.
- Reopens ADR 0001's original quality-vs-speed tradeoff with a different answer than the project's earlier, more cautious call — a legitimate revision given ADR 0001 was working from projections, this one from real measurement on real hardware, but worth flagging explicitly since it reverses a prior explicit rejection ("chosen not to go this low").

### Neutral

- `RAG_MIN_SCORE` was investigated in the same session (see `TEST-INGESTION-0001.md` Appendix C) but is a **separate, still-unresolved** problem: real testing across 0.35–0.9 found no single threshold that cleanly separates relevant from irrelevant retrieved content for this KB/embedding-model combination. That is not fixed by this ADR and is not a `RAG_TOP_K`/model-size question — it needs a different technique (re-ranking, hybrid search) or a KB/embedding-model change, tracked separately.

## Alternatives Considered

### Alternative A: Keep Qwen2.5-3B, tune only `RAG_TOP_K`/`RAG_MIN_SCORE`

Rejected: real benchmarking showed 3B with a reduced context (top_k=2, same context size tested for 1.5B) would likely still be meaningfully slower than 1.5B at the same context size, since prefill and decode both scale with model size — a config-only change without the model swap leaves real, measured speed on the table for no quality benefit demonstrated in this session's testing.

### Alternative B: Qwen2.5-0.5B-Instruct

Rejected on quality: real testing found a comprehension failure (missed a date in plain, unambiguous context) and an identity-confusion error (claimed to *be* Gaspare Spontini). For a government-facing assistant under a zero-hallucination-tolerance constitution, this is disqualifying regardless of its attractive speed (2.3–37s).

### Alternative C: GPU acceleration

Rejected for now, not fully explored: this Docker Desktop setup has no confirmed path to pass through GPU/Metal to the `llama.cpp` container on this Intel Mac; would need real infrastructure investigation before it could be relied on, and is a bigger change than this ADR's scope. Worth a dedicated investigation if <5s remains a hard requirement.

### Alternative D: External/hosted inference API

Not seriously considered: directly violates [Constitution §3's Locality principle](../docs/CONSTITUTION.md#3-core-principles) ("The entire stack runs on-premises / local infrastructure. No external LLM APIs.") — would require its own Constitution amendment, not a model-swap ADR.

## Compliance

- `docker-compose.yml`: `llama-generate` command points to `qwen2.5-1.5b-instruct-q4_k_m.gguf`; `backend` service has `RAG_TOP_K=2` in its `environment:` block.
- `bin/provision-models.sh` and `models/generate/README.md` updated to the new model file/rationale.
- `TEST-INGESTION-0001.md` §1 (architecture summary), §10.3 (latency target), and Appendix C record the real before/after measurements and the full `RAG_MIN_SCORE` investigation data.
- Verified live against the real running stack (not just the isolated benchmark): `POST /chat` for a real Category C question returned the correct answer in 45.2s and 17.6s across two real samples, down from Wave 0's ~95s average for comparable questions.
