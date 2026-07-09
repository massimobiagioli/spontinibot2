---
description: Code-review the implementation of a plan in review state. Produces .project/<ID>-<name>-review.md.
---

# Review Plan

You are code-reviewing the implementation of the plan identified by **$ARGUMENTS** (a Feature ID like `0001`, or a path to a plan file). You are a reviewer, not an implementer. Read-only with respect to source code.

## Steps

### 1. Resolve the plan file

- If `$ARGUMENTS` is a 4-digit ID, resolve to `.project/<ID>-*-plan.md`.
- If `$ARGUMENTS` is a path, use it directly.
- If missing, STOP and list plans with their statuses.
- If no match, STOP with an error.

### 2. Verify the plan is in review

- Read the plan file.
- If `Status` is not `review`, STOP. Only `review` plans can be reviewed. If `open`, tell the user to run `/implement-plan <ID>` first. If `closed`, the plan is already finalized.

### 3. Gather the diff

- Identify the plan's branch (`- **Branch**: feat/<name>`).
- Compute the diff against `main`:

```bash
git diff main...feat/<name>
```

- Read every changed file in full. Do not review only the diff — read the surrounding context.

### 4. Review against the binding documents

Load and apply, in this order of precedence:

1. [docs/CONSTITUTION.md](../../docs/CONSTITUTION.md) — mission, truthfulness, locality, scope. Reject anything that violates §3 (Truthfulness) or §5 (Knowledge Base Rule).
2. [docs/PRINCIPLES.md](../../docs/PRINCIPLES.md) — Clean Code, Clean Architecture, SOLID, TDD, BDD, Clean Design, 100% coverage.
3. [docs/STACK.md](../../docs/STACK.md) — stack constraints (Rust versions, libSQL schema, llama.cpp instances, crate boundaries).

### 5. Review dimensions

For each dimension, produce explicit findings. Use the severity scale: `blocker`, `major`, `minor`, `nit`.

#### 5.1 Architecture (Clean Architecture + SOLID)

- Does every dependency point inward?
- Are ports defined in the application/domain layer and implemented outside?
- No framework types in domain or application code?
- SRP respected on every `*Service`, `*Manager`, `*Handler`?
- Use `spontini-clean-arch-guard` crate matrix as a checklist.

#### 5.2 Truthfulness and RAG correctness

- Does the final prompt keep persona / context / question as three separate parts?
- Does every citizen answer cite its source document?
- Is the honest-unknown fallback covered by a test?
- No hallucination path exists?
- Use `spontini-rag-build` rules.

#### 5.3 Ingest correctness (if touched)

- Same embedding model on ingest and query sides?
- Adapters do not embed or write to kb.db directly?
- No logic duplication between `ingest-cli` and `admin-ui`?
- Use `spontini-ingest-flow` rules.

#### 5.4 Tests

- 100% line / 80% branch coverage on changed production code.
- TDD followed (tests are behavioral, not tautological).
- BDD scenarios exist for every user-visible behavior.
- No `#[ignore]`, no deleted tests, no hardcoded assertions.

#### 5.5 Clean Code

- Names reveal intent.
- Functions small, one thing each, one level of abstraction.
- No magic numbers, no dead code, no `unwrap()` without justification.

#### 5.6 Clean Design (if UI/UX touched)

- One thing per screen.
- Generous whitespace, material honesty, max 2 colors, max 3 type sizes.
- Every answer has an expandable citation.
- Honest loading states, no fake typing delays.

#### 5.7 Plan conformance

- Every task's deliverables exist.
- Every task's verification passes.
- No unrequested scope creep.

### 6. Produce the review file

Write `.project/<ID>-<normalized-name>-review.md` using this template:

```markdown
# Review <ID>: <Feature Name>

- **Plan**: [<plan file name>](./<ID>-<normalized-name>-plan.md)
- **Branch**: feat/<name>
- **Reviewed**: <YYYY-MM-DD>
- **Reviewer**: <agent or human name>
- **Verdict**: <approved | changes-requested | blocked>

## Summary

<2-4 sentences: what was implemented, overall quality, whether it ships.>

## Findings

### Blockers

- **[B1]** <file:line> — <finding>. <expected vs actual>. <suggested fix>.
- **[B2]** <...>

### Major

- **[M1]** <file:line> — <finding>. <expected vs actual>. <suggested fix>.

### Minor

- **[m1]** <file:line> — <finding>. <suggested fix>.

### Nits

- **[n1]** <file:line> — <comment, no fix required>.

## Dimension Checklist

| Dimension | Result | Notes |
|---|---|---|
| Architecture (Clean Arch + SOLID) | <pass/fail> | <...> |
| Truthfulness & RAG | <pass/fail/n/a> | <...> |
| Ingest correctness | <pass/fail/n/a> | <...> |
| Tests (coverage + TDD + BDD) | <pass/fail> | <...> |
| Clean Code | <pass/fail> | <...> |
| Clean Design (UI/UX) | <pass/fail/n/a> | <...> |
| Plan conformance | <pass/fail> | <...> |

## Coverage Report

- Line coverage on changed files: <N>%
- Branch coverage on changed files: <N>%
- Excluded files: <list or none>

## Required Fixes Before Close

If verdict is `changes-requested` or `blocked`, list the exact fixes that `/fix-review <ID>` must implement:

1. <fix description, referencing finding IDs like B1, M1>
2. <...>
```

### 7. Verdict rules

- **approved** — zero blockers, zero majors. The plan can move to `closed`.
- **changes-requested** — at least one blocker or major. `/fix-review` must run.
- **blocked** — a finding cannot be fixed without re-planning (e.g., architectural violation requiring a new plan). Recommend the user create a follow-up plan.

### 8. Do not edit source code

This command is read-only with respect to the implementation. It only writes the review file. It does NOT change the plan's status. It does NOT commit anything.

### 9. Report

- Print the path to the review file.
- Print the verdict.
- If `approved`, tell the user to run `/fix-review <ID>` (which will close the plan) — or, if the user prefers, they can close it manually.
- If `changes-requested` or `blocked`, tell the user to run `/fix-review <ID>` to address the findings.

## Forbidden

- Editing source code.
- Changing the plan's status (only `/fix-review` closes, only `/implement-plan` moves to review).
- Committing or pushing.
- Skipping any dimension in §5. If a dimension is not applicable, mark it `n/a` with a reason.
- Approving with any unresolved blocker or major.
