---
description: Move a plan from status draft to open, unlocking implementation.
---

# Approve Plan

You are approving the plan identified by **$ARGUMENTS** (a Feature ID like `0001`, or a path to a plan file).

## Steps

### 1. Resolve the plan file

- If `$ARGUMENTS` is a 4-digit ID (`^\d{4}$`), resolve it to `.project/<ID>-*-plan.md` (glob match).
- If `$ARGUMENTS` is a path, use it directly.
- If the argument is missing, STOP and list every `.project/*-plan.md` file with its current Status, then ask which one to approve.
- If no plan file matches, STOP with an error.

### 2. Read the plan and check its status

- Open the plan file.
- Find the `- **Status**:` line in the frontmatter.
- If the status is **not `draft`**, STOP and report the current status. Only `draft` plans can be approved. A plan that is already `open`, `review`, or `closed` must not be re-approved.

### 3. Transition the status

Edit the plan file: change `- **Status**: draft` to `- **Status**: open`.

Do not touch any other field. Do not edit the phases, tasks, or deliverables. Do not reorder content.

### 4. Add an approval log line

Append a line immediately below the `Status` field:

```markdown
- **Approved**: <YYYY-MM-DD> by <agent or human name>
```

### 5. Report

- Print the path to the plan file.
- Print the new status: `open`.
- Tell the user to run `/implement-plan <ID>` to start implementation.

## Forbidden

- Approving a plan that is not in `draft` state.
- Editing anything other than the Status field and the Approved line.
- Creating a new file. Modify the existing plan in place.
