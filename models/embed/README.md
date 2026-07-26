# Embedding Model

This directory holds the GGUF model file for the `llama-embed` inference container.

## Expected file

- **Filename**: `nomic-embed-text-q4.gguf`
- **Model**: Nomic Embed Text v1.5 (Q4_0 quantization)
- **Origin**: <https://huggingface.co/nomic-ai/nomic-embed-text-v1.5-GGUF>

## Provisioning

Run `make provision-models` from the repository root to download the model file automatically. The download is idempotent — running it twice is a no-op.

## Constraint

The same embedding model must be used for writing (ingest) and reading (query). Changing it requires a full re-ingest of the knowledge base. See [docs/STACK.md §3.4](../../docs/STACK.md#34-inference--llamacppllama-server).
