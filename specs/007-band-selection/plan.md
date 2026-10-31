# Implementation Plan: Selection by relevance bands and the token budget

**Branch**: `feat/007-band-selection` | **Date**: 2026-10-31 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/007-band-selection/spec.md`

## Summary

The end of the search pass (`fuse_and_rerank`) stops using a fixed count, a per-file cap with backfill and a character ceiling. After the irrelevant cut of IMP-044, the candidates are already in score order (pinned matches, then relevant, then maybe); a new selection step drops near-identical candidates and admits the rest in order while their estimated tokens fit the request's token budget, skipping whole any chunk that does not fit. Each request builder sets its own budget. `RetrievalPreset`, `top_k`, `max_chunks_per_source`, `max_density_chars` and `apply_density_budget` go away; `apply_source_cap` stays only for the reranker input slots. The benchmark reports what the delivered pass selected and keeps gating its hit rate; the budgets are confirmed with it.

## Technical Context

**Language/Version**: Rust (stable, edition of the workspace)

**Primary Dependencies**: existing only; no new crate.

**Storage**: none changed.

**Testing**: `cargo test --workspace`; `actx-bench` lexical and dense runs in CI against the committed baselines; local runs to confirm the budgets.

**Target Platform**: Windows, Linux, macOS desktop; `ubuntu-latest` in CI

**Project Type**: desktop CLI/TUI application with a Rust core library

**Performance Goals**: none changed (the dedup compares at most a few dozen candidates).

**Constraints**: no reindex; ranking metrics unchanged; delivered hit rate within the regression threshold.

**Scale/Scope**: `retrieval/` (new `selection.rs`, `pipeline.rs`, `requests.rs`, `diversifier.rs`, `mod.rs`), `lib.rs` re-exports, `actx-bench` (runner, metrics, summary).

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| I. Design: modularity, explicit errors, testability | Selection is a pure function over scored candidates and a budget, tested apart from the pipeline | Pass |
| II. Simplicity First | One budget per mode, one ordered pass, one dedup rule; dead presets removed | Pass |
| III. Hexagonal | Selection in the core; surfaces only pick the request builder | Pass |
| IV.2 Evidence | Unit tests for selection and dedup, pipeline tests with a fixture reranker, benchmark runs | Pass |
| IV.3 Documentation describes today | `retrieval.md` and the benchmark page edited in place; ADR-0159 | Pass |
| IV.6 Security | No new input; no data leaves the machine | Pass |
| IV.7 Bugs first | Backlog has no open bugs | Pass |
| IV.9 No AI attribution | Plan, commits and PR authored by the repository identity | Pass |

## Design

1. **Selection** (new `retrieval/selection.rs`): `select(candidates, token_budget) -> Vec<index>` over items already in score order. For each candidate: skip it if it is a near-duplicate of one already admitted; admit it if it is the first or if its tokens fit in what is left; otherwise skip it and go on (a smaller one may still fit). Near-duplicate: equal normalized text (lowercase, whitespace collapsed), or Jaccard similarity of word 3-shingles at or above `NEAR_DUPLICATE_JACCARD = 0.9` (texts shorter than three words compare by normalized text only). A `Selection` summary (selected, tokens, skipped for budget, duplicates) is returned for the benchmark and the logs.
2. **Pipeline** (`retrieval/pipeline.rs`): `fuse_and_rerank` calls `selection::select` with `first.token_budget` on the sorted results, instead of `apply_source_cap` + `apply_density_budget`. The ordering is the existing `compare_by_relevance` (pinned at 1.0, then reranker score); with bands, score order is band order because the bands are monotonic in the score. `RetrievalOutput` gains `selection: SelectionCounts` for observability (the core has no logger; the counts reach the benchmark and callers).
3. **Request** (`HybridSearchRequest`): `token_budget: usize` (serde default: the Fast budget) replaces `top_k`, `max_chunks_per_source` and `max_density_chars`. `RetrievalPreset` and its re-exports are removed (only tests used it). `apply_density_budget` is removed; `apply_source_cap` stays for `rerank_picks`, now documented as the reranker input slots only.
4. **Budgets** (`retrieval/requests.rs`): `FAST_TOKEN_BUDGET = 4_000`, `DEEP_TOKEN_BUDGET = 6_000` (per batch; Deep runs at most two batches), `TOOL_TOKEN_BUDGET = 3_000`. Today's ceilings are 14, 16 and 12 thousand characters (about 3.5, 4 and 3 thousand tokens), so Fast keeps about the same room, Deep gets more, and both now fill it by relevance instead of by count. IMP-049 later bounds them by the model window.
5. **Benchmark** (`actx-bench`): the ranking request sets `token_budget = usize::MAX` and keeps its own top 10 (`DEPTH`) by truncating the ranked list it reads, instead of overriding `top_k`; the summary gains "delivered chunks (avg)" and "delivered tokens (avg)". Local runs with the mmarco reranker (lexical and dense) compare budgets of 3,000, 4,000 and 6,000 for Fast; the result goes to `research.md`, and the baselines are refreshed from the CI run because the delivered metrics change by design.

## Project Structure

```text
specs/007-band-selection/   spec.md, plan.md, research.md, tasks.md, checklists/requirements.md
crates/any-context-core-rs/src/retrieval/{selection.rs (new), pipeline.rs, requests.rs, diversifier.rs, mod.rs}
crates/any-context-core-rs/src/lib.rs
crates/actx-bench/src/{runner.rs, metrics.rs, summary/compare}
benchmarks/retrieval/baseline.*.json (refreshed from CI)
docs/adr/0159-*.md, docs/architecture/retrieval.md, benchmark page, tests/MANUAL_TESTS.md
```

## Complexity Tracking

None.

## Dev Cycle Protocol

### Backlog items and release

IMP-045 (medium), release R1 v1.0.0 Personal, step 7 of the retrieval design after IMP-044. On completion it is closed. "Current version only" moves to FEAT-027's acceptance (version families), since there is no version data to select on; the parent expansion stays with FEAT-098 and the model-window bound with IMP-049.

### Tests (quality-assurance)

- `selection.rs`: all relevant enter past any old count (US1.1); relevant then maybe up to the budget (US1.2); a chunk that does not fit is skipped whole and a smaller one enters (US1.3); an oversized first chunk enters alone (US1.4); no per-file limit (US2.1); exact and near duplicates dropped, different texts kept (US3); empty input and zero budget.
- Pipeline with a fixture reranker: pinned first, bands in order, budget respected, no "condensed" marker (SC-001, SC-004); reranker unavailable follows fused order within the budget (US5); a batch makes one selection.
- `requests.rs`: each builder's budget; no fixed count fields (SC-003 by compilation).
- Benchmark: request test updated, summary fields; CI lexical and dense runs gate the delivered hit rate (SC-002).
- Manual tests (`tests/MANUAL_TESTS.md`): a question answered by one long document cites several of its parts; a workspace with a copied file shows each passage once.

### Documentation (architecture-docs, user-docs)

- ADR-0159 "Selection admits chunks by relevance band within a token budget per mode, with no fixed count or per-file cap" (amends ADR-0139 on per-source caps).
- `docs/architecture/retrieval.md` (selection step), the benchmark page (delivered averages) edited in place; TECDOC index regenerated. No README change (no command or flag).

### CI, release and observability (devops-release)

- No workflow change; the benchmark runs on these paths and fails on a delivered hit rate drop.
- Version impact: behavior inside R1 v1.0.0; no bump in this PR. Release note: answers get every relevant excerpt that fits, without a fixed count or a per-file limit, and repeated passages once.
- Observability: selection counts (selected, tokens, budget skips, duplicates) in `RetrievalOutput::selection` and in the benchmark summary.

### Security, migration and rollback

- No new input; no data migration. Serialized requests with the old fields still deserialize (unknown fields ignored, budget defaulted). Rollback is a revert of the PR.

## Milestones

1. **M1 Core** (T001 to T005): selection and dedup, the request budget, the pipeline step, removal of the fixed counts.
2. **M2 Benchmark** (T006 to T008): delivered averages, local budget runs, budgets confirmed.
3. **M3 Docs and checks** (T009 to T013): ADR-0159, pages, manual tests, baselines, backlog, full tests.
