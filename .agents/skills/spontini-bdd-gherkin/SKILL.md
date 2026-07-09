---
name: spontini-bdd-gherkin
description: Behavior-Driven Development workflow for Spontini. Use BEFORE implementing any user-visible feature or citizen-facing behavior. Write the Gherkin scenario first, wire step definitions to use cases, then implement. Truthfulness and source-citation are first-class scenario concerns.
---

# Spontini BDD (Gherkin, Outside-In)

You are about to implement a feature whose correctness is visible to a citizen, an operator, or an external system. Load this skill and follow it in order.

## Non-Negotiable Rule

**Scenarios are written before the implementation.** They are a design conversation, not a post-hoc test. If you find yourself writing Gherkin to describe code you already wrote, stop and rewrite the scenarios as if you had not.

## File Layout

```
features/
└── <feature-name>.feature
tests/bdd/
└── <feature-name>_steps.rs
```

Feature files group scenarios by citizen-facing capability, not by internal module.

## Scenario Structure (Mandatory)

```gherkin
Feature: <capability, in domain language>

  Scenario: <one-line behavior statement>
    Given <precondition in domain terms>
    When <one citizen-visible action>
    Then <one observable outcome>
    And <additional outcome or side effect>
```

### Rules

- **One `When` per scenario.** If you need a second `When`, split the scenario.
- `Given` establishes state; `When` triggers; `Then` observes. Never reorder.
- Scenarios speak the language of the domain: citizens, documents, answers, sources. Never HTTP routes, database tables, or crate names.
- Every scenario line is a complete, executable step. No prose, no comments inside scenarios.

## Spontini-Specific Scenario Concerns

These concerns MUST appear as explicit steps whenever the feature touches an answer to a citizen:

### Truthfulness — the answer cites its source

```gherkin
Then Spontini answers with text found in a municipal document
And Spontini cites the source document by title
```

### Honesty — the unknown case is stated

```gherkin
Given the knowledge base contains no document about "<topic>"
When the citizen asks "<question about topic>"
Then Spontini answers that no information was found
And Spontini does not invent any detail
```

### Persona separation — system prompt does not leak

```gherkin
Then the final prompt keeps the persona, retrieved context, and question as three separate parts
```

Step definitions must assert that these three parts remain structurally separated in the prompt sent to `llama-generate`.

## Step Definition Rules

- Step definitions wire Gherkin to **use cases** (application layer), never to controllers or the HTTP layer directly.
- Use real adapters behind test doubles that respect the port contract. Mock only at port boundaries; never mock the use case itself.
- Scenarios are end-to-end where feasible: ingest → retrieve → generate → answer, using a test `kb.db` seeded by the `Given` steps.
- Each scenario is independent and isolated. A scenario must not depend on state left by another scenario.

## Workflow

1. Write the `.feature` file with all scenarios for the capability.
2. Review scenarios against the [Constitution](../../../docs/CONSTITUTION.md) §5 (Knowledge Base Rule) and §3 (Truthfulness). Adjust until every Truthfulness/Honesty concern is explicit.
3. Run the feature — it fails (no step definitions yet).
4. Write step definitions that call use cases. Re-run — they fail because use cases do not exist or behave wrongly.
5. Drop into `spontini-tdd-rust` for the use case and domain code.
6. Scenarios are green when the behavior is real, not when the steps are mocked to pass.

## Forbidden

- Scenarios that describe HTTP status codes or JSON payloads.
- Step definitions that bypass the use case layer.
- Scenarios without an explicit source-citation or unknown-case assertion when the feature produces a citizen answer.
- `Given` steps that set up database rows directly instead of going through the ingest port (use a test adapter).
