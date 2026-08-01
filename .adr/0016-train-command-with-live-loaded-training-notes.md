# ADR 0016: `/train` Command with Live-Loaded Training Notes

- **Status**: accepted
- **Date**: 2026-08-01
- **Deciders**: massimobiagioli, Claude Code
- **Related**: [ADR 0012](./0012-categorical-refusal-rules-and-standard-fallback-text.md), [ADR 0014](./0014-instant-identity-imprinting-answers-bypass-rag.md)

## Context

`/analyze-feedback` (the existing command for closing out `.project/test-reports/feedback/` items) sends corrected question/answer pairs to `POST /admin/api/training/sessions/:id/messages`, which persists them to `kb_store`'s `training_messages` table. Two independent test sessions (`session-20260727-102811`, `session-20260727-111155`) confirmed this mechanism is **data-logging, not an online learning loop**: sending an `expected_answer` and re-running the identical question produced zero measurable change in the bot's actual answer, including a case (Q8, "Sei un'intelligenza artificiale...") that failed identically three times in a row across two sessions, immediately after the correct answer had been sent as training data. `RagEngine::answer()` never reads the `training_messages` table — nothing in the answer path consumes it.

This leaves the project with a real gap: a durable, reproducible way to fix a confirmed behavioral defect (e.g. "always describe yourself as a digital assistant, not a person" or "never attach citations to a fallback answer") requires either an ADR-and-code intervention (ADR 0014's identity fast path being the prior example) or an edit to the persona's `system_prompt` via `POST /admin/api/persona` — both correct but heavyweight for the volume of small, specific behavioral corrections that test sessions turn up.

## Decision

We add:

1. **A `TrainingNotesPort`/`TrainingNotesAdapter`** (`backend/src/rag_engine/training_notes.rs`) that reads every `.md` file directly under a configured directory (`TRAINING_NOTES_DIR`, default `/app/training`), concatenates them, and returns the result. It is read **fresh from disk on every `RagEngine::answer()` call that reaches the generation step** — no cache, no reload endpoint, unlike `PersonaAdapter`. A missing or empty directory is not an error; it degrades to no notes.
2. **`RagEngine`** gains a `training_notes: Arc<dyn TrainingNotesPort>` field, defaulted to a `NoopTrainingNotes` (empty string) inside `new()` and opted into a real adapter via a `with_training_notes()` builder — chosen specifically so every existing `RagEngine::new()` call site (the entire existing test suite, `admin::training_messages`) keeps compiling and behaving unchanged, and only the real `lib.rs` wiring opts in.
3. **`prompt::assemble()`** gains a `training_notes: &str` parameter. When non-empty, its content is appended to `PromptParts.system` under a `--- Note di addestramento ---` heading, after `persona.system_prompt`. This is deliberately still inside `system`, not a fourth prompt part: the `spontini-rag-build` skill's non-negotiable 3-part rule is about **chunks and the question never entering `system`**, not about `system` being nothing but the literal `persona.system_prompt` string — training notes are supplementary instructions, the same category of content as the persona prompt itself, never retrieved chunks, never the citizen's question.
4. **`docker-compose.yml`** bind-mounts `./.project/training:/app/training:ro` into the `backend` service (read-only — the backend only ever reads this directory). This is a bind mount, not the `kb-data` named volume, specifically so the directory's contents live in the repo working tree (even though `.project/` itself is gitignored — see Consequences) rather than inside the opaque `kb-data` volume, and so a file written on the host is visible to the running container without an image rebuild.
5. **A new `/train` command** (`.claude/commands/train.md`, `.opencode/commands/train.md`) that reads every file under `.project/test-reports/` (the feedback synthesis files and the per-question `rep.md`/`rep-fix.md` cards), and regenerates `.project/training/*.md` — one file per distinct, confirmed systemic behavioral pattern, phrased as a direct instruction to the bot in Italian (since this content is folded into the citizen-facing `system` prompt — AGENTS.md §3.1 applies). The command **fully regenerates** the directory contents on each run rather than appending, so it stays a deterministic function of the current `test-reports` corpus and never accumulates stale or superseded notes.

## Rationale

Evaluated against Constitution §6:

1. **Serves the mission?** Yes — it closes the exact gap the two test sessions found: there was no path from "a test session found and confirmed a specific behavioral defect" to "the bot's next answer reflects the fix" that didn't require a full code change or a new persona version. Training notes are the middle path: reviewable, version-control-adjacent (see Consequences), reversible (delete the file), and scoped to *behavioral instructions*, not data the model is meant to have "learned" in any statistical sense — it is prompt content, not fine-tuning, and the design is honest about that.
2. **Keeps the stack local?** Unaffected — no new external dependency, no new network call. One additional local-disk read per chat request that reaches the generation step (the identity fast path and the honest-unknown fallback path, both already skipping `GenerationPort`, also skip this read).
3. **Reduces complexity?** The `TrainingNotesPort`/adapter pair mirrors the existing `PersonaPort`/`PersonaAdapter` pattern exactly (same clean-architecture shape, same crate), so it introduces no new architectural concept to the codebase — only a new instance of an existing one. The builder-injection (`with_training_notes`) was chosen over changing `RagEngine::new()`'s signature specifically to avoid a) breaking ~20 existing call sites across `engine.rs`, `admin/training_messages/adapter.rs`, and `backend/tests/bdd.rs`, and b) forcing every test double to model a port it doesn't care about.
4. **Improves UX?** Yes for the *operator* workflow (a `/train` run after a test session can turn a confirmed root cause into a live behavioral change in seconds, no deploy), and potentially for the *citizen* (test-derived corrections reach the live bot without waiting for the next code release or persona edit) — though see Consequences for the honesty caveat this introduces.

### Why append to `system` rather than a fourth prompt part

A fourth structurally-separate part (e.g. `training`) was considered and rejected: the 3-part rule exists to guarantee retrieved chunks and the citizen's own question can never be mistaken for instructions by the model (the actual risk that rule defends against — see `spontini-rag-build`'s "Forbidden: concatenating persona, context, and question into a single string"). Training notes carry the same trust level and same *kind* of content as `persona.system_prompt` — both are operator-authored instructions about how to behave, never citizen input, never retrieved document text. Extending `system` keeps the meaningful boundary (instructions vs. retrieved-content vs. citizen-input) intact; adding a fourth co-equal part would blur that boundary by implying training notes are a distinct trust category from persona instructions, which they are not.

## Consequences

### Positive

- Closes the "training API doesn't change behavior" gap documented by two independent test sessions, without requiring a code change or a new persona version per fix.
- `TrainingNotesAdapter` fails soft everywhere (missing directory, unreadable file, empty file) — a broken or absent `.project/training/` can never break citizen-facing answering, only silently omit notes.
- `/train`'s full-regenerate-per-run design means `.project/training/` is always a deterministic function of the current `test-reports` corpus — no manual pruning, no accumulating cruft from superseded findings.
- Zero impact on any existing `RagEngine::new()` caller — verified by the full existing test suite (233 unit tests, 68 BDD scenarios) passing unchanged.

### Negative

- **This bypasses the persona's own versioning/audit trail.** `POST /admin/api/persona` inserts an immutable, versioned row with `created_by` and `created_at` (ADR 0004) specifically so every change to what the bot says is attributable and reversible via version history. Training notes have none of that: they are unversioned files on disk, silently folded into every answer, with no admin-UI visibility, no activation step, no "who changed this and when" trail beyond `git log`/filesystem mtimes on the *host* — and since `.project/` is gitignored (see below), not even that. **This is the sharpest tradeoff in this decision**: it trades the persona system's deliberate governance (Constitution §2/§5, "every edit inserts a new row, never `UPDATE`") for speed of iteration on narrow behavioral corrections. It is accepted here because training notes are explicitly scoped to *narrow, test-confirmed behavioral corrections* (see `spontini-rag-build`'s Forbidden list is unchanged — this does not touch persona `UPDATE` semantics or add one), not a parallel channel for arbitrary persona changes; a correction significant enough to be a real identity/policy change belongs in a persona version or an ADR-backed code change, not a training note.
- **`.project/training/*.md` is not committed to git.** `.project/` is gitignored project-wide (confirmed: no file under `.project/` is currently tracked, despite `git log` showing it was tracked in the past — e.g. `.project/ROADMAP.md` — before being dropped from tracking). This means the notes actually steering the live bot's answers exist only on the host filesystem of whichever machine last ran `/train`, with no version history and no way to diff "what changed" via git. This is consistent with how `.project/test-reports/` already behaves (local working state, not shipped in the repo) but is a materially bigger deal here because, unlike test reports, these files directly and silently change citizen-facing answers. Anyone re-provisioning the backend on a new host starts with an empty (or bind-mount-absent) training-notes directory until `/train` is re-run there.
- Training notes are plain, unstructured Italian prose folded into `system` with a heading — there is no schema, no per-note enable/disable, no way to see which note affected which answer. A note that turns out to be wrong or badly phrased degrades silently exactly like a bad `system_prompt` edit would.
- One additional local-disk read (`std::fs::read_dir` + `read_to_string` per `.md` file) on every chat request that reaches generation. At the expected small note count this is negligible next to the ~seconds-scale `llama-generate` round trip (ADR 0013), but it is a real, uncached I/O cost added to the hot path.

### Neutral

- Does not change `RAG_TOP_K`/`RAG_MIN_SCORE`, the identity fast path (ADR 0014), or the honest-unknown fallback path (ADR 0012) — all three continue to skip `GenerationPort` entirely and therefore never read training notes.
- Does not touch the `persona` table or its versioning semantics in any way — `POST /admin/api/persona` remains the only way to change `system_prompt`/`tone`/`fallback_message` itself.

## Alternatives Considered

### Alternative A: Store training notes in `kb_store` (a new table) instead of the filesystem

Would give proper versioning, an audit trail, and admin-UI visibility for free, matching the persona table's governance model. Rejected for this iteration: it is materially more work (schema, migration, admin API, admin-UI screen) for what `/train` is meant to be — a fast, low-ceremony way to close out test-session findings, not a second persona-management surface. The Negative consequences above (no audit trail, not git-tracked) are the explicit, accepted cost of keeping this lightweight; if training notes prove to need real governance, the correct fix is a new ADR moving them into `kb_store`, not silently growing filesystem-based ad hoc structure.

### Alternative B: Have `/train` write directly into the active persona's `system_prompt` (new persona version per run)

Reuses the existing versioned, auditable mechanism exactly. Rejected: conflates two different lifecycles — the persona is the bot's stable, deliberately-edited identity (Constitution §2), while training notes are meant to be frequent, small, mechanically-derived corrections from test sessions. Writing every one as a new persona version would make the version history noisy and hard to review, and would require `/train` to have write access to persona activation (a bigger blast radius than writing local files).

### Alternative C: Cache training notes like `PersonaAdapter` does, with a `/admin/persona/reload`-style endpoint

Rejected: the entire point of this feature is that a `/train` run takes effect on the *next* chat request with zero extra operator action. A cache would silently reintroduce the exact "I sent the correction but nothing changed" failure mode the two test sessions already found once, this time for a mechanism explicitly built to fix that. The uncached read's cost is negligible relative to `llama-generate` latency (see Negative consequences), so there is no performance reason to cache it.

## Compliance

- Enforced by unit tests: `rag_engine::training_notes::tests` (missing/empty directory, sorted concatenation, non-`.md` files ignored, blank-file/whitespace handling), `rag_engine::prompt::tests` (notes appended to `system` only, never `context`/`user`; blank notes leave `system` unchanged), `rag_engine::engine::tests` (`with_training_notes` content reaches the prompt sent to `GenerationPort`; default `RagEngine::new()` still sends an unmodified `system_prompt`).
- The full existing backend test suite (233 unit tests, 68 BDD scenarios / 299 steps) passes unchanged after this feature, confirming no regression to any existing `RagEngine::new()` caller.
