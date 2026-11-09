---
id: 160
title: The answer path reads the evidence from the reranker bands
status: accepted
date: '2026-11-09'
version: v1.0.0
subsystems: [retrieval, agent]
supersedes: []
superseded_by: []
amends: [128]
amended_by: []
---

# ADR-0160: The answer path reads the evidence from the reranker bands

## Context

After ADR-0158 and ADR-0159 every chunk that reaches an answer is a pinned file-name match or was judged by the cross-encoder as relevant or maybe; irrelevant ones are discarded. The answer path did not use this. The Zero Hallucination Gate of ADR-0128 answered "nothing found" only when no chunk scored at least 0.005, a cut chosen for RRF scores, so after the reranker it fired only on an empty selection. An answer built only on maybe-band chunks looked like a grounded one. Deep Search always called the synthesis, even with no chunk. In automatic mode a question routed to Fast stayed in Fast whatever it found. The target design (`docs/architecture/retrieval-pipeline-v2.md`, step 7, item 5, and step 2) asks for a gate on the bands and a Fast-to-Deep escalation. Levi approved the plan on 2026-11-09, with escalation on both nothing and maybe-only evidence (`specs/008-nothing-found-gate/`).

## Decision

- **Evidence classes.** The delivered chunks are nothing (none), weak (every chunk in the maybe band) or sufficient (any relevant chunk, any pinned match, or chunks without a band). `retrieval::gate::classify_evidence` in the core; `actx_agent::Evidence` and `RetrievedChunk::weak` carry it to the agent. The 0.005 score cut is removed.
- **Nothing found without the LLM.** Fast and Deep answer with `nothing_found_message`: the workspace, its monitored folders and web sources, and two suggestions (rephrase; add sources with `/folder` or `/web`). Deep checks after its turns and skips the synthesis call. The message is stored in the session like an answer.
- **Weak evidence is said first.** The answer starts with `WEAK_EVIDENCE_WARNING`, sent as `AgentEvent::Caveat` before the first token and kept out of the session history; the answer prompt (Fast and the Deep synthesis) gets `WEAK_EVIDENCE_RULE`, so the model says what the excerpts do not answer.
- **Automatic Fast escalates.** In `/search auto`, a Fast route whose evidence is nothing or weak sends `AgentEvent::Escalation { reason }` and runs Deep Search instead of the Fast LLM call. Pinned Fast and pinned Deep never escalate. The live status route is never gated (ADR-0145).
- **Without bands nothing is judged.** When the reranker could not run or its model has no thresholds, only an empty selection is "nothing found"; there is no warning and no escalation.
- **Benchmark.** The delivered pass reports the share of negatives and positives that would get the warning (`weak_evidence_rate`, `positive_weak_evidence_rate`), not gated.

## Alternatives Considered

- **Escalate only when nothing is found**: rejected by Levi; maybe-only answers are the ones Deep's refined sub-questions improve, and the design asks for it.
- **A share of relevant chunks as the threshold for weak**: rejected; one relevant chunk answers most questions.
- **Treat band-less chunks as weak**: rejected; every Lite answer would carry the warning until IMP-055.
- **Let the model write the warning**: rejected; a fixed line does not depend on the model and cannot be dropped.

## Consequences

- No answer LLM call is spent when nothing is found, in Fast or Deep.
- Loosely grounded answers are marked; the session history stays clean for later turns.
- An escalation costs the Deep calls (decomposition, reflections, synthesis or none).
- Still open, with their items: automatic mode always starting in Fast without keywords, and the model saying the context is not enough as a second trigger (IMP-038); `@source` in the suggestions (FEAT-088); the warning inside the streamed answer with visible progress (IMP-050); per-workspace thresholds (IMP-008).
