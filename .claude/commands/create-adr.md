---
description: Create an Architecture Decision Record. Writes .adr/<ID>.md, updates the ADR registry, and references it in AGENTS.md.
---

# Create ADR

You are creating an Architecture Decision Record for: **$ARGUMENTS** (the ADR title / decision subject).

## Steps

### 1. Assign the ADR ID

The ADR ID is a **4-digit zero-padded number** (e.g., `0001`, `0042`), independent from the Feature ID sequence.

- Scan `.adr/` for existing files matching `^(\d{4})-.*\.md$` (excluding `README.md`).
- Take the highest existing number, add 1, zero-pad to 4 digits.
- If `.adr/` is empty or only contains `README.md`, start at `0001`.

### 2. Normalize the title

- Take `$ARGUMENTS` and normalize to a short, descriptive title in Title Case (e.g., "Use libSQL for Vector Storage").
- Derive the file slug: kebab-case of the title (e.g., `use-libsql-for-vector-storage`).

### 3. Write the ADR file

Write `.adr/<ID>-<slug>.md` using EXACTLY this template (based on Michael Nygard's ADR format, adapted to this project):

```markdown
# ADR <ID>: <Title>

- **Status**: proposed
- **Date**: <YYYY-MM-DD>
- **Deciders**: <agent or human name(s)>
- **Related**: <Feature ID(s) or other ADR IDs, or "none">

## Context

<Describe the architectural forces at play. What is the problem? What constraints exist (technical, political, social, project-specific)? Reference the [Constitution](../docs/CONSTITUTION.md) and [Principles](../docs/PRINCIPLES.md) where relevant.>

## Decision

<State the decision clearly, in one or two sentences. "We will X" — not "We considered X".>

## Rationale

<Why this decision. What forces it resolves. What criteria from the Constitution §6 (Decision-Making) it satisfies. One paragraph.>

## Consequences

### Positive

- <consequence 1>
- <consequence 2>

### Negative

- <consequence 1>
- <consequence 2>

### Neutral

- <consequence 1>

## Alternatives Considered

### Alternative A: <name>

<One-sentence description. Why it was rejected.>

### Alternative B: <name>

<One-sentence description. Why it was rejected.>

## Compliance

<How this decision is enforced. E.g., "The `spontini-clean-arch-guard` skill crate dependency matrix rejects any attempt to depend on an external LLM client crate." Reference a skill, a test, a CI gate, or a review checklist.>
```

### 4. Update the ADR registry

If `.adr/README.md` does not exist, create it with this header:

```markdown
# Architecture Decision Records (ADR)

This directory holds all architecture decisions for Spontini Bot 2. Each ADR is a short, immutable record of a decision: context, decision, rationale, consequences, alternatives.

Referenced by [AGENTS.md](../AGENTS.md) §5.

| ID | Title | Status | Date |
|---|---|---|---|
```

Then append a new row to the table:

```markdown
| [<ID>](./<ID>-<slug>.md) | <Title> | proposed | <YYYY-MM-DD> |
```

Keep the table sorted by ID ascending.

### 5. Reference in AGENTS.md

Open `AGENTS.md`. In Section 5 (Prompts, Skills, and Agents Registry), locate or create an "ADR Registry" subsection at the end of Section 5 (after the Type Values block). Add a line:

```markdown
### ADR Registry

Architecture decisions live in [.adr/](./.adr/), indexed in [.adr/README.md](./.adr/README.md). When a new ADR is added, append a row to that index. This entry in AGENTS.md is a permanent pointer — no per-ADR row is added here.
```

If this subsection already exists, do not duplicate it.

### 6. Tick the related feature in the roadmap

After the ADR file is written and the registries are updated, **tick the related feature** in `.project/ROADMAP.md`:

1. Read the ADR's `Related:` line. It lists one or more Feature IDs (4-digit numbers, e.g. `0003`) or other ADR IDs, or `none`.
2. For each Feature ID found in the `Related:` line:
   - Locate the row in `.project/ROADMAP.md` whose checkbox line is `- [ ] **<ID>** — <Title>`.
   - Change `- [ ]` to `- [x]`.
   - Append (or update) a `Closed:` line under the row's `Description:` line, linking the plan and this ADR. Format: `Closed: Plan [<ID>](../.project/<ID>-<name>-plan.md), ADR [<ID>](../.adr/<ID>-<slug>.md).` Use the actual plan filename found in `.project/` and the actual ADR filename just written. If no plan file exists for the Feature ID, link only the ADR and note `Plan: n/a`.
   - If the row is already ticked (the feature had a prior related ADR), keep it ticked and add this ADR to the existing `Closed:` line (comma-separated) rather than duplicating the line.
3. If `Related:` is `none` or lists only other ADR IDs (no Feature ID), DO NOT tick any roadmap row — this ADR is not tied to a feature close.
4. Ticking is the **last** action of the feature-close sequence and is performed by this command, never by `/create-plan`, `/implement-plan`, or `/fix-review`.

### 7. Report

- Print the path to the new ADR file.
- Print the ADR ID.
- Print the path to `.adr/README.md` (updated).
- Print the path to `.project/ROADMAP.md` if any feature row was ticked, and list the Feature ID(s) ticked.
- Tell the user the ADR is in `proposed` status and should be discussed before being marked `accepted`.

## ADR Status Lifecycle

ADRs use these statuses. Only the first is set by this command:

- `proposed` — decision is drafted, not yet ratified.
- `accepted` — decision is ratified and binding.
- `deprecated` — superseded or no longer applies; a successor ADR should reference it.
- `superseded` — replaced by another ADR; the successor ADR ID is noted in the status line.

Changing an ADR's status is a manual edit (not a command) and should be done as part of a separate, deliberate approval conversation.

## Forbidden

- Editing an existing ADR's content after it is `accepted` — ADRs are immutable once accepted. Create a new ADR that supersedes it instead.
- Creating an ADR without updating `.adr/README.md`.
- Creating an ADR without the `AGENTS.md` pointer existing at least once.
- Using a non-4-digit ADR ID.
- Omitting any section of the template.
- Ticking a roadmap feature row whose `Related:` Feature ID has no closed plan in `.project/` — the plan must be closed first; if it is not, STOP and tell the user to close the plan before creating the ADR.
- Ticking a roadmap row when `Related:` is `none` or lists no Feature ID.
