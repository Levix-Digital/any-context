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

To be filled by T007 (Fast budgets 3,000, 4,000, 6,000; mmarco; lexical and dense).
