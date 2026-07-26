---
description: Run one bounded execution session against an operational test plan under .project/<FILE>.md (e.g. TEST-INGESTION). Argument (required): the plan's file name without extension. Resolves the next incomplete phase or wave, executes its steps for real, checks off completed items, updates any logs the plan requires, and stops at that boundary — never running the whole plan in one block.
---

# Test Session

You are executing **one bounded session** of an operational test plan — a document like `.project/TEST-INGESTION.md` that describes a real-world test campaign (checklists, phases, waves) rather than a code feature under the `/create-plan` lifecycle. This command is modeled on [next-steps.md](./next-steps.md)'s orchestrator shape (parse argument → resolve target → execute → record → report), adapted to plans that are executed against a live stack instead of implemented as code.

## Binding principle

Every plan this command runs is bound by the same rule `TEST-INGESTION.md` states explicitly: **"Don't invent anything. Don't hallucinate."** This applies to you as the executor too:

- Never fill in a checklist result, a log row, a measured number, or a "to be completed" placeholder with an invented value. If a step calls for something to be measured or observed (e.g. a latency number, a real document title, a real ingested-item count), it must come from a command you actually ran and a real output you actually saw.
- If a step cannot be completed for real right now (missing prerequisite, unreachable service, ambiguous instruction), STOP and report it — do not mark it done, do not guess a plausible-looking result.

## Step 0 — Parse the argument

`$ARGUMENTS` is **required**: the plan's file name without path or extension (e.g. `TEST-INGESTION`). If empty, STOP and ask the user which plan file to run — do not guess or default to any file.

Accept the argument with or without a `.md` suffix or a `.project/` prefix; normalize to the bare name for reporting.

## Step 1 — Resolve and validate the plan file

Resolve `.project/<FILE>.md`. If it doesn't exist, STOP and report the resolved path that was checked (do not search elsewhere or assume a similarly-named file was intended).

Read the full file. Confirm it is a checklist-style operational plan (contains `- [ ]` / `- [x]` items and `## <N>. Phase ...` or `Wave` headings). If the file doesn't look like this shape, STOP and ask the user to confirm before proceeding — this command is not the right tool for a Feature Plan (`.project/<ID>-<name>-plan.md`), which belongs to `/implement-plan` instead.

## Step 2 — Determine this session's scope

Parse every `- [ ]` (unchecked) and `- [x]` (checked) item in document order, together with the phase/wave heading each falls under.

Find the **first phase or wave, in document order, that still has at least one unchecked item**. That phase/wave is this session's scope — nothing outside it.

- If the plan defines explicit **Waves** (e.g. "Wave 0", "Wave 1", ... under an iteration-loop phase) and the first incomplete unit is wave-level, scope to that single wave, not the whole phase it lives in.
- Otherwise scope to the single top-level `## N. Phase ...` section containing the first unchecked item.
- If a phase/section has no checkboxes at all (e.g. a pure narrative/reference section like a scoring rubric), skip it when scanning for scope — it has nothing to execute — but do consult it as context if a step you execute needs it (e.g. the scoring rubric while logging a result).

If every checklist item in the file is already checked, STOP and report the plan is fully complete — do not re-run anything.

## Step 3 — Execute the scoped items, in order

For each unchecked item in scope, in document order:

1. Read the item's own instructions literally (the plan writes out the actual commands/criteria — run them, don't paraphrase or substitute your own approach).
2. Execute it for real against the actual local stack (`docker compose`, `curl`, `sqlite3`, etc., exactly as written in the plan). If the plan's command needs a value only known at runtime (an id returned by a previous step, a session cookie path), carry it forward from what you actually observed, not from the plan's illustrative placeholder.
3. Verify the item's own stated success condition before checking it off (e.g. "poll until status is done", "must answer 1851 and cite the storia source"). If the condition isn't met, do not check the item off — treat it as a failure (Step 4).
4. If the item asks you to record something in the plan itself (a table row, a measured number, a filled-in placeholder like `[TO BE COMPLETED POST-INGESTION]`), edit the plan file with the real value you just observed.
5. If the item asks you to produce an external artifact (a CSV log, a dated report file, a results file under a path the plan specifies), create it now, with real data only.

## Step 4 — Handle failures

If a step's success condition isn't met, or a command errors, or the plan's own exit/gate criteria for this scope aren't satisfied:

- STOP executing further items in this session.
- Do not check off the failed item or any item after it, even if they look independent — leave the plan's checkbox state exactly reflecting reality.
- Report the failure with enough detail to act on it (which item, what was expected per the plan, what actually happened, any diagnostic the plan itself suggests — e.g. `TEST-INGESTION.md`'s tuning-lever table).

## Step 5 — Write the session report file

Before committing, write the durable, file-based record of this session — separate from Step 7's chat summary, which is ephemeral to this conversation.

Create `.project/test-reports/` if it doesn't exist. The report file is `.project/test-reports/<FILE>-rep.md` (same bare `<FILE>` resolved in Step 0/1, suffixed `-rep.md`) — one persistent, accumulating file per plan, not per session.

- If the file doesn't exist yet, create it with a `# <FILE> — Test Session Report` heading.
- Append a new dated section for this session: `## Session <date> — <phase/wave name>`, covering the same ground Step 7 reports to the user — scope, items completed vs. failed/skipped and why, real values/logs actually recorded (their real content, not a restatement of placeholder text), and what the next session would pick up.
- **If this session asked the bot any questions** (a smoke test, a wave, a training-session batch — anything that called `/chat` or `POST .../training/sessions/:id/messages`), include a per-question table with exactly these columns, one row per question actually asked:

  | Domanda | Risposta attesa | Risposta bot | Tempo di risposta | Feedback |
  |---|---|---|---|---|

  - **Domanda**: the exact text sent.
  - **Risposta attesa**: the expected answer or behavior, grounded in what the plan itself already establishes — a verified anchor fact (e.g. Appendix B), an explicit success criterion the item states (e.g. "must answer 1851 and cite the storia source"), or a category-level rule (e.g. Category F's "exactly the fallback_message, zero citations, fell_back=true"). For open questions with no single correct string (e.g. identity/tone questions), state the expected *behavior* instead of inventing a canonical answer. Never fabricate a fact-shaped expected answer that isn't actually grounded this way.
  - **Risposta bot**: the real returned `answer` text, verbatim.
  - **Tempo di risposta**: the real measured latency (same number logged in the CSV, if this session also produces one).
  - **Feedback**: your own honest assessment of this specific exchange — free text, using terms like *imprecisa*, *troppo lenta*, *incompleta*, *troppo prolissa*, *allucinazione importante* where they actually apply, or noting it's correct/on-target when it is. Base this only on what you actually observed (the real answer vs. the real KB content), never a guess.
- Same binding principle as everywhere else in this command: never invent content to fill this file out. A session that found nothing beyond "still blocked" gets a report section that says exactly that.

## Step 6 — Check off progress and commit

For every item that genuinely completed successfully, flip `- [ ]` to `- [x]` in the plan file (and only those — leave everything outside this session's scope untouched, including later phases/waves).

Stage the plan file, any artifacts created in Step 3.5, and the report file from Step 5, and commit locally:

```
test(<FILE>): session — <phase/wave name> <complete|partial>
```

Do not push. This command only commits locally — pushing a shared plan/log file is a separate, explicit decision for the user to make.

## Step 7 — Report

Report, concisely:

- Which phase/wave was this session's scope.
- Which items completed (checked off) vs. which failed or were skipped, and why.
- Any values/logs actually recorded in the plan or in a new artifact file, with their real content (not a restatement of the plan's placeholder text).
- Whether the plan is now fully complete, or what the next `/test-session <FILE>` invocation would pick up.

## Forbidden

- Running more than one phase/wave in a single invocation. The plan itself may explicitly warn against big-bang execution (e.g. "don't run the 1000 questions in one blind block") — respect that even when the plan doesn't spell it out, by staying inside the single scope resolved in Step 2.
- Marking a checklist item done without having actually executed and verified it this session.
- Inventing a fact, number, title, URL, or log row to fill a placeholder — if it wasn't observed for real in this session, leave the placeholder as-is and report the gap instead.
- Continuing to the next item after a failure in the current session's scope.
- Pushing to the remote. This command commits locally only.
- Treating this command as a substitute for `/implement-plan` on a Feature Plan — it's exclusively for checklist-style operational plans under `.project/`.
