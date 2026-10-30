# Research: Reranker relevance bands

## Decision 1: where the thresholds live

**Decision**: two constants per reranker in the model catalog (`OnnxModelSpec::relevance`).

**Rationale**: the score scale is a property of the model (mmarco and bge-reranker-v2-m3 do not spread their sigmoid scores the same way), and the catalog already pins each model's files and hashes, so a model change and its thresholds move together. Per-workspace refinement needs the evaluation set (IMP-008, step 10) and is left to it.

**Alternatives considered**: one global pair of thresholds (wrong as soon as the kit binds another model); a user setting (contradicts the minimal-configuration principle of the design).

## Decision 2: what the benchmark measures

**Decision**: the ranking pass bands without discarding, the delivered pass applies the cut, and the delivered hit rate becomes a gated metric.

**Rationale**: the ranking metrics (MRR@10, HitRate@5) measure order and stay comparable with the baselines; the delivered metrics measure what the LLM would receive: the share of negatives with nothing delivered should rise, and the delivered hit rate on positives must not fall.

## Decision 3: how the thresholds are chosen

**Decision**: the irrelevant threshold is the highest candidate that loses no relevant document the ranking found, in the lexical and the dense run; the relevant threshold sits above the best score of the negative questions.

**Rationale**: a lost relevant document turns a correct answer into "nothing found"; an irrelevant chunk that gets through is still judged by the LLM and, after IMP-046, by the weak-evidence path.

## Initial reading of the committed baselines

mmarco reranker, revision 3be850d, 70 positive and 15 negative questions. Scores are document-level (the best chunk of each document in the top 10).

| irrelevant threshold | lexical: negatives empty | lexical: positives that lose the relevant doc | dense bge-m3: negatives empty | dense: positives that lose it |
|---:|---:|---:|---:|---:|
| 0.005 | 5 / 15 | 3 (the 3 the ranking already misses) | 1 / 15 | 0 |
| 0.01 | 8 / 15 | 4 | 5 / 15 | 0 |
| 0.02 | 9 / 15 | 4 | 9 / 15 | 1 |
| 0.03 | 12 / 15 | 5 | 10 / 15 | 1 |
| 0.05 | 13 / 15 | 6 | 11 / 15 | 2 |

Best score of the negative questions: 0.473 (lexical), 0.469 (dense); all others below 0.13. Median best relevant score of the positives: above 0.99.

## Calibration runs on the current code

To be filled by T008 (mmarco and bge-reranker-v2-m3, lexical and dense).
