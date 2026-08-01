# AGENTS.md — Spontini Bot 2

This file is the **root index** for every AI agent (human or automated) working on this repository. It lists the authoritative documents, rules, and references that govern the project.

Read this file first. Read every file it references before making any change.

---

## 1. Project Mission

Spontini Bot 2 builds **Spontini** — a conversational AI assistant for the Comune di Maiolati Spontini, powered by a fully local LLM stack, answering citizens exclusively from official municipal documents.

Full mission, identity, scope, and decision-making criteria live in the [Constitution](./docs/CONSTITUTION.md).

---

## 2. Authoritative Documents (`docs/`)

Every Markdown file under `docs/` is a binding reference. Agents must consult them before and during work.

| File | Purpose |
|---|---|
| [docs/CONSTITUTION.md](./docs/CONSTITUTION.md) | Project mission, identity, core principles, scope, and decision-making criteria. The highest-level authority. |
| [docs/PRINCIPLES.md](./docs/PRINCIPLES.md) | Engineering and design standards: Clean Code, Clean Architecture, SOLID, TDD, BDD, Clean Design (Jobs-era Apple aesthetic), 100% test coverage. |
| [docs/STACK.md](./docs/STACK.md) | Technical stack specification: Rust workspace (axum, libSQL, llama.cpp), Vue frontend, Dockerized runtime services, cargo workspace layout. |

---

## 3. Binding Rules

These rules apply to **every** contribution — code, documentation, configuration, prompts, skills, agents — without exception.

### 3.1 Language

- **All files — codebase and documentation — must be written rigorously in English.**
- This includes: source code, comments, commit messages, pull request titles and descriptions, documentation, README files, prompts, agent instructions, skill definitions, test names, Gherkin scenarios, and log messages.
- User-facing strings produced by Spontini at runtime (the chatbot's answers to citizens) are the **only** exception: they are in Italian, because the citizens of Maiolati Spontini speak Italian.
- Comments and documentation must be English even when describing Italian-language runtime behavior.

### 3.2 Documentation Indexing

- **Every time a new Markdown (`.md`) file is added to the repository, it must be referenced in this `AGENTS.md` file.**
- Add it to the appropriate table in Section 2 (for `docs/`) or Section 4 (for root-level or other locations).
- A Markdown file that is not referenced in `AGENTS.md` is considered orphaned and will be flagged in review.
- When a Markdown file is renamed, moved, or deleted, update `AGENTS.md` in the same change.

### 3.3 Prompts, Skills, and Agents

- **Every time a new prompt, skill, or agent definition is added to the repository, it must be referenced in `AGENTS.md`.**
- Register them in Section 5 with: name, location, purpose, and invocation trigger.
- This applies to: opencode skills (`.opencode/skills/`), opencode agents (`.opencode/agents/`), prompt templates (`prompts/`), MCP server definitions, and any similar artifact.
- An unregistered prompt, skill, or agent is considered dead configuration and may be removed.

### 3.4 Engineering Standards

All code and design work must comply with [docs/PRINCIPLES.md](./docs/PRINCIPLES.md). In short:

- Clean Code, Clean Architecture, SOLID.
- TDD (Red-Green-Refactor) and BDD (Gherkin scenarios written before implementation).
- Clean Design — Jobs-era Apple aesthetic: radical simplicity, one thing per screen, honest materials, every answer cites its source.
- 100% test coverage on production code; 80% branch coverage minimum, enforced by CI.

### 3.5 ADRs Are Inviolable

**Every accepted Architecture Decision Record (ADR) in `.adr/` is binding and permanent.** No change — code, configuration, documentation, or design — may contradict a decision recorded in an accepted ADR. If a future need conflicts with an existing ADR, the correct path is: write a new ADR that explicitly supersedes the old one, then update the old ADR's status to `superseded`. Never silently overwrite or ignore an ADR.

### 3.6 Identity/Imprinting Questions Never Touch the Database or the Generation Model

**A question about the bot's own identity/imprinting (e.g. "Chi sei?", "Come ti chiami?") must be answered directly from the active persona's own configuration (`system_prompt`), never via `EmbeddingPort`/`RetrievalPort`/`GenerationPort`.** The answer already exists in memory; a full RAG round trip (embed → retrieve → generate, ~20-30s per [ADR 0013](./.adr/0013-generation-model-1-5b-and-reduced-rag-top-k-for-latency.md)) is both unnecessary and too slow to count as the "ultra-immediate" response this class of question requires. See [ADR 0014](./.adr/0014-instant-identity-imprinting-answers-bypass-rag.md) for the full design and rationale — this is a narrow, closed-set literal-match short-circuit, not a general-purpose question classifier (which ADR 0012 explicitly rejected for the harder categorical-refusal cases).

### 3.7 Non-Backward-Compatible Data Changes Must Backfill Existing Data

**Any change that alters the meaning, shape, or population rule of persisted data — a new field an operator expects to see populated, a corrected computation, a changed classification — must also address the rows that already exist, not just rows written from now on.** Shipping the code fix alone and leaving existing records in the old, now-inconsistent state is an incomplete fix: it reads as "fixed" in review and in a fresh install, but stays broken for the live system the fix was written for. Before closing a plan or task of this kind, explicitly decide and record one of:

- **Backfill it** — a migration (see `kb-store/src/migrations/` for the established SQL-backfill pattern, e.g. `V9`–`V12`) or a dedicated, tested one-time tool when the backfill needs more than SQL (e.g. re-deriving data from an external source).
- **Document why backfill is impossible or out of scope** — e.g. the original data needed to reconstruct the new field was never recorded and cannot be recovered truthfully (per the Constitution's honesty rule, never fabricate what can't be recovered) — and say so explicitly in the plan/ADR rather than leaving it unstated.

A feature that changes what existing data means is not closed until the existing data has been reconciled or the impossibility of doing so has been recorded.

### 3.8 Training Notes Are Read Live on Every Chat Answer

**Every `.md` file directly under `.project/training/` is read fresh from disk and folded into the system prompt sent to `llama-generate` on every chat request that reaches generation** (i.e. every question except an identity-question fast-path match or a genuine zero-chunk honest-unknown fallback — see [§3.6](#36-identityimprinting-questions-never-touch-the-database-or-the-generation-model) and [Constitution §5](./docs/CONSTITUTION.md#5-knowledge-base-rule), neither of which calls the generation model at all). There is no cache and no reload step: a file written to `.project/training/` takes effect on the very next matching chat answer. The `/train` command (see §5) is the only intended writer of this directory — it regenerates its contents from `.project/test-reports/` on every run, so the directory is always a deterministic function of the current test-report corpus, never hand-edited or accumulated ad hoc. See [ADR 0016](./.adr/0016-train-command-with-live-loaded-training-notes.md) for the full design, including the explicit tradeoff this makes against the persona table's versioned/audited governance (§ Consequences).

---

## 4. Root-Level and Other Documentation

| File | Purpose |
|---|---|
| [AGENTS.md](./AGENTS.md) | This file. Root index for all agents. |
| [README.md](./README.md) | Project front door: mission pointer, prerequisites, quick start via `make`, architecture overview, repository layout, contributing. Spec: [docs/STACK.md §7.2](./docs/STACK.md#72-readmemd). |
| [.project/ROADMAP.md](./.project/ROADMAP.md) | Feature roadmap by milestone. Source of truth for `/create-plan` (reads the next unchecked feature when no argument is given) and for the feature-close tick rule (performed by `/create-adr` after the related ADR is written). |
| [models/embed/README.md](./models/embed/README.md) | Notes the expected GGUF filename, model origin, and provisioning instructions for the embedding model (`llama-embed` container). |
| [models/generate/README.md](./models/generate/README.md) | Notes the expected GGUF filename, model origin, and provisioning instructions for the generation model (`llama-generate` container). |

*When a Markdown file is added at the root or outside `docs/`, register it here in the same change.*

---

## 5. Prompts, Skills, and Agents Registry

The following skills are registered for this repository. They live under [`.agents/skills/`](./.agents/skills/) and are auto-discovered by the opencode `skill` tool.

| Name | Type | Location | Purpose | Trigger |
|---|---|---|---|---|
| spontini-tdd-rust | skill | [`.agents/skills/spontini-tdd-rust/SKILL.md`](./.agents/skills/spontini-tdd-rust/SKILL.md) | Red-Green-Refactor TDD workflow for the Rust workspace, with exact cargo commands and the coverage gate. | Before writing or modifying any production Rust code in `backend/`, `ingest-core/`, `ingest-cli/`, `kb-store/`. |
| spontini-bdd-gherkin | skill | [`.agents/skills/spontini-bdd-gherkin/SKILL.md`](./.agents/skills/spontini-bdd-gherkin/SKILL.md) | Behavior-Driven Development workflow: Gherkin scenarios written before implementation, wired to use cases, with explicit truthfulness and source-citation concerns. | Before implementing any user-visible feature or citizen-facing behavior. |
| spontini-clean-arch-guard | skill | [`.agents/skills/spontini-clean-arch-guard/SKILL.md`](./.agents/skills/spontini-clean-arch-guard/SKILL.md) | Clean Architecture dependency-rule guard: crate-level dependency matrix, ports/adapters placement, DTO boundary rules. | When adding a crate, a module, a port/adapter, or any import. |
| spontini-rag-build | skill | [`.agents/skills/spontini-rag-build/SKILL.md`](./.agents/skills/spontini-rag-build/SKILL.md) | RAG flow construction: 3-part prompt separation, source citation, honest-unknown fallback, persona table rules, embedding model constraint. | When working on query embedding, retrieval, prompt assembly, generation, or the persona table. |
| spontini-ingest-flow | skill | [`.agents/skills/spontini-ingest-flow/SKILL.md`](./.agents/skills/spontini-ingest-flow/SKILL.md) | Ingest pipeline construction: two entry points (cli + admin-ui), source adapters, chunking, embedding writes, kb.db access rules. | When working on ingest-core, ingest-cli, source adapters, admin-ui upload, chunking, or embedding writes. |
| spontini-verify-gate | skill | [`.agents/skills/spontini-verify-gate/SKILL.md`](./.agents/skills/spontini-verify-gate/SKILL.md) | Pre-completion verification gate: build, test, clippy, format, coverage, LSP, Docker config, BDD, embedding consistency, manual sanity. | Before claiming any task complete. |
| spontini-ui-craft | skill | [`.agents/skills/spontini-ui-craft/SKILL.md`](./.agents/skills/spontini-ui-craft/SKILL.md) | UI craft checklist for admin-ui/frontend work: reuse `ds/` primitives and Bootstrap Italia tokens, native `<dialog>`/`<details>` conventions, spacing between adjacent interactive elements, consistent button-variant pairing, state honesty. | Before writing or modifying any `.vue` template/style in `admin-ui/` or `frontend/`, or adding/changing a `ds/` component. |

### Commands

The following custom commands are registered for this repository. They live under [`.opencode/commands/`](./.opencode/commands/) and are invoked via the opencode `/<name>` syntax.

| Name | Location | Purpose | Trigger |
|---|---|---|---|
| create-plan | [`.opencode/commands/create-plan.md`](./.opencode/commands/create-plan.md) | Create a feature plan: switch to main, pull, create `feat/<name>` branch, write `.project/<ID>-<name>-plan.md` with status `draft`. | Starting a new feature. |
| approve-plan | [`.opencode/commands/approve-plan.md`](./.opencode/commands/approve-plan.md) | Transition a plan's status from `draft` to `open`. | When a plan is ready for implementation. |
| implement-plan | [`.opencode/commands/implement-plan.md`](./.opencode/commands/implement-plan.md) | Implement an `open` plan phase by phase, task by task. Transitions status to `review` at completion. | When a plan is `open` and ready to be built. |
| review-plan | [`.opencode/commands/review-plan.md`](./.opencode/commands/review-plan.md) | Code-review an implementation. Produces `.project/<ID>-<name>-review.md` with findings and a verdict. | When a plan is in `review` state. |
| fix-review | [`.opencode/commands/fix-review.md`](./.opencode/commands/fix-review.md) | Implement the fixes required by a review. Transitions status to `closed`. | After `/review-plan` returns `changes-requested`. |
| create-adr | [`.opencode/commands/create-adr.md`](./.opencode/commands/create-adr.md) | Create an Architecture Decision Record at `.adr/<ID>.md`, update the ADR registry, and ensure the `AGENTS.md` pointer exists. | When recording a binding architectural decision. |
| next-steps | [`.opencode/commands/next-steps.md`](./.opencode/commands/next-steps.md) | Run the full plan lifecycle (create → approve → implement → review → fix → ADR) for the next N unchecked roadmap features, merging each to main. Default: 1 feature. Pass an integer N or `all`. | When implementing one or more roadmap features end-to-end without manual orchestration. |
| new-test-session | [`.opencode/commands/new-test-session.md`](./.opencode/commands/new-test-session.md) | Run a full 100-question test session: generate mixed questions, invoke the bot, create reports, run training based on feedback, re-test, and synthesize feedback. | When running a comprehensive test session against the live bot. |
| analyze-feedback | [`.opencode/commands/analyze-feedback.md`](./.opencode/commands/analyze-feedback.md) | Analyze all unresolved feedback from test sessions and run a training session to address them. | When processing feedback from test sessions to improve bot performance. |
| train | [`.opencode/commands/train.md`](./.opencode/commands/train.md) | Read everything under `.project/test-reports/` and regenerate `.project/training/*.md` — instructional notes the running bot reads live on every chat answer (§3.8, [ADR 0016](./.adr/0016-train-command-with-live-loaded-training-notes.md)). | After one or more test sessions, to turn confirmed behavioral findings into a live behavioral change without a code or persona-version change. |

### ADR Registry

Architecture decisions live in [.adr/](./.adr/), indexed in [.adr/README.md](./.adr/README.md). When a new ADR is added, append a row to that index. This entry in `AGENTS.md` is a permanent pointer — no per-ADR row is added here.

*No prompts or custom agent definitions exist in this repository at this time. When one is added, register it in the table above using the template that follows.*

### Registration Template

| Name | Type | Location | Purpose | Trigger |
|---|---|---|---|---|
| _(example) review-work_ | _skill_ | _`.agents/skills/review-work/SKILL.md`_ | _Post-implementation QA orchestrator_ | _After any significant implementation_ |

### Type Values

- `skill` — a reusable, loadable instruction set invoked via the opencode skill system.
- `agent` — a subagent definition with a specific role, tools, and model.
- `prompt` — a prompt template invoked by name.
- `mcp` — an MCP server configuration providing tools, resources, or prompts.

---

## 6. Working Agreement for Agents

1. **Read before writing.** Read the [Constitution](./docs/CONSTITUTION.md) and [Principles](./docs/PRINCIPLES.md) before your first change.
2. **English only.** Every artifact you produce is in English, except runtime Italian strings for citizens.
3. **Index everything.** When you add a Markdown file, a prompt, a skill, or an agent, update this file in the same change.
4. **Test what you ship.** No code lands without tests. Coverage gates are enforced.
5. **Cite the source.** Every Spontini answer must trace to a document. No hallucination — including the categorical refusal rules (no predictions, no weather, no personal/sensitive data) and the standard fallback text in [Constitution §5](./docs/CONSTITUTION.md#5-knowledge-base-rule) / [ADR 0012](./.adr/0012-categorical-refusal-rules-and-standard-fallback-text.md).
6. **Keep it simple.** When two approaches satisfy the Constitution, choose the simpler one.
7. **Leave the campsite cleaner.** Every touch improves the file you worked on — names, structure, tests. But never refactor beyond the task at hand.
