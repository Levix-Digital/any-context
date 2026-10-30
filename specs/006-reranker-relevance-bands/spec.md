# Feature Specification: Reranker relevance bands

**Feature Branch**: `feat/006-reranker-relevance-bands`

**Created**: 2026-10-30

**Status**: Draft

**Input**: User description: "IMP-044: reranker scores map to relevant, maybe and irrelevant bands with per-model thresholds in the catalog, refined per workspace by the evaluation. Irrelevant chunks never go to the LLM. The bands feed the nothing-found gate, the Fast-to-Deep escalation and selection."

**Backlog**: IMP-044 (plan `imp_044_reranker_relevance_bands`, release v1.0.0 Personal). Design: `docs/architecture/retrieval-pipeline-v2.md`, step 6, Reranking, item 4.

## Context

After IMP-043 the cross-encoder scores every candidate that reaches it (sigmoid, from 0 to 1) and the unscored tail is discarded. Every scored candidate still goes on to the selection, whatever its score: a chunk scored 0.002 reaches the LLM when there is nothing better, which is the IMP-007 problem. The score is not interpreted anywhere.

The benchmark already shows the separation. In the committed baselines (mmarco reranker, 70 positive and 15 negative questions) the best relevant document scores above 0.9 in most positive questions, while the best candidate of most negative questions scores below 0.05.

Parts of the design that use the bands stay with their items:

| Part | Item that delivers it | Why |
|---|---|---|
| Selection by bands and token budget (relevant first, then the best of maybe) | IMP-045 | Replaces top-k, per-file caps and character ceilings. |
| Nothing-found gate, Fast-to-Deep escalation and the weak-evidence warning | IMP-046 | Replaces the answer path when there is no relevant chunk. |
| Thresholds refined per workspace by the evaluation | IMP-008 and step 10 (history and evaluation) | There is no per-workspace evaluation set yet. |
| Reranker chosen by the hardware kit (a small one on Lite) | IMP-055 | The bands come with whatever reranker the kit binds. |

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Irrelevant chunks never reach the LLM (Priority: P1)

The user asks a question. After the cross-encoder scores the candidates, every chunk whose score is in the irrelevant band of the active reranker is discarded before the selection, even when it is the best one left.

**Why this priority**: It is the IMP-044 acceptance and solves IMP-007: a low-scored chunk must not be presented to the LLM as evidence.

**Independent Test**: With a fixture reranker that returns known scores, search and check that the candidates below the irrelevant threshold are gone and the others keep score order.

**Acceptance Scenarios**:

1. **Given** scored candidates with scores 0.95, 0.30 and 0.002 under the mmarco reranker, **When** the search finishes, **Then** the result holds the first two and not the third.
2. **Given** every candidate scored in the irrelevant band, **When** the search finishes, **Then** the result is empty and the answer takes today's path for no results.
3. **Given** pinned file-name matches, **When** the irrelevant chunks are discarded, **Then** the pinned matches stay on top, unscored and without a band.
4. **Given** the reranker cannot run (IMP-043 unavailable outcome), **When** the search finishes, **Then** no band is assigned and nothing is discarded.

---

### User Story 2 - Each scored chunk carries its band (Priority: P1)

Every candidate scored by the cross-encoder gets a band, relevant or maybe (the irrelevant ones are gone), from the thresholds of the active reranker in the model catalog. The search output reports how many candidates fell in each band, so the selection (IMP-045) and the gate (IMP-046) can use them.

**Why this priority**: The bands are the input of IMP-045 and IMP-046 and must exist, tested, before them.

**Independent Test**: Map scores to bands for each catalog reranker at and around its thresholds; search with a fixture reranker and check the band of each result and the counts in the output.

**Acceptance Scenarios**:

1. **Given** the mmarco reranker, **When** a chunk scores at or above the relevant threshold, **Then** its band is relevant; between the two thresholds, maybe; below the irrelevant threshold, irrelevant.
2. **Given** a search with scored candidates, **When** it finishes, **Then** the output has the counts of relevant, maybe and discarded irrelevant candidates.
3. **Given** every reranker in the catalog, **When** the catalog is checked, **Then** each has both thresholds, with 0 < irrelevant threshold < relevant threshold < 1.
4. **Given** a reranker model id that is not in the catalog, **When** a search is scored, **Then** no band is assigned and nothing is discarded (no guessed thresholds).

---

### User Story 3 - The user can see the bands of the last search (Priority: P3)

`/status` shows, in the reranker line, how many candidates of the last search were relevant, maybe and discarded as irrelevant.

**Why this priority**: It makes the cut visible when an answer says nothing was found, without a debug mode.

**Independent Test**: After a search with a fixture reranker, `/status` has the band counts in the reranker line.

**Acceptance Scenarios**:

1. **Given** a search that scored candidates, **When** the user runs `/status`, **Then** the reranker line ends with `bands <r> relevant / <m> maybe / <i> irrelevant`.
2. **Given** no search yet, or a search the reranker did not score, **When** the user runs `/status`, **Then** the reranker line has no band counts.

---

### User Story 4 - The thresholds are calibrated and checked on the benchmark (Priority: P2)

The development benchmark measures the effect of the cut: its ranking pass keeps every scored candidate (so the ranking metrics stay comparable with the baselines), its delivered pass uses the product thresholds, and a calibration report shows, for a set of candidate thresholds, how many negative questions end with nothing found and how many positive questions lose their relevant document. The benchmark can run with any catalog reranker.

**Why this priority**: The thresholds are numbers that decide what the LLM sees; they need evidence and a way to recalibrate when a model changes.

**Independent Test**: Run the benchmark with the mmarco reranker and with bge-reranker-v2-m3; the calibration report and the delivered metrics are in the result; the gated metrics stay within the threshold.

**Acceptance Scenarios**:

1. **Given** a benchmark run, **When** it finishes, **Then** the ranking metrics are computed without the cut and the delivered metrics (delivered hit rate, nothing-found rate on negatives, false nothing-found rate on positives) with it.
2. **Given** `--reranker bge-reranker-v2-m3`, **When** the benchmark runs, **Then** it uses that reranker and records it in the result configuration.
3. **Given** a run, **When** the summary is written, **Then** it has a calibration table of irrelevant thresholds against negatives with nothing found and positives that lost their relevant document.
4. **Given** a change that makes the cut drop relevant documents, **When** CI compares with the baseline, **Then** a drop of the delivered hit rate beyond the regression threshold fails the run.

### Edge Cases

- A chunk scored exactly at a threshold belongs to the higher band.
- Fewer candidates than the depth, all relevant: nothing is discarded.
- Deep runs several sub-questions in one reranking pass (IMP-041): each chunk is banded by the score against the question it was scored with; the same thresholds apply.
- The search tool used by the agent goes through the same pass and the same cut.
- The `--rerank-model` per-run override and the stored `reranker_model` setting pick the model; the thresholds follow that model.
- The Lite profile has no reranker installed: the IMP-043 unavailable outcome applies, with no bands and no cut, until IMP-055.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Each reranker entry of the model catalog MUST carry two relevance thresholds: the irrelevant threshold (below it the chunk is irrelevant) and the relevant threshold (at or above it the chunk is relevant); between them the chunk is maybe.
- **FR-002**: After a reranking pass with the `Scored` outcome, each scored candidate MUST receive the band of its score under the thresholds of the active reranker, and the candidates in the irrelevant band MUST be discarded before the selection.
- **FR-003**: Pinned file-name matches MUST be kept without a band; with any outcome other than `Scored`, or with a reranker that has no thresholds in the catalog, no band MUST be assigned and nothing discarded.
- **FR-004**: The search output MUST report the counts of relevant, maybe and discarded irrelevant candidates of the pass; the counts of the last pass MUST be kept process-wide for `/status`.
- **FR-005**: The `/status` reranker line MUST show the band counts of the last scored pass.
- **FR-006**: The pipeline MUST allow the relevance cut to be turned off for evaluation only (the benchmark's ranking pass); no command, flag or setting of the product exposes it.
- **FR-007**: The benchmark MUST take a `--reranker <catalog id>` option (default mmarco), record the reranker in the result configuration, write a calibration table in the summary and gate the delivered hit rate with the same regression threshold as the ranking metrics.
- **FR-008**: The initial thresholds MUST come from the benchmark calibration documented in `research.md`, favoring keeping relevant documents over discarding irrelevant ones.

### Key Entities

- **Relevance band**: relevant, maybe or irrelevant, from the score of the cross-encoder and the thresholds of its model.
- **Relevance thresholds**: the irrelevant and relevant thresholds of one reranker model, in the catalog.
- **Band counts**: how many candidates of a pass were relevant, maybe and discarded as irrelevant.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: No chunk scored in the irrelevant band reaches the selection (fixture test).
- **SC-002**: On the benchmark with the default reranker, the share of negative questions with nothing delivered rises from 0 to at least a third, in the lexical and the dense configuration.
- **SC-003**: On the benchmark, the delivered hit rate does not drop more than the regression threshold (0.02) in the lexical and the dense configuration, and MRR@10 and HitRate@5 do not change (the ranking pass has no cut).
- **SC-004**: Both catalog rerankers have calibrated thresholds with their benchmark evidence in `research.md`.

## Assumptions

- The benchmark corpus (70 positive and 15 negative questions in English and Portuguese) is representative enough for a conservative first calibration; the per-workspace refinement comes with the evaluation set.
- Losing a relevant document is worse than letting an irrelevant one through, so the irrelevant threshold is placed where the benchmark loses no relevant document that the ranking found, accepting that some negative questions still get chunks.
- Until IMP-045 the selection still applies top-k, per-file caps and the character ceiling to what remains.
- Until IMP-046 an empty result after the cut takes today's no-result path (the LLM is told nothing relevant was found).
