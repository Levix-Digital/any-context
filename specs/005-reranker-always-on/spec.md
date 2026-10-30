# Feature Specification: Reranker always on

**Feature Branch**: `feat/005-reranker-always-on`

**Created**: 2026-10-30

**Status**: Draft

**Input**: User description: "IMP-043: the cross-encoder from the hardware kit judges relevance in both modes, over the child chunk with its contextual header; summaries are reranked, SQL results are not. Acceptance: the /rerank command and the option to disable reranking are removed; Deep's chunk-by-chunk LLM yes/no is removed; candidates beyond the budget are discarded, not pushed to the end."

**Backlog**: IMP-043 (plan `imp_043_reranker_always_on`, release v1.0.0 Personal). Design: `docs/architecture/retrieval-pipeline-v2.md`, step 6, Reranking.

## Context

After IMP-041 and IMP-042, one search (Fast, or every Deep sub-question together) has one fusion and one reranking pass over at most `depth` candidates. Three things still contradict the target design:

- **Reranking can be turned off.** The `rerank_mode` setting (`auto`, `on`, `off`) is set by `/rerank on|off|auto` and by `actx --rerank <mode>`, kept in the TUI state and passed on every request; with `off`, the answer is ranked by RRF position only. The TUI header shows a re-rank badge with an `Off` state.
- **The unscored tail stays.** Candidates beyond the depth are not scored, but they are kept below the scored ones (rescaled under half of the lowest cross-encoder score) and can still reach the LLM when there are not enough scored ones.
- **Deep has a second judge.** `DeepSearchConfig::enable_reranker` sends each chunk below a score of 0.15 to the chat LLM with a yes/no prompt (one call per chunk). It is off in the product, but the code path, its prompt and its parser remain and duplicate the cross-encoder.

Parts of step 6 depend on items that are still open, and stay with them:

| Part of step 6 | Item that delivers it | Why |
|---|---|---|
| Reranker model chosen by the hardware kit, Model Server in enterprise mode | IMP-054, IMP-055 | The reranker is one model for the machine; the kit bindings do not exist yet. |
| Reranker input with the contextual header (title and section path) | IMP-026 | Chunks have no contextual header yet; the reranker reads the chunk text as today. |
| Summaries reranked, SQL results not | FEAT-061, FEAT-093, FEAT-092 | Neither summaries nor SQL results exist in retrieval yet. |
| Relevance bands (relevant, maybe, irrelevant) and dropping irrelevant chunks | IMP-044, then IMP-045 and IMP-046 | Thresholds per model need calibration on the benchmark. |

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Every answer is ranked by the cross-encoder, with no way to turn it off (Priority: P1)

The user asks a question in Fast or Deep. The candidates are always scored by the cross-encoder; there is no command, flag or setting that disables it.

**Why this priority**: It is the IMP-043 acceptance: relevance has one judge, and it is an architecture decision, not a user option.

**Independent Test**: Run `/rerank` and `actx --rerank off`: both are unknown. With a stored `rerank_mode = off` from an older version, ask a question with a fixture reranker and check that the result is in reranker order.

**Acceptance Scenarios**:

1. **Given** the TUI or the one-shot CLI, **When** the user types `/rerank` (or `/reranker`), **Then** it is an unknown command and does not appear in `/help`.
2. **Given** the CLI, **When** the user passes `--rerank <mode>`, **Then** the argument is rejected as unknown.
3. **Given** a settings database where an older version stored `rerank_mode = off`, **When** a question is searched, **Then** the candidates are reranked; the stored value is ignored.
4. **Given** the TUI header, **When** it is drawn, **Then** the reranker badge shows the active model when it is ready, or that the reranker is unavailable, never an `Off` state.

---

### User Story 2 - Candidates the reranker did not score are discarded (Priority: P1)

After the reranking pass, only the scored candidates (and the pinned file-name matches) go on to the selection. Candidates beyond the depth are dropped instead of being kept at the bottom.

**Why this priority**: It is the IMP-043 acceptance and the precondition of IMP-044 to IMP-046: a candidate without a relevance judgment must not reach the LLM.

**Independent Test**: With a fixture reranker and a fusion of more candidates than the depth, check that the result holds exactly the scored candidates plus the pinned ones, in score order.

**Acceptance Scenarios**:

1. **Given** more fused candidates than the depth, **When** the search finishes, **Then** the result contains only the pinned candidates and the scored ones.
2. **Given** pinned file-name matches, **When** the tail is discarded, **Then** the pinned matches stay on top, unscored, as today.
3. **Given** fewer candidates than the depth, **When** the search finishes, **Then** every candidate was scored and none is discarded.

---

### User Story 3 - The answer says when the reranker could not run (Priority: P2)

When the reranker cannot run (ONNX Runtime missing, weights not installed, the model failed to load, or the scoring failed), the search keeps the fusion order without discarding anything, and the answer ends with a warning that names the cause and the fix, the same way it does when dense search did not run.

**Why this priority**: With the option gone, a reranker that cannot run is a fault the user must see, not a silent mode.

**Independent Test**: Search with a pipeline whose reranker is missing or not neural-active; check the result is in fusion order and the output carries the unavailable outcome with its notice.

**Acceptance Scenarios**:

1. **Given** no reranker weights installed, **When** a question is answered, **Then** the answer ends with "Answered without relevance ranking: ..." and suggests `actx download reranker`.
2. **Given** ONNX Runtime missing, **When** a question is answered, **Then** the warning says the runtime is missing and suggests reinstalling.
3. **Given** the reranker ready, **When** a question is answered, **Then** no reranker warning is shown.
4. **Given** dense search also did not run, **When** a question is answered, **Then** both warnings are shown.
5. **Given** `/status`, **When** it is shown, **Then** it has a reranker line with the model, its state, the profile's Fast/Deep depths and the last pass's latency and candidate count.

---

### User Story 4 - Deep has no LLM relevance check per chunk (Priority: P2)

Deep accepts the chunks the search returns; the LLM only evaluates the set (what is still missing) in its reflection step.

**Why this priority**: It removes a second relevance judge and its per-chunk LLM calls.

**Independent Test**: The Deep configuration has no reranker switch or threshold; a Deep run with a mock LLM makes no yes/no calls.

**Acceptance Scenarios**:

1. **Given** a Deep question, **When** chunks are retrieved, **Then** no chat request is made per chunk to judge it.
2. **Given** the Deep configuration, **When** it is built, **Then** it has no `enable_reranker` or `rerank_score_threshold`.

### Edge Cases

- No candidate at all (empty workspace or no match): no reranking pass, no reranker warning, as today.
- Only pinned candidates: nothing to score; no warning (the reranker was not needed).
- The reranker returns a number of scores different from the number of candidates: treated as a failure (fusion order kept, warning shown).
- A per-run model given with `--rerank-model` that is not installed: the reranker is unavailable for that run and the warning says the weights are missing.
- The benchmark always runs with the reranker; a reranker that cannot run is an environment error (exit code 3), as today.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The `/rerank` command (with its alias `/reranker` and its `auto`, `on`, `off`, `status` and `model` options) MUST be removed from the command engine, the command registry and the help.
- **FR-002**: The `--rerank` CLI flag MUST be removed; `--rerank-model` (per-run model, not persisted) MUST stay.
- **FR-003**: `rerank_mode` MUST be removed from the pipeline, `HybridSearchRequest`, the request builders, the command execution context, the command state updates and the TUI state; a stored `rerank_mode` setting MUST be ignored.
- **FR-004**: After the reranking pass, unpinned candidates that were not scored MUST be discarded before the selection.
- **FR-005**: When the reranker cannot score (no reranker in the pipeline, runtime missing, weights missing, load failure, scoring failure or wrong score count), the fusion order MUST be kept, nothing MUST be discarded, and the search output MUST report a reranker outcome other than `Scored` with its cause.
- **FR-006**: A reranker outcome other than `Scored` MUST add an answer notice ("Answered without relevance ranking: <cause>. <fix>") in Fast, in Deep and in the status route, next to the dense search notice when there is one.
- **FR-007**: `/status` MUST show a reranker line: model id, state (ready or the cause), Fast/Deep depth of the profile and the last pass's latency and candidate count.
- **FR-008**: The TUI badge MUST show the reranker's model when ready and an unavailable state otherwise; it MUST NOT have an off state.
- **FR-009**: Deep's per-chunk LLM yes/no (`enable_reranker`, `rerank_score_threshold`, the rerank prompt and its parser) MUST be removed.
- **FR-010**: The persisted reranker model choice (`/rerank model <id>`) is removed with the command; the existing `reranker_model` setting MUST still be honored until the hardware kit chooses the model (IMP-055).
- **FR-011**: The development retrieval benchmark MUST show no regression beyond its threshold in the lexical and dense configurations.

### Key Entities

- **Reranker outcome**: how the reranking pass of a search went: `Scored`, `NotNeeded` (no unpinned candidate) or `Unavailable(cause)`, where the cause says what is missing or what failed.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: No command, flag or setting can disable the cross-encoder (`/rerank` and `--rerank` unknown; a stored `off` has no effect).
- **SC-002**: With 40 unpinned candidates and depth 30, the result holds at most 30 unpinned candidates, all scored.
- **SC-003**: With the reranker unavailable, every answer ends with the reranker warning; with it ready, none does.
- **SC-004**: Benchmark MRR@10 and HitRate@5 do not drop beyond the regression threshold (0.02) in the lexical and the dense configuration.

## Assumptions

- Users who stored `rerank_mode = off` accept that reranking comes back on; the design makes it an architecture decision. The stored value is left in the database (no migration) and ignored.
- The top 10 of the benchmark never comes from the unscored tail (the depth is 30), so discarding the tail does not change its metrics.
- `actx download reranker` remains the way to install or repair the reranker weights.
