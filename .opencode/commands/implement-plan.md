---
description: Implement an open plan phase by phase, task by task. At completion, transitions the plan status to review.
---

# Implement Plan

You are implementing the plan identified by **$ARGUMENTS** (a Feature ID like `0001`, or a path to a plan file).

## Steps

### 1. Resolve the plan file

- If `$ARGUMENTS` is a 4-digit ID, resolve to `.project/<ID>-*-plan.md`.
- If `$ARGUMENTS` is a path, use it directly.
- If the argument is missing, STOP and list every `.project/*-plan.md` with its current status.
- If no plan file matches, STOP with an error.

### 2. Verify the plan is open

- Read the plan file.
- If `Status` is not `open`, STOP. Report the current status. Only `open` plans can be implemented. If `draft`, tell the user to run `/approve-plan <ID>` first. If `review` or `closed`, the plan has already been implemented.

### 3. Verify you are on the correct branch

- The plan's frontmatter declares `- **Branch**: feat/<name>`.
- Run `git branch --show-current`. If the current branch does not match the plan's branch, STOP and ask the user to switch. Do not switch branches yourself — another agent may have work in progress.

### 4. Implement phase by phase, task by task

For each phase, in order:

1. Read the phase goal and all its tasks.
2. For each task, in order:
   a. **Load the skills** listed in the task's `Skills to load` field. Load them BEFORE writing any code.
   b. **Mark the task checkbox** `- [ ]` → `- [~]` (in-progress) at the start of work. Save the plan file.
   c. **Implement the What** using the loaded skills' workflows (TDD red-green-refactor, BDD scenario-first, clean-arch import rules, etc.).
   d. **Produce every Deliverable** listed. If a deliverable cannot be produced, STOP and report — do not silently skip.
   e. **Run the task's Verification**. It must pass.
   f. **Load `spontini-verify-gate`** and run the full verification suite for the changed files (build, test, clippy, fmt, coverage, LSP).
   g. If verification passes, mark the task `- [x]` (done) in the plan file. Save.
   h. If verification fails, fix the root cause. Do not mark the task done until green.

3. After all tasks in a phase are `- [x]`, move to the next phase.

### 5. Respect the skills' rules

- **TDD**: no production code without a failing test. Load `spontini-tdd-rust` for any Rust change.
- **BDD**: for any user-visible behavior, load `spontini-bdd-gherkin` and write the Gherkin scenario BEFORE the use case.
- **Clean Architecture**: for any new crate, module, port, adapter, or import, load `spontini-clean-arch-guard`.
- **RAG flow**: for any change to embedding, retrieval, prompt assembly, generation, or persona, load `spontini-rag-build`.
- **Ingest flow**: for any change to ingest-core, ingest-cli, source adapters, admin-ui upload, chunking, or embedding writes, load `spontini-ingest-flow`.
- **Verification**: before claiming any task or phase done, load `spontini-verify-gate`.

### 6. Do not skip ahead

- Never implement a task in a later phase before the current phase is complete.
- Never mark a task done without its deliverables existing and its verification passing.
- If a task is blocked, STOP, document the blocker in the plan file under a `## Blockers` section, and report to the user.

### 7. Transition to review

When EVERY task in EVERY phase is `- [x]`:

1. Load `spontini-verify-gate` and run the full gate suite once more across the whole workspace.
2. Edit the plan file: change `- **Status**: open` to `- **Status**: review`.
3. Append below the Approved line:

```markdown
- **Implemented**: <YYYY-MM-DD> by <agent or human name>
```

4. Commit the changes (the user has implicitly authorized commits as part of implementation, but do NOT push). Commit message format: `feat(<ID>): <feature short name> — implementation complete`.
5. Print the plan file path, the new status (`review`), the commit SHA, and tell the user to run `/review-plan <ID>`.

## Forbidden

- Implementing a plan that is not `open`.
- Implementing on a branch that does not match the plan's declared branch.
- Marking a task done without its verification passing.
- Skipping the `spontini-verify-gate` between tasks and at the end.
- Editing deliverables or scope of the plan (only the checkboxes and the status/approved/implemented fields).
- Pushing to the remote. Commits only.
