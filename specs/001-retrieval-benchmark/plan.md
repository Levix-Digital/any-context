# Implementation Plan: Development retrieval benchmark on a fixed corpus

**Branch**: `feat/001-retrieval-benchmark` | **Date**: 2026-10-25 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/001-retrieval-benchmark/spec.md`

## Summary

A development-only benchmark (IMP-065) that indexes a fixed, fictitious pt/en corpus with the product's real ingestion into a temp directory, runs a hand-labeled question file through the product's real Fast retrieval request, and reports HitRate@1/3/5/10, MRR@10, NDCG@10, passage hits, the nothing-found rates and latency. Results are JSON, compared with a committed baseline; a drop of more than 0.02 in MRR@10 or HitRate@5 fails the run. CI runs it (lexical + reranker, no dense until FEAT-067) when retrieval, ingestion or storage code changes.

Technical approach (see [research.md](research.md)): a new workspace crate `actx-bench`; the Fast and Deep request builders move from `actx-cli/src/engine.rs` into the core so that the CLI and the benchmark share the defaults; a new CI workflow with a path filter and a cached reranker.

## Technical Context

**Language/Version**: Rust (stable, edition of the workspace)

**Primary Dependencies**: `any-context-core-rs` (ingestion, storage, retrieval), `actx-onnx` (reranker), `serde`/`serde_json`, `sha2`, `clap` (already used by `actx-cli`), `tokio`. No new third-party crate.

**Storage**: temp directory per run (`settings.db`, `lancedb`); JSON files under `benchmarks/retrieval/`

**Testing**: `cargo test -p actx-bench` (metrics, validation, comparison, isolation); the benchmark run itself in the new CI workflow

**Target Platform**: developer machines (Windows, Linux, macOS) and `ubuntu-latest` in CI

**Project Type**: development tool (workspace binary) over the core library

**Performance Goals**: a CI run (indexing plus about 70 questions, two passes, no dense) under 5 minutes excluding the first model download (SC-001)

**Constraints**: no provider keys or network in CI except the verified reranker download; never touches the user's settings, workspaces or keys; deterministic quality metrics

**Scale/Scope**: 30 to 40 documents, 60 to 80 questions (about 15 negative, at least 20 per language)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Check | Result |
|---|---|---|
| I. Design: modularity, explicit errors, testability | The pure parts (question file, validation, metrics, comparison) are plain functions with unit tests; the runner is the only part with I/O; every failure has its own exit code and message | Pass |
| II. Simplicity First | No new trait or port: the benchmark calls the existing `NativeSyncOrchestrator` and `NativeHybridPipeline`. The only refactor (request builders into the core) removes a duplication the benchmark would otherwise copy | Pass |
| III. Hexagonal | `actx-bench` is a driving adapter with its own composition root, like `actx-cli`; the core gains `retrieval::requests` (plain functions, no adapter types); `clap` stays out of the core (`forbidden_core_imports`) | Pass |
| IV.2 Evidence | Unit tests for every metric and rule; the CI workflow is the automated proof of the gate; SC-003 checked once by a deliberate regression | Pass |
| IV.3 Documentation describes today | `docs/architecture/retrieval.md` gains a "Benchmark" section; ADR-0153 records the decision | Pass |
| IV.6 Security | No secrets in Git; `--embedding` keys only from the environment into the temp settings, deleted with the temp directory | Pass |
| IV.9 No AI attribution | Plan, commits and PR authored by the repository identity | Pass |

Post-design re-check: unchanged, no violations, Complexity Tracking empty.

## Project Structure

### Documentation (this feature)

```text
specs/001-retrieval-benchmark/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/actx-bench-cli.md
├── checklists/requirements.md
└── tasks.md
```

### Source Code (repository root)

```text
Cargo.toml                                   # + crates/actx-bench member
crates/any-context-core-rs/src/retrieval/
├── requests.rs                              # NEW: fast_request, deep_request (moved from actx-cli)
└── mod.rs                                   # re-export
crates/actx-cli/src/engine.rs                # uses retrieval::requests (no behavior change)
crates/actx-bench/
├── Cargo.toml                               # publish = false
└── src/
    ├── main.rs                              # clap commands, exit codes, composition root
    ├── questions.rs                         # question file model + validation
    ├── corpus.rs                            # corpus walk, version hash, extracted text for passages
    ├── metrics.rs                           # HitRate, MRR, NDCG, nothing-found, latency
    ├── compare.rs                           # comparability, deltas, gate, Markdown summary
    └── runner.rs                            # temp env, indexing, ranking and delivered passes
benchmarks/retrieval/
├── corpus/pt/..., corpus/en/..., corpus/forms/cmr_single_pickup.pdf
├── questions.json
├── baseline.lexical-rerank.json
└── results/                                 # git-ignored
.github/workflows/retrieval-benchmark.yml    # NEW
docs/adr/0153-...md, docs/architecture/retrieval.md, TECDOC.md, .specify/memory/constitution.md
```

**Structure Decision**: a separate crate keeps the product build and the default `cargo test --workspace` free of model downloads; the core change is limited to one new module of request builders.

## Complexity Tracking

None.

## Dev Cycle Protocol

### Backlog items and release

IMP-065 (high), release R1 v1.0.0 Personal. Order decided with Levi on 2026-10-25: this plan, then FEAT-067 with IMP-027 and IMP-029 (model picked by this benchmark, dense turned on in CI), then IMP-040.

### Tests (quality-assurance)

- Unit tests (`actx-bench`): each metric on hand-computed cases (rank 1, rank 3, not found, graded NDCG, a document repeated in several chunks, fewer than 10 results); negatives excluded from ranking metrics; validation errors (missing document, missing passage, negative with labels, duplicate id); comparability rules; the 0.02 gate at, just under and just over the limit; latency never gating; rounding.
- Isolation test: a run with temp overrides leaves a sentinel default settings path untouched (FR-006).
- Core: unit test that `fast_request` and `deep_request` return today's numbers (6/30/14 000 and 8/40/16 000), so the move changes nothing; existing `actx-cli` tests keep passing.
- Determinism: `--check-determinism` in CI (SC-002); if it fails, a stable tie-break by chunk id is added to the pipeline with a unit test.
- SC-003 checked once during implementation (reranker off and pool 5) and recorded in the PR description.
- Manual tests: none; this is development tooling with no user-facing behavior (`tests/MANUAL_TESTS.md` unchanged).
- Acceptance: the spec's success criteria SC-001 to SC-006.

### Documentation (architecture-docs, user-docs)

- `docs/architecture/retrieval.md`: new "Benchmark" section (what it measures, the configurations, the gate, how to update the baseline), edited in place.
- ADR-0153 "A development retrieval benchmark gates retrieval changes": fixed corpus with a committed baseline and a 0.02 gate, over per-workspace evaluation now (FEAT-073 in v1.1.0) or no measurement. Then `constitution.py generate` in the same PR.
- `TECDOC.md` index regenerated. README unchanged (no end-user change).
- Backlog: IMP-065 closed in the completion PR.

### CI, release and observability (devops-release)

- New workflow `retrieval-benchmark.yml` (see research R10): path filter, `ubuntu-latest`, ONNX Runtime provisioned like `e2e-tests.yml`, reranker cached by SHA-256, `actx-bench retrieval run --compare ... --check-determinism --summary $GITHUB_STEP_SUMMARY`, result JSON uploaded as an artifact. Not a required check at first; Levi decides when to make it required.
- Version impact: none. No product binary, command, setting or data changes; the request-builder move is internal. No version bump.
- Observability: the run prints which retrievers and models it used and the reranker load time; no product telemetry.

### Security, migration and rollback

- Corpus is fictitious; no personal or customer data. The CMR PDF is already a repository fixture.
- No secrets: CI has none; a local `--embedding` run takes the key from the environment into the temp settings only.
- No data migration. Rollback: revert the PR; the product is unaffected.

## Milestones

Taken from the phases of [tasks.md](tasks.md):

1. **M1 Setup and shared request builders** (Phases 1 and 2, T001 to T007): crate skeleton, `retrieval::requests` in the core with the CLI switched to it, question model, corpus hash.
2. **M2 Measure** (Phase 3, T008 to T015): metrics, isolated runner, result JSON, `--embedding`, determinism check.
3. **M3 Compare and validate** (Phases 4 and 5, T016 to T020): comparison, gate, baseline handling, label validation.
4. **M4 Corpus and questions** (Phase 6, T021 to T025): fictitious pt/en corpus with distractors and the CMR, labeled questions, Levi's review.
5. **M5 Baseline, CI and docs** (Phases 7 and 8, T026 to T033): first baseline, workflow, SC-001 and SC-003 checks, ADR-0153, architecture page, TECDOC, constitution.
