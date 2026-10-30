# Feature Specification: Unified fusion and reranker depth

**Feature Branch**: `feat/004-unified-fusion`

**Created**: 2026-10-30

**Status**: Draft

**Input**: User description: "IMP-041 + IMP-042: all lists (every Deep sub-question, every source, future retrievers) enter one RRF with k = 60 and equal weights; each document takes at most a few reranker slots, the rest in reserve; the RRF minimum-score cut and Deep's deduplication with score summing are removed. Candidates sent to the reranker: Fast 30, Deep 50, adjusted by hardware profile (about 20 on light profiles, up to 100 on strong ones) within a reranking time budget (about 1 second in Fast)."

**Backlog**: IMP-041, IMP-042 (plan `imp_041_042_unified_fusion`, release v1.0.0 Personal). Design: `docs/architecture/retrieval-pipeline-v2.md`, step 5, Fusion.

## Context

After IMP-040 each question (or Deep sub-question) gets a dense and a lexical list of at most 30 candidates. What happens next, inside `NativeHybridPipeline::search_with_outcome`, is not a clean fusion:

- The file-name and date mentions found in the question are looked up in LanceDB and **injected into the dense list with invented scores** (1.0 and 0.95), mixed with cosine similarities, and the `Global` workspace's dense hits are merged into the same map. The dense "list" is therefore several lists sorted on scores that do not share a scale.
- RRF (k = 60, equal weights) fuses that list with the lexical one, then a **cut by the RRF score** (`min_score`, 0.005 in the product requests, 0.012/0.008/0.005 in the presets) drops candidates. The RRF score measures position in the lists, not relevance, so the cut is arbitrary (today it is low enough to keep every candidate, which shows it does nothing useful).
- The cross-encoder rescored the **30 best by RRF** (`MAX_RERANK_CANDIDATES`), whatever the hardware: a long document can take most of the 30 slots, and a light laptop pays the same latency as a workstation.
- In **Deep**, every sub-question runs that whole chain on its own (its own RRF, its own reranking against its own text, its own per-file cap and character budget), and the batch then **deduplicates the survivors by summing their scores**. A chunk found by three sub-questions gets three cross-encoder scores added, so the sum mixes relevance and popularity, and the per-sub-question caps decide what the final fusion even sees.

Parts of the design for step 5 depend on items that are still open, and stay with them:

| Part of step 5 | Item that delivers it | Why |
|---|---|---|
| RRF weights and reranker depth tuned per workspace by the evaluation set | FEAT-105 (self-tuning loop), with FEAT-073 (per-workspace evaluation set) | There is no per-workspace evaluation set before v1.1.0; R1 uses the defaults measured on the development benchmark (IMP-065). |
| Reranker model chosen by the hardware kit, Model Server | IMP-054, IMP-055 | The reranker is still one global model; only the depth follows the profile here. |
| Candidates beyond the reranker budget discarded, `/rerank` removed | IMP-043 | This feature keeps today's tail (unscored candidates stay below every scored one). |
| File-name and date mentions as filters instead of lists | FEAT-088 | Until then they enter the fusion as their own clean lists. |

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Deep ranks a chunk by its relevance, not by how many sub-questions found it (Priority: P1)

A Deep question is split into sub-questions. Every list of every sub-question enters one fusion, and the cross-encoder scores each candidate once. A chunk found by several sub-questions rises in the fusion (it collects RRF contributions from each list), which decides whether it reaches the reranker; its final position comes from its cross-encoder score, not from a sum of scores.

**Why this priority**: It is the IMP-041 acceptance and it fixes how Deep picks its evidence: today the per-sub-question caps and the score sum decide, and the reranker is run once per sub-question.

**Independent Test**: With a mock reranker that scores by text, run a batch of two sub-questions where one chunk is found by both and another, more relevant, by one only; check that the more relevant chunk ranks first and that each candidate was scored once.

**Acceptance Scenarios**:

1. **Given** a Deep batch of several sub-questions, **When** it is searched, **Then** all their lists enter one RRF, and one reranking pass scores each candidate at most once.
2. **Given** a chunk found by two sub-questions and another found by one, **When** both reach the reranker, **Then** their order is the order of their cross-encoder scores; nothing is summed.
3. **Given** a chunk found by several sub-questions, **When** it is scored, **Then** it is scored against the sub-question whose lists ranked it best, and it keeps the list of every sub-question that found it.
4. **Given** a batch of one sub-question, **When** it is searched, **Then** the result is the same as a Fast search of that question with the same request.

---

### User Story 2 - Fusion over clean lists, without a score cut (Priority: P1)

For one question, each retriever and each source gives its own list ranked by its own score: dense in the workspace, dense in `Global`, lexical, the file-name matches and the date matches. All of them enter one RRF with k = 60 and equal weights. No candidate is dropped because of its RRF score.

**Why this priority**: The RRF only makes sense over lists ranked on their own scale; mixing invented scores into the dense list distorts it.

**Independent Test**: Unit-test the fusion with several lists (ranks known) and check the RRF scores; integration-test a question with a file-name mention and check the file's chunks arrive as their own list and stay pinned on top.

**Acceptance Scenarios**:

1. **Given** a question with a file-name mention, **When** it is searched, **Then** the matching chunks form their own list in the fusion and stay pinned at the top of the result, as today, without being scored by the reranker.
2. **Given** a question with a date mention, **When** it is searched, **Then** the chunks of the date range form their own list in the fusion; no score is invented for them.
3. **Given** `Global` among the request's workspaces, **When** it is searched, **Then** its dense hits form their own list.
4. **Given** any request, **When** the fusion runs, **Then** no candidate is dropped because of its RRF score; the `min_score` field and the preset minimum scores no longer exist.

---

### User Story 3 - A long document cannot take every reranker slot (Priority: P2)

When the reranker candidates are chosen, each document (file or page) takes at most 3 slots in fusion order; its other chunks wait in reserve and fill the slots only if there are not enough candidates from other documents.

**Why this priority**: Breadth across documents is secured before the reranker; the final selection (step 7, IMP-045) is free of per-file limits later.

**Independent Test**: Fuse lists where one file holds the 10 best chunks and four other files hold one chunk each; with a depth of 8, check that the reranker gets 3 chunks of the long file and the 4 others plus 1 reserve.

**Acceptance Scenarios**:

1. **Given** a fusion where one document holds most of the top positions, **When** the reranker candidates are chosen, **Then** that document takes at most 3 of them while other documents have candidates.
2. **Given** fewer distinct documents than needed to fill the depth, **When** the candidates are chosen, **Then** the reserve fills the remaining slots in fusion order.
3. **Given** pinned file-name matches, **When** the candidates are chosen, **Then** they do not use reranker slots (as today).

---

### User Story 4 - Reranker depth follows the mode, the hardware profile and a time budget (Priority: P2)

The number of candidates scored by the cross-encoder is 30 in Fast and 50 in Deep on a standard machine, fewer on light profiles and more on strong ones. When the reranker on this machine is slow, the depth drops so a Fast question spends about 1 second reranking and a Deep turn about 3 seconds, never below a floor.

**Why this priority**: Answer latency on light laptops, and use of the spare capacity of strong machines (IMP-042).

**Independent Test**: Unit-test the depth function for every profile and mode, and the budget with a given measured cost per candidate; benchmark Fast at the default depth.

**Acceptance Scenarios**:

1. **Given** the hardware profile in effect, **When** a Fast or Deep request is built, **Then** its depth is the profile's value for that mode (table in the plan).
2. **Given** a measured reranking cost per candidate on this machine, **When** the depth would exceed the mode's time budget, **Then** it is lowered to fit, but never below 10.
3. **Given** no measurement yet in the process, **When** the first question is reranked, **Then** the profile's depth is used, and its cost is measured for the next questions.
4. **Given** the development benchmark, **When** it runs, **Then** the depth is fixed (Fast default of the standard profile, no time budget), so runs stay comparable.

---

### Edge Cases

- Two files with identical content share chunk ids: fusion keys on the chunk id as today; the per-document cap counts the file path the candidate carries.
- A sub-question whose search fails makes the batch fail, as today; a sub-question whose dense half did not run still gives its lexical list, and the dense outcome reported is the first that is not `Dense` (IMP-040).
- The reranker not neural-active (no ONNX Runtime) keeps the RRF order, as today; the depth and time budget do not matter then.
- `rerank_mode` `off` (setting or request) keeps the RRF order, as today, until IMP-043 removes the option.
- A candidate scored against one sub-question keeps the `matched_subqueries` of every sub-question that found it, so Deep's reflection still sees them.
- The cross-encoder's text is the candidate's clear text (decrypted), as today.
- An empty batch returns no results and the `Dense` outcome, as today.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Each question MUST produce separate ranked lists: dense per searched workspace, lexical, file-name matches and date matches; no list may carry scores invented for other lists.
- **FR-002**: All lists of all questions of a search (one in Fast, every sub-question in Deep) MUST be fused by one RRF with k = 60 and equal weights.
- **FR-003**: The pipeline MUST NOT drop candidates by RRF score; `HybridSearchRequest::min_score`, `RRF_MIN_SCORE` and `RetrievalPreset::min_score` MUST be removed.
- **FR-004**: Deep MUST NOT deduplicate per-sub-question results by summing scores; a chunk found by several sub-questions MUST be one candidate of the single fusion, with all those sub-questions recorded.
- **FR-005**: Each search MUST run one reranking pass that scores each candidate at most once, each candidate paired with the sub-question whose lists ranked it best (its highest single RRF contribution).
- **FR-006**: Reranker candidates MUST be chosen in fusion order with at most 3 per document; the rest of each document MUST stay in reserve and fill the slots left in fusion order.
- **FR-007**: Pinned file-name matches MUST keep today's behavior (on top, not scored, no reranker slot).
- **FR-008**: The reranker depth MUST come from the mode and the hardware profile in effect, defined once in the product's request builders.
- **FR-009**: When a reranking cost per candidate has been measured in the process, the depth MUST be lowered to fit the request's time budget (Fast 1 s, Deep 3 s), with a floor of 10; a request without a budget MUST use the profile depth.
- **FR-010**: The final selection (per-file cap, top-k, character budget) MUST run once on the reranked list, with the parameters of the request (the first one in Deep); its values do not change here (IMP-045).
- **FR-011**: The development retrieval benchmark MUST show no regression beyond its threshold in the lexical and dense configurations, and its results MUST be saved as the new baselines.

### Key Entities

- **Ranked list**: the ordered candidates of one retriever and one source for one question, with the question it belongs to.
- **Fused candidate**: a chunk with its RRF score over all lists, the sub-questions that found it and the one that ranked it best.
- **Reranker depth**: the number of fused candidates the cross-encoder scores in one search, from the mode, the profile and the time budget.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a Deep batch, each candidate is scored by the cross-encoder once, in one reranking pass (today: one pass per sub-question, with overlaps).
- **SC-002**: With one document holding the 10 best fused chunks and depth 8, the reranker input has at most 3 chunks of it while other documents have candidates.
- **SC-003**: Benchmark MRR@10 and HitRate@5 do not drop beyond the regression threshold (0.02) in the lexical and the dense configuration.
- **SC-004**: On the Lite profile a Fast question reranks at most 20 candidates; with a measured cost of 50 ms per candidate, a Fast question reranks 20 (1 s budget), and never fewer than 10.

## Assumptions

- The hardware profile in effect is the `active_profile` setting, else the recommended profile for the machine (same rule as the embedding model, `ingestion::embedding::active_profile`).
- The reranking cost is roughly linear in the number of candidates, so the cost per candidate of the last reranking predicts the next one.
- The per-document cap of 3 and the depths per profile are R1 defaults; FEAT-105 tunes them per workspace later.
- The Deep agent's chunk-by-chunk LLM check (score below 0.15) stays until IMP-043; its scores are now single cross-encoder scores instead of sums, which is the scale the threshold was meant for.
