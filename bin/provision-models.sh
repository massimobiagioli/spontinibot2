#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EMBED_DIR="${REPO_ROOT}/models/embed"
GENERATE_DIR="${REPO_ROOT}/models/generate"

EMBED_FILE="${EMBED_DIR}/nomic-embed-text-q4.gguf"
EMBED_URL="https://huggingface.co/nomic-ai/nomic-embed-text-v1.5-GGUF/resolve/main/nomic-embed-text-v1.5.Q4_0.gguf"

GENERATE_FILE="${GENERATE_DIR}/qwen2.5-1.5b-instruct-q4_k_m.gguf"
GENERATE_URL="https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf"

download() {
    local label="$1"
    local target="$2"
    local url="$3"

    if [ -f "${target}" ]; then
        echo "  ✓ ${label} already present"
        return
    fi

    echo "  Downloading ${label}..."
    curl -L --fail -o "${target}" "${url}"
    echo "  ✓ ${label} downloaded"
}

echo "Provisioning GGUF models..."
download "nomic-embed-text (Q4_0, ~74 MB)" "${EMBED_FILE}" "${EMBED_URL}"
download "qwen2.5-1.5b-instruct (Q4_K_M, ~1.1 GB)" "${GENERATE_FILE}" "${GENERATE_URL}"
echo "Done."
