---
id: 156
title: One fusion of all lists per search, and reranker depth by mode, profile and time budget
status: accepted
date: '2026-10-30'
version: v1.0.0
subsystems: [retrieval, cli]
supersedes: []
superseded_by: []
amends: [129, 139]
amended_by: []
---

# ADR-0156: One fusion of all lists per search, and reranker depth by mode, profile and time budget

## Context

After IMP-040 each question gets a dense and a lexical list, but the fusion that follows was not clean. File-name and date mentions were injected into the dense list with invented scores (1.0 and 0.95) and `Global`'s dense hits were merged into the same map, so the dense ranks fed to RRF mixed scales. The fused list was cut by its RRF score (`min_score`, ADR-0139), which measures position in the lists, not relevance, and in practice never dropped anything. The cross-encoder rescored the 30 best by RRF on every machine (ADR-0129), so a long document could take most slots and a light laptop paid the latency of a workstation. In Deep, every sub-question ran its own fusion, reranking and selection, and the batch summed the scores of the survivors, which mixed relevance with how many sub-questions found a chunk. IMP-041 and IMP-042 (`docs/architecture/retrieval-pipeline-v2.md`, step 5) ask for one fusion over clean lists and a reranker depth set by the mode, the hardware profile and a time budget. Levi chose them on 2026-10-30 (`specs/004-unified-fusion/spec.md`).

## Decision

- **Clean lists.** Each question gives one list per source, ranked by its own score: dense per searched workspace (the request's and `Global`), lexical, file-name matches and date matches. File-name matches stay pinned on top with score 1.0 and are never rescored, as before.
- **One fusion per search.** All lists of all questions of a search (the Fast question, or every Deep sub-question) enter one RRF (`rrf::fuse`, k = 60, equal weights). A fused candidate records the questions whose lists found it and the one of its highest single contribution. Fast is a batch of one question: one code path.
- **No score cut.** `HybridSearchRequest::min_score`, `RRF_MIN_SCORE`, the presets' minimum scores and `apply_threshold` are removed. Relevance is the reranker's to judge.
- **One reranking pass.** The unpinned candidates in fusion order go through the per-source cap with 3 slots per document (`PER_DOCUMENT_RERANK_SLOTS`); the reserve fills the slots left; the first `depth` are scored. Each candidate is scored once, against the question that ranked it best; the pass groups candidates by question on one blocking thread. The unscored tail stays below every scored candidate (until IMP-043 discards it).
- **Depth.** `rerank_depth::profile_depth`: Fast/Deep 20/30 on Lite, 30/50 on Smart Balanced and Air-Gapped Power, 40/75 on Workstation Pro, 50/100 on Sovereign Ultra. The request builders take the hardware profile in effect (the `active_profile` setting, else the recommended one, probed once per process) and set a time budget of 1 s in Fast and 3 s in Deep. Each pass records its cost per candidate (averaged with the previous one); the next depth is lowered to fit the budget, never below 10. A request without a budget (the benchmark) keeps the profile depth.
- **Selection once.** The per-source cap, top-k and character budget of the first request run once on the reranked list; the condensed text of the last chunk is kept in both modes.

## Alternatives Considered

- **Rerank against the original question in Deep**: rejected; the retriever port receives only the sub-questions, and the sub-questions exist because the original is too broad for one passage.
- **Rerank against every sub-question that found a candidate and keep the maximum**: rejected for R1; it multiplies the cost and breaks the time budget.
- **Keep the RRF cut with a lower floor**: rejected; it never measured relevance.
- **A fixed depth per mode**: rejected; light machines need fewer candidates and strong ones can afford more.
- **RRF weights and depth tuned per workspace now**: deferred to FEAT-105, which needs the per-workspace evaluation set (FEAT-073, v1.1.0).

## Consequences

- Deep ranks by cross-encoder relevance, not by popularity across sub-questions, and pays one reranking pass per turn.
- A long document can no longer take every reranker slot; the final selection is unchanged until IMP-045.
- `/rerank` shows the depth of the profile and the number of candidates of the last pass.
- The scores the Deep agent compares with its 0.15 threshold are single cross-encoder scores again.
