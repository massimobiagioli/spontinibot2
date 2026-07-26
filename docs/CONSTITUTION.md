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
| **Simplicity** | The citizen-facing `/chat` surface has no authentication, no user management — it stays anonymous and frictionless by design. No persistence layers beyond what is strictly needed. The operator-facing admin surface requires single-operator authentication (feature 0027), scoped as narrowly as the admin surface itself demands: no multi-user accounts, no roles, no persistent session store. |
| **Accessibility** | The chatbot presents as a familiar bottom-right popup widget on the municipal website. |
| **Openness** | All components are open-source, Dockerized, and reproducible with a single `make` command. |

## 4. Scope

- **In scope:** Municipal document ingestion (scraping), document storage (Minio), MCP-based knowledge retrieval, local LLM inference (Ollama), chatbot UI popup.
- **Out of scope:** Citizen-facing authentication, multi-tenancy, production deployment, mobile apps, real-time streaming, analytics dashboards. (Operator-facing admin authentication is in scope — see feature 0027 and Core Principles §3.)

## 5. Knowledge Base Rule

Spontini MUST only answer from documents stored in the KB. If the answer is not found in any document, Spontini MUST explicitly say so. No hallucination, no extrapolation.

The following are explicit, non-negotiable instances of this rule — never overridable by a persona's `system_prompt`, by a retrieved chunk that happens to touch the topic, or by any future tuning (see [ADR 0012](../.adr/0012-categorical-refusal-rules-and-standard-fallback-text.md)):

- **No predictions.** Spontini MUST NOT answer questions about the future (e.g. who a future office-holder will be, the outcome of a future event). The KB only contains records of what has already happened or been officially published — it cannot contain the future.
- **No weather forecasts.** Spontini MUST NOT answer questions about current or forecast weather. This is live external data, never a municipal document.
- **No personal or sensitive data.** Spontini MUST NOT answer questions asking for an individual's personal information (e.g. home address, private contact details) even if that person is a public official, unless that exact fact is itself published in an ingested official document.
- **Standard fallback text.** When Spontini cannot answer from the KB — for any of the reasons above or any other case where the KB doesn't cover the question — the response MUST be exactly: *"Mi dispiace ma è un'informazione che non conosco."* No partial attempt, no hedged guess, no invented detail.

## 6. Decision-Making

All architectural decisions must be justified against these criteria (in order):

1. Does it serve the mission?
2. Does it keep the stack local?
3. Does it reduce complexity?
4. Does it improve user experience?

When two options conflict, the one that better satisfies the higher criterion wins.
