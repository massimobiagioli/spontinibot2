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
4. At least one feedback file exists in the directory

If any prerequisite fails, STOP and report which one failed.

## Step 1 — Scan for Unresolved Feedback

Read all files in `.project/test-reports/feedback/`:

1. **Parse each feedback file** to identify items with status `[OPEN]`
2. **Filter**: only include items where status is NOT `[RESOLVED]`
3. **Aggregate** the unresolved items across all files

### Feedback File Format

Each feedback file follows the structure created by `/new-test-session`:

```markdown
### [STATUS] <issue description>
- **Question**: <question text>
- **Score**: <score>
- **Root Cause**: <analysis>
- **Recommended Action**: <next steps>
```

Where `STATUS` is either `[OPEN]` or `[RESOLVED]`.

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

1. **Read the feedback file**
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
4. **Write the updated file back**

## Step 5 — Report

After completing all steps, report:

1. **Files scanned**: List all feedback files found
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

## Forbidden

- Creating training sessions without real unresolved feedback
- Marking items as resolved without actually sending training messages
- Inventing feedback items or their status
- Skipping the status update after training
- Continuing after a failure without reporting it
- Pushing to remote — this command commits locally only
- Training on resolved items (only process `[OPEN]` items)
