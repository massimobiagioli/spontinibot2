---
description: Read everything under .project/test-reports/ and regenerate .project/training/*.md — instructional notes the running bot reads live on every chat answer.
---

# Train

You are turning confirmed, test-session-verified behavioral findings into instructional notes the bot actually reads on every live chat answer. Unlike `/analyze-feedback` (which logs corrected question/answer pairs to the `training_messages` table — data that `RagEngine::answer()` never reads back, confirmed across two independent test sessions to produce zero measurable behavior change), the files this command writes to `.project/training/` are read fresh from disk and folded into the system prompt sent to `llama-generate` on every chat request that reaches generation (see [ADR 0016](../../.adr/0016-train-command-with-live-loaded-training-notes.md), [AGENTS.md §3.8](../../AGENTS.md#38-training-notes-are-read-live-on-every-chat-answer)). A file this command writes takes effect on the bot's very next matching answer — no restart, no reload, no rebuild.

## Binding Principle

**"Don't invent anything. Don't hallucinate."** This applies to you as the executor, exactly as it does for `/analyze-feedback`:

- Never invent a behavioral pattern or instruction that isn't grounded in a real, confirmed finding from `.project/test-reports/`.
- Never phrase a note more strongly than the evidence supports (e.g. don't write "always X" for something observed once with no root-cause analysis backing it as systemic — write it for what it is).
- Every note file must be traceable to specific session(s)/question(s) in a source-provenance line.
- If `.project/test-reports/` contains no usable findings at all, STOP and report it — do not write placeholder or generic notes.

## What This Command Does NOT Do

- It does not call any `/admin/api/training/*` endpoint. That system (used by `/analyze-feedback`) is a separate, already-documented data-logging mechanism unrelated to this one.
- It does not edit the persona (`POST /admin/api/persona`). Training notes are supplementary instructions layered on top of the persona's own `system_prompt`, not a replacement for it — a correction significant enough to be a real identity/policy change belongs in a persona version, not a training note (see ADR 0016's Consequences).
- It does not append to existing note files or accumulate history. It **fully regenerates** `.project/training/` from the current `.project/test-reports/` corpus every run, so the directory is always a deterministic function of what test reports currently say — never hand-edited, never stale.

## Step 0 — Validate Prerequisites

Before starting, verify:

1. `.project/test-reports/` exists and contains at least one file (either under `.project/test-reports/feedback/` or directly as `session-*-rep.md`/`session-*-rep-fix.md`).
2. `.project/training/` is writable (create it if it doesn't exist yet).

If prerequisite 1 fails, STOP and report it — do not write anything to `.project/training/`.

## Step 1 — Read Everything Under `.project/test-reports/`

Read, in full:

1. **Every file in `.project/test-reports/feedback/`** — the synthesized `[OPEN]`/`[RESOLVED]` items with their `Root Cause` and `Recommended Action` analysis. This is the richest source: root-cause analysis is already written in a form close to what a note needs. Status (`[OPEN]` vs `[RESOLVED]`) does not matter here — a resolved feedback item's root cause is still a real, durable behavioral lesson worth encoding, regardless of whether a `training_messages` entry was ever logged for it.
2. **Every `session-*-rep.md` / `session-*-rep-fix.md`** in `.project/test-reports/` — the per-question "Per-Question Detail" tables and the "Feedback Analysis" / "Pattern N" sections. Use these to corroborate and enrich patterns already found in step 1, and to catch a systemic pattern that a session's feedback synthesis never separately named as its own item.

Do not skip any file. Do not sample — this command is meant to run over the whole corpus, however many sessions exist.

## Step 2 — Extract Distinct, Confirmed Behavioral Patterns

Group everything you read into a small number of **distinct behavioral patterns** — not one entry per question, one entry per root cause. Two items from different sessions describing the same underlying defect (e.g. "bot doesn't self-identify as a digital assistant when asked in a different phrasing") are the same pattern, not two.

For each distinct pattern, note:

- A short, stable slug (kebab-case, e.g. `bot-nature-ia-vs-persona`).
- The behavioral instruction itself — what the bot should actually do, derived from the `Root Cause` / `Recommended Action` analysis and, where available, the KB ground truth (the active persona's own `system_prompt`, fetchable via `GET /admin/api/persona?name=<VITE_PERSONA_NAME, default "gaspare">` with the operator session cookie — see `/analyze-feedback`'s Step 0 for how to authenticate) or the exact ADR-mandated string (e.g. ADR 0012's fallback text) when the pattern concerns one.
- Which session(s)/question(s) it's grounded in, for the provenance line.
- Whether it's confirmed systemic (recurred across sessions or across multiple questions in one session) or a single-session finding — reflect this honestly in how strongly the note is phrased.

A pattern found in only one session with weak/inconclusive evidence is not disqualified, but phrase its note accordingly (e.g. "sembra che..." / describe the specific case) rather than as a universal rule — do not inflate confidence beyond what the evidence shows.

## Step 3 — Regenerate `.project/training/`

1. Delete every existing `.md` file directly under `.project/training/` (non-recursive; leave any non-`.md` file, e.g. `.gitkeep`, untouched).
2. For each distinct pattern from Step 2, write `.project/training/<slug>.md` with this shape:

   ```markdown
   # <Short title in Italian>

   <One or more paragraphs in Italian, written as a direct second-person
   instruction to the bot — e.g. "Quando ti chiedono se sei un'intelligenza
   artificiale...", "Non allegare mai una fonte quando...". This is the exact
   text that gets appended to the system prompt sent to the generation model,
   so it must read as an instruction to follow, not as a bug report or a
   third-person description of the bot.>

   _Fonte: <session id(s)>, <question ref(s) if applicable> — <one-line
   confirmation strength, e.g. "confermato 3/3 su due sessioni indipendenti"
   or "riscontrato una sola volta">._
   ```

   The body **must be in Italian** — this content is folded directly into what the LLM reads when answering citizens, so [AGENTS.md §3.1](../../AGENTS.md#31-language)'s runtime-facing exception applies (this command's own prose, this file, and your final report to the user stay in English as usual — only the generated note *content* is Italian).

3. Keep each note focused on one behavior. Do not merge unrelated patterns into one file — the point of separate files is that a future `/train` run, or a human, can reason about and regenerate one pattern independently, and that a bad note can be identified and pulled without touching the others.

## Step 4 — Report

After completing all steps, report:

```
## Training Notes Report — YYYYMMDD-HHmmss

### Summary

- **Test-report files scanned**: <count and list>
- **Distinct patterns extracted**: <count>

### Notes Written

| File | Pattern | Source | Confirmation strength |
|---|---|---|---|
| ... | ... | ... | ... |

### Notes Removed (regenerated away)

<list any `.project/training/*.md` files that existed before this run and were deleted/superseded, if any — or "None (first run)">
```

## Forbidden

- Writing a note not grounded in an actual finding from `.project/test-reports/`.
- Phrasing a single-session, unconfirmed finding as if it were a confirmed systemic rule.
- Calling any `/admin/api/training/*` endpoint (that is `/analyze-feedback`'s job, a separate mechanism).
- Editing the active persona's `system_prompt` via `POST /admin/api/persona`.
- Appending to or hand-preserving old note files instead of fully regenerating the directory.
- Writing non-Italian instructional content into a note file (breaks AGENTS.md §3.1's runtime exception — this content is citizen-facing via the generation prompt).
- Pushing to remote — this command only writes local files; committing/pushing is a separate, explicit step if requested.
