---
description: Run a full 100-question test session: generate mixed questions, invoke the bot, create reports, run training based on feedback, re-test, and synthesize feedback.
---

# New Test Session

You are running a complete 100-question test session against the live Spontini bot. This command orchestrates question generation, bot invocation, report creation, feedback-driven training, re-testing, and feedback synthesis.

## Binding Principle

**"Don't invent anything. Don't hallucinate."** This applies to you as the executor:

- Never invent question text, expected answers, or bot responses.
- Every question must be grounded in real KB content or be a legitimate out-of-KB test case.
- Every expected answer must come from verified facts in the knowledge base or be marked as a behavioral expectation (for open questions).
- If a step cannot be completed for real, STOP and report it — do not guess or fill in placeholders.

## Step 0 — Validate Prerequisites

Before starting, verify:

1. Stack is running: `docker compose ps` shows all containers healthy
2. Operator credential exists and is valid
3. KB has content (not empty): check `documents` table has rows
4. Persona is active: `GET /admin/api/persona?name=<VITE_PERSONA_NAME, default "Gaspare">` returns `is_active:true`. Note the persona `name` field is case-sensitive — the admin-ui default is `Gaspare` (capital G), not `spontini-bot` or lowercase `gaspare`.

If any prerequisite fails, STOP and report which one failed.

## Step 1 — Generate 100 Mixed Questions

Create `.project/test/` directory if it doesn't exist. Generate the question file with timestamp:

```
.project/test/session-YYYYMMDD-HHmmss.md
```

The file must contain **100 questions** of varying nature and difficulty, distributed across these categories:

| Category | Count | Description |
|---|---|---|
| A | 15 | Bot identity / imprinting |
| B | 20 | Comune history (general) |
| C | 15 | Gaspare Spontini (life and works) |
| D | 15 | News (last 3 months) |
| E | 15 | Delibere/Determine (last 3 months) |
| F | 10 | Out-of-KB / honest refusal |
| G | 10 | Edge-case/adversarial questions |

### Question Format

Each question in the file must include:

```markdown
### Q<number>. [<Category>]

**Domanda**: <The question in Italian>

**Risposta attesa**: <Expected answer or behavioral expectation>
```

### Question Generation Rules

1. **Categories A/B/C/D/E**: `.project/TEST-INGESTION-0001.md` no longer exists (removed in a historical cleanup) — do not reference it. There is no admin API that returns raw chunk text either. Derive every anchor fact directly from the live KB instead:
   ```bash
   docker cp spontini-bot-2-backend-1:/data/kb.db <scratchpad>/kb.db
   sqlite3 <scratchpad>/kb.db
   ```
   Schema: `documents(id, source, source_ref, content, metadata, embedding, section, created_at)`. This is a read-only local snapshot — never write to it or copy it back, delete it when done. Note that RAG retrieval (`backend/src/rag_engine/retrieval.rs`) does **not** filter by the `section` column — it searches the entire `documents` table — so also consider any blank-`section` rows when picking source facts, not just the admin UI's configured sections. `SELECT content FROM documents WHERE source_ref = '...'` and actually read the real text before writing a question/expected-answer around it — never infer content from a filename or summary alone.
2. **Categories D/E**: Only if documents are ingested — check via the query above whether `section` (or a source_ref pattern) has real content for news/delibere. If none, skip the category and note it.
3. **Category F**: Must include questions about:
   - Plausible but not-in-KB facts (phone numbers, population of frazioni)
   - Topics entirely unrelated to the comune (weather, recipes)
   - Questions asking for opinions or predictions
   - Questions about personal/sensitive data
4. **Category G**: Include:
   - Open narrative questions
   - Double-negation questions
   - Mixed in-KB and out-of-KB facts
   - Questions asking for source citation
   - Typos and informal language

### Difficulty Variation

Apply these variations systematically:
- **Register**: formal, direct, informal
- **Form**: direct question, polite imperative, affirmative-with-confirmation
- **Breadth**: single-fact vs. multi-fact questions
- **Specificity**: with/without explicit source reference

## Step 2 — Create Training Session and Invoke Bot

1. Create a training session:
   ```bash
   curl -sS -b /tmp/spontini-session.txt -X POST http://localhost:8080/admin/api/training/sessions \
     -H 'Content-Type: application/json' \
     -d '{"title": "Test Session YYYYMMDD-HHmmss", "created_by": "new-test-session"}'
   ```

2. For each of the 100 questions:
   - Send to `POST /admin/api/training/sessions/:id/messages` with **both** `question` and `expected_answer` (the "Risposta attesa" text from the question file) in the body — e.g. `{"question": "...", "expected_answer": "..."}`. This is what populates "Domanda attesa" on the question card in the admin UI; sending it at creation avoids an extra round trip, but a message asked without one can still be backfilled via `PATCH /admin/api/training/messages/:id` (`{"expected_answer": "..."}`) — see [AGENTS.md §3.8](../../AGENTS.md#38-every-test-session-question-must-receive-feedback).
   - The response's own `execution_time_ms` field is the latency — no need for `curl -w` timing tricks.
   - Record the response for the report

3. If session cookie expires (30-minute TTL), re-authenticate and continue.

## Step 3 — Create Initial Report

Create `.project/test-reports/` if it doesn't exist. Write:

```
.project/test-reports/session-YYYYMMDD-HHmmss-rep.md
```

### Report Structure

```markdown
# Test Session Report — YYYYMMDD-HHmmss

## Session Info

- **Date**: YYYY-MM-DD HH:mm:ss
- **Questions**: 100
- **Training Session ID**: <id>

## Summary

- **Total Questions**: 100
- **Average Score**: <calculated>
- **Average Latency**: <calculated>
- **Hallucinations Found**: <count>
- **Category Breakdown**: <table>

## Per-Question Detail

| # | Cat. | Domanda | Risposta attesa | Risposta bot | Tempo di risposta | Feedback |
|---|---|---|---|---|---|---|
| 1 | A | ... | ... | ... | ... | ... |
```

### Feedback Requirements

For each question, the Feedback column must include:
- **Accuracy**: correct/imprecise/wrong
- **Speed**: fast/slow/acceptable
- **Citation**: correct/incorrect/missing
- **Hallucination**: none/minor/major
- **Conciseness**: concise/too verbose/incomplete
- **Issues**: any specific problems observed

## Step 4 — Analyze Feedback and Identify Patterns

After creating the initial report, analyze the feedback:

1. **Read the full report**
2. **Identify systemic patterns**:
   - Which categories have the lowest scores?
   - What types of errors recur?
   - Are there latency hotspots?
   - Which questions triggered hallucinations?
3. **Document findings** in a "Feedback Analysis" section at the end of the report

## Step 5 — Run Training Session Based on Feedback

Per [AGENTS.md §3.8](../../AGENTS.md#38-every-test-session-question-must-receive-feedback), **every one of the 100 questions must get a `POST /admin/api/training/feedback` entry** (`message_id`, `answer_span`, `sentiment`, `comment`) before this session counts as complete — not only the ones with an issue. A correct answer gets a brief `positive` entry; an operator reviewing the question card must never find one with no feedback at all. Do this for every message_id from Step 2, using the per-question assessment already written in Step 3/4's report as the `comment`.

Based on the analysis in Step 4:

1. **Identify training focus areas**:
   - Questions with hallucinations → train on factual grounding
   - Questions with wrong answers → train on correct information
   - Questions with poor citations → train on source attribution
   - Questions with high latency → note for architecture review (not training)

2. **Create a new training session**:
   ```bash
   curl -sS -b /tmp/spontini-session.txt -X POST http://localhost:8080/admin/api/training/sessions \
     -H 'Content-Type: application/json' \
     -d '{"title": "Training Fix YYYYMMDD-HHmmss", "created_by": "new-test-session"}'
   ```

3. **For each problematic question**, send the correct answer with feedback:
   ```bash
   curl -sS -b /tmp/spontini-session.txt -X POST http://localhost:8080/admin/api/training/sessions/:id/messages \
     -H 'Content-Type: application/json' \
     -d '{
       "question": "<original question>",
       "feedback": {
         "sentiment": "negative",
         "comment": "<specific issue>"
       }
     }'
   ```

## Step 6 — Re-run Questions and Create Fix Report

After training, re-run the same 100 questions:

1. Create a new training session for the re-run
2. Send all 100 questions again
3. Create a new report:

```
.project/test-reports/session-YYYYMMDD-HHmmss-rep-fix.md
```

4. Use the same format as the initial report, but add:
   - **Improvement column**: comparing initial vs. fix run
   - **Remaining issues**: what still needs work

## Step 7 — Synthesize Feedback

Create the final feedback synthesis file:

```
.project/test-reports/feedback/session-YYYYMMDD-HHmmss-feedback.md
```

### Feedback Synthesis Structure

```markdown
# Feedback Synthesis — YYYYMMDD-HHmmss

## Session Summary

- **Initial Run**: <date>
- **Fix Run**: <date>
- **Improvement**: <percentage>

## Feedback Items

### [RESOLVED] <issue description>
- **Question**: <question text>
- **Initial Score**: <score>
- **Fix Score**: <score>
- **Resolution**: <what training fixed it>

### [OPEN] <issue description>
- **Question**: <question text>
- **Score**: <score>
- **Root Cause**: <analysis>
- **Recommended Action**: <next steps>

## Systemic Patterns

### Pattern 1: <pattern name>
- **Affected Questions**: <list>
- **Root Cause**: <analysis>
- **Resolution**: <what was done>

## Recommendations

1. <recommendation>
2. <recommendation>
```

## Step 8 — Report

After completing all steps, report:

1. **Files created**:
   - Question file: `.project/test/session-YYYYMMDD-HHmmss.md`
   - Initial report: `.project/test-reports/session-YYYYMMDD-HHmmss-rep.md`
   - Fix report: `.project/test-reports/session-YYYYMMDD-HHmmss-rep-fix.md`
   - Feedback synthesis: `.project/test-reports/feedback/session-YYYYMMDD-HHmmss-feedback.md`

2. **Key findings**:
   - Initial average score
   - Fix run average score
   - Top 3 issues found
   - Top 3 improvements

3. **Remaining work**:
   - Questions still scoring low
   - Systemic issues requiring code changes
   - Recommendations for next steps

## Step 9 — Do Not Commit

`.project/` is entirely gitignored in this repo (`git ls-files .project/` returns zero results — every plan, review, ADR-adjacent doc, and test artifact under it is deliberately local-only, added by a dedicated `fix: gitignore` commit). **Do not `git add`, commit, or push the files created in this session** — `git add` on a gitignored path fails outright (or silently no-ops under `-A`), and force-adding (`git add -f`) would override a deliberate repo-wide policy that isn't this command's call to make. The 4 files from Steps 1/3/6/7 are the deliverable; they stay on disk as local working artifacts, same as every other `.project/` file in this repo.

If a future session genuinely needs these tracked in version control, that requires a deliberate `.gitignore` change made explicitly by a human, not a side effect of running this command.

## Forbidden

- Running fewer than 100 questions (unless categories are empty due to no ingested content)
- Inventing questions, answers, or feedback values
- Skipping the training step even if scores are high
- Creating reports without real data from actual bot invocations
- Continuing after a failure without reporting it
- Committing or pushing `.project/` files — the directory is gitignored by deliberate repo policy; force-adding past that is not this command's call to make
- Referencing `.project/TEST-INGESTION-0001.md` — it no longer exists in this repo
