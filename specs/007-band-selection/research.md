# Research: Selection by relevance bands and the token budget

## Today's selection (dev d801056)

| Path | Count | Per-file cap | Ceiling | About |
|---|---|---|---|---|
| Fast (`fast_request`) | 6 | 3, with backfill | 14,000 chars | 3,500 tokens |
| Deep batch (`deep_request`) | 8 | 3, with backfill | 16,000 chars | 4,000 tokens |
| Agent search tool (`tool_request`) | 5 | 3, with backfill | 12,000 chars | 3,000 tokens |
| `RetrievalPreset` (tests only) | 10/20/40 | 2/3/5 | 20/40/60 thousand chars | unused by the product |

The last chunk that crosses the ceiling is truncated with "[...additional snippet condensed for density limit...]" when more than 200 characters are left, so the LLM can receive a cut passage.

## Decisions

- **Order = score order.** After IMP-044 the bands are monotonic in the reranker score and pinned matches carry 1.0, so `compare_by_relevance` already orders pinned, relevant, maybe. No second sort key is needed.
- **Skip, not truncate.** A chunk that does not fit is skipped whole; the first one always enters. Alternative (truncate the last) rejected: a cut passage can lose the sentence that answers.
- **Near-duplicates by word 3-shingles, Jaccard >= 0.9.** Cheap for 30 to 60 candidates (quadratic over the selected set only). Alternative (embedding similarity) rejected: needs vectors of lexical-only hits and costs more for the same effect on copies and templates.
- **Budgets per mode as constants.** The chat model window is unknown to the pipeline until IMP-049.
- **Current version only deferred to FEAT-027.** No version data exists; deducing it from the path (ADR-0140) is what the v2 design removes.

## Benchmark runs

Local runs, lexical configuration (`lexical-rerank`, mmarco), 2026-10-31, ONNX Runtime 1.29 on the development container:

| Revision | MRR@10 | HitRate@5 | delivered hit rate | nothing-found (negatives) | false nothing-found | delivered chunks / tokens (mean) |
|---|---:|---:|---:|---:|---:|---|
| dev `d801056` (before) | 0.9357 | 0.9571 | 0.9571 | 0.3333 | 0.0143 | not measured |
| feat `6ef7dc2` (Fast budget 4,000) | 0.9357 | 0.9571 | 0.9571 | 0.3333 | 0.0143 | 4.62 / 418 |

- Every question has the same first relevant rank, delivered hit and nothing-found outcome on both revisions: the selection loses nothing the old one delivered. The gap to the committed baseline (MRR 0.9500) is the same on both and comes from the local runtime, not from this change; the baselines are refreshed from the CI run.
- The largest delivered context is 15 chunks and 2,213 estimated tokens: on this corpus (short chunks of about 90 tokens) the bands, not the budget, bound the selection. Budgets of 3,000, 4,000 and 6,000 give the same result, so the corpus cannot tell them apart; 4,000 stays (about today's 14,000 characters) and IMP-049 revisits it with the model window.
- CI on the pull request (head `225a7d9`), both gates green; baselines refreshed from its RESULT-JSON:

| Configuration | MRR@10 | HitRate@5 | delivered hit rate | nothing-found | false nothing-found | delivered chunks / tokens (mean) |
|---|---:|---:|---:|---:|---:|---|
| `lexical-rerank` (baseline `872575d` -> PR) | 0.9500 -> 0.9429 | 0.9571 -> 0.9571 | 0.9571 -> 0.9571 | 0.4000 -> 0.4000 | 0.0286 -> 0.0286 | 4.67 / 401 |
| `dense-local-bge-m3-rerank` | 0.9714 -> 0.9714 | 1.0 -> 1.0 | 1.0 -> 1.0 | 0.1333 -> 0.1333 | 0.0 -> 0.0 | 6.92 / 590 |

The only change is `pt-cross-002` (rank 1 -> 2): `api-rate-limits-v1.md` (0.9991) now edges `api-rate-limits-v2.md` (0.9990), a near-tie in reranker scores of the ranking pass, which the selection does not touch; the local runs show the same rank on dev and on this branch.
