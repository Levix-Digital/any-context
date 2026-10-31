---
id: 158
title: Reranker scores are read as relevance bands and irrelevant chunks are discarded
status: accepted
date: '2026-10-30'
version: v1.0.0
subsystems: [retrieval, commands]
supersedes: []
superseded_by: []
amends: [157]
amended_by: []
---

# ADR-0158: Reranker scores are read as relevance bands and irrelevant chunks are discarded

## Context

After ADR-0157 the cross-encoder scores every candidate that reaches it (a sigmoid in [0, 1]) and the unscored tail is discarded, but the score is not interpreted: a chunk scored 0.002 still reaches the LLM when nothing better is left (IMP-007). The target design (`docs/architecture/retrieval-pipeline-v2.md`, step 6, item 4) reads the score as three bands, relevant, maybe and irrelevant, with per-model thresholds, and never sends irrelevant chunks to the LLM. The bands then feed the selection (IMP-045) and the nothing-found gate and Fast-to-Deep escalation (IMP-046). The retrieval benchmark (ADR-0153) has 70 positive and 15 negative questions with reranker scores, which is enough for a first calibration. Levi approved the plan with the conservative threshold on 2026-10-30 (`specs/006-reranker-relevance-bands/`).

## Decision

- **Thresholds in the catalog.** `OnnxModelSpec::relevance` holds `RelevanceThresholds { irrelevant_below, relevant_from }` for each reranker and `None` for every other model; a test requires both rerankers to have valid thresholds (0 < irrelevant < relevant < 1). `mmarco-reranker`: irrelevant below 0.005, relevant from 0.6. `bge-reranker-v2-m3`: irrelevant below 0.03, relevant from 0.1 (evidence in `specs/006-reranker-relevance-bands/research.md`).
- **Bands after a scored pass.** Each scored candidate gets the band of its score (a score at a threshold takes the higher band) and the irrelevant ones are discarded before the selection, even when they are the best ones left. Pinned file-name matches have no band. With any outcome other than `Scored`, or a reranker that is not in the catalog, nothing is banded or discarded: no thresholds are guessed.
- **Counts.** `RetrievalOutput::bands` reports the relevant, maybe and discarded irrelevant counts of the pass; the last ones are kept process-wide and shown at the end of the `Reranker` line of `/status`.
- **Calibration rule.** The irrelevant threshold is the highest one that loses no relevant document the ranking found, in the lexical and the dense benchmark runs; the relevant threshold sits above the best score of the negative questions. Losing a relevant document turns a correct answer into "nothing found", while an irrelevant chunk that gets through is still judged by the LLM and, after IMP-046, by the weak-evidence path.
- **Benchmark.** The ranking pass bands without discarding (`NativeHybridPipeline::set_relevance_cut(false)`, not reachable from any product surface), so the ranking metrics measure order; the delivered pass applies the cut. `--reranker <id>` runs any catalog reranker (the configuration id gets `-<reranker>`), each positive records its best relevant score, the summary has a calibration table, and the delivered hit rate is gated like MRR@10 and HitRate@5.

## Alternatives Considered

- **One global pair of thresholds**: rejected; the two rerankers spread their scores differently, and the hardware kit will bind other models (IMP-055).
- **A user setting for the thresholds**: rejected; the design keeps configuration minimal and gives the tuning to the evaluation set.
- **A more aggressive threshold (0.02)**: makes 9 of 15 negatives come back empty in the dense run but loses one relevant document in 70; Levi chose the conservative one.
- **Dropping the maybe band until IMP-045**: rejected; maybe chunks are the weak-evidence answers of IMP-046 and must reach the selection.

## Consequences

- No chunk the reranker judged irrelevant reaches the LLM; some negative questions now return nothing and take the existing no-result path.
- The selection (IMP-045) and the gate (IMP-046) can read the band of each chunk and the counts of the pass.
- A new reranker in the catalog needs thresholds and a benchmark run before it can be used with bands.
- Still open, with their items: per-workspace refinement by the evaluation set (IMP-008, step 10), selection by bands and token budget (IMP-045), the gate and escalation (IMP-046), a reranker for the Lite profile (IMP-055).
