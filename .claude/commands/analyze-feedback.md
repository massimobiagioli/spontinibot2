---
description: Analyze all unresolved feedback from test sessions and run a training session to address them.
---

# Analyze Feedback

You are analyzing unresolved feedback from test sessions and running a training session to address them. This command reads feedback files, identifies open issues, and creates a training session to improve the bot's performance.

## Binding Principle

**"Don't invent anything. Don't hallucinate."** This applies to you as the executor:

- Never invent feedback items or their resolution status.
- Every feedback item must come from a real test report.
- Every training action must be based on actual issues found in the reports.
- If no unresolved feedback exists, STOP and report it — do not create training sessions without real issues.

## Step 0 — Validate Prerequisites

Before starting, verify:

1. Stack is running: `docker compose ps` shows all containers healthy
2. Operator credential exists and is valid
3. Feedback directory exists: `.project/test-reports/feedback/`
4. At least one feedback file exists in the directory — or, if a session name was passed (see **Arguments**), that specific session's files exist

If any prerequisite fails, STOP and report which one failed.

## Arguments

This command optionally takes a session name, e.g.:

```
/analyze-feedback 20260727-111155
```

- **No argument**: scan every file in `.project/test-reports/feedback/`, as before.
- **Session name given**: scope the scan to that one session only, but read feedback from BOTH of its sources:
  1. **Session notes (closed session)** — `.project/test-reports/feedback/session-<name>-feedback.md`, the synthesized `[OPEN]`/`[RESOLVED]` items.
  2. **Per-question cards** — `.project/test-reports/session-<name>-rep.md` (initial run) and `.project/test-reports/session-<name>-rep-fix.md` (fix run, if it exists) — specifically their "Per-Question Detail" table, which carries its own per-row feedback (the `Feedback` column in the initial-run report; the `Remaining issues` column in the fix-run report) that is not always promoted into its own item in the session notes.

  If neither file exists for that session name, STOP and report the session was not found.

## Step 1 — Scan for Unresolved Feedback

### No session argument (default)

Read all files in `.project/test-reports/feedback/`:

1. **Parse each feedback file** to identify items with status `[OPEN]`
2. **Filter**: only include items where status is NOT `[RESOLVED]`
3. **Aggregate** the unresolved items across all files

### Session argument given

1. **Read the session notes** — `.project/test-reports/feedback/session-<name>-feedback.md` — and parse `[OPEN]` items exactly as above. This file remains the source of truth for status tracking.
2. **Read the per-question cards** for the same session — `session-<name>-rep.md` and, if present, `session-<name>-rep-fix.md` — and go row-by-row through the "Per-Question Detail" table. For each question extract the raw feedback signal (e.g. `Accuracy: wrong/imprecise`, `Citation: incorrect`, `Hallucination: minor/major`, `Conciseness: too verbose/incomplete`, or a non-"Nessuno"/non-"Invariato" `Remaining issues` entry).
3. **Merge, don't duplicate**: a card-level issue for question N is only a *new* unresolved item if the session notes don't already carry a distinct `[OPEN]` or `[RESOLVED]` item covering that same question/root cause. If the notes already track it, use the notes' status — don't create a second item for the same problem. If a card surfaces a problem the notes never promoted to its own item, add it as a new unresolved item, citing the question number and source file (rep.md or rep-fix.md).
4. **Aggregate** the unresolved items from both sources for this one session.

### Feedback File Format

The session notes file follows the structure created by `/new-test-session`:

```markdown
### [STATUS] <issue description>
- **Question**: <question text>
- **Score**: <score>
- **Root Cause**: <analysis>
- **Recommended Action**: <next steps>
```

Where `STATUS` is either `[OPEN]` or `[RESOLVED]`.

The per-question card files (`rep.md`/`rep-fix.md`) instead use a "Per-Question Detail" table — one row per question, with the feedback embedded in a `Feedback`/`Remaining issues` column rather than an explicit `[STATUS]` tag. See "Session argument given" above for how to read those.

## Step 2 — Group Feedback by Category

Group the unresolved feedback items by:

1. **Category** (A-G): Which question category does this belong to?
2. **Issue Type**: What type of problem is this?
   - Hallucination
   - Wrong answer
   - Missing citation
   - Poor conciseness
   - Fallback not triggered
   - Latency issue
   - Other

3. **Priority**: Based on score and severity:
   - **Critical**: Score < 30 OR hallucination present
   - **High**: Score 30-50 OR missing citation
   - **Medium**: Score 50-70 OR conciseness issues
   - **Low**: Score > 70 AND minor issues only

## Step 3 — Create Training Session

Based on the grouped feedback:

1. **Create a training session**:
   ```bash
   curl -sS -b /tmp/spontini-session.txt -X POST http://localhost:8080/admin/api/training/sessions \
     -H 'Content-Type: application/json' \
     -d '{"title": "Feedback Training YYYYMMDD-HHmmss", "created_by": "analyze-feedback"}'
   ```

2. **For each feedback item**, send a training message with the correct answer and feedback:

   ```bash
   curl -sS -b /tmp/spontini-session.txt -X POST http://localhost:8080/admin/api/training/sessions/:id/messages \
     -H 'Content-Type: application/json' \
     -d '{
       "question": "<original question from feedback>",
       "answer": "<correct answer based on KB>",
       "feedback": {
         "sentiment": "<positive or negative>",
         "comment": "<specific issue and how to fix it>"
       }
     }'
   ```

3. **Training message format**:
   - For wrong answers: provide the correct answer with `sentiment: "negative"` and explain what was wrong
   - For missing citations: provide the answer with correct sources and `sentiment: "negative"`
   - For hallucinations: provide the correct answer with `sentiment: "negative"` and explicitly note the hallucination
   - For conciseness issues: provide a concise version with `sentiment: "negative"` and note the verbosity

## Step 4 — Update Feedback Status

After sending each training message, update the feedback file:

1. **Read the feedback file** — the session notes file, `.project/test-reports/feedback/session-<name>-feedback.md`. This is the only file status updates are ever written to; `rep.md`/`rep-fix.md` are immutable historical logs and must never be edited.
2. **Change the status** from `[OPEN]` to `[RESOLVED]`
3. **Add resolution details**:
   ```markdown
   ### [RESOLVED] <issue description>
   - **Question**: <question text>
   - **Score**: <score>
   - **Root Cause**: <analysis>
   - **Recommended Action**: <next steps>
   - **Resolution**: Training session <id> sent with correct answer and feedback
   - **Resolved Date**: YYYY-MM-DD HH:mm:ss
   ```
4. If the item originated from a per-question card (Step 1, "Session argument given" flow) and had no existing entry in the session notes file, **add a new `[RESOLVED]` entry** there too (include a `- **Source**: card, Q<n> (rep.md|rep-fix.md)` line) — so status tracking stays centralized in one file.
5. **Write the updated file back**

## Step 5 — Report

After completing all steps, report:

1. **Files scanned**: List all feedback files found (when a session name was passed, this includes the session notes file plus any per-question card files read — rep.md/rep-fix.md)
2. **Unresolved items found**: Count and breakdown by category/issue type
3. **Training session created**: ID and title
4. **Items resolved**: Count and list
5. **Remaining issues**: Any items that couldn't be resolved (if any)

### Report Format

```
## Feedback Analysis Report — YYYYMMDD-HHmmss

### Summary

- **Files Scanned**: <count>
- **Total Unresolved Items**: <count>
- **Critical**: <count>
- **High**: <count>
- **Medium**: <count>
- **Low**: <count>

### Training Session

- **Session ID**: <id>
- **Title**: <title>
- **Messages Sent**: <count>

### Resolved Items

| File | Issue | Question | Action Taken |
|---|---|---|---|
| ... | ... | ... | ... |

### Remaining Issues

<list any items that couldn't be resolved and why>
```

## Step 6 — Invoke `/train`

After delivering the report, automatically invoke the `/train` command (no argument — full regeneration). `/train` reads the entire `.project/test-reports/` corpus, including the status updates this run just wrote in Step 4, and regenerates `.project/training/` — the instructional notes actually read live on every chat answer (see [ADR 0016](../../.adr/0016-train-command-with-live-loaded-training-notes.md)). Run this even if Step 1 found zero unresolved items this run: `/train` operates on the full corpus, not just this run's deltas, so it's the only way newly-`[RESOLVED]` items from Step 4 actually reach the bot's system prompt.

Do not skip this step, and do not substitute it with a manual edit of `.project/training/` — `/train` is a full regeneration and must run as its own command invocation.

## Forbidden

- Creating training sessions without real unresolved feedback
- Marking items as resolved without actually sending training messages
- Inventing feedback items or their status
- Skipping the status update after training
- Continuing after a failure without reporting it
- Pushing to remote — this command commits locally only
- Training on resolved items (only process `[OPEN]` items)
- Editing `rep.md`/`rep-fix.md` card files — they are immutable historical logs; only the session notes file gets status updates
- Double-counting a per-question card issue that the session notes already track as a distinct item
