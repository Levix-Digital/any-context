# Implementation Plan: Unified fusion and reranker depth

**Branch**: `feat/004-unified-fusion` | **Date**: 2026-10-30 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/004-unified-fusion/spec.md`

## Summary

A search (one question in Fast, all sub-questions in Deep) now has three stages in the pipeline instead of one function run per sub-question plus a score-summing merge: each question collects its clean ranked lists (dense per workspace, lexical, file-name matches, date matches); all lists of all questions enter one RRF (k = 60, equal weights, no score cut); one reranking pass scores at most `depth` fused candidates, at most 3 per document, each against the sub-question that ranked it best; the existing selection (per-file cap, top-k, character budget) runs once at the end. The depth comes from the mode and the hardware profile and is lowered by a time budget when the measured reranking cost on the machine requires it.

## Technical Context

**Language/Version**: Rust (stable, edition of the workspace)

**Primary Dependencies**: existing only (`tokio`, `futures`, `actx-onnx`); no new crate.

**Storage**: none changed; LanceDB and the BM25 file are read as today.

**Testing**: `cargo test --workspace`; `actx-bench` lexical and dense runs against the committed baselines.

**Target Platform**: Windows, Linux, macOS desktop; `ubuntu-latest` in CI

**Project Type**: desktop CLI/TUI application with a Rust core library

**Performance Goals**: Deep reranks each candidate once (today once per sub-question that kept it); Fast latency p95 of the benchmark not higher than the baseline.

**Constraints**: no reindex, no setting added; Fast results on the benchmark within the regression threshold.

**Scale/Scope**: the retrieval pipeline's fusion and reranking, the request builders, one new small core module, the agent tool request in `actx-cli`.

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| I. Design: modularity, explicit errors, testability | Fusion (`rrf.rs`), depth (`rerank_depth.rs`) and candidate choice (`diversifier::apply_source_cap`, reused) are pure functions tested apart; the pipeline only orchestrates | Pass |
| II. Simplicity First | Fast is a Deep batch of one: one code path instead of two; the existing per-source cap function is reused for the per-document cap; no weights plumbing until FEAT-105 needs it | Pass |
| III. Hexagonal | Everything in the core; the CLI only passes the profile-based requests it already builds | Pass |
| IV.2 Evidence | Unit tests per function, pipeline tests with a fixture reranker, benchmark runs saved | Pass |
| IV.3 Documentation describes today | `docs/architecture/retrieval.md` edited in place; ADR-0156 | Pass |
| IV.6 Security | No new input reaches a query; no data leaves the machine | Pass |
| IV.7 Bugs first | Backlog has no open bugs | Pass |
| IV.9 No AI attribution | Plan, commits and PR authored by the repository identity | Pass |

## Design

1. **`retrieval/rrf.rs`**: `RankedList { query: usize, ids: Vec<String> }` and `fuse(lists: &[RankedList], k: usize) -> Vec<FusedCandidate>` with `FusedCandidate { id, score, queries: Vec<usize>, best_query: usize }`: the RRF sum over every list, the questions whose lists contain it, and the question of its highest single contribution. Order: score descending, then id (deterministic). `reciprocal_rank_fusion` and `RRFScoreResult` are removed (only the pipeline used them).
2. **`retrieval/rerank_depth.rs` (new)**: `SearchMode { Fast, Deep }`; `profile_depth(ProfileTier, SearchMode)` (table in research R5); `time_budget_ms(SearchMode)` (1000 / 3000); `MIN_RERANK_DEPTH = 10`; `PER_DOCUMENT_RERANK_SLOTS = 3`; a process-wide cost per candidate (`AtomicU64`, microseconds, exponentially weighted with weight 0.5) with `record_rerank_cost(candidates, elapsed)` and `effective_depth(depth, budget_ms: Option<u64>) -> usize`. A pure `depth_within_budget(depth, budget_ms, cost_us)` carries the arithmetic so it is tested without the global.
3. **Request**: `HybridSearchRequest` loses `min_score`; gains `rerank_depth: usize` (default 30) and `rerank_budget_ms: Option<u64>` (default `None`). `RetrievalPreset` loses `min_score`; `RRF_MIN_SCORE` and `diversifier::apply_threshold` are removed.
4. **Request builders** (`retrieval/requests.rs`): `fast_request` and `deep_request` take the `ProfileTier` and set `rerank_depth = profile_depth(tier, mode)` and `rerank_budget_ms = Some(time_budget_ms(mode))`. A new `tool_request` replaces the request written inline in `actx-cli/src/engine.rs` (agent search tool: top 5, 12,000 characters, Fast depth). The CLI passes `ingestion::embedding::active_profile(db)` (made public if needed). The benchmark builds its requests with the standard profile (`SmartBalanced`) and `rerank_budget_ms = None`, so its depth is fixed at 30.
5. **Pipeline** (`retrieval/pipeline.rs`), `search_with_outcome` split in three private steps, and both public entry points use them (`search_with_outcome(req)` = the batch path with one request):
   - `collect_lists(req, query) -> QueryLists`: today's steps 1 to 8 (metadata lookups, embedding, dense per workspace, lexical in parallel), but each source keeps its own list ranked by its own score: dense per workspace, lexical, file-name matches (pinned) and date matches. It returns the lists, the stored rows found on the way (dense and metadata hits), the pinned ids and the dense outcome.
   - `fuse_and_rerank(requests, Vec<QueryLists>)`: `rrf::fuse` over every list; rows of lexical-only hits read from LanceDB in one lookup (as today); candidates built with the RRF score (pinned ones 1.0, `matched_subqueries` from `queries`); reranker input = unpinned candidates in fusion order through `apply_source_cap(…, PER_DOCUMENT_RERANK_SLOTS, effective_depth(first.rerank_depth, first.rerank_budget_ms))`; one `spawn_blocking` scores them grouped by `best_query` (one `score_batch` per sub-question, each candidate once), records the cost, and the unscored tail is rescaled below the scored ones as today.
   - selection: `apply_source_cap` (`max_chunks_per_source`, `top_k`) and `apply_density_budget` once, with the first request's values, as the batch does today. The condensed text of the last chunk is kept in both modes (Fast used to drop it and return the uncut text).
   - `retrieve_hybrid_batch_with_outcome` runs `collect_lists` of all sub-questions concurrently (`join_all`), then the same two steps; the score-summing deduplication is removed. Dense outcome: first that is not `Dense`, as today.
6. **`MAX_RERANK_CANDIDATES`** is removed; `apply_neural_rerank` takes the chosen candidate ids instead of taking the first 30.

## Project Structure

```text
specs/004-unified-fusion/   spec.md, plan.md, research.md, tasks.md, checklists/requirements.md
crates/any-context-core-rs/src/retrieval/
├── rrf.rs             # fuse over N lists with best question
├── rerank_depth.rs    # NEW: mode, profile depth, time budget, measured cost
├── pipeline.rs        # collect_lists, fuse_and_rerank, selection; min_score and score summing removed
├── diversifier.rs     # apply_threshold removed
├── requests.rs        # profile-based depth and budget; tool_request
└── mod.rs             # exports
crates/any-context-core-rs/src/ingestion/embedding.rs   # active_profile public
crates/actx-cli/src/engine.rs                           # builders with the active profile; tool_request
crates/actx-bench/src/runner.rs                         # fixed depth, no budget
benchmarks/retrieval/baseline.*.json                    # refreshed
docs/adr/0156-*.md, docs/architecture/retrieval.md, docs/architecture/retrieval-pipeline-v2.md (status note), tests/MANUAL_TESTS.md
```

## Complexity Tracking

None.

## Dev Cycle Protocol

### Backlog items and release

IMP-041 and IMP-042 (medium), release R1 v1.0.0 Personal. Chosen by Levi on 2026-10-30 after IMP-040. On completion both are closed; the per-workspace tuning goes into FEAT-105's description, the discarding of the tail stays in IMP-043, so nothing is lost.

### Tests (quality-assurance)

- `rrf.rs`: scores over 1, 2 and 5 lists; a chunk in lists of two questions has both questions and the best one by contribution; deterministic order on ties.
- `rerank_depth.rs`: every profile and mode; budget with a given cost (SC-004: 50 ms per candidate → 20 in Fast); floor of 10; no budget → profile depth; no measurement → profile depth; the weighted average.
- `requests.rs`: Fast and Deep builders per profile; tool request values.
- Pipeline (real LanceDB and BM25 in a temp dir, fixture reranker as in the existing tests): file-name matches as their own list and pinned; date list without invented scores; batch of two sub-questions → one pass, each candidate scored once, against its best question (counting reranker); a batch of one equals Fast; per-document cap with reserve (SC-002); no candidate dropped by RRF score.
- Agent and CLI: existing Deep and Fast tests keep passing with the new scores.
- Benchmark: lexical and dense runs within the 0.02 gate; latency compared; new baselines committed.
- Manual tests (`tests/MANUAL_TESTS.md`, "Retrieval"): a Deep question on a workspace with one long document and several short ones cites more than one document; `/status` after a question still shows the rerank latency.

### Documentation (architecture-docs, user-docs)

- ADR-0156 "One fusion of all lists per search, reranker depth by mode, profile and time budget".
- `docs/architecture/retrieval.md` (fusion, reranking, presets, Deep batch) edited in place; the "Today" paragraph of step 5 in `retrieval-pipeline-v2.md` marked as delivered; TECDOC index regenerated; README unchanged (no new user setting).

### CI, release and observability (devops-release)

- No workflow change: the retrieval benchmark already runs on these paths.
- Version impact: behavior inside R1 v1.0.0; no bump in this PR.
- Observability: `last_rerank_latency_ms` keeps working; a debug log line per search gives lists, fused candidates, depth and reranked count.

### Security, migration and rollback

- No new input reaches a query; no data migration. Rollback is a revert of the PR.

## Milestones

1. **M1 Fusion** (T001 to T006): clean lists, one RRF over all lists, no score cut, Deep without score summing.
2. **M2 Reranker input and depth** (T007 to T011): per-document slots, depth by mode and profile, time budget, builders.
3. **M3 Docs and baselines** (T012 to T016): benchmark, ADR-0156, architecture pages, manual tests, backlog.
