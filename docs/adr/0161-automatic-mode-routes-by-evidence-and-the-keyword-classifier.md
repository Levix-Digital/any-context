---
id: 161
title: Automatic mode routes by evidence and the keyword classifier is removed
status: accepted
date: '2026-11-10'
version: v1.0.0
subsystems: [agent, onnx, model-catalog]
supersedes: []
superseded_by: []
amends: [130, 118]
amended_by: []
---

# ADR-0161: Automatic mode routes by evidence and the keyword classifier is removed

## Context

Automatic mode (`/search auto`, the default) chose Fast or Deep before any search with a keyword and length heuristic: `DeterministicClassifier` in `actx-agent`, wrapped in the application by the `actx-onnx` query classifier and the core's `OnnxComplexityClassifierAdapter`, which was the same heuristic (ADR-0130). PT/EN words such as "arquitetura", "compare" or "end-to-end", or more than 25 words, sent a question to Deep even when one excerpt answered it; a question in another language almost never went to Deep. The intent it also returned (code, architecture, document...) was shown but drove nothing. Its `bge-small-onnx` weights (133 MB) were in the catalog (ADR-0118) and in every hardware kit, and were never executed. Since ADR-0160 automatic Fast already escalates on nothing found or weak evidence, but relevant excerpts that miss the asked fact were still answered in Fast. The target design (`docs/architecture/retrieval-pipeline-v2.md`, step 2) asks for no keyword routing in any language, escalation when the evidence is weak or the model says the context is not enough, and pinned modes. Levi approved the plan on 2026-11-10 (`specs/009-evidence-routing/`).

## Decision

- **The route comes from the search mode.** `router::route` gives Fast for automatic mode and pinned Fast, Deep for pinned Deep, and records whether the mode was pinned. The classifier, the query intent, the complexity, the confidence and `ChatModelRouter` are removed, with the `actx-onnx` query classifier and its core adapter.
- **Live status keeps its terms for now.** `is_status_query` still sends status questions to the status retriever (ADR-0145): it picks a data source, not a search depth, and goes away when status becomes an agent tool (FEAT-082).
- **The model can ask for a deeper search.** In automatic mode with a Deep Search engine and sufficient evidence, the Fast prompt ends with a rule: reply only with `[[ACTX_NEEDS_DEEPER_SEARCH]]` when the excerpts do not contain the answer. `SignalFilter` holds back the streamed answer while it can still be the signal; on the signal the attempt is dropped before any token is shown, `AgentEvent::Escalation` gives the reason "the quick search excerpts do not answer the question", and Deep Search runs. Nothing of the Fast attempt is stored. A signal later in an answer is removed. Pinned Fast gets no rule and never escalates.
- **The routing event says what is true.** `AgentEvent::RoutingDecision { route, pinned, reason }`; the interfaces show quick search, deep search or live status, automatic or pinned.
- **The classifier weights leave the catalog.** `bge-small-onnx` is removed from the model catalog, the hardware kits, the installer and the Document AI menu (whose Query Routing submenu goes away). Files already downloaded are left on disk, unused.

## Alternatives Considered

- **A neural classifier on the BGE-Small weights**: rejected; a decision taken before seeing the evidence is still a guess, and it costs 133 MB per kit.
- **An LLM routing call before the search**: rejected; one more call on every question, while the bands are free after the search.
- **A separate judge call after the Fast retrieval**: rejected; it adds a call to every question, while the signal reuses the call Fast makes anyway.
- **Reading the answer for phrases like "I could not find"**: rejected; keywords again, and language dependent.
- **Removing the status terms now**: rejected; status questions would go to search, which cannot answer them, until FEAT-082.

## Consequences

- Questions the old classifier sent to Deep are answered by one Fast call when the evidence is sufficient; automatic mode no longer depends on the question's language.
- A normal Fast answer waits at most the signal's length (28 characters) before its first token is shown.
- An escalation through the signal costs the short Fast call plus the Deep run; the reason is shown, and the user can pin Fast or Deep.
- Models that ignore the rule answer as before; the band escalation of ADR-0160 is unchanged.
- Hardware kit downloads are 133 MB smaller. ADR-0118's catalog entry for `bge-small-onnx` and ADR-0130's mention of the query classifier no longer describe the code.
