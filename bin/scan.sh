#!/usr/bin/env bash
set -euo pipefail

# Zero-high-CVE image scanning gate, container-first (no host tooling beyond
# Docker, per STACK.md §7.3 Rule 3). Runs trivy — itself a container — against
# every image this project builds (production images: runtime stage for
# backend/ingest, since that is what ships).
#
# The upstream `ghcr.io/ggml-org/llama.cpp:server` image is intentionally NOT
# scanned here: we don't own its Dockerfile (same accepted limitation as the
# non-root exception documented in the ADR), so gating our build on CVEs we
# cannot patch would make this target permanently, unactionably red.

TRIVY_IMAGE="aquasec/trivy:latest"
TRIVY_CACHE_VOLUME="spontini-trivy-cache"

IMAGES=(
    "spontini-bot-2-backend:prod"
    "spontini-bot-2-ingest:prod"
    "spontini-bot-2-frontend:prod"
    "spontini-bot-2-admin-ui:prod"
)

echo "Scanning ${#IMAGES[@]} image(s) with trivy (severity HIGH,CRITICAL, zero-finding gate)..."

for image in "${IMAGES[@]}"; do
    echo "--- ${image} ---"
    docker run --rm \
        -v /var/run/docker.sock:/var/run/docker.sock \
        -v "${TRIVY_CACHE_VOLUME}:/root/.cache/" \
        "${TRIVY_IMAGE}" image \
        --scanners vuln \
        --severity HIGH,CRITICAL \
        --exit-code 1 \
        --ignore-unfixed \
        "${image}"
done

echo "scan: no HIGH/CRITICAL vulnerabilities found in any image"
