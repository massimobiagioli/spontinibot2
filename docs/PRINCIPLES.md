# Spontini Bot 2 — Engineering & Design Principles

This document defines the non-negotiable engineering and design principles that govern every line of code, every architectural decision, and every user-facing surface of Spontini Bot 2.

These principles are not aspirational. They are **enforced standards**. Any pull request, prototype, or spike that violates them must be justified explicitly against the [Constitution](./CONSTITUTION.md) decision-making criteria.

---

## 1. Clean Code

Code is read far more often than it is written. We write code for **humans first**, compilers second.

### Rules

- **Meaningful names.** A name must reveal intent. `d` is forbidden; `daysSinceLastInvoice` is required. No disinformation, no encodings, no noise words (`Manager`, `Processor`, `Data` appended meaninglessly).
- **Small functions.** A function does **one thing**. If a function contains the word "and" in its description, it does two things — split it. Target: functions rarely exceed 20 lines.
- **Single level of abstraction per function.** Mixing high-level orchestration with low-level details inside one function is a violation.
- **No comments that lie.** Comments do not compensate for bad code. Prefer a well-named function over a comment. The only acceptable comments explain *why*, never *what*.
- **No dead code.** Delete it. Version control remembers.
- **Error handling is not an afterthought.** Exceptions over return codes. Never swallow errors. Never return `null` when you can return an empty collection or a Result type.
- **No magic numbers or strings.** Extract named constants.
- **Pure functions where possible.** Side effects are explicit, isolated, and documented.

### Anti-Patterns (Rejected on Review)

- Deep nesting (>3 levels)
- Boolean flag arguments that change function behavior
- Functions with more than 3 parameters (collapse into an object if genuinely needed)
- "And" in a function name (`validateAndSave` → `validate` + `save`)

Reference: Robert C. Martin, *Clean Code*.

---

## 2. Clean Architecture

The system is organized around **dependencies that point inward** toward stable domain policy. Outer layers are details; inner layers are intent.

### Layered Structure

```
┌─────────────────────────────────────┐
│  Frameworks & Drivers (UI, DB, Web) │  ← Details, replaceable
├─────────────────────────────────────┤
│  Interface Adapters (Controllers,   │  ← Translation only
│  Presenters, Gateways)              │
├─────────────────────────────────────┤
│  Application (Use Cases)            │  ← Orchestrates domain
├─────────────────────────────────────┤
│  Domain (Entities, Policy)          │  ← Pure business logic
└─────────────────────────────────────┘
```

### Rules

- **Dependency Rule.** Source code dependencies must point **only inward**. The domain knows nothing about the database, the web framework, or the LLM.
- **Entities are framework-agnostic.** `GaspareSpontini`, `MunicipalDocument`, `Answer` — none reference Spring, React, or Ollama.
- **Use cases orchestrate.** Each use case (e.g., `AnswerCitizenQuestion`) is an application-layer service that depends on ports (interfaces), not concrete adapters.
- **Ports and Adapters (Hexagonal).** External systems (Minio, Ollama, MCP server, web) are reached exclusively through interfaces defined in the application layer.
- **Crossing boundaries uses DTOs.** Data crosses layer boundaries in purpose-built structures, never as raw domain entities leaking ORM annotations.
- **One direction of flow.** A request enters from the outside, is transformed at each boundary, is processed by a use case, and returns outward. No layer skipping.

### Forbidden

- A domain class importing `org.springframework.*`, `react`, `axios`, or any framework type.
- A controller calling a repository directly (it must go through a use case).
- Business rules living in a controller, a route handler, or a UI component.

Reference: Robert C. Martin, *Clean Architecture*.

---

## 3. SOLID

Five principles for designing change-resilient systems. They are mandatory; they are not preferences.

| Principle | Rule | Application in Spontini |
|---|---|---|
| **S** — Single Responsibility | A class has one reason to change. | `DocumentIngestor` ingests; `DocumentRetriever` retrieves. Never both. |
| **O** — Open/Closed | Open for extension, closed for modification. | New document sources are added by implementing `DocumentSource`, not by editing `Ingestor`. |
| **L** — Liskov Substitution | Subtypes must be substitutable for their base types. | A `MockLlmClient` behaves identically to `OllamaLlmClient` from the caller's perspective. |
| **I** — Interface Segregation | No client is forced to depend on methods it does not use. | Split `DocumentRepository` from `DocumentIndex` rather than one fat interface. |
| **D** — Dependency Inversion | Depend on abstractions, not concretions. | Use cases depend on `LlmPort`, `RetrievalPort`, `DocumentPort` — never on `OllamaClient`. |

### Enforcement

- Every class that has "Service", "Manager", or "Handler" in its name must be auditable against SRP. If you cannot state its single responsibility in one sentence, it is wrong.
- Every `new` keyword inside application or domain code is a violation (except value objects and factories). Use dependency injection.
- Every interface with more than 5 methods is a candidate for segregation.

Reference: Robert C. Martin, *Agile Software Development: Principles, Patterns, and Practices*.

---

## 4. TDD — Test-Driven Development

Tests are not a safety net added after the fact. They are the **first artifact** of a feature.

### The Red-Green-Refactor Cycle

1. **Red.** Write a failing test that describes the smallest possible increment of behavior.
2. **Green.** Write the minimum code to make it pass. No more.
3. **Refactor.** Improve structure without changing behavior. Tests stay green.

### Rules

- **No production code without a failing test.** The only exception is spike exploration, which is thrown away.
- **One assertion concept per test.** Test one behavior; use multiple assertions only if they verify the same behavior.
- **Tests are behavior, not implementation.** They verify *what* the system does, never *how* it does it. Refactoring must never break tests.
- **Fast.** The full unit test suite runs in under 10 seconds. Slow tests are integration tests and live separately.
- **Isolated.** A test's outcome does not depend on another test's execution or order.
- **Arrange-Act-Assert.** Every test follows the AAA structure. No branching logic inside tests.
- **Meaningful test names.** `shouldReturnEmptyAnswerWhenDocumentNotFound`, not `test1`.

### What TDD is Not

- Writing all tests after the feature is "done".
- Writing tests that mirror the implementation 1:1 (this is a tautology, not a test).
- Testing getters and setters.

Reference: Kent Beck, *Test-Driven Development: By Example*.

---

## 5. BDD — Behavior-Driven Development

Where TDD specifies units, BDD specifies **behavior from the outside-in**, in language the business can read.

### Gherkin Format

```gherkin
Feature: Answering citizen questions from the knowledge base

  Scenario: A citizen asks a question answerable from a municipal document
    Given a document titled "Orari di apertura sportello anagrafe" exists in the knowledge base
    And the document contains the text "Lo sportello anagrafe è aperto dal lunedì al venerdì dalle 9:00 alle 12:30"
    When the citizen asks "A che ora apre l'anagrafe?"
    Then Spontini answers "Lo sportello anagrafe è aperto dal lunedì al venerdì dalle 9:00 alle 12:30"
    And Spontini cites the source document

  Scenario: A citizen asks a question not answerable from any document
    Given the knowledge base contains no document about "tasse comunali 2025"
    When the citizen asks "Quanto pago di tasse comunali?"
    Then Spontini answers "Non ho trovato informazioni nei documenti comunali su questo argomento"
    And Spontini does not invent any detail
```

### Rules

- **Scenarios are written before the implementation.** They are a design conversation, not a post-hoc test.
- **Scenarios live in the language of the domain**, not the infrastructure. No "when the HTTP request is sent to `/api/chat`".
- **Given-When-Then is mandatory structure.** One `When` per scenario; multiple `Given` and `Then` are allowed.
- **Each scenario exercises one behavior.** If "And" appears between `When` steps, split the scenario.
- **Scenarios are executable.** They are wired to the system through step definitions that exercise real use cases end-to-end (not mocked at the boundary).

Reference: Dan North, *Introducing BDD*; Gojko Adzic, *Specification by Example*.

---

## 6. Clean Design (UI and UX) — The Jobs Aesthetic

> "Design is not just what it looks like and feels like. Design is how it works." — Steve Jobs

We design like Apple in the 2000s: **radical simplicity, ruthless editing, materials that tell the truth.** Spontini is a municipal chatbot. Its users are citizens of every age and digital literacy. The interface must disappear so the conversation remains.

### 6.1 UI Principles

- **One thing on the screen does one thing well.** The chatbot popup is a chatbot popup. No hamburger menus, no settings tabs, no chrome.
- **Generous whitespace.** Space is not wasted; it is the loudest design element. Content breathes.
- **Material honesty.** A button looks like a button. A link looks like a link. No faux 3D, no skeuomorphic leather, no gradient noise. Flat, crisp, intentional.
- **Typography is the interface.** One typeface family. A clear hierarchy: large for the question, regular for the answer, small for the citation. No more than three sizes.
- **Two colors, maximum.** One neutral (the Comune's institutional tone) and one accent (for the user's message and primary actions). Everything else is grayscale.
- **Motion is meaning.** Animation exists only to explain a state change (popup opening, message arriving, source expanding). No decorative motion. No loaders that lie.
- **Crisp iconography.** Line icons, single weight, consistent metaphor. No emoji as UI.

### 6.2 UX Principles

- **It opens when needed, closes when dismissed.** Bottom-left popup. One click to open, one click to close. No modal stacks.
- **The conversation is the entire surface.** No sidebars, no history panel, no "new chat" button for v1. Past turns are visible by scroll; nothing else.
- **Every answer cites its source.** Inline, expandable. The citizen can verify. This is non-negotiable — it is how trust is built.
- **Honesty over confidence.** If Spontini does not know, the UI says so clearly and calmly. Never render a confident hallucination.
- **Forgiving input.** The input box accepts typos, lowercase, informal Italian. The system forgives; the citizen should never feel they "asked wrong".
- **Keyboard-first, mouse-friendly.** Enter to send, Shift+Enter for newline, Esc to close. Touch targets are at least 44×44 px.
- **Accessible by default.** WCAG 2.1 AA at minimum. Keyboard navigable, screen-reader labeled, contrast ratios respected, reduced-motion honored.

### 6.3 What We Reject

- Chatbot personas with cartoon avatars and bouncing dots.
- Typing indicators that simulate "thinking" for longer than the LLM actually takes.
- Welcome messages longer than one line.
- Toolbars, menus, and feature toggles visible to the citizen.
- Any UI element that exists to show off rather than to be used.

Reference: Steve Jobs presentations 1997–2007; Apple Human Interface Guidelines (Aqua era); Dieter Rams' Ten Principles of Good Design.

---

## 7. 100% Test Coverage on the Codebase

Coverage is a **floor, not a ceiling**. Every line of production code must be exercised by an automated test.

### Rules

- **100% line and 80% branch coverage is the minimum gate.** The CI pipeline rejects any merge below this threshold for production code.
- **Coverage is measured on production code only.** Generated code, migration scripts, and infrastructure glue may be excluded — but only explicitly, with a documented reason in `coverage-exclusions.txt`.
- **Coverage ≠ quality.** 100% coverage with tautological tests is worse than 70% with meaningful ones. Every test must assert behavior, not execution.
- **No untested branches.** Every `if`, `catch`, `switch` case, and `null` guard has a test for both sides.
- **Integration tests cover the boundaries.** The seam between application and infrastructure (MCP calls, Minio reads, Ollama inference) is covered by integration tests using test doubles that respect the port contract.
- **BDD scenarios cover the features.** Every Gherkin scenario in `features/` is green in CI. A feature without a passing scenario is not done.

### Exemptions (Rare, Documented)

Only the following may be excluded from the 100% rule, and each exclusion must be justified in the PR:

1. `main` entry points and composition roots (covered by smoke tests instead).
2. Framework configuration classes with no logic.
3. Pure data transfer objects with no behavior.

Everything else is tested. No exceptions, no "I'll add tests later."

---

## 8. Precedence and Conflict Resolution

When principles appear to conflict, resolve in this order:

1. **Truthfulness** (from [Constitution](./CONSTITUTION.md) §3) — always wins.
2. **User experience** — Clean Design serves the citizen.
3. **Correctness** — TDD/BDD and 100% coverage guarantee it.
4. **Maintainability** — SOLID and Clean Architecture enable it.
5. **Readability** — Clean Code delivers it.

A faster implementation that sacrifices Truthfulness or UX is rejected. A slower implementation that preserves them is preferred.

---

## 9. Adoption

These principles take effect immediately upon merge of this document. All existing code is subject to them at the next touch. New code is subject to them at the first commit.
