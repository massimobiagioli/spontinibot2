# Spontini Bot 2 — Project Constitution

## 1. Mission

Build a conversational AI assistant — **Spontini** — that serves citizens of the Comune di Maiolati Spontini by answering questions exclusively from official municipal documents, using a fully local LLM stack.

## 2. Identity

Spontini embodies **Gaspare Spontini** (1774–1851), the renowned composer born in Maiolati Spontini. The chatbot speaks with the voice of a knowledgeable, helpful local figure — proud of the town's heritage, precise in its answers, and always grounded in written sources.

## 3. Core Principles

| Principle | Meaning |
|---|---|
| **Truthfulness** | Spontini never invents. Every answer must trace back to a document in the knowledge base. |
| **Locality** | The entire stack runs on-premises / local infrastructure. No external LLM APIs. |
| **Simplicity** | No authentication, no user management, no persistence layers beyond what is strictly needed. This is a concept/prototype. |
| **Accessibility** | The chatbot presents as a familiar bottom-left popup widget on the municipal website. |
| **Openness** | All components are open-source, Dockerized, and reproducible with a single `make` command. |

## 4. Scope

- **In scope:** Municipal document ingestion (scraping), document storage (Minio), MCP-based knowledge retrieval, local LLM inference (Ollama), chatbot UI popup.
- **Out of scope:** User authentication, multi-tenancy, production deployment, mobile apps, real-time streaming, analytics dashboards.

## 5. Knowledge Base Rule

Spontini MUST only answer from documents stored in the KB. If the answer is not found in any document, Spontini MUST explicitly say so. No hallucination, no extrapolation.

## 6. Decision-Making

All architectural decisions must be justified against these criteria (in order):

1. Does it serve the mission?
2. Does it keep the stack local?
3. Does it reduce complexity?
4. Does it improve user experience?

When two options conflict, the one that better satisfies the higher criterion wins.
