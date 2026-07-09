---
name: spontini-tdd-rust
description: Red-Green-Refactor TDD workflow for the Spontini Rust workspace. Use BEFORE writing or modifying any production Rust code in backend/, ingest-core/, ingest-cli/, kb-store/. Enforces failing-test-first, minimal-green, refactor, then coverage gate.
---

# Spontini TDD (Rust Workspace)

You are writing or modifying Rust production code in this workspace. Before you touch any implementation, load and follow this skill end-to-end.

## Non-Negotiable Rule

**No production code without a failing test.** The only exception is throwaway spike code, which is deleted before commit.

## The Cycle

### 1. RED — Write the failing test first

- Place the test in the same module as the code under test (Rust idiom: `#[cfg(test)] mod tests` at the bottom of the file), OR in a sibling `tests/` integration file if it crosses crate boundaries.
- Test name format: `should_<expected_behavior>_when_<condition>` in snake_case.
- One behavior per test. If you wrote "and" between assertions, split the test.
- Arrange-Act-Assert structure. No branching logic inside tests.
- Run the test and watch it **fail for the right reason** (assertion failure, not compile error in unrelated code):

```bash
cargo test -p <crate> <test_name> -- --nocapture
```

If it fails to compile because the function/type does not exist yet, write the minimum stub (return `todo!()`) so the test compiles and fails on assertion, not on resolution.

### 2. GREEN — Write the minimum code to pass

- Write the smallest amount of code that makes the test pass. No extra features, no "while I'm here" generalization.
- No `unwrap()` / `expect()` in production code unless the failure is genuinely unrecoverable and you document why. Prefer `?` with typed errors.
- Re-run the test:

```bash
cargo test -p <crate> <test_name> -- --nocapture
```

It must pass. If it does not, you are not allowed to edit the test to make it pass — fix the code.

### 3. REFACTOR — Improve structure, keep tests green

- Rename for clarity, extract functions, remove duplication, align with SOLID.
- **Do not change behavior.** After each refactor step, re-run:

```bash
cargo test -p <crate> -- --nocapture
```

Tests stay green. If a test breaks during refactor, you changed behavior — revert and try again.

## Coverage Gate (Before Claiming Done)

```bash
# Build and test the whole workspace
cargo test --workspace --all-targets

# Lint — warnings are errors
cargo clippy --workspace --all-targets -- -D warnings

# Format check
cargo fmt --all -- --check
```

100% line coverage and 80% branch coverage are required on production code. Run the coverage tool configured for the workspace and confirm the changed files meet the threshold. Uncovered branches must either gain a test or be added to `coverage-exclusions.txt` with a documented reason.

## Forbidden

- Writing tests after implementation.
- Testing implementation details (private function signatures, internal call order) rather than behavior.
- `#[cfg(test)]` leaking conditional production behavior.
- Skipping a test with `#[ignore]` to make CI green.
- Deleting or weakening a test to make it pass.
- Asserting on values that mirror the implementation (tautology).

## When This Skill Does Not Apply

- Writing Gherkin scenarios → use `spontini-bdd-gherkin`.
- Adding/wiring crates or ports → also load `spontini-clean-arch-guard`.
- The change crosses the rag-engine or ingest pipeline boundary → also load `spontini-rag-build` or `spontini-ingest-flow`.
