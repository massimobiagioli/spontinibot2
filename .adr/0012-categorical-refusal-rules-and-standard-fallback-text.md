# ADR 0012: Categorical Refusal Rules and Standard Fallback Text

- **Status**: accepted
- **Date**: 2026-07-26
- **Deciders**: massimobiagioli, Sisyphus (Claude Code)
- **Related**: none (informed by the `TEST-INGESTION-0001` test campaign, Wave 0 execution)

## Context

`TEST-INGESTION-0001.md`'s Wave 0 (20 mixed-category questions, run against the real live stack) included Category F out-of-KB questions taken verbatim from the plan's own examples: *"Il prossimo sindaco chi sarà?"* (a future prediction), *"Che tempo fa oggi a Maiolati Spontini?"* (live weather), and *"Dove abita l'assessore X?"* (personal data about a public official). Reviewing the bot's real answers to these during the session, the project owner flagged that the existing Constitution §5 rule — "Spontini MUST only answer from documents stored in the KB... No hallucination, no extrapolation" — is correct in principle but too general to be a reliable, defense-in-depth guard against these three specific question shapes. The current enforcement mechanism is entirely retrieval-driven (`RAG_TOP_K`/`RAG_MIN_SCORE`, §1 of the test plan: zero chunks above threshold → honest fallback, generation model never called). That mechanism only refuses correctly *as a side effect* of the KB genuinely containing nothing relevant — it has no explicit, named rule against answering a future prediction, a weather forecast, or a personal-data request even if some retrieved chunk happens to touch the topic (e.g. a document mentioning the current mayor by name could tempt the model to speculate about a "next mayor" question).

The project owner also specified an exact, standard fallback string to use project-wide — replacing reliance on whatever a given persona version's free-text `fallback_message` happens to say.

## Decision

We add four explicit, non-negotiable rules to Constitution §5 (Knowledge Base Rule), each a concrete instance of the existing general rule, not a new principle:

1. No predictions about future events or future office-holders.
2. No weather forecasts (current or predicted).
3. No personal or sensitive data about individuals, including public officials, unless that exact fact is itself published in an ingested official document.
4. When Spontini cannot answer from the KB, for any reason, the response text MUST be exactly: *"Mi dispiace ma è un'informazione che non conosco."*

These are documentation-level, prompt-enforced rules (persona `system_prompt` + the standard fallback text), not a new code-level classifier. The existing retrieval-fallback mechanism (Appendix C of `TEST-INGESTION-0001.md` already tracks its known permissiveness risk via `RAG_MIN_SCORE`) remains the primary technical guard; these rules are the explicit, testable specification of what "no hallucination, no extrapolation" concretely forbids for the question shapes most likely to tempt the model into a fluent-sounding but ungrounded answer.

## Rationale

Evaluated against [Constitution §6](../docs/CONSTITUTION.md#6-decision-making):

1. **Serves the mission?** Yes — citizens trust Spontini precisely because it doesn't guess. A confidently-wrong prediction, weather report, or leaked personal detail is a worse failure mode than an honest "I don't know," and is exactly the zero-tolerance hallucination case Constitution §5 and the test plan's scoring rubric (§9, "Absence of hallucination", weight 20) already treat as maximally costly.
2. **Keeps the stack local?** Unaffected — no new external dependency; this is a documentation and prompt change.
3. **Reduces complexity?** Yes relative to the alternative (a code-level pre-classifier, see Alternatives) — it reuses the persona `system_prompt` mechanism (ADR-free, already-shipped feature 0012's persona table) rather than adding a new component.
4. **Improves UX?** Yes — a single, predictable, standard refusal sentence is easier for citizens to recognize as "the bot genuinely doesn't know" than a varying free-text message, and easier to test for exact-match in the scoring rubric (§9's "Fallback honesty" criterion).

## Consequences

### Positive

- Constitution §5 now names the three question shapes most likely to produce a plausible-sounding but ungrounded answer, giving future contributors (and future persona `system_prompt` edits) an explicit, testable bar instead of only the general principle.
- A single standard fallback string makes Category F scoring (§9 of the test plan) a simple exact-match check instead of a judgment call on whether a free-text message "counts" as an honest refusal.

### Negative

- This is enforced via the persona's `system_prompt` and `fallback_message`, which are LLM-steerable, not hard-coded — a sufficiently adversarial or unusual phrasing (Category G edge cases) could still in principle produce a violation. This ADR does not add a code-level guarantee; the retrieval-fallback mechanism (zero chunks → fallback, generation model never called) remains the only hard guarantee, and it does not itself understand "this is a future-prediction question" as a category.
- Every existing and future persona version must be kept in sync with the exact fallback string; a persona created without updating `fallback_message` to match would drift from this rule silently until caught by testing.

### Neutral

- The `RagEngine`'s retrieval-threshold behavior (Appendix C's "Retrieval threshold known to be potentially permissive" risk) is unchanged by this ADR — still the first lever to tune (§10.2 of the test plan) if irrelevant chunks keep surfacing on these question shapes.

## Alternatives Considered

### Alternative A: Code-level question-category classifier

Add a pre-RAG classifier (regex or a small model) that detects "future prediction" / "weather" / "personal data" question shapes and short-circuits straight to the fallback, before retrieval even runs. Rejected for now: this is real new code and a new failure surface (false positives refusing legitimate historical questions containing similar words), and Constitution §3 (Locality/Simplicity) plus the "no deterministic extractors" design already established for this project (`TEST-INGESTION-0001.md` §1: "no regex/lookup layer... every answer goes through the 3B model") argue against bolting on a special-cased filter for three categories today. Revisit as a real ADR-worthy decision if Wave 1/2 testing shows the prompt-level rule alone is insufficient.

### Alternative B: Leave the standard fallback text as free-text persona configuration, undocumented as a hard rule

Keep `fallback_message` as an arbitrary per-persona string with no canonical value. Rejected: the project owner explicitly wants one exact, standard string enforced project-wide, and an undocumented convention isn't binding on future persona versions the way an accepted ADR + Constitution rule is (AGENTS.md §3.5: accepted ADRs are binding and permanent).

## Compliance

- Constitution §5 states the four rules explicitly and points back to this ADR.
- Every persona version's `fallback_message` must be set to exactly *"Mi dispiace ma è un'informazione che non conosco."* — enforced by review checklist when a new persona version is created (`POST /admin/api/persona`), not by an automated check today.
- `TEST-INGESTION-0001.md`'s Category F (§7.7) and its scoring rubric (§9, "Fallback honesty") are the test-side enforcement: any Category F answer that doesn't return this exact string, zero citations, `fell_back=true`, fails that criterion outright.
