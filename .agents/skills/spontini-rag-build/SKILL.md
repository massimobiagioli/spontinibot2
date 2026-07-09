---
name: spontini-rag-build
description: Build or modify the rag-engine retrieval-generation flow and the persona prompt. Use WHEN working on query embedding, retrieval, prompt assembly, answer generation, or the persona table. Enforces the 3-part prompt separation, the source-citation requirement, and the honest-unknown fallback.
---

# Spontini RAG Build

You are touching the retrieval-augmented generation flow: query embedding, retrieval from `kb.db`, prompt assembly, generation via `llama-generate`, or the `persona` table. Load this skill.

## The Flow

```
citizen question
  → embed query (llama-embed, HTTP)
  → retrieve chunks (kb.db, vector_distance_cos)
  → assemble prompt (persona + context + question, 3 separate parts)
  → generate (llama-generate, HTTP)
  → answer + cited sources
```

## The 3-Part Prompt (Non-Negotiable)

The final prompt sent to `llama-generate` MUST keep three parts structurally separated:

```
[SYSTEM: persona.system_prompt]
[CONTEXT: chunks retrieved from documents]
[USER: question]
```

- The persona system prompt never contains retrieved chunks.
- The retrieved context never contains the question.
- The question never contains persona instructions.
- Mixing parts is a Truthfulness violation (Constitution §3) and a review blocker.

The persona row is read from the `persona` table where `is_active = 1` (enforced by `idx_persona_active` unique index). It is cached at startup and reloadable via `/admin/persona/reload`.

## Source Citation (Non-Negotiable)

Every answer MUST cite its source. Concretely:

- Each retrieved chunk carries its `documents.id`, `source`, and `source_ref`.
- The generation prompt instructs the model to answer ONLY from the provided context and to reference the source title.
- The response DTO returns the answer text AND the list of cited `documents.id` values.
- The frontend renders the citation as an expandable inline reference.

## Honest Unknown (Non-Negotiable)

If retrieval returns no chunk above the relevance threshold, or no chunk contains the answer:

- Spontini MUST answer using the persona's `fallback_message`.
- The answer MUST state that no information was found in the municipal documents.
- The answer MUST NOT invent details, infer, or extrapolate.
- The answer MUST NOT cite a document that was not retrieved.

This is Constitution §5. Violating it is the most severe defect in this system.

## Embedding Model Constraint

The same embedding model must be used at ingest time (writing embeddings to `kb.db`) and at query time (embedding the citizen question). Changing the embedding model requires a **full re-ingest** of the knowledge base.

- The `llama-embed` container runs ONE embedding model (e.g., `nomic-embed-text`, GGUF).
- The `rag-engine` calls the same `/embedding` endpoint that `ingest-core` uses.
- If you are touching the embedding call in either flow, verify both sides match.

## Persona Table Rules

- Persona is NOT a document in the KB. It must never compete during retrieval.
- Every edit inserts a **new row** with an incremented `version`. Never `UPDATE` an existing persona row.
- Only one row may have `is_active = 1` (enforced by partial unique index).
- Activating a new persona sets `is_active = 0` on the previous row and `is_active = 1` on the new one, in a single transaction.

## Retrieval Parameters (Defaults, Tunable)

- Distance function: `vector_distance_cos` (cosine, libSQL native).
- No ANN index — exact search. The expected data volume stays under 100ms.
- Relevance threshold and `LIMIT` (top-k) are configurable in the rag-engine, not hardcoded in the SQL.
- Metadata filter (category, tags, priority/trust_score) is applied before vector distance when present.

## Workflow

1. Identify whether you are touching: embedding call, retrieval SQL, prompt assembly, generation call, or persona management.
2. Confirm the 3-part prompt structure is intact or being restored.
3. Confirm source citation is present in the response DTO.
4. Confirm the honest-unknown path is covered by a test (load `spontini-bdd-gherkin`).
5. If you touched the embedding model or endpoint, confirm ingest and query sides still match.
6. Load `spontini-tdd-rust` for the code changes.
7. Load `spontini-verify-gate` before claiming done.

## Forbidden

- Concatenating persona, context, and question into a single string.
- Returning an answer without cited document IDs.
- Returning an answer that does not appear in any retrieved chunk.
- Hardcoding the embedding model name in two places — define it once, share via config.
- `UPDATE` on the `persona` table.
- Caching the persona without a reload path.
