# Implementation Plan: Pre-filtered dense and lexical retrievers

**Branch**: `feat/003-prefiltered-retrievers` | **Date**: 2026-10-30 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/003-prefiltered-retrievers/spec.md`

## Summary

Each retrieval request gets a filter (workspace plus optional source scopes, modification date range and content types) that both retrievers apply before taking their candidates: a LanceDB pre-filter for the dense one and a predicate checked before ranking for BM25. The two retrievers of a question start together instead of one after the other, and each list takes 30 candidates in Fast and in Deep. When the dense half does not run, the search reports why with its results, and the answer ends with one warning line. The parts of IMP-040 that depend on open items (per-target tables, child chunks, weighted fields, leaving `Global` out) stay with those items, as listed in the spec.

## Technical Context

**Language/Version**: Rust (stable, edition of the workspace)

**Primary Dependencies**: `lancedb` 0.39 (pre-filtered `only_if`), `tokio` (`join!`, `spawn_blocking`), `chrono`; no new crate.

**Storage**: LanceDB `workspace_chunks` and the BM25 file, read only; no schema or file format change.

**Testing**: `cargo test --workspace`; `actx-bench` lexical and dense runs against the committed baselines.

**Target Platform**: Windows, Linux, macOS desktop; `ubuntu-latest` in CI

**Project Type**: desktop CLI/TUI application with a Rust core library

**Performance Goals**: retrieval latency p95 of the benchmark not higher than the baseline; the per-question copy of the whole BM25 index removed.

**Constraints**: no reindex; results without a filter unchanged apart from the candidate count of Deep (40 to 30).

**Scale/Scope**: one new core type, two retrievers, two request builders, two agent ports, two front ends.

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| I. Design: modularity, explicit errors, testability | `RetrievalFilter` is one pure unit with two renderings (LanceDB predicate, lexical predicate), tested apart; the dense outcome is returned, not only logged | Pass |
| II. Simplicity First | No retriever trait or plugin layer: two functions inside the pipeline; per-target tables, child chunks, weighted fields and `Global` left to their items | Pass |
| III. Hexagonal | Filter and retrievers in the core; the agent ports gain an optional notice; CLI and TUI only render it | Pass |
| IV.2 Evidence | Unit tests per filter dimension and escaping, integration test with a real LanceDB table and BM25 index, benchmark runs saved | Pass |
| IV.3 Documentation describes today | `docs/architecture/retrieval.md` edited in place; ADR-0155 for filters inside the retrievers and the answer warning | Pass |
| IV.6 Security | Filter values are SQL literals (`'` doubled; prefixes through `starts_with`, so no wildcard); no new data leaves the machine | Pass |
| IV.7 Bugs first | Backlog has no open bugs | Pass |
| IV.9 No AI attribution | Plan, commits and PR authored by the repository identity | Pass |

## Design

1. **`retrieval/filter.rs` (new)**: `RetrievalFilter { sources: Vec<String>, modified_from: Option<DateTime<Utc>>, modified_to: Option<DateTime<Utc>>, content_types: Vec<String> }`, `Default` = no narrowing. `lance_predicate() -> Option<String>` builds `(file_path = 'x' OR starts_with(file_path, 'x/'))` per source, `last_modified >= '…' AND last_modified < '…'` (stored values are UTC RFC 3339, so string order is time order), `content_type IN (…)`; every value goes through one escaping function. `matches(file_path, content_type, last_modified) -> bool` is the same rule for the lexical side. Paths are compared with `/` separators on both sides.
2. **Request**: `HybridSearchRequest.filter: RetrievalFilter` (`#[serde(default)]`). The workspace stays in `workspace`; the `Global` rule is unchanged.
3. **Dense retriever**: the pipeline's dense step becomes `dense_list(&req, query_vec)`: the existing workspace, target and `Global` clauses plus `filter.lance_predicate()`, all in `only_if` (LanceDB pre-filters by default). The file-name and date metadata lookups also AND the filter.
4. **Lexical retriever**: `BM25Index::search` takes the filter and skips non-matching documents before scoring, as it does for the workspace today. `bm25_indices` holds `Arc<BM25Index>`, so a question no longer clones the whole index, and the search runs in `spawn_blocking`.
5. **Parallelism**: `search_single_async` starts the lexical search, then embeds the question and runs the dense search, and joins both (`tokio::join!`). Deep keeps running its sub-questions concurrently.
6. **Candidates**: `requests::CANDIDATES_PER_LIST = 30` used by `fast_request` and `deep_request`. The benchmark is run with 30 and 50; the numbers go to research.md and the PR.
7. **Dense outcome with the results**: `NativeHybridPipeline::search_with_outcome` and `retrieve_hybrid_batch_with_outcome` return `RetrievalOutput { results, dense: QueryEmbeddingOutcome }` (for a batch: the first outcome that is not `Dense`). The existing functions call them and drop the outcome. `QueryEmbeddingOutcome::answer_notice() -> Option<String>` gives the one warning line, with the fix (`actx embedding status`, or "sync to embed it").
8. **Agent ports**: `DeterministicRetrievalOutput::{ContextReady, ZeroContext}` gain `notice: Option<String>`; `DeepSearchRetriever` gains `fn last_notice(&self) -> Option<String> { None }`. The agent sends `AgentEvent::Notice(String)` once, before `Done`. TUI shows it as a dimmed line under the answer; the one-shot CLI prints it after the answer. Mocks keep compiling through the default method.

## Project Structure

```text
specs/003-prefiltered-retrievers/   spec.md, plan.md, research.md, tasks.md, checklists/requirements.md
crates/any-context-core-rs/src/retrieval/
├── filter.rs          # NEW: RetrievalFilter, escaping, LanceDB and lexical predicates
├── pipeline.rs        # request.filter, dense and lexical lists in parallel, RetrievalOutput, answer_notice
├── bm25.rs            # search with the filter
├── requests.rs        # CANDIDATES_PER_LIST = 30 for Fast and Deep
└── mod.rs             # exports
crates/actx-agent/src/ lib.rs, deep_search.rs, events.rs   # notice on both ports, AgentEvent::Notice
crates/actx-cli/src/   engine.rs (adapters), tui/app.rs, cli/oneshot.rs   # render the notice
crates/actx-bench/src/runner.rs                            # candidates option for the 30 vs 50 run
benchmarks/retrieval/baseline.*.json                       # refreshed
docs/adr/0155-*.md, docs/architecture/retrieval.md, docs/architecture/agent.md, tests/MANUAL_TESTS.md
```

## Complexity Tracking

None.

## Dev Cycle Protocol

### Backlog items and release

IMP-040 (medium), release R1 v1.0.0 Personal. Order: after IMP-065 and the local embedding plan (`feat_002_local_embedding`), as decided on 2026-10-25. On completion, the backlog entry of IMP-040 is closed and the parts left out are written into the descriptions of IMP-033, FEAT-036, IMP-026 and FEAT-082 so nothing is lost.

### Tests (quality-assurance)

- `filter.rs`: each dimension alone and combined; escaping of `'`, `%`, `_` and `\`; folder scope does not match a sibling prefix; open date ranges; inverted range matches nothing; missing date never matches; empty filter gives no predicate.
- `bm25.rs`: filtered search returns only matching documents and fills the limit from them.
- Pipeline integration (real LanceDB in a temp dir, as the existing pipeline tests): SC-001 scenario (small source with weak matches, large source with strong ones); empty result for a filter matching nothing; unfiltered results unchanged; lexical list present when the embedding provider fails, with the outcome returned.
- Agent: notice sent once before `Done` for Fast, nothing-found and Deep; no notice when dense ran (mock retrievers).
- CLI: one-shot output ends with the line; TUI renders it (existing app test pattern).
- Benchmark: lexical and dense runs within the 0.02 gate; latency compared; 30 vs 50 recorded.
- Manual tests (`tests/MANUAL_TESTS.md`, "Retrieval"): question with the embedding model removed shows the warning in the TUI and one-shot; with the model back, no warning.

### Documentation (architecture-docs, user-docs)

- ADR-0155 "Retrieval filters are applied inside each retriever, and the answer says when dense search did not run".
- `docs/architecture/retrieval.md` (search steps, filter, candidates, outcome) and `agent.md` (notice) edited in place; TECDOC index regenerated; README unchanged (no new user setting).

### CI, release and observability (devops-release)

- No workflow change: the retrieval benchmark already runs on these paths.
- Version impact: patch-level behavior inside R1 v1.0.0; no bump in this PR.
- Observability: the dense outcome keeps its log line and status line; the warning reaches the answer.

### Security, migration and rollback

- Filter values are escaped; unit tests cover injection attempts.
- No data migration: no schema or file format change. Rollback is a revert of the PR.

## Milestones

1. **M1 Filter** (T001 to T005): `RetrievalFilter`, predicates, request field, both retrievers filtered.
2. **M2 Parallel retrievers and candidates** (T006 to T009): shared BM25 index, `join!`, 30 per list, benchmark runs.
3. **M3 Dense outcome in the answer** (T010 to T015): `RetrievalOutput`, agent notice, TUI and one-shot.
4. **M4 Docs and baselines** (T016 to T020): ADR-0155, architecture pages, manual tests, baselines, backlog notes.
