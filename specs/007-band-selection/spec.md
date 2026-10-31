# Feature Specification: Selection by relevance bands and the token budget

**Feature Branch**: `feat/007-band-selection`

**Created**: 2026-10-31

**Status**: Draft

**Input**: User description: "IMP-045: chunks of the relevant band enter, then the best of the maybe band, up to a context token budget set by AnyContext from the chat model and the mode. Near-identical chunks deduplicated, current version only, no hard per-file limit. The fixed top-k (10/20/40), per-file caps (2/3/5) and character ceilings (20/40/60 thousand) are removed."

**Backlog**: IMP-045 (plan `imp_045_band_selection`, release v1.0.0 Personal). Design: `docs/architecture/retrieval-pipeline-v2.md`, step 7, Selection, items 1 and 3.

## Context

After IMP-044 every candidate the cross-encoder scored carries a band (relevant or maybe) and the irrelevant ones are gone. The selection that follows still ignores the bands: it takes a fixed number of chunks (6 in Fast, 8 per Deep batch, 5 for the agent's search tool), at most 3 per file with backfill, and then cuts the text at a character ceiling (14, 16 and 12 thousand characters), truncating the last chunk with a "[...condensed...]" marker. A second set of numbers (top-k 10/20/40, per-file caps 2/3/5, ceilings 20/40/60 thousand) lives in `RetrievalPreset`, which no product path uses any more.

Parts of step 7 stay with their items:

| Part | Item that delivers it | Why |
|---|---|---|
| Child replaced by its parent (small-to-big) | FEAT-098 | Parents are stored by FEAT-036, not yet done. |
| Current version only (previous ones in comparisons) | FEAT-027 | There is no version data yet: version families come from ingestion (FEAT-027); today the version is read from the path only for the prompt (ADR-0140). |
| Budget bounded by the model window and the history | IMP-049 | The model window is not known to the pipeline yet. IMP-045 sets the per-mode ceiling that IMP-049 then bounds. |
| Nothing-found gate, Fast-to-Deep escalation, weak-evidence warning | IMP-046 | The answer path when there is no relevant chunk. |
| SQL results and summaries as evidence | FEAT-085 and the table items | Those evidence types do not exist yet. |

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The context is chosen by relevance, not by a count (Priority: P1)

The user asks a question. The chunks of the relevant band enter the context first, in score order; then, while there is room, the best chunks of the maybe band. Nothing beyond the token budget of the mode enters, and no chunk is cut in the middle.

**Why this priority**: It is the IMP-045 acceptance: the fixed top-k, per-file caps and character ceilings are replaced by the bands and a token budget.

**Independent Test**: With a fixture reranker and known chunk sizes, search and check which chunks were selected and that their total tokens stay within the budget.

**Acceptance Scenarios**:

1. **Given** 9 relevant chunks of 300 tokens and a budget of 4,000 tokens, **When** the search finishes, **Then** all 9 are selected (no fixed count stops them).
2. **Given** 2 relevant and 10 maybe chunks of 500 tokens and a budget of 4,000, **When** the search finishes, **Then** the 2 relevant and the 6 best maybe chunks are selected, in score order.
3. **Given** a chunk that does not fit in what is left of the budget, **When** the selection reaches it, **Then** it is skipped whole and a smaller chunk further down can still enter; no chunk is truncated.
4. **Given** the first chunk alone is larger than the budget, **When** the search finishes, **Then** it is selected whole, so a relevant answer is never lost to the budget.
5. **Given** pinned file-name matches, **When** the selection runs, **Then** they enter first, within the budget, like relevant chunks.

---

### User Story 2 - One file can give all the evidence (Priority: P1)

When the relevant chunks all come from one file, they all enter: there is no per-file limit in the final selection. Breadth comes earlier, from the per-document slots at the reranker input (IMP-042), which stay.

**Why this priority**: The per-file caps are part of the acceptance; the answer to a question about one long document needs all of its relevant parts.

**Independent Test**: Search a workspace where five relevant chunks come from one file and one maybe chunk from another; check that the five enter before the maybe one.

**Acceptance Scenarios**:

1. **Given** 5 relevant chunks from `a.md` and 1 maybe chunk from `b.md`, **When** the search finishes, **Then** the 5 from `a.md` come first, then `b.md` if it fits.
2. **Given** the reranker input, **When** candidates are picked for scoring, **Then** the per-document slots of IMP-042 still apply (unchanged).

---

### User Story 3 - Near-identical chunks enter once (Priority: P2)

Two chunks with the same text, or almost the same text (a copied file, a template repeated in many documents), take only one place in the context: the one with the higher score.

**Why this priority**: Duplicates waste the budget and make the LLM read the same evidence twice.

**Independent Test**: Search with two chunks whose texts differ only in spacing and case, and two whose words overlap at 95 %; check that one of each pair is selected.

**Acceptance Scenarios**:

1. **Given** two chunks whose texts are equal after normalizing case and spacing, **When** the selection runs, **Then** only the higher-scored one enters.
2. **Given** two chunks whose word sequences overlap at or above the near-duplicate threshold, **When** the selection runs, **Then** only the higher-scored one enters.
3. **Given** two chunks on the same topic with different text, **When** the selection runs, **Then** both can enter.

---

### User Story 4 - Each mode has its own budget (Priority: P2)

Fast, each Deep batch and the agent's search tool each have a context token budget set by AnyContext, smaller in Fast and larger in Deep. The user does not configure it.

**Why this priority**: The design asks for a budget per mode; the model-window bound comes with IMP-049.

**Independent Test**: Check the budget of each request builder, and that the benchmark's delivered pass uses the Fast one.

**Acceptance Scenarios**:

1. **Given** the Fast, Deep and tool requests, **When** they are built, **Then** each carries its token budget and none carries a top-k, a per-file cap or a character ceiling.
2. **Given** the benchmark, **When** it runs, **Then** its summary reports how many chunks and tokens the delivered pass selected on average.

---

### User Story 5 - Without bands, the budget still holds (Priority: P3)

When no band is available (the reranker could not run, or its model has no thresholds), the scored or fused order is used and the token budget still bounds the context.

**Why this priority**: The Lite profile has no reranker until IMP-055, and the answer must keep working.

**Independent Test**: Search with the reranker unavailable and check that the results follow the fused order within the budget.

**Acceptance Scenarios**:

1. **Given** the reranker unavailable, **When** the search finishes, **Then** the selection follows the fused order, deduplicated, up to the budget.

### Edge Cases

- No candidate left after the irrelevant cut: the selection is empty and the answer takes today's no-result path (IMP-046 changes it).
- A Deep batch with several sub-questions: one selection for the batch, with the Deep budget.
- Chunks with no text: zero tokens; they are dropped by the dedup of empty text.
- Two chunks with the same text in different workspaces (the user's and `Global`): one enters.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The selection MUST admit candidates in this order: pinned matches, relevant band, maybe band, each in score order; candidates without a band (no reranker or no thresholds) follow the scored or fused order.
- **FR-002**: The selection MUST stop admitting candidates whose estimated tokens exceed what is left of the request's token budget, skipping them whole; it MUST never truncate a chunk.
- **FR-003**: The first admitted candidate MUST enter even when it alone exceeds the budget.
- **FR-004**: The selection MUST have no per-file limit.
- **FR-005**: The selection MUST keep one of each set of near-identical candidates (equal normalized text, or word-shingle overlap at or above the near-duplicate threshold), the higher-scored one.
- **FR-006**: The search request MUST carry a context token budget and MUST NOT carry a top-k, a per-file cap or a character ceiling; `RetrievalPreset` and the character-budget function MUST be removed.
- **FR-007**: The Fast, Deep and tool request builders MUST set their own budgets (initial values: Fast 4,000, Deep 6,000 per batch, tool 3,000 tokens), confirmed by the benchmark.
- **FR-008**: The per-document slots at the reranker input (IMP-042) MUST stay unchanged.
- **FR-009**: The benchmark MUST report the average chunks and tokens of the delivered pass and keep gating the delivered hit rate.

### Key Entities

- **Token budget**: the most estimated tokens of evidence a search may select; one per mode.
- **Near-identical chunks**: two candidates with equal normalized text, or word 3-shingle Jaccard similarity of 0.9 or more.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: No selection exceeds its token budget except a single first chunk larger than the budget (unit and pipeline tests).
- **SC-002**: The benchmark's delivered hit rate does not drop below the committed baseline by more than the regression threshold, lexical and dense.
- **SC-003**: The code has no top-k, per-file cap or character ceiling in the selection (`max_chunks_per_source`, `max_density_chars`, `top_k` and `RetrievalPreset` are gone from the request).
- **SC-004**: No chunk text in an answer's context ends with the "condensed" marker.

## Assumptions

- Token counts use the existing estimator (`estimate_token_count`); the chat model's tokenizer comes with IMP-049.
- The budgets are per mode constants until IMP-049 bounds them by the model window and the history.
- The near-duplicate threshold (0.9 Jaccard on word 3-shingles) is fixed; it is cheap for the 30 to 60 candidates of a search.
