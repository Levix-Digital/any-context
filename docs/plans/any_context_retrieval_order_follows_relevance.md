---
plan_id: any_context_retrieval_order_follows_relevance
project: any-context
title: 'Retrieval order follows relevance: per-source cap in score order and recency as a tie-breaker'
kind: bug
version: v0.45.1
status: in_progress
author: Claude
created_at: '2026-10-17T18:50:49'
updated_at: '2026-10-17T18:56:48'
session_id: null
tags:
- retrieval
- ranking
- rag
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- BUG-004
- BUG-005
- BUG-006
milestones:
- name: M1 Ranking fix
  done: false
- name: M2 Docs and ADR
  done: false
- name: M3 Review and PR
  done: false
status_log:
- at: '2026-10-17T18:50:49'
  by: Claude
  status: draft
  note: Plan created
- at: '2026-10-17T18:52:12'
  by: Claude
  status: active
  note: Plan updated
- at: '2026-10-17T18:56:11'
  by: Claude
  note: 'Backlog items added: BUG-006'
- at: '2026-10-17T18:56:48'
  by: Claude
  status: in_progress
  note: Approved by the user on 2026-10-17 with BUG-006 included (option a)
---

# Retrieval order follows relevance: per-source cap in score order and recency as a tie-breaker

Bug plan: discovery and planning of the backlog items below (`dev-cycle-protocol` Phases 1 and 2). Replace every instruction paragraph with the findings.

## Backlog items

### BUG-004: Source round-robin puts diversity above relevance and breaks the U-shaped order
- **Severity**: medium
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Description**: `search_single_async` returns chunks in the order produced by `apply_source_diversification` (round-robin across sources), and `ContextFormatter::format_xml_chunks` applies the U-shaped reordering to that order. The second-best chunk can land in the middle of the context while a weak chunk sits next to the user query. The round-robin also only reorders candidates that were already retrieved (dense + BM25 over the whole index), so it does not widen document coverage; its first pass gives every source one slot before any second chunk, so a low-scoring chunk from another source can displace a strong second chunk from the same source. Decision (2026-10-17): remove the round-robin and keep the per-source cap (`max_chunks_per_source`) applied in score order, in both `search_single_async` and `retrieve_hybrid_batch_async`; revisit the cap once hierarchical chunking (Frente D) and the evaluation (IMP-008) exist.

### BUG-005: Recency boost overrides relevance across sources
- **Severity**: medium
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Description**: The recency boost in `search_single_async` adds up to 0.01 to the RRF score, while the best possible single-list RRF score is 1/61 ≈ 0.0164, so a recently modified file far down the rankings can outrank an older file ranked first. The intent was a tie-breaker between chunks of the same source, but all chunks of a file or web page share the same `last_modified`, and sync deletes the old chunks of a changed source, so the boost has no effect inside a source and only acts across sources. It also changes which candidates reach the cross-encoder.

### BUG-006: Deep Search minimum score discards single-list hits
- **Severity**: medium
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Description**: Deep Search requests use `min_score: 0.02`, applied to RRF scores before re-ranking. A chunk found only by the dense search or only by BM25 scores at most 1/61 ≈ 0.0164, so it is discarded unless the recency boost (BUG-005) lifts it. Supersedes IMP-001.

## 1. Reproduction

Both defects are deterministic ranking logic in `crates/any-context-core-rs/src/retrieval/` (no logs involved), reproduced on `dev` at `a8c1d5b` (v0.45.0 code).

**BUG-004.** The existing unit test `diversifier::tests::test_diversification_round_robin` documents the defect as expected behavior: for candidates `a1 0.95, a2 0.94, a3 0.93, b1 0.89, b2 0.88, c1 0.85` with `max_per_source = 2` and `top_k = 5`, the output is `[a1, b1, c1, a2, b2]`. `c1` (0.85) sits in third place ahead of `a2` (0.94). `ContextFormatter::u_shaped_reorder` assumes a best-to-worst input and places input rank 2 last, next to the query: `b1` lands there instead of `a2`, the real second best. With `top_k = 2`, the result is `[a1, b1]` and `a2` (0.94) is dropped for `b1`; with a weak `b1` (0.30) the same happens.

**BUG-005.** With RRF k = 60 and the boost `0.01 / (1 + days_old / 30)`:

| Chunk | Lists | RRF | Age | Boost | Final |
|---|---|---|---|---|---|
| File A, rank 1 in BM25 only | 1 | 1/61 = 0.01639 | 365 days | 0.00076 | 0.01715 |
| File B, rank 30 in BM25 only | 1 | 1/90 = 0.01111 | 0 days | 0.01000 | 0.02111 |

File B, 29 positions below, outranks file A. The boost is applied before the `min_score` filter and before the top-30 cut that feeds the cross-encoder, so it also decides which candidates are re-ranked.

**BUG-006.** The Deep Search adapter (`PipelineBatchRetriever` in `actx-cli/src/engine.rs`) sends every sub-query with `min_score: 0.02`. A chunk found by only one list scores at most 1/61 ≈ 0.0164 and is dropped unless the boost lifts it (above, file B passes only because of its boost). A chunk found by both lists passes only while 1/(60+a) + 1/(60+b) ≥ 0.02, roughly when both ranks are below 40. Once the boost is removed (BUG-005), no single-list hit reaches Deep Search at all, so the two fixes ship together (decision at approval, 2026-10-17).

The regression tests in section 5 reproduce the three cases and fail before the fix.

## 2. Evidence and root cause

Confirmed facts (code read on `dev`):

- `search_single_async` (`retrieval/pipeline.rs`) sorts the results by score (cross-encoder or RRF), then calls `apply_source_diversification(results, max_chunks_per_source, top_k)` (`retrieval/diversifier.rs`). Pass 1 iterates `pass_idx in 0..max_per_source` and, for each pass, takes the `pass_idx`-th chunk of every source in first-appearance order. The output order is pass order, not score order. Pass 2 backfills from the score-ordered list when fewer than `top_k` were selected, ignoring the cap, and appends those chunks at the end.
- `apply_density_budget` keeps chunks from the start of the list and condenses or drops the tail, so with the round-robin order it can cut a strong chunk instead of the weakest one.
- `retrieve_hybrid_batch_async` (Deep Search) sorts the merged candidates by accumulated score and applies the same diversification and budget, with the same effect.
- `ContextFormatter::format_xml_chunks` (`retrieval/prompt.rs`) applies `u_shaped_reorder` to the list as received; the formatter is correct for a score-ordered input.
- Callers: `engine.rs` `search_db` tool (`top_k` 5), Deep Search batch (`top_k` 8) and the deterministic retriever (`top_k` 6) all use the default `max_chunks_per_source = 3`.
- The recency boost is added in step 10 of `search_single_async` to every non-guaranteed result: `final_score = res.score + recency_boost`. ADR-0116 introduced it "to break score ties in favor of newer versions". All chunks of a file or web page share one `last_modified`, and sync deletes the old chunks of a changed source, so inside a source the boost is a constant; across sources it is up to 0.01 on a scale whose single-list maximum is 0.0164, so it acts as a ranking signal, not a tie-breaker.

- The other two callers use `min_score: 0.005`, below the lowest RRF score a candidate can have inside the pool (1/(60 + `candidate_pool_k`) ≥ 1/100), so for them the threshold never cuts a candidate of the pool; Deep Search is the only caller whose threshold does.

Root cause: two ordering steps that were meant as secondary adjustments (fair source representation, recency tie-break) override the primary relevance order that the rest of the pipeline (density budget, U-shaped placement) relies on. The Deep Search threshold was set above the single-list RRF maximum, and the boost hid it for recent files.

## 3. Fix design (`software-design`)

Principle: the relevance score is the only ranking signal; source fairness and recency may only choose between otherwise equal candidates or limit how many slots one source takes, and the output of retrieval is always in descending relevance order.

**3.1 Per-source cap in score order (BUG-004).** Replace `apply_source_diversification` with `apply_source_cap(chunks, max_per_source, target_k)` in `diversifier.rs`, input assumed score-ordered:

1. Walk the chunks in order; select a chunk while its source (`file_path`, else `file_name`, else `unknown_source`, as today) has fewer than `max_per_source` selected chunks; stop at `target_k`.
2. Flexible cap (user decision 2026-10-17): if fewer than `target_k` were selected, fill the remaining slots with the skipped chunks in score order. The cap only gives other sources priority; it never leaves a slot empty (a single-file workspace still gets `top_k` chunks).
3. Return the selection in the original score order (merge by original index), so backfilled chunks sit at their score position, not at the end.

`max_per_source = 0` is treated as no cap. The function name changes because the behavior changes; the old name is removed (the re-export in `retrieval/mod.rs` is its only public use, no caller outside the core). `search_single_async` and `retrieve_hybrid_batch_async` call `apply_source_cap` in place of the old function; their surrounding code (sort, density budget, map back) stays.

**3.2 Recency as a tie-breaker (BUG-005).** Remove the additive boost: `final_score = if is_guaranteed { 1.0 } else { res.score }`. Add one comparator in `pipeline.rs`, `compare_by_relevance(a, b)`: score descending; on an exact score tie, newer `last_modified` first (parsed as RFC 3339; missing or unparseable sorts last); then the existing stable order. It decides order in three places:

- a new sort after assembling the fused candidates (today they keep the RRF output order), before `min_score` and the top-30 re-ranking cut, so recency can only break ties at that boundary, never lift a chunk past a better one;
- the final sort in `search_single_async`;
- the merged-candidate sort in `retrieve_hybrid_batch_async`.

Exact ties do occur in RRF (a chunk at ranks (i, j) and another at (j, i), or two chunks at the same rank of a single list in different sub-queries) and between guaranteed file-name matches (all 1.0), which is where newer versions should win, keeping ADR-0116's intent.

**3.3 Deep Search minimum score (BUG-006).** Add `pub const RRF_MIN_SCORE: f64 = 0.005` to the core (`retrieval/pipeline.rs`, re-exported), documented as the RRF floor that keeps every candidate of a pool of up to 140 per list (1/(60 + 140) = 0.005); it is a guard against empty matches, not a relevance cutoff (a calibrated cutoff on cross-encoder scores is IMP-007). The three adapters in `actx-cli/src/engine.rs` use the constant in place of their literals; Deep Search moves from 0.02 to 0.005, the other two keep their value. To make the Deep Search request testable, its construction moves from the `retrieve_batch` closure into a small function `deep_search_request(query, workspace, rerank_mode) -> HybridSearchRequest` in the same file.

**Deliberately left out:**

- Score scale and `rank` attribute (ADJ-005), cross-encoder cutoff (IMP-007), retuning `max_chunks_per_source` (revisit after Frente D and IMP-008, as recorded in BUG-004).
- Temporal query handling (`QueryPreprocessor` temporal clauses and the LanceDB temporal filter) is unchanged; questions that ask for recent content keep using it.

**ADR**: ADR-0139 "Retrieval order follows relevance: score-ordered per-source cap and recency tie-break", superseding the round-robin part of ADR-0076 and the micro-boost part of ADR-0116, and recording `RRF_MIN_SCORE` as an empty-match guard on the RRF scale. Alternatives recorded: keep round-robin and re-sort afterwards (still lets a weak chunk displace a strong one); hard cap (empties slots in single-source workspaces); multiplicative or smaller recency boost (still a ranking signal with no calibration, since IMP-008 does not exist yet); remove recency entirely (loses the newer-version tie-break of ADR-0116).

## 4. Affected surfaces and files

No change to the `cli` or `tui` surfaces, commands, settings, file formats or stored data. Only the order and selection of retrieved chunks change.

| File | Change |
|---|---|
| `crates/any-context-core-rs/src/retrieval/diversifier.rs` | `apply_source_cap` replaces `apply_source_diversification`; unit tests. |
| `crates/any-context-core-rs/src/retrieval/mod.rs` | Re-export `apply_source_cap`. |
| `crates/any-context-core-rs/src/retrieval/pipeline.rs` | Remove the recency boost, add `compare_by_relevance`, use it in the three sorts, call `apply_source_cap` in both paths; tests. |
| `docs/architecture/retrieval.md` | Steps 7 and 10, batch search paragraph, module table, flowchart, decisions list. |
| `docs/adr/0139-...md` and the ADR index | New ADR; ADR-0076 and ADR-0116 get a note pointing to it. |
| `crates/any-context-core-rs/src/retrieval/mod.rs` (again) | Re-export `RRF_MIN_SCORE`. |
| `crates/actx-cli/src/engine.rs` | `deep_search_request` extracted, the three adapters use `RRF_MIN_SCORE`; test. |
| `docs/BACKLOG.md` | BUG-004, BUG-005 and BUG-006 follow the plan status. |

Other places with the same defect: none found. `apply_threshold` and `apply_density_budget` preserve order; `ContextFormatter` is correct given a score-ordered input.

## 5. Tests (`quality-assurance`)

Regression tests, written first and failing on the current code:

- `diversifier.rs`:
  - `cap_keeps_score_order`: the round-robin fixture above now gives `[a1, a2, b1, b2, c1]` (cap 2, `top_k` 5), descending scores.
  - `weak_other_source_does_not_displace_strong_second_chunk`: `a1 0.95, a2 0.94, b1 0.30`, cap 3, `top_k` 2 gives `[a1, a2]`.
  - `cap_limits_a_dominant_source_in_favor_of_others`: four chunks of A above one of B, cap 3, `top_k` 4 gives `[a1, a2, a3, b1]`.
  - `flexible_cap_backfills_in_score_order`: single source with six chunks, cap 3, `top_k` 6 gives all six in score order; a mixed case where a backfilled chunk lands between others by score, not at the end.
  - `zero_cap_means_no_cap` and the empty input.
- `pipeline.rs`:
  - `compare_by_relevance` unit tests: higher score wins regardless of date; equal scores, newer first; missing or invalid dates last.
  - `search_single_returns_descending_scores_across_sources`: BM25-only pipeline (no embedding provider, as in the existing tests) with several files and several matching chunks each; assert the output is in non-increasing score order and no source exceeds the cap while others are available.
  - `recency_does_not_outrank_relevance`: two BM25-only chunks in different files, the better match with an old `last_modified` and the weaker with today's date; the better match comes first and the scores equal the plain RRF scores.
  - `batch_returns_descending_scores`: same assertion for `retrieve_hybrid_batch`.
  - `batch_keeps_single_list_hits_at_rrf_min_score`: BM25-only chunk (no embedding provider, so every hit is single-list) in a batch request with `min_score: RRF_MIN_SCORE` is returned; `rrf_min_score_keeps_whole_pool`: `1/(60 + 140) >= RRF_MIN_SCORE`.
- `actx-cli/src/engine.rs`:
  - `deep_search_request_keeps_single_list_hits`: the request built for Deep Search has `min_score <= 1/(rrf_k + candidate_pool_k)` (fails with 0.02).
- Existing `u_shaped_reorder` and `format_xml_chunks` tests stay as they are.

Full run: `cargo test --workspace` green before leaving Phase 5.

Manual tests: no new test. MT-ANS-001 (strict grounding) and MT-MOD-001 (re-ranker ordering) already exercise answers from several sources; their steps do not depend on the old order, so they stay unchanged. Re-checked during the next release candidate run.

## 6. Documentation (`architecture-docs`, `user-docs`)

- `docs/architecture/retrieval.md`, edited in place: step 7 without the micro-boost; step 8 and the request parameters table with `RRF_MIN_SCORE` (Deep Search 0.005); step 10 describing the per-source cap in score order with backfill; ordering rule (score, then recency on ties) stated once; batch search paragraph; module table entry for `diversifier.rs`; flowchart labels (`RRF k=60`, `Per-source cap`); ADR-0139 in the decisions list. Run `build_index.py` and `check_docs.py` (zero errors).
- ADR-0139 as described in section 3; short supersession notes in ADR-0076 and ADR-0116.
- README: no user-visible behavior is documented at this level; `check_readme.py` only to confirm.

## 7. Release, migration and rollback (`devops-release`)

- **Version**: patch, ships in the next release (v0.45.1 line). Release notes: "Retrieved context now follows relevance order: the per-source limit no longer pushes a weaker chunk ahead of a stronger one, file recency only breaks exact ties, and Deep Search no longer discards chunks found by only one of the dense and keyword searches."
- **Data**: nothing stored changes; no reindex or migration. The fix applies to the next question.
- **Rollback**: revert the merge commit; no data to restore.
- **Observability**: no new logs; ranking is covered by the automated tests.
- **R1 (resolved at approval, 2026-10-17)**: removing the boost (BUG-005) alone would make Deep Search drop every single-list hit, because of its `min_score: 0.02` (BUG-006). BUG-006 was added to this plan (option a), so both ship together.
- **R2**: Deep Search now keeps more, weaker candidates before its `top_k` (8) cut; the cross-encoder still orders them per sub-query, and the per-source cap and density budget still bound the context. Calibrated pruning is IMP-007.

## 8. Execution steps and milestones

Branch: `fix/retrieval-relevance-order` from `dev`; pull request into `dev`.

**M1 Ranking fix**
1. Write the regression tests of section 5 and confirm they fail.
2. Implement `apply_source_cap` and switch both call sites; update the re-export.
3. Remove the recency boost, add `compare_by_relevance` and use it in the three sorts.
3b. Add `RRF_MIN_SCORE`, extract `deep_search_request` and use the constant in the three adapters.
4. `cargo test --workspace` green; `cargo clippy` clean on the core crate.

**M2 Docs and ADR**
5. ADR-0139, notes in ADR-0076 and ADR-0116, `retrieval.md`; `build_index.py`, `check_docs.py`.

**M3 Review and PR**
6. Review the diff (code review, `software-design` checklist, `check_design.py`).
7. Commit (Conventional Commits), push, open the pull request into `dev`, follow CI to green; the user merges.
8. On merge: complete the plan, which closes BUG-004, BUG-005 and BUG-006 as done in the backlog.
