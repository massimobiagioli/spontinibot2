# ADR 0003: Commit and Merge Strategy

- **Status**: proposed
- **Date**: 2026-07-09
- **Deciders**: Sisyphus (opencode)
- **Related**: 0001, 0002

## Context

The project uses a feature-branch workflow with opencode plan lifecycle (`/create-plan` → `/approve-plan` → `/implement-plan` → `/review-plan` → `/fix-review`). Each feature branch is merged to `main` after review approval. This ADR codifies the commit and merge conventions to ensure consistent, traceable history.

## Decision

We will use **squash-and-merge** for feature branches into `main`, with a commit message that references the Feature ID (e.g., `feat(0003): rag-engine for /chat`). Individual commits on the feature branch are preserved during development but squashed into a single semantic commit on merge.

## Rationale

Squash-and-merge keeps `main` history clean and each commit corresponds to a complete, reviewable feature. This aligns with PRINCIPLES.md §8 (Traceability) — every commit on `main` maps to a plan and a review. It also simplifies bisect and rollback: one commit = one feature, not a noisy stream of WIP commits.

## Consequences

### Positive

- `main` history is linear and each commit is a complete, reviewable unit
- Commit messages on `main` always reference the Feature ID for traceability
- Easy to revert a feature (single `git revert`)
- No merge commits cluttering the history

### Negative

- Intermediate commits on the feature branch are lost on squash (preserved only in the branch ref until deleted)
- If the branch is not deleted promptly, stale branches accumulate

### Neutral

- The existing opencode plan lifecycle (`/create-plan` → `/fix-review`) is unchanged
- CI gates run on the feature branch before merge

## Alternatives Considered

### Alternative A: Merge commit (no squash)

Preserves every intermediate commit on `main`. Rejected because it creates noisy history where a single feature appears as 10+ commits, making bisect and rollback harder.

### Alternative B: Rebase and merge

Linear history without merge commits, but keeps every intermediate commit. Rejected for the same reason as merge commits — noisy history.

## Compliance

The `git-master` skill enforces squash-and-merge semantics. The `/fix-review` command commits with a message referencing the Feature ID, and the merge is performed as squash. The ADR Registry in AGENTS.md §5 is the permanent pointer to this decision.
