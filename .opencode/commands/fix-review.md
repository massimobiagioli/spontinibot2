---
description: Implement the fixes required by a plan's review file, then close the plan (status to closed).
---

# Fix Review

You are implementing the fixes required by the review of the plan identified by **$ARGUMENTS** (a Feature ID like `0001`, or a path to a plan file).

## Steps

### 1. Resolve the plan and review files

- If `$ARGUMENTS` is a 4-digit ID, resolve to `.project/<ID>-*-plan.md` and `.project/<ID>-*-review.md`.
- If `$ARGUMENTS` is a path, derive the review file from it (replace `-plan.md` with `-review.md`).
- If the plan file is missing, STOP with an error.
- If the review file is missing, STOP and tell the user to run `/review-plan <ID>` first.

### 2. Verify the plan is in review

- Read the plan file.
- If `Status` is not `review`, STOP. Only `review` plans can be fixed-and-closed.

### 3. Read the review and the required fixes

- Open the review file.
- Read the `Verdict` and the `Required Fixes Before Close` section.
- If the verdict is `approved` (no required fixes), skip to step 6 (close the plan).
- If the verdict is `blocked`, STOP and tell the user a new plan is required — `/fix-review` cannot resolve a `blocked` review.
- If the verdict is `changes-requested`, proceed.

### 4. Implement each required fix

For each fix in `Required Fixes Before Close`:

1. Identify the finding ID (e.g., `B1`, `M1`) and the file/line in the review.
2. Load the skill(s) relevant to the finding:
   - Architecture finding → `spontini-clean-arch-guard`
   - RAG / prompt / citation / persona finding → `spontini-rag-build`
   - Ingest / adapter / embedding-write finding → `spontini-ingest-flow`
   - Test / coverage finding → `spontini-tdd-rust`
   - BDD / scenario finding → `spontini-bdd-gherkin`
3. Implement the fix following the loaded skill's workflow (TDD red-green-refactor for code changes).
4. Verify the fix addresses the exact finding — not a symptom, not a workaround.
5. After the fix, load `spontini-verify-gate` and run the full gate suite on the changed files.

### 5. Track fix status

Append a `## Fix Log` section at the bottom of the review file. For each required fix:

```markdown
## Fix Log

- **[B1]** FIXED on <YYYY-MM-DD>. <one-line description of what was done>. Verification: <gate result>.
- **[M1]** FIXED on <YYYY-MM-DD>. <...>.
```

If a fix cannot be implemented (e.g., the suggested fix turns out to be wrong), STOP and report. Do not mark it fixed.

### 6. Close the plan

When every required fix is logged as FIXED and the `spontini-verify-gate` passes across the workspace:

1. Edit the plan file: change `- **Status**: review` to `- **Status**: closed`.
2. Append below the Implemented line:

```markdown
- **Closed**: <YYYY-MM-DD> by <agent or human name>
- **Review verdict**: <approved | changes-requested (resolved)>
```

3. Commit the changes (commits are implicitly authorized by this command; do NOT push). Commit message: `fix(<ID>): address review findings — plan closed`.
4. Print the plan file path, the new status (`closed`), the commit SHA.
5. Tell the user the plan is closed and the branch is ready for merge / PR.

## Forbidden

- Implementing fixes for findings not listed in `Required Fixes Before Close` (scope creep). If you spot a new issue, note it in the review file under a `## New Findings (during fix)` section, but do not fix it here.
- Closing the plan with any unresolved `blocker` or `major` finding.
- Closing a `blocked` review.
- Pushing to the remote.
- Changing the plan's scope or deliverables.
