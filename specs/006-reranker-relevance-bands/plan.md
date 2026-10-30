# Implementation Plan: Reranker relevance bands

**Branch**: `feat/006-reranker-relevance-bands` | **Date**: 2026-10-30 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/006-reranker-relevance-bands/spec.md`

## Summary

Each reranker in the model catalog gets two calibrated thresholds. After a scored reranking pass, every scored candidate gets a band (relevant, maybe, irrelevant) and the irrelevant ones are discarded before the selection; pinned file-name matches and passes the reranker could not score are left as they are. The search output and `/status` report the band counts. The benchmark keeps its ranking pass without the cut, measures the cut on its delivered pass, can run with any catalog reranker, writes a calibration table and gates the delivered hit rate.

## Technical Context

**Language/Version**: Rust (stable, edition of the workspace)

**Primary Dependencies**: existing only; no new crate.

**Storage**: none changed; the thresholds are constants in the catalog.

**Testing**: `cargo test --workspace`; `actx-bench` lexical and dense runs in CI against the committed baselines; local calibration runs with both rerankers.

**Target Platform**: Windows, Linux, macOS desktop; `ubuntu-latest` in CI

**Project Type**: desktop CLI/TUI application with a Rust core library

**Performance Goals**: none changed (one comparison per scored candidate).

**Constraints**: no reindex; ranking metrics unchanged; delivered hit rate within the regression threshold.

**Scale/Scope**: `retrieval/` (new `relevance.rs`, `pipeline.rs`), `ingestion/model_manager.rs`, `commands/engine.rs` (`/status`), `actx-bench` (runner, metrics, compare, main).

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| I. Design: modularity, explicit errors, testability | Bands are a small pure module (`score -> band`), tested apart; the cut is one step of the pass | Pass |
| II. Simplicity First | Two constants per model, one enum, one retain; no per-workspace tuning yet | Pass |
| III. Hexagonal | Bands and thresholds in the core; `/status` only shows the counts | Pass |
| IV.2 Evidence | Unit tests for the mapping and the catalog, pipeline tests with a fixture reranker, benchmark calibration with both rerankers | Pass |
| IV.3 Documentation describes today | `retrieval.md`, `commands.md`, benchmark page edited in place; ADR-0158 | Pass |
| IV.6 Security | No new input; no data leaves the machine | Pass |
| IV.7 Bugs first | Backlog has no open bugs | Pass |
| IV.9 No AI attribution | Plan, commits and PR authored by the repository identity | Pass |

## Design

1. **Bands** (new `retrieval/relevance.rs`): `RelevanceBand { Relevant, Maybe, Irrelevant }`; `RelevanceThresholds { irrelevant_below: f32, relevant_from: f32 }` with `band(score)` (a score at a threshold takes the higher band); `BandCounts { relevant, maybe, irrelevant }`.
2. **Catalog** (`ingestion/model_manager.rs`): `OnnxModelSpec` gains `relevance: Option<RelevanceThresholds>`, set on the two rerankers and `None` elsewhere; `OnnxModelManager::relevance_thresholds(model_id)` looks it up. A test checks every `ReRanker` entry has valid thresholds.
3. **Pipeline** (`retrieval/pipeline.rs`): `HybridSearchResult` gains `relevance_band: Option<RelevanceBand>` (serde default). After `apply_neural_rerank` returns `Scored`, the pass looks up the thresholds of `reranker.model_id()`, sets the band of each scored candidate, removes the irrelevant ones and counts them. `RetrievalOutput` gains `bands: Option<BandCounts>` (`None` when nothing was banded), kept process-wide as the last pass, like the reranker outcome. `NativeHybridPipeline::set_relevance_cut(bool)` (default on) lets the benchmark band without discarding; it is not reachable from any command or flag.
4. **Status** (`commands/engine.rs`): the reranker line appends `│ bands <r> relevant / <m> maybe / <i> irrelevant` from the last pass when there is one.
5. **Benchmark** (`actx-bench`): `--reranker <id>` (default `mmarco-reranker`) for the loaded model and the result configuration; the ranking pass runs with the cut off, the delivered pass with it on; each question records the best score of the ranking pass for the calibration; the summary gets a calibration table (irrelevant thresholds 0.005, 0.01, 0.02, 0.03, 0.05 against negatives with nothing found and positives that lost their relevant document); `delivered_hit_rate` becomes a gated metric. The baselines are refreshed from the CI run, because the delivered metrics change by design.
6. **Calibration** (`research.md`): local runs with both rerankers in the lexical and the dense (bge-m3) configuration; the irrelevant threshold is the highest one that loses no relevant document the ranking found; the relevant threshold is placed above the best score of the negative questions.

Initial reading of the committed baselines (mmarco, revision 3be850d): an irrelevant threshold of 0.01 makes 5 to 8 of the 15 negatives come back empty with no relevant document lost in the dense run (one more lost in the lexical run); 0.02 makes 9 empty and loses one in each. The relevant threshold starts at 0.5 (the best negative scores 0.47). These numbers are confirmed with runs on the current code before they are fixed.

## Project Structure

```text
specs/006-reranker-relevance-bands/   spec.md, plan.md, research.md, tasks.md, checklists/requirements.md
crates/any-context-core-rs/src/retrieval/{relevance.rs (new), pipeline.rs, mod.rs}
crates/any-context-core-rs/src/ingestion/model_manager.rs
crates/any-context-core-rs/src/commands/engine.rs
crates/actx-bench/src/{main.rs, runner.rs, metrics.rs, compare.rs, result.rs}
benchmarks/retrieval/baseline.*.json (refreshed from CI)
tests: relevance and pipeline unit tests, catalog test, command_tests.rs (/status), actx-bench unit tests
docs/adr/0158-*.md, docs/architecture/{retrieval,commands}.md, benchmark page, tests/MANUAL_TESTS.md
```

## Complexity Tracking

None.

## Dev Cycle Protocol

### Backlog items and release

IMP-044 (medium), release R1 v1.0.0 Personal, the next step of the reranking design after IMP-043. On completion it is closed; the per-workspace refinement stays with IMP-008 and step 10, and the uses of the bands with IMP-045 and IMP-046 (already in their descriptions).

### Tests (quality-assurance)

- `relevance.rs`: band at, just below and just above each threshold; counts.
- Catalog: every reranker has thresholds with 0 < irrelevant < relevant < 1; a non-reranker has none; unknown id → `None`.
- Pipeline with a fixture reranker: irrelevant discarded and the rest in score order (SC-001); all irrelevant → empty; pinned kept without band; unavailable → no band, nothing discarded; unknown model → no band; cut off → banded but nothing discarded; batch of two sub-questions → one count.
- `/status`: band counts after a scored pass, none before.
- Benchmark: argument parsing, calibration table, `delivered_hit_rate` gated; CI lexical and dense runs.
- Manual tests (`tests/MANUAL_TESTS.md`): a question with no answer in the workspace shows the irrelevant count in `/status` and no unrelated excerpt in the answer.

### Documentation (architecture-docs, user-docs)

- ADR-0158 "Reranker scores are read as relevance bands with per-model thresholds, and irrelevant chunks are discarded" (amends ADR-0157).
- `docs/architecture/retrieval.md` (reranking step), `commands.md` (`/status`), the benchmark page (calibration, gated delivered hit rate) edited in place; TECDOC index regenerated. No README change (no new command or flag).

### CI, release and observability (devops-release)

- No workflow change; the benchmark runs on these paths and fails on a delivered hit rate drop.
- Version impact: behavior inside R1 v1.0.0; no bump in this PR. Release note: answers no longer receive excerpts the reranker judged irrelevant.
- Observability: band counts in `/status`.

### Security, migration and rollback

- No new input; no data migration. Rollback is a revert of the PR.

## Milestones

1. **M1 Core** (T001 to T005): bands, catalog thresholds, the cut in the pass, counts in the output.
2. **M2 Status and benchmark** (T006 to T009): `/status`, benchmark options and calibration, thresholds fixed from the runs.
3. **M3 Docs and checks** (T010 to T014): ADR-0158, pages, manual tests, baselines, backlog, full tests.
