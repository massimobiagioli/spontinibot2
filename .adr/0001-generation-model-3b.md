# ADR-0001: Generation model — Qwen2.5-3B-Instruct instead of 7B

- **Status**: Accepted
- **Date**: 2026-07-09
- **Decider**: Massimo Biagioli (with Sisyphus analysis)

## Context

[docs/STACK.md §3.4](../docs/STACK.md#34-inference--llamacppllama-server) originally specified `Qwen2.5-7B-Instruct, GGUF Q4_K_M` as the generation model for the `llama-generate` container. The target hardware ([docs/STACK.md §Purpose](../docs/STACK.md#purpose)) is a Mac Intel i7 with 16GB RAM, approximately 9 years old, with no GPU.

During the bootstrap-infra walking skeleton (Plan 0001, Task 5.1), the 7B model was downloaded (~4.4 GB, 2-part split) and loaded successfully in the `llama-generate` container. The container became healthy, but the model load took ~3 seconds and inference on the target CPU is projected at 1-3 tokens/second. A 100-word citizen answer would take 30-60 seconds to generate — unacceptable for a citizen-facing chatbot that aims to be "the most intuitive municipal chatbot an Italian citizen has ever used" ([docs/STACK.md §4.5](../docs/STACK.md#45-usability-ambition--the-most-intuitive-ever)).

## Decision

Switch the generation model from `Qwen2.5-7B-Instruct Q4_K_M` to `Qwen2.5-3B-Instruct Q4_K_M` (~2.1 GB, single file).

## Rationale (against Constitution §6 criteria)

1. **Serves the mission?** Yes. A responsive chatbot serves citizens better than a slow one. The mission is a trustworthy, always-available channel — speed is part of availability.
2. **Keeps the stack local?** Yes. Still a local GGUF model on `llama.cpp`, no external API.
3. **Reduces complexity?** Yes. Smaller file (2.1 GB vs 4.4 GB split), faster download, faster load, simpler provisioning (single file vs 2-part split).
4. **Improves UX?** Yes. Projected ~4-8 tokens/second on the target CPU — a 100-word answer in 10-20 seconds instead of 30-60. Leaves ~2-3 GB more RAM headroom for the 5 other containers sharing the 16GB host.

## Consequences

- **Quality tradeoff**: The 3B model has lower reasoning/citation quality than the 7B. Acceptable for a walking skeleton and early prototype. The [Constitution §5 Knowledge Base Rule](../docs/CONSTITUTION.md#5-knowledge-base-rule) is still enforced — Spontini only answers from retrieved documents, so the model's job is to synthesize, not to know. A 3B model can synthesize retrieved context adequately.
- **Re-evaluation**: When the project moves to better hardware (GPU or newer CPU), this decision should be revisited. The `rag-engine` reads the model name from config, so swapping back to 7B (or a larger model) is a config + re-provision change, not an architecture change.
- **Embedding model unchanged**: `llama-embed` still uses `nomic-embed-text` (74 MB). The embedding model choice is independent of the generation model choice.

## Implementation

- `Makefile` `provision-models` target downloads `qwen2.5-3b-instruct-q4_k_m.gguf` from `Qwen/Qwen2.5-3B-Instruct-GGUF`.
- `docker-compose.yml` `llama-generate` service `command` points to the new filename.
- `models/generate/README.md` updated with the new expected file and rationale.
- `docs/STACK.md §3.4` table and compose excerpt updated.

## Alternatives considered

- **Qwen2.5-7B Q4_K_M (original)**: Works but too slow on the target hardware.
- **Qwen2.5-1.5B Q4_K_M (~1 GB)**: Even faster (~10-15 tok/s) but quality drops noticeably. Chosen not to go this low — the 3B is the sweet spot for prototype quality.
- **Qwen2.5-7B Q2_K (~3 GB)**: Smaller than Q4_K_M but quality degradation from aggressive quantization is worse than dropping to a 3B model at Q4_K_M.
