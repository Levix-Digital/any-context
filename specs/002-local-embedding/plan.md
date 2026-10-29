# Implementation Plan: Local multilingual embedding model, independent of the chat model

**Branch**: `feat/002-local-embedding` | **Date**: 2026-10-29 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/002-local-embedding/spec.md`

## Summary

AnyContext gets a local, multilingual embedding model that runs on ONNX Runtime and is the default of every local profile (FEAT-067). The embedding model becomes an application-wide setting, separate from the chat model, and every chunk records the vector space that produced it (IMP-027). The resolution never falls back to another model or to fake vectors (IMP-028). The catalog declares each model's query and document prefixes and both sides apply them (IMP-029). The development benchmark measures the three candidates (multilingual-e5-small, multilingual-e5-base, BGE-M3), the profile defaults follow the numbers, and CI runs the benchmark with the dense retriever on.

Technical approach (see [research.md](research.md)): `OnnxEmbedder` in `actx-onnx`, built like `OnnxReranker`, behind the existing `LmProvider::embed` port; the catalog gains an `Embedding` category with dimension, pooling and prefixes; `EmbeddingTarget` gains dimension and prefix revision in its key; a one-time migration adopts existing cloud indexes; a nullable `embedding_target` column on chunks filters dense search.

## Technical Context

**Language/Version**: Rust (stable, edition of the workspace)

**Primary Dependencies**: `ort` 2.0.0-rc.14 (`load-dynamic`) and `tokenizers`, both already in `actx-onnx`; `lancedb`/`arrow` (existing); `rusqlite` (existing). No new third-party crate.

**Storage**: `settings.db` (new settings keys), LanceDB `workspace_chunks` (new nullable column), model files under the models directory (verified download)

**Testing**: `cargo test --workspace` (unit and integration); local embedder tests gated on the model files being present (downloaded once in CI and cached); `actx-bench` runs for SC-002 and SC-005

**Target Platform**: Windows, Linux, macOS desktop; `ubuntu-latest` in CI

**Project Type**: desktop CLI/TUI application with a Rust core library

**Performance Goals**: indexing the benchmark corpus with the reference default under 5 minutes on a CI runner (SC-006); question embedding latency reported by the benchmark

**Constraints**: no network during sync or query except the one-time verified model download; no silent model switch; INT8 models within each profile's disk and RAM budget

**Scale/Scope**: three local models, three cloud models, five profiles, one new setting, one new chunk column

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Check | Result |
|---|---|---|
| I. Design: modularity, explicit errors, testability | `OnnxEmbedder` is one unit (tokenize, run, pool, normalize); resolution returns `Result<Option<_>, EmbeddingUnavailable>` with model, cause and fix; pure parts (key parse, prefixes, migration choice, pooling) unit-tested | Pass |
| II. Simplicity First | No new port: the local model plugs into `LmProvider::embed`; per-target tables and file-by-file re-embedding deferred (IMP-033, IMP-040); batches unchanged (IMP-030) | Pass |
| III. Hexagonal | ONNX code stays in the `actx-onnx` adapter; the core holds the catalog data, the resolution and the migration; CLI/TUI only call core commands; `clap` stays out of the core | Pass |
| IV.2 Evidence | Reference-vector test against a Python `onnxruntime` computation; regression tests for no fallback and chat-model independence; dense CI benchmark with its own baseline | Pass |
| IV.3 Documentation describes today | ADR-0154 supersedes the derivation part of ADR-0132/ADR-0138; ingestion and retrieval architecture pages and README updated in place | Pass |
| IV.6 Security | Models only through the verified download (SHA-256); cloud model choice states that text leaves the machine; no secrets in Git | Pass |
| IV.7 Bugs first | Backlog has no open bugs | Pass |
| IV.9 No AI attribution | Plan, commits and PR authored by the repository identity | Pass |

Post-design re-check: unchanged, no violations, Complexity Tracking empty.

## Project Structure

### Documentation (this feature)

```text
specs/002-local-embedding/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/embedding-cli.md
├── checklists/requirements.md
└── tasks.md
```

### Source Code (repository root)

```text
crates/actx-onnx/src/
├── embedder.rs                         # NEW: OnnxEmbedder (load, batch, pool, normalize)
└── lib.rs                              # export
crates/any-context-core-rs/src/
├── ingestion/model_manager.rs          # Embedding category, three local entries
├── ingestion/embedding_catalog.rs      # NEW: EmbeddingModelSpec (local + cloud), prefixes, lookup
├── ingestion/embedding.rs              # EmbeddingTarget v2 key, resolve_embedding without fallback,
│                                       #   EmbeddingUnavailable, query-side rule, prefixes helper
├── ingestion/embedding_migration.rs    # NEW: one-time adoption + chunk backfill (ported from 4042b89)
├── ingestion/local_embedding.rs        # NEW: LmProvider adapter over OnnxEmbedder
├── ingestion/orchestrator.rs           # uses resolve_embedding, document prefix, records target per chunk
├── hardware/mod.rs                     # default_embedding_model() per profile, kit lists it
├── storage/lancedb.rs                  # embedding_target column, filter in search_vector
├── storage/sqlite.rs                   # embedding_model setting helpers
├── retrieval/pipeline.rs               # query prefix, target filter, no fallback on query side
└── commands/                           # embedding show/set command (shared by CLI and TUI)
crates/actx-cli/src/
├── commands/registry.rs, tui/...       # /embedding
├── cli/args.rs, cli/oneshot.rs         # actx embedding [id] [--yes]; -m no longer touches embeddings
└── engine.rs                           # query embedding resolution via the core
crates/actx-bench/src/runner.rs         # --embedding local/<id>
benchmarks/retrieval/baseline.dense-local-<id>-rerank.json   # NEW
.github/workflows/retrieval-benchmark.yml                    # dense job, model cache
docs/adr/0154-...md, docs/architecture/ingestion*.md, docs/architecture/retrieval.md, README.md,
tests/MANUAL_TESTS.md, TECDOC.md, .specify/memory/constitution.md
```

**Structure Decision**: the ONNX execution stays in the adapter crate; the core owns the decisions (catalog, identity, resolution, migration) so the CLI, the TUI and the benchmark share them.

## Complexity Tracking

None.

## Dev Cycle Protocol

### Backlog items and release

FEAT-067, IMP-027, IMP-028, IMP-029 (all medium), release R1 v1.0.0 Personal. Order decided with Levi on 2026-10-25 and confirmed on 2026-10-29: after IMP-065, before IMP-040. IMP-028 added to the scope on 2026-10-29. IMP-004 (superseded) is the origin of the ported M1 work; branch `feat/imp-004-embedding-model` can be deleted after this plan merges.

### Tests (quality-assurance)

- `actx-onnx`: pooling (mean with mask, CLS) and normalization on hand-built tensors; `OnnxEmbedder` against committed reference vectors for six pt/en sentences (cosine ≥ 0.9999), skipped with a message when the model files are absent locally, required in CI; batch order preserved; truncation counted; missing or corrupt files give an error, never a vector.
- Core: target key round trip and legacy parsing; prefix application per role; `resolve_embedding` cases (setting present and buildable, missing key, missing file, setting absent with each profile, `Lite`) never returning another model or the mock; migration (adopt most used real target, ignore mock, idempotent, backfill); orchestrator regression tests: chat-model change re-embeds zero chunks (SC-003), unavailable model writes zero chunks (SC-004), failed files retried; LanceDB filter keeps spaces apart; pipeline query prefix and no fallback.
- CLI: `/embedding` and `actx embedding` output, unknown id, confirmation, `--yes`; `-m` does not change the embedding.
- Benchmark: dense configuration with each candidate; a swapped-prefix run fails the gate (SC-005, checked once and recorded in the PR).
- Manual tests (`tests/MANUAL_TESTS.md`, section "Embedding model"): fresh install without keys finds a cross-language answer; `/embedding` switch with the reindex notice; cloud choice notice; offline first sync with a missing model shows the install message; `Lite` lexical-only notice.
- Acceptance: SC-001 to SC-006.

### Documentation (architecture-docs, user-docs)

- ADR-0154 "A local multilingual ONNX embedding model, chosen apart from the chat model, is the default": supersedes the derivation from the chat model (ADR-0132) and amends ADR-0138 (key format, no fallback on the query side). Then `constitution.py generate` in the same PR.
- `docs/architecture/` ingestion and retrieval pages: embedding section edited in place (catalog, resolution, prefixes, per-chunk target, migration).
- `docs/architecture/retrieval.md`, "Benchmark": the dense configuration and its baseline.
- README: "Configuration" section gains the embedding model (local by default, `/embedding`, cloud by choice).
- TECDOC index regenerated; backlog items closed in the completion PR.

### CI, release and observability (devops-release)

- `retrieval-benchmark.yml`: a dense job with the reference default, its baseline and the 0.02 gate; candidate models cached by the catalog file; a `workflow_dispatch` input to run any candidate (used once for R11). The lexical-only job stays.
- `e2e-tests.yml`/unit tests: the reference model is cached and provided so the reference-vector test runs.
- Version impact: minor (new user-facing setting and behavior) inside R1 v1.0.0; no separate bump in this PR.
- Observability: the sync summary names the embedding model and the count of truncated inputs; `EmbeddingUnavailable` is logged with model, cause and fix; diagnostics (`actx --diagnostics`) shows the embedding model, its source (setting or profile) and whether it is installed.

### Security, migration and rollback

- Models only through the verified download; a SHA-256 mismatch never loads.
- Cloud embedding only by explicit choice, with the data notice.
- Migration is additive (new setting keys, nullable column) and idempotent; a rollback to the previous version ignores the new column and keys and keeps working on the old resolver, which would reindex workspaces whose recorded key it does not know (the new-format keys); the release notes state this.
- Existing Gemini indexes are reindexed once because of the new prefixes; stated in the release notes.

## Milestones

Taken from the phases of [tasks.md](tasks.md):

1. **M1 Catalog and identity** (Phases 1 and 2, T001 to T008): embedding catalog with prefixes, target key v2 with legacy parsing, profile defaults, LanceDB column and filter, setting helpers.
2. **M2 Local embedder** (Phase 3, T009 to T014): `OnnxEmbedder`, reference vectors, `LmProvider` adapter, verified download of the local models.
3. **M3 Setting, resolution and migration** (Phases 4 and 5, T015 to T026): resolution without fallback, orchestrator and pipeline wiring, migration, `/embedding` and `actx embedding`, `-m` decoupled.
4. **M4 Prefixes** (Phase 6, T027 to T030): document and query prefixes applied, reindex on prefix change.
5. **M5 Benchmark, defaults, CI and docs** (Phases 7 and 8, T031 to T042): candidate runs, defaults, dense CI job and baseline, ADR-0154, docs, manual tests, README.
