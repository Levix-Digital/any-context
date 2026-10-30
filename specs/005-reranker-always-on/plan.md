# Implementation Plan: Reranker always on

**Branch**: `feat/005-reranker-always-on` | **Date**: 2026-10-30 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/005-reranker-always-on/spec.md`

## Summary

The cross-encoder becomes the only relevance judge and cannot be turned off: `rerank_mode` disappears from the core, the CLI and the TUI, together with `/rerank` and `--rerank`. After the reranking pass only scored candidates (plus pinned file-name matches) continue; the unscored tail is discarded. When the reranker cannot run, the fusion order is kept and the search reports a reranker outcome that the answer shows as a warning, next to the dense search one. Deep's per-chunk LLM yes/no is removed. The reranker's status moves to `/status`.

## Technical Context

**Language/Version**: Rust (stable, edition of the workspace)

**Primary Dependencies**: existing only; no new crate.

**Storage**: none changed; the `rerank_mode` setting is no longer read (left in place, no migration).

**Testing**: `cargo test --workspace`; `actx-bench` lexical and dense runs in CI against the committed baselines.

**Target Platform**: Windows, Linux, macOS desktop; `ubuntu-latest` in CI

**Project Type**: desktop CLI/TUI application with a Rust core library

**Performance Goals**: none changed (same depth, same pass); Deep loses the optional per-chunk LLM calls.

**Constraints**: no reindex; benchmark within the regression threshold.

**Scale/Scope**: `retrieval/pipeline.rs` and `requests.rs`, the command engine and models, `actx-cli` (engine, args, oneshot, TUI), `actx-agent` (Deep), `actx-bench` runner.

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| I. Design: modularity, explicit errors, testability | The reranker outcome is an explicit value in `RetrievalOutput` with its own notice, tested apart; no silent fallback | Pass |
| II. Simplicity First | One code path (no mode branches); code removed in core, CLI, TUI and agent | Pass |
| III. Hexagonal | Outcome and notice in the core; the CLI only shows them | Pass |
| IV.2 Evidence | Pipeline tests with a fixture reranker (tail discarded, unavailable outcome), command and CLI tests updated, agent tests | Pass |
| IV.3 Documentation describes today | `retrieval.md`, `commands.md`, `cli.md`, `tui.md`, `agent.md`, README edited in place; ADR-0157 | Pass |
| IV.6 Security | No new input; no data leaves the machine | Pass |
| IV.7 Bugs first | Backlog has no open bugs | Pass |
| IV.9 No AI attribution | Plan, commits and PR authored by the repository identity | Pass |

## Design

1. **Reranker outcome** (`retrieval/pipeline.rs`): `RerankOutcome { Scored, NotNeeded, Unavailable(String) }` with `answer_notice()` ("Answered without relevance ranking: <cause>. <fix>"; fix: `actx download reranker` for missing weights, reinstall for a missing runtime, `/status` otherwise). `RetrievalOutput` gains `rerank: RerankOutcome` and a `notices()` helper that returns the dense notice and the reranker notice in that order. The last outcome is also kept process-wide for `/status`, like the dense one.
2. **Pass** (`fuse_and_rerank`): always reranks; `should_rerank` and `rerank_mode` (pipeline field, setter, getter, request field) are removed. `neural_rerank_scores` returns `Result<Vec<Vec<f32>>, String>` with the cause (not neural-active with the reason from the reranker: runtime missing, weights missing or the recorded load error; scoring error; wrong count). `apply_neural_rerank` sets the scores of the picks and returns the outcome; on `Scored` the pipeline drops every unpinned candidate that was not picked; on `Unavailable` it keeps the fusion order. No pipeline reranker → `Unavailable("no reranker is configured")`. No picks → `NotNeeded`. The batch path reports one outcome for the whole search.
3. **Requests** (`requests.rs`): `fast_request`, `deep_request` and `tool_request` lose their `rerank_mode` argument.
4. **Commands** (`commands/engine.rs`, `models.rs`, `actx-cli/src/commands/registry.rs`): `execute_rerank` and the `rerank`/`reranker` route and registry entry removed; `ExecutionContext::rerank_mode` and `CommandStateUpdates::rerank_mode` removed; `/status` gains `• Reranker: <model> — <state> │ depth <fast>/<deep> │ last pass <ms> ms for <n> candidates` (or `n/a`). `/download` help keeps the reranker entries.
5. **CLI** (`actx-cli`): `--rerank` removed from `args.rs`, `oneshot.rs` and `tui/mod.rs`; `--rerank-model` kept; the retrievers in `engine.rs` stop reading `rerank_mode` and pass both notices to the agent (joined by a newline where the agent takes one notice).
6. **TUI** (`tui/app.rs`, `ui.rs`): `rerank_mode` and `with_rerank_mode` removed; the badge is `⚡ ONNX: <model> (CPU)` when ready, else `⚠ Reranker unavailable` in the warning color.
7. **Deep** (`actx-agent/src/deep_search.rs`, `lib.rs`): `enable_reranker`, `rerank_score_threshold`, `build_rerank_prompt`, `parse_rerank_decision` and the per-chunk branch removed; every new chunk is accepted.
8. **Benchmark** (`actx-bench/src/runner.rs`): drops `set_rerank_mode("on")`; a reranker that is not `Scored` on a question with candidates is an environment error (exit 3), as the bench already requires a neural reranker.

## Project Structure

```text
specs/005-reranker-always-on/   spec.md, plan.md, research.md, tasks.md, checklists/requirements.md
crates/any-context-core-rs/src/retrieval/{pipeline.rs, requests.rs, mod.rs}
crates/any-context-core-rs/src/commands/{engine.rs, models.rs}
crates/actx-cli/src/{engine.rs, cli/args.rs, cli/oneshot.rs, commands/registry.rs, tui/mod.rs, tui/app.rs, tui/ui.rs}
crates/actx-agent/src/{deep_search.rs, lib.rs}
crates/actx-bench/src/runner.rs
tests: pipeline unit tests, crates/any-context-core-rs/tests/command_tests.rs, crates/actx-cli/tests/cli_tests.rs, crates/actx-agent/tests/deep_search_tests.rs
docs/adr/0157-*.md (amends ADR-0129), docs/architecture/{retrieval,commands,cli,tui,agent}.md, README.md, tests/MANUAL_TESTS.md
```

## Complexity Tracking

None.

## Dev Cycle Protocol

### Backlog items and release

IMP-043 (medium), release R1 v1.0.0 Personal. Chosen by Levi on 2026-10-30 after IMP-041 and IMP-042. On completion it is closed; the parts owned by other items (contextual header IMP-026, kit model IMP-054/IMP-055, summaries and SQL FEAT-061/FEAT-093/FEAT-092, bands IMP-044) are already in their descriptions, so nothing is lost.

### Tests (quality-assurance)

- Pipeline: more candidates than the depth → only pinned and scored remain, in score order (SC-002); fewer than the depth → none discarded; no reranker / not neural-active → fusion order, `Unavailable` with the cause; only pinned → `NotNeeded`; batch of two sub-questions → one outcome.
- `RerankOutcome::answer_notice` per cause; `RetrievalOutput::notices` with both outcomes.
- Commands: `/rerank` unknown; `/status` has the reranker line; command registry without `rerank`.
- CLI: `--rerank` rejected, `--rerank-model` parsed; TUI badge without off state.
- Agent: Deep makes no per-chunk chat call (mock LLM call count); existing Deep tests updated.
- Benchmark: lexical and dense runs within the 0.02 gate in CI.
- Manual tests (`tests/MANUAL_TESTS.md`): the `/rerank` test becomes a `/status` reranker test; a test with the reranker weights removed checks the warning at the end of the answer.

### Documentation (architecture-docs, user-docs)

- ADR-0157 "The cross-encoder is always on, discards what it does not score and reports when it cannot run" (amends ADR-0129, ADR-0156).
- `docs/architecture/retrieval.md`, `commands.md`, `cli.md`, `tui.md`, `agent.md` edited in place; TECDOC index regenerated; README: `/rerank` and `--rerank` rows removed, troubleshooting row points to `/status`.

### CI, release and observability (devops-release)

- No workflow change; the retrieval benchmark runs on these paths.
- Version impact: behavior inside R1 v1.0.0; no bump in this PR. A user-visible change for release notes: `/rerank` and `--rerank` are gone.
- Observability: `/status` reranker line and the answer warning.

### Security, migration and rollback

- No new input reaches a query; no data migration (the old setting is ignored). Rollback is a revert of the PR.

## Milestones

1. **M1 Core** (T001 to T005): outcome, always-on pass, tail discarded, requests without mode.
2. **M2 Surfaces** (T006 to T011): commands, `/status`, CLI, TUI, Deep, benchmark.
3. **M3 Docs and checks** (T012 to T016): ADR-0157, pages, README, manual tests, backlog, full tests.
