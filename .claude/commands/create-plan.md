---
description: Create a feature plan and a feat/ branch. With no argument, reads .project/ROADMAP.md and picks the next unchecked feature. Switches to main, pulls, creates feat/<name>, writes .project/<ID>-<name>-plan.md with status draft.
---

# Create Plan

You are creating a plan for a new feature. The feature is resolved from `$ARGUMENTS` if provided, otherwise from the roadmap (see Step 0).

## Steps

### 0. Resolve the feature (roadmap fallback when no argument is given)

If `$ARGUMENTS` is missing or empty, read `.project/ROADMAP.md` and resolve the next unchecked feature:

1. Parse the roadmap in document order, top to bottom.
2. Find the **first** row matching `- [ ] **<ID>** — <Title>` (checkbox unchecked).
3. If no unchecked feature exists, STOP and tell the user the roadmap is complete (or that a new feature must be appended first).
4. Otherwise: set the resolved **Feature ID** to `<ID>` (the 4-digit number from the roadmap), the **title** to `<Title>`, and the **brief** to the row's `Description:` line (the indented line immediately under the title row).
5. The title (NOT the brief) is normalized in Step 1 to derive the branch and file name. The brief is fed into the Objective and Non-Goals sections in Step 4.

If `$ARGUMENTS` is provided, the roadmap is NOT consulted; proceed with Step 1 using `$ARGUMENTS` as the feature name and let Step 2 auto-assign the next 4-digit ID.

### 1. Normalize the feature short name

- Take the feature title (from Step 0 when the roadmap was used, or `$ARGUMENTS` when it was provided) and normalize it to `kebab-case`: lowercase, words separated by single hyphens, no leading/trailing hyphens, ASCII only.
- If the argument is missing, empty, AND the roadmap has no unchecked feature, STOP and ask the user for a feature name (or to append a new feature to the roadmap).
- Reject names that collide with an existing branch or an existing `.project/*-<name>-plan.md` file.

### 2. Assign the Feature ID

The Feature ID is a **4-digit zero-padded number** (e.g., `0001`, `0042`, `1337`).

- If the roadmap was used (Step 0), the Feature ID is already known — use it verbatim. Verify it does not collide with an existing `.project/<ID>-*-plan.md` file.
- Otherwise (argument was provided): scan `.project/` for existing files matching `^(\d{4})-.*-plan\.md$`. Take the highest existing number, add 1, zero-pad to 4 digits. If `.project/` is empty or does not exist, start at `0001`.

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

When the feature was resolved from the roadmap (Step 0), seed the **Objective** paragraph from the roadmap row's `Description:` line — expand it into full sentences, tie it to the [Constitution](../docs/CONSTITUTION.md) mission, and state explicitly what is in scope and what is out of scope. Do not copy the brief verbatim; elevate it into a plan-grade objective.

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
4. If the feature was resolved from the roadmap, print the roadmap row (`<ID> — <Title>`) and confirm it is the next unchecked feature.
5. Tell the user to run `/approve-plan <ID>` when the plan is ready to move to `open`.

## Forbidden

- Starting implementation in this command. This is planning only.
- Setting `Status` to anything other than `draft`.
- Creating the plan file outside `.project/`.
- Using a non-4-digit Feature ID.
- Skipping the git branch creation.
- Ticking the roadmap row in this command. The roadmap is ticked only after the plan is closed AND the resulting ADR (if any) is accepted — that is the final action of the feature-close sequence, not of planning.
- Editing the roadmap to reorder, insert, or remove features. The roadmap is append-only within a milestone; only the checkbox state changes.
