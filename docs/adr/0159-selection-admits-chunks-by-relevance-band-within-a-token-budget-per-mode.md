---
id: 159
title: Selection admits chunks by relevance band within a token budget per mode
status: accepted
date: '2026-10-31'
version: v1.0.0
subsystems: [retrieval]
supersedes: []
superseded_by: []
amends: [139]
amended_by: []
---

# ADR-0159: Selection admits chunks by relevance band within a token budget per mode

## Context

After ADR-0158 every candidate the cross-encoder scored carries a band, and the irrelevant ones are gone. The selection that followed ignored the bands: a fixed count (6 in Fast, 8 per Deep batch, 5 for the agent's search tool), at most 3 chunks per file with backfill (ADR-0139), and a character ceiling (14, 16 and 12 thousand characters) that truncated the last chunk with a "condensed" marker. `RetrievalPreset` held a second, unused set of numbers (top 10/20/40, 2/3/5 per file, 20/40/60 thousand characters). The target design (`docs/architecture/retrieval-pipeline-v2.md`, step 7, items 1 and 3) selects by relevance and a token budget set by AnyContext per mode, with no hard per-file limit and near-identical chunks once. Levi approved the plan on 2026-10-31 (`specs/007-band-selection/`).

## Decision

- **Order.** The candidates are taken in `compare_by_relevance` order: pinned file-name matches (score 1.0), then the relevant band, then the maybe band, since the bands follow the score. Without bands (no reranker, or a model without thresholds) the scored or fused order is used.
- **Token budget per mode.** `HybridSearchRequest::token_budget` replaces `top_k`, `max_chunks_per_source` and `max_density_chars`. The builders set Fast 4,000, Deep 6,000 per batch and the search tool 3,000 estimated tokens (`FAST_TOKEN_BUDGET`, `DEEP_TOKEN_BUDGET`, `TOOL_TOKEN_BUDGET`), checked with the benchmark (`specs/007-band-selection/research.md`). The user does not configure them.
- **Whole chunks.** A candidate that does not fit in what is left of the budget is skipped whole and the walk goes on, so a smaller one can still enter. The first candidate always enters, even alone over the budget. No chunk is truncated.
- **No per-file limit.** The final selection has none; breadth comes from the per-document slots at the reranker input (ADR-0156), which stay. This amends ADR-0139, whose per-source cap applied to the final selection.
- **Near-duplicates once.** A candidate whose normalized text (lowercase, whitespace collapsed) equals one already taken, or whose word 3-shingles overlap one already taken with a Jaccard similarity of 0.9 or more, is skipped; the higher-scored one stays.
- **Removed.** `RetrievalPreset`, `apply_density_budget` and the three request fields. Old serialized requests still deserialize.
- **Observability.** `RetrievalOutput::selection` (`SelectionCounts`: selected, tokens, over the budget, near-duplicates) for the callers; the benchmark reports the mean chunks and tokens of its delivered pass.

## Alternatives Considered

- **Keep a top-k as a second limit**: rejected; the count is what the design removes, and the budget already bounds the context.
- **Truncate the last chunk to fill the budget**: rejected; a cut passage can lose the sentence that answers, and the LLM reads a broken excerpt.
- **Near-duplicates by embedding similarity**: rejected; lexical-only hits have no vector at hand, and copies and templates are caught by the shingle overlap at a fraction of the cost.
- **A cap on the maybe band**: rejected for now; the budget bounds it and the weak-evidence path of IMP-046 judges answers made only of maybe chunks. The benchmark can reopen it.

## Consequences

- An answer can use every relevant part of one long document, and repeated passages take one place.
- Fewer chunks reach the LLM when few are relevant; more when many are, up to the budget.
- Still open, with their items: the child replaced by its parent (FEAT-098), current version only (FEAT-027, which brings version families), the budget bounded by the model window and the history (IMP-049), the nothing-found gate and weak-evidence warning (IMP-046), SQL results and summaries as evidence (FEAT-085 and the table items).
