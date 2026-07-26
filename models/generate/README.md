# Generation Model

This directory holds the GGUF model file for the `llama-generate` inference container.

## Expected file

- **Filename**: `qwen2.5-1.5b-instruct-q4_k_m.gguf`
- **Model**: Qwen2.5-1.5B-Instruct (Q4_K_M quantization)
- **Origin**: <https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF>
- **Size**: ~1.1 GB

## Provisioning

Run `make provision-models` from the repository root to download the model file automatically. The download is idempotent — running it twice is a no-op.

## Rationale

The 3B model (ADR 0001) was itself replaced by the 1.5B variant after real latency measurement during `TEST-INGESTION-0001`'s Wave 0 found ~95s average response time — prompt processing of the RAG context, not generation, was the dominant cost, and both are hardware-bound on this target machine (2016-era quad-core Intel i7-7820HQ, no GPU acceleration available to the Docker Desktop setup). Live benchmarking on this exact hardware showed 1.5B holds equivalent-or-better answer quality to 3B while running meaningfully faster, whereas going smaller still (0.5B) traded further speed for real reliability problems (missed a plainly-present fact in context, once claimed to *be* Gaspare Spontini instead of an assistant referencing him). A sub-5-second target turned out to not be reachable on this hardware at any model size tested without unacceptable quality loss.

See [ADR-0013](../../.adr/0013-generation-model-1-5b-and-reduced-rag-top-k-for-latency.md) (supersedes [ADR-0001](../../.adr/0001-generation-model-3b.md)) for the full decision record.
