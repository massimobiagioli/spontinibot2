---
description: Create a feature plan and a feat/ branch. Switches to main, pulls, creates feat/<name>, writes .project/<ID>-<name>-plan.md with status draft.
---

# Create Plan

You are creating a plan for a new feature: **$ARGUMENTS**.

## Steps

### 1. Normalize the feature short name

- Take `$ARGUMENTS` (the feature name) and normalize it to `kebab-case`: lowercase, words separated by single hyphens, no leading/trailing hyphens, ASCII only.
- If the argument is missing or empty, STOP and ask the user for a feature name.
- Reject names that collide with an existing branch or an existing `.project/*-<name>-plan.md` file.

### 2. Assign the Feature ID

The Feature ID is a **4-digit zero-padded number** (e.g., `0001`, `0042`, `1337`).

- Scan `.project/` for existing files matching `^(\d{4})-.*-plan\.md$`.
- Take the highest existing number, add 1, zero-pad to 4 digits.
- If `.project/` is empty or does not exist, start at `0001`.

### 3. Git: switch to main, pull, create branch

Run, in order, stopping on any failure:

```bash
git switch main
git pull --ff-only
git switch -c feat/<normalized-name>
```

- If `main` does not exist (fresh repo), use `git switch -c main` then proceed.
- If the working tree is dirty, STOP and report the dirty files. Do not stash or discard another agent's work.

### 4. Author the plan file

Write `.project/<ID>-<normalized-name>-plan.md` using EXACTLY this template. Fill every section. The plan must be concrete enough that another agent can implement it without guessing.

```markdown
# Plan <ID>: <Feature Name>

- **Status**: draft
- **Branch**: feat/<normalized-name>
- **Feature ID**: <ID>
- **Created**: <YYYY-MM-DD>
- **Owner**: <agent or human name>

## Objective

<One paragraph: what this feature delivers and why. Tie it to the [Constitution](../docs/CONSTITUTION.md) mission. State what is in scope and what is explicitly out of scope.>

## Non-Goals

- <Explicit exclusion 1>
- <Explicit exclusion 2>

## Phases

### Phase 1: <short phase name>

Goal: <what this phase accomplishes>

- [ ] **Task 1.1** — <atomic, self-contained task title>
  - What: <one-sentence implementation statement>
  - Deliverables:
    - <concrete file / module / test / artifact>
    - <concrete file / module / test / artifact>
  - Skills to load: <list relevant skills from: spontini-tdd-rust, spontini-bdd-gherkin, spontini-clean-arch-guard, spontini-rag-build, spontini-ingest-flow, spontini-verify-gate>
  - Verification: <how this task is confirmed done>

- [ ] **Task 1.2** — <...>
  - What: <...>
  - Deliverables: <...>
  - Skills to load: <...>
  - Verification: <...>

### Phase 2: <short phase name>

Goal: <...>

- [ ] **Task 2.1** — <...>
  - What: <...>
  - Deliverables: <...>
  - Skills to load: <...>
  - Verification: <...>

## Acceptance Criteria

- <observable, testable criterion that proves the feature works>
- <observable, testable criterion>
- <BDD scenarios in features/ that must be green>

## Risks

- <risk 1> — mitigation: <...>
- <risk 2> — mitigation: <...>

## Out-of-Scope

- <explicit non-goal>
```

### 5. Rules for authoring

- Every task is **atomic and self-contained**. If a task has "and" in its title, split it.
- Every task has **concrete deliverables** (named files, modules, tests, artifacts — not "a solution").
- Every task lists the **skills** the implementing agent must load. Choose only the skills the task actually triggers; do not list all six.
- Every task has a **verification** step that is observable (test passes, file exists, command succeeds, scenario green).
- Phases are ordered by dependency. A later phase must not start until the earlier one is complete.
- Scope must be small enough to fit in a single PR. If not, split the feature.

### 6. Report

After writing the plan file:

1. Print the absolute path to the plan file.
2. Print the branch name.
3. Print the Feature ID.
4. Tell the user to run `/approve-plan <ID>` when the plan is ready to move to `open`.

## Forbidden

- Starting implementation in this command. This is planning only.
- Setting `Status` to anything other than `draft`.
- Creating the plan file outside `.project/`.
- Using a non-4-digit Feature ID.
- Skipping the git branch creation.
