# ADR 0014: Instant Identity/Imprinting Answers Bypass RAG Retrieval and Generation

- **Status**: accepted
- **Date**: 2026-07-26
- **Deciders**: massimobiagioli, Claude Code
- **Related**: none

## Context

Spontini's persona (`Constitution` §2: "Spontini embodies Gaspare Spontini... speaks with the voice of a knowledgeable, helpful local figure") already carries a complete, first-person self-description in its own `system_prompt` field — e.g. "Sono Gaspare, l'assistente digitale del Comune di Maiolati Spontini... in onore [di Gaspare Spontini]... il paese aggiunse il suo nome nel 1939." Yet a citizen asking a plain identity question ("Chi sei?", "Come ti chiami?") is currently routed through the exact same path as every other question: embed the question, retrieve up to `RAG_TOP_K` chunks above `RAG_MIN_SCORE` from the KB, assemble a three-part prompt, and call `llama-generate`. Real latency measurements (ADR 0013) put that round trip at roughly 20-30 seconds even after the 1.5B/`RAG_TOP_K=2` tuning — and, separately, live testing during Plan 0028 (`.project/0028-content-ingestion-log.md`) found that at the current corpus size this path frequently doesn't even retrieve anything genuinely relevant to "who are you", since no KB *document* describes the bot itself (it's not a municipal record — it's configuration).

The project owner's explicit instruction: identity/imprinting questions must never hit the database or the generation model for their answer — the answer already exists, in the active persona's own fields — and the response time must be "ultra-immediate". A network round trip to `llama-generate` cannot be ultra-immediate under any tuning; the only architecture that satisfies this requirement is one that never calls it for this question shape.

This is a narrower, different case from [ADR 0012](./0012-categorical-refusal-rules-and-standard-fallback-text.md)'s explicit rejection of a code-level question-category classifier for future-prediction/weather/personal-data refusals. That decision was about *open-ended, adversarial-phrasing-prone* categories where a missed classification risks the worst failure mode this project has (a hallucinated, confidently-wrong answer, Constitution §5). Identity questions are a *closed, narrow* set of canonical phrasings about one specific, non-adversarial topic (the bot's own name/nature), and a missed match here has a benign failure mode: the question simply falls through to the normal RAG path and is answered as it is today, just slower — not wrong, not unsafe, no hallucination risk introduced by this feature.

## Decision

We add a persona-identity fast path to `RagEngine::answer()`, checked **before** `EmbeddingPort`/`RetrievalPort`/`GenerationPort` are invoked. If the incoming question, normalized (lowercased, trimmed, trailing punctuation stripped), exactly matches one of a small, explicit, curated set of canonical identity-question phrasings (e.g. "chi sei", "chi sei tu", "come ti chiami", "qual è il tuo nome", "presentati", "parlami di te", plus the persona-name-parametrized "chi è `<persona.name>`" / "chi è `<persona.name>`?"), `RagEngine` returns the active persona's own `system_prompt` verbatim as the answer text, with `sources: []` and `fell_back: false` — no embedding call, no retrieval call, no generation call. Any question that does not match falls through to the existing full RAG flow unchanged.

## Rationale

Evaluated against [Constitution §6](../docs/CONSTITUTION.md#6-decision-making):

1. **Serves the mission?** Yes — the persona's own self-description is exactly the ground truth for "who are you", and returning it directly is more honest (it's the actual configured identity, not an LLM's paraphrase of it) as well as far faster.
2. **Keeps the stack local?** Unaffected — this reduces external calls (zero for a matched question) rather than adding any.
3. **Reduces complexity?** Net small increase (one new matcher + one new `RagEngine` branch), but it removes complexity from the citizen's experience (a ~20-30s wait for a question whose answer is already in memory) and from operators diagnosing why "Chi sei?" sometimes cited unrelated documents (the exact anomaly recorded live in `TEST-INGESTION-0001.md` §4 and again in `.project/0028-content-ingestion-log.md`).
4. **Improves UX?** Yes, substantially — sub-millisecond response for the single most common first question a citizen or operator asks the bot, versus tens of seconds today.

The classifier-avoidance principle from ADR 0012 is deliberately not extended here: that ADR's own "Alternatives Considered" section scoped its rejection to the future/weather/personal-data refusal categories specifically, citing their adversarial-phrasing risk and Constitution §3's Simplicity concern for a *general-purpose* classifier. This decision is not a general-purpose classifier — it is a closed, enumerable lookup against a handful of literal phrasings for one narrow, benign topic, explicitly requested by the project owner, with a fail-safe (non-matching phrasings still get a correct, if slower, answer via the unchanged full RAG path).

## Consequences

### Positive

- Identity questions answer in effectively zero latency (no network call), a dramatic UX improvement over the ~20-30s full RAG path.
- Eliminates the "irrelevant citation on 'Chi sei?'" failure mode observed live twice (`TEST-INGESTION-0001.md` §4, Plan 0028's live testing) — a matched identity question now always answers from the persona's own truthful self-description, never from a coincidentally-retrieved unrelated document.
- No new external dependency, no new port, no new adapter — implemented entirely inside the existing `RagEngine` use case.

### Negative

- The phrasing list is necessarily incomplete — natural language has many ways to ask "who are you" that this literal-match approach will not catch (e.g. typos, unusual phrasing, other languages). Those fall through to the existing RAG path, which still answers correctly, just without the speed benefit — a graceful degradation, not a failure, but it does mean the "ultra-immediate" guarantee only holds for the enumerated phrasings, not universally.
- The persona's `system_prompt` is now surfaced verbatim to citizens on a match, whereas previously only a `RagEngine`/`GenerationPort`-mediated paraphrase of persona-flavored answers ever reached them. Any future edit to `system_prompt` must be written with the awareness that it may be shown directly, not just used as a steering prompt — already true in spirit (Constitution §5: the fallback text and system_prompt are already citizen-facing-quality Italian), but now literally true for this path.

### Neutral

- This does not change `RAG_TOP_K`/`RAG_MIN_SCORE` or any other retrieval tuning — the broader retrieval-permissiveness risk flagged in Plan 0028's content log remains open and unrelated to this decision.

## Alternatives Considered

### Alternative A: Skip retrieval but still call generation with persona-only context

Detect identity questions the same way, but still call `GenerationPort` with a prompt containing only the persona (no retrieved chunks), letting the LLM phrase the answer in its own words. Rejected: this still incurs the full `llama-generate` latency (~20-30s per ADR 0013), which does not satisfy "ultra-immediate" — the network/inference round trip is the dominant cost, not the retrieval step.

### Alternative B: General-purpose question-category classifier (extending ADR 0012's rejected approach)

Build a broader classifier covering identity questions alongside the categorical-refusal categories. Rejected: conflates two different risk profiles (a missed refusal-category match risks hallucination; a missed identity-question match is merely slower) under one mechanism, and reopens exactly the complexity/adversarial-phrasing concern ADR 0012 already weighed and rejected for the harder cases. A small, purpose-built, literal-match lookup scoped only to identity questions is simpler and its failure mode is benign.

## Compliance

- Enforced by unit tests on the new identity-question matcher (covering match/no-match cases, case-insensitivity, punctuation variance) and a `RagEngine` unit test asserting `EmbeddingPort`/`RetrievalPort`/`GenerationPort` are never invoked when a question matches — a mock-based test that fails loudly if a future change reintroduces a network call on this path.
- A BDD scenario in `features/chat.feature` (or an identity-specific feature file) covers the live "Chi sei?"-shape question against the real active persona, asserting `sources: []`, `fell_back: false`, and the returned text equals the active persona's `system_prompt`.
