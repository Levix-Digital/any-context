# Research: Unified fusion and reranker depth

## R1 Where today's fusion mixes scales

`search_with_outcome` puts the file-name matches (score 1.0), the date matches (0.95), the dense hits of the request's workspace and of `Global` into one map keyed by chunk id, sorts it by score and treats it as the dense list. The cosine similarities (0 to 1) compete with fixed 1.0 and 0.95, so the dense ranks fed to RRF depend on invented numbers. File-name matches are then forced to 1.0 after RRF and protected from the reranker; that behavior is kept (FR-007), only their list becomes clean.

## R2 The RRF score cut does nothing today

The product requests use `RRF_MIN_SCORE` = 0.005 = 1/(60 + 140). With 30 candidates per list, the weakest single-list hit scores 1/(60 + 30) ≈ 0.011, so the cut never drops a candidate. The presets' 0.012/0.008/0.005 are only used by tests. Removing the field changes no product result.

## R3 Which question the reranker sees in Deep

A cross-encoder scores a (question, passage) pair, and a fused Deep candidate may come from several sub-questions. Options:

| Option | Pairs scored | Effect |
|---|---|---|
| Original user question | depth | The Deep retriever port only receives the sub-questions; the original is not always among them, and sub-questions exist precisely because the original is too broad for one passage. |
| Every sub-question that found it, keep the maximum | up to depth × sub-questions | Most faithful, but multiplies the cost and breaks the time budget. |
| **The sub-question whose lists ranked it best (highest single RRF contribution)** | depth | One pair per candidate, the question it answers best by the retrievers' own evidence. Chosen. |

## R4 Reranking cost per candidate

`neural_rerank_scores` already measures the wall time of the last pass (`LAST_RERANK_LATENCY_MS`). Cross-encoder inference runs one batch of pairs of bounded length, so its time is close to linear in the number of pairs. The cost per candidate of the last pass (an exponentially weighted average, so one slow pass does not halve the next depth) predicts the next: `depth = min(profile depth, max(10, budget / cost per candidate))`.

## R5 Depth per profile

| Profile | Fast | Deep | Why |
|---|---|---|---|
| Lite (< 8 GB) | 20 | 30 | Design's "20 in Fast on light profiles". |
| Smart Balanced (8 GB) | 30 | 50 | Design's defaults. |
| Air-Gapped Power (16 GB) | 30 | 50 | Design's defaults; this profile runs bge-m3 for embedding. |
| Workstation Pro (32 GB) | 40 | 75 | Spare capacity, still inside the budget. |
| Sovereign Ultra (64 GB+) | 50 | 100 | Design's "up to 100 in Deep on strong profiles". |

The time budget then caps any of them on a slow machine.

## Results

Development benchmark (IMP-065), 2026-10-30, Fast request at the fixed depth of 30 (standard profile, no time budget), `--check-determinism`, against the baselines of IMP-040:

| configuration | MRR@10 | HitRate@1 | HitRate@5 | NDCG@10 | latency mean / p95 (ms) |
|---|---:|---:|---:|---:|---:|
| `lexical-rerank`, baseline | 0.9429 | 0.9286 | 0.9571 | 0.9342 | 1431 / 2334 |
| `lexical-rerank`, this change | 0.9429 | 0.9286 | 0.9571 | 0.9342 | 1557 / 2787 |
| `dense-local-bge-m3-rerank`, baseline | 0.9714 | 0.9429 | 1.0000 | 0.9761 | 1183 / 2444 |
| `dense-local-bge-m3-rerank`, this change | 0.9786 | 0.9571 | 1.0000 | 0.9814 | 1385 / 2815 |

- No regression (SC-003). Lexical only: identical, as expected: with one list per source and no dense list, the fusion gives the same ranks. Dense: one more question answered first (HitRate@1 +0.014, MRR@10 +0.007), from the clean dense list and the per-document reranker slots.
- Latency is reported only and depends on the machine and its load. Both runs got slower by 9 to 17% in the mean, the lexical one too, although its ranking and its reranker input did not change, so the difference cannot be told apart from run-to-run noise here; the CI run of this pull request measures it on the reference runner. The new baselines are saved from these runs.
