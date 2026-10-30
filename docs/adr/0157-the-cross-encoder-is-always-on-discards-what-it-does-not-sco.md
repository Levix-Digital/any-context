---
id: 157
title: The cross-encoder is always on, discards what it does not score and reports when it cannot run
status: accepted
date: '2026-10-30'
version: v1.0.0
subsystems: [retrieval, commands, cli, tui, agent]
supersedes: []
superseded_by: []
amends: [129, 156]
amended_by: [158]
---

# ADR-0157: The cross-encoder is always on, discards what it does not score and reports when it cannot run

## Context

After IMP-041 and IMP-042 one search has one fusion and one reranking pass over at most `depth` candidates (ADR-0156), but the reranker was still a user option and not the only judge. The `rerank_mode` setting (`auto`, `on`, `off`, ADR-0129) was set by `/rerank` and `actx --rerank`, kept in the TUI state and passed on every request; with `off` the answer followed RRF positions only. Candidates beyond the depth were kept, rescaled below the scored ones, so unjudged chunks could still reach the LLM. Deep kept a per-chunk LLM yes/no (`DeepSearchConfig::enable_reranker`, off in the product) that duplicated the cross-encoder. A reranker that could not run (no ONNX Runtime, weights missing, a failed load) fell back to RRF silently, apart from a header badge. IMP-043 (`docs/architecture/retrieval-pipeline-v2.md`, step 6) makes the cross-encoder the only relevance judge, always on. Levi chose it on 2026-10-30 (`specs/005-reranker-always-on/spec.md`).

## Decision

- **Always on.** `rerank_mode` is removed from `NativeHybridPipeline`, `HybridSearchRequest`, the request builders, `ExecutionContext`, `CommandStateUpdates` and the TUI. `/rerank` (with `/reranker`, its modes and `model`) and `--rerank` are removed. A stored `rerank_mode` setting is ignored; it is not migrated.
- **Unscored candidates are discarded.** After a successful pass the result keeps the pinned file-name matches and the scored candidates only (`keep_scored`); the selection never sees an unjudged candidate.
- **A reranker that cannot run is reported.** The search returns a `RerankOutcome` in `RetrievalOutput`: `Scored`, `NotNeeded` (nothing unpinned to score) or `Unavailable(cause)` (no reranker, runtime missing, weights missing, load failure, scoring failure, wrong score count). `Unavailable` keeps the fusion order and discards nothing. `RetrievalOutput::notices()` adds "Answered without relevance ranking: <cause>. <fix>" after the dense search warning (ADR-0155) at the end of Fast, Deep and status-route answers. `/status` has a `Reranker` line: model, state (from the last outcome, else the installed state), the profile's Fast/Deep depths and the last pass. The TUI badge shows the model when it can run, else `⚠ Reranker unavailable`.
- **One judge in Deep.** `enable_reranker`, `rerank_score_threshold`, `RERANK_PROMPT`, its builder and `parse_rerank_decision` are removed; Deep keeps every new chunk the retriever returns and its LLM only evaluates the set in the reflection step.
- **Model.** The persistent choice (`/rerank model <id>`) goes with the command. The stored `reranker_model` setting is still honored and `--rerank-model` still sets the model for one run; the hardware kit chooses the model in IMP-055.
- **Benchmark.** A ranking whose reranking is `Unavailable` is an environment error (exit 3), so no unjudged run is compared with a baseline.

## Alternatives Considered

- **Keep `/rerank` as a status command**: rejected; the status fits `/status`, and a command named after a switch invites the question of how to switch it.
- **A `/reranker <model>` command until IMP-055**: rejected; the design gives the model to the hardware kit, and the command would be removed again.
- **Fail the question when the reranker cannot run**: rejected; a missing optional download on Lite would leave the user without answers. Discarding every candidate was rejected for the same reason.
- **Keep the fusion order silently**: rejected; it would bring back the off mode without the user knowing.

## Consequences

- No user can disable the cross-encoder; a user who had stored `off` gets reranking back.
- The selection works only on judged candidates, which the relevance bands (IMP-044) and the selection by bands (IMP-045, IMP-046) need.
- A machine whose reranker cannot run answers in fusion order with a warning that names the cause and the fix.
- Deep makes no per-chunk LLM calls.
- Still open, with their items: the contextual header as reranker input (IMP-026), the model per hardware kit (IMP-054, IMP-055), summaries and SQL results (FEAT-061, FEAT-093, FEAT-092).
