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

Revision 0a0ff21 (dev after IMP-043), mmarco reranker, ranking pass (top 10 documents, best chunk score each).

| irrelevant threshold | lexical: negatives empty | lexical: positives that lose the relevant doc | dense bge-m3: negatives empty | dense: positives that lose it |
|---:|---:|---:|---:|---:|
| 0.005 | 5 / 15 | 3 (the 3 the ranking already misses) | 2 / 15 | 0 |
| 0.01 | 9 / 15 | 4 | 3 / 15 | 0 |
| 0.02 | 10 / 15 | 5 | 9 / 15 | 1 |
| 0.03 | 13 / 15 | 6 | 10 / 15 | 1 |
| 0.05 | 13 / 15 | 6 | 11 / 15 | 2 |

- Lowest relevant scores: `en-pro-004` 0.0095 (lexical) and 0.0122 (dense), `en-cross-001` 0.019 (lexical), `pt-fin-003` 0.029 / 0.047.
- Best negative scores: 0.489 (lexical) and 0.506 (dense); the next ones are 0.099 and 0.12. No positive has its best relevant score between 0.47 and 0.68, so a relevant threshold of 0.6 separates them; 54 (lexical) and 58 (dense) of the 70 positives have a relevant-band document.
- The conservative mmarco thresholds are therefore irrelevant below 0.005 and relevant from 0.6 (approved by Levi at gate 3).

## Results with the bands (T008)

Revision 3931549 (this branch), local runs on Linux, delivered pass with the cut, ranking pass without it.

Calibration table of each run (negatives with nothing / positives that lose the relevant document; 70 positives, 15 negatives):

| irrelevant below | mmarco lexical | mmarco dense bge-m3 | bge-reranker lexical | bge-reranker dense bge-m3 |
|---:|---:|---:|---:|---:|
| 0.005 | 5 / 3 | 2 / 0 | 12 / 3 | 12 / 0 |
| 0.01 | 9 / 4 | 4 / 0 | 13 / 3 | 13 / 0 |
| 0.02 | 10 / 5 | 10 / 1 | 13 / 3 | 13 / 0 |
| 0.03 | 13 / 6 | 11 / 1 | 13 / 3 | 15 / 0 |
| 0.05 | 13 / 6 | 12 / 1 | 15 / 4 | 15 / 1 |

(The 3 lexical losses at the lowest thresholds are the questions the lexical ranking already misses.)

- **mmarco** (lexical and dense against the committed baselines): no regression. Lexical: MRR@10 0.9357 (baseline 0.9429), HitRate@5 0.9571 (=), delivered hit rate 0.9571 (=), nothing-found on negatives 0 → 0.333, false nothing-found 0 → 0.014 (a question whose relevant document the lexical ranking already misses). Dense: delivered hit rate 1.0 (=), nothing-found on negatives 0 → 0.133, false nothing-found 0. The MRR change of the lexical run is present on dev before this branch (ranking pass has no cut) and is within the gate.
- **bge-reranker-v2-m3**: the lowest relevant scores are 0.0346 (lexical) and 0.0452 (dense); the best negatives 0.0434 and 0.0244. Thresholds: irrelevant below **0.03** (the highest grid value that loses no relevant document), relevant from **0.1** (above every negative; 69 of 70 dense positives reach it). With them, 13 (lexical) and 15 (dense) of the 15 negatives return nothing. Dense run: MRR@10 1.0, HitRate@5 1.0, delivered hit rate 1.0, false nothing-found 0; mean latency of the delivered pass 3.2 s against 1.3 s with mmarco.
- SC-002 holds (nothing-found on negatives above 0 in both configurations), SC-003 holds (delivered hit rate unchanged, gated metrics within 0.02), SC-004 holds (both rerankers calibrated).
