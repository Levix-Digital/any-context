# Research: Automatic mode routes by evidence, not by keywords

## Decision 1: automatic mode always starts Fast

- **Decision**: no pre-search classification; automatic mode starts Fast and escalates on evidence.
- **Rationale**: the design says "no keywords, in any language". Since IMP-046 the band trigger already sends Fast misses to Deep, so the up-front classifier only adds cost (Deep for questions one excerpt answers) and language bias (PT/EN only).
- **Alternatives**: a neural classifier on the `bge-small` weights (rejected: a model decision before seeing the evidence is still a guess, and it costs 133 MB); an LLM routing call (rejected: one more call on every question, while the evidence is free after the search).

## Decision 2: the LLM says "not enough" with a signal at the start of the Fast answer

- **Decision**: in automatic mode with a Deep engine, the Fast prompt adds a rule: when the excerpts do not contain the information needed, reply only with `[[ACTX_NEEDS_DEEPER_SEARCH]]`. The agent holds back the streamed tokens until they can no longer be the signal (at most the signal's length, after leading whitespace); on the signal it drops the attempt and escalates.
- **Rationale**: it uses the call Fast makes anyway, adds no latency to normal answers (a few characters held back) and costs nothing when the evidence is enough.
- **Cost**: an escalation through the signal costs the Fast call (short: only the signal) plus the Deep run.
- **Alternatives**: a separate judge call after retrieval (rejected: one more call on every question); reading the answer for phrases like "I could not find" (rejected: keywords again, language dependent); letting Deep's gap analysis decide (rejected: Deep only runs after the route is chosen).

## Decision 3: the status route keeps its term list until FEAT-082

- **Decision**: `is_status_query` stays and moves next to the new route function; it picks the live status data source, not a search depth.
- **Rationale**: FEAT-082 turns status and help into agent tools the LLM calls, which removes the term list. Removing it now would send "is it indexed?" to search, which cannot answer it.

## Decision 4: the classifier weights leave the catalog

- **Decision**: remove `bge-small-onnx` from the catalog, the four hardware kits, the installer list, the model menu and the `bge`/`query` aliases. Files already downloaded stay on disk, unused.
- **Rationale**: the weights were only kept for the classifier this item removes (ADR-0130 already says they are never executed). Keeping them means 133 MB per kit download for nothing.
- **Alternatives**: keep them for a future neural router (rejected by Decision 1); delete existing files on startup (rejected: touching user files for 133 MB is not worth a migration; the model manager's cleanup is IMP-054's).

## Decision 5: the routing event is reduced to what is true

- **Decision**: `AgentEvent::RoutingDecision { route, pinned, reason }` with `route` one of `fast`, `deep`, `status`. Intent, complexity and confidence go away with the classifier.
- **Rationale**: those values came only from the keyword classifier; showing a fixed placeholder would mislead.
