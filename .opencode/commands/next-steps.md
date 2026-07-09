---
description: Run the full plan lifecycle for the next N unchecked roadmap features. Default: 1. Pass an integer N for the next N features, or "all" for every unchecked feature. Each feature runs create-plan → approve-plan → implement-plan → review-plan → fix-review → create-adr, then merges to main.
---

# Next Steps

You are running the full feature lifecycle for one or more features resolved from `.project/ROADMAP.md`. This is an orchestrator: for each feature you execute the six-command sequence in strict order, merge the feature branch to `main`, then proceed to the next feature.

## Step 0 — Parse the argument

`$ARGUMENTS` selects how many unchecked features to process:

- **Empty or omitted** → process exactly **1** feature (the first unchecked row in the roadmap).
- **A positive integer** (e.g. `3`) → process the next **N** unchecked features in ID order.
- **The literal `all`** → process **every** unchecked feature in the roadmap, in ID order.

Parse `$ARGUMENTS` accordingly. If the argument is something else (negative number, non-integer text other than `all`), STOP and report the error.

## Step 1 — Resolve the target features from the roadmap

Read `.project/ROADMAP.md`. Parse every row matching `- [ ] **<ID>** — <Title>` (checkbox **unchecked**). Collect them in document order (which is ID order within each milestone, milestones in order). Take the first N (or all if `all`).

If the roadmap has fewer unchecked features than requested, process all remaining and warn the user at the start: "Requested N features, only M unchecked remain — processing all M."

If the roadmap has zero unchecked features, STOP and tell the user the roadmap is complete.

Record the resolved list as `(Feature ID, Title)` pairs. You will iterate this list.

## Step 2 — For each feature, run the lifecycle

Iterate the resolved list. For each feature with ID `<ID>` and title `<Title>`:

### 2.1 Create the plan

Run the `/create-plan` flow **with no argument** so it reads the roadmap and resolves this feature (it picks the first unchecked row, which is `<ID>` because all prior features have been ticked by the previous iterations). Follow the instructions in [.opencode/commands/create-plan.md](./create-plan.md) exactly: switch to `main`, pull, create `feat/<name>`, author `.project/<ID>-<name>-plan.md` with status `draft`.

### 2.2 Approve the plan

Run the `/approve-plan <ID>` flow. Follow [approve-plan.md](./approve-plan.md): transition the plan status from `draft` to `open`, add the Approved line.

### 2.3 Implement the plan

Run the `/implement-plan <ID>` flow. Follow [implement-plan.md](./implement-plan.md): implement phase by phase, task by task, loading the skills each task lists, running the verify gate between tasks, transitioning the plan to `review` at the end, committing with `feat(<ID>): <name> — implementation complete`.

### 2.4 Review the plan

Run the `/review-plan <ID>` flow. Follow [review-plan.md](./review-plan.md): gather the diff against `main`, review against the binding docs, produce `.project/<ID>-<name>-review.md` with a verdict.

### 2.5 Fix the review and close the plan

Run the `/fix-review <ID>` flow. Follow [fix-review.md](./fix-review.md): implement any required fixes, append the Fix Log, transition the plan to `closed`, commit with `fix(<ID>): address review findings — plan closed`.

### 2.6 Create the ADR (or tick the roadmap directly)

Determine whether the feature introduced a **binding architectural decision** worth recording as an ADR. A feature warrants an ADR if it made a non-obvious structural choice (crate boundaries, port/adapter split, model selection, schema design with a non-obvious trade-off). A pure data-layer addition, a CRUD endpoint, or a UI section typically does **not** warrant a standalone ADR.

- **If the feature warrants an ADR**: run the `/create-adr` flow with the ADR title derived from the feature's primary architectural decision (e.g. "Use libSQL for Vector Storage" for a storage choice). Follow [create-adr.md](./create-adr.md): assign the next ADR ID, write `.adr/<ADR-ID>-<slug>.md`, update `.adr/README.md`, ensure the `AGENTS.md` pointer exists, set the `Related:` line to `Feature <ID>`, and **tick the roadmap** (Step 6 of create-adr). Commit the ADR with message `adr(<ADR-ID>): <title> — feature <ID> closed`.

- **If the feature does NOT warrant an ADR**: tick the roadmap manually. Edit `.project/ROADMAP.md`: change the feature's `- [ ]` to `- [x]`, and append a `Closed:` line under the `Description:` line in the format `Closed: Plan [<ID>](../.project/<ID>-<name>-plan.md).` (no ADR link). Commit with message `chore(<ID>): tick roadmap — feature closed (no ADR)`.

### 2.7 Merge the feature branch to main

After the plan is closed and the roadmap is ticked, merge the feature branch to `main` so the next feature branches from an up-to-date main:

```bash
git switch main
git merge feat/<name> --no-ff -m "Merge feat/<name>: <Title> (Plan <ID>)"
git push origin main
git branch -d feat/<name>
```

If `git push` fails (no remote, no tracking branch, network issue), STOP and report — do not proceed to the next feature with an unmerged branch.

## Step 3 — Report progress after each feature

After each feature completes (or fails), print a one-line status:

```
✅ <ID> — <Title>: closed, merged to main, roadmap ticked.
```

or on failure:

```
❌ <ID> — <Title>: FAILED at step <2.1-2.7>. <reason>. Branch feat/<name> preserved for inspection.
```

## Step 4 — Continue or stop

- After a feature completes successfully, proceed to the next feature in the resolved list.
- After a feature fails, **STOP**. Do not attempt the next feature. Report the failure with enough detail for the user to decide whether to retry, fix, or abort the whole batch.
- After all resolved features complete, print a final summary:

```
Next-steps complete. <M>/<N> features processed.
  ✅ <ID1> — <Title1>
  ✅ <ID2> — <Title2>
  ...
```

After the summary, push `main` to the remote:

```bash
git push origin main
```

If `git push` fails, report the error. All local work is complete; the only issue is remote synchronization.

## Forbidden

- Running the six steps out of order. The lifecycle is strictly: create → approve → implement → review → fix → ADR/tick.
- Skipping any of the six steps. Even if a step seems trivial (e.g. approve is a one-line status change), execute it — it is the audit trail.
- Starting the next feature before the current one is merged to `main`. A feature is not "done" until it is on `main`.
- Pushing the feature branch instead of merging it to main. The feature branch is a local artifact; main is the source of truth.
- Reordering, inserting, or removing roadmap rows. The roadmap is append-only; only the checkbox state changes.
- Running features in parallel. The lifecycle is strictly sequential — each feature may depend on the previous feature's merged changes.
- Continuing past a failed `spontini-verify-gate` check. If any gate fails during implementation or review-fix, STOP and fix the root cause before proceeding.
