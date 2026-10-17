---
plan_id: anycontext_bugfix_v0451_query_embedding_model
project: any-context
title: Dense query embedding ignores the workspace embedding model
kind: bug
version: v0.45.1
status: in_progress
author: Levi Silveira
created_at: '2026-10-17T07:06:04'
updated_at: '2026-10-17T07:13:02'
session_id: null
tags: []
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- BUG-001
milestones:
- name: M1 Shared resolver
  done: false
- name: M2 Query embedder and pipeline
  done: false
- name: M3 Adapters
  done: false
- name: M4 Docs and manual test
  done: false
- name: M5 Release
  done: false
status_log:
- at: '2026-10-17T07:06:04'
  by: Levi Silveira
  status: draft
  note: Plan created
- at: '2026-10-17T07:08:23'
  by: Levi Silveira
  status: active
  note: Discovery and plan written; awaiting user approval (gate 3)
- at: '2026-10-17T07:13:02'
  by: Levi Silveira
  status: in_progress
  note: Approved by the user (gate 3); implementation started
---

# Dense query embedding ignores the workspace embedding model

Bug plan for patch `v0.45.1` (`dev-cycle-protocol` Phases 1 and 2). Discovery on 2026-10-17 against `dev` at `a552352`.

## Backlog items

### BUG-001: Dense query embedding ignores the workspace embedding model
- **Severity**: high
- **Found**: 2026-10-16, v0.45.0 (roadmap handoff, PR #328)
- **Description**: retrieval/pipeline.rs:536 always embeds the query with text-embedding-3-small, while ingestion resolves the model per workspace (ingestion/orchestrator.rs:198). A workspace indexed with another model gets query vectors in another space or dimension and dense search silently degrades or fails. Fix in Frente C or earlier as patch v0.45.1.

## 1. Reproduction

No log covers this path: the query-embedding failure is swallowed without a message (see §2), so the reproduction is by code reading and is confirmed by the regression test in §5.

1. Configure a workspace whose chat model is not an OpenAI model, for example `claude-…` with an Anthropic key (an OpenAI key may also be present), or `gemini-…` with a Gemini key.
2. Sync a folder into that workspace (`/folder` + sync).
3. Ask a question in the TUI, in the CLI, or through the `search_workspace` tool.
4. Result: dense search contributes nothing; only BM25 and temporal matches reach RRF and the reranker. No message is shown.

## 2. Evidence and root cause

Confirmed facts:

- **Ingestion** resolves the embedding provider and model from the workspace configuration: `ingestion/orchestrator.rs:714` `resolve_embedding_provider` maps the workspace model prefix to a provider and an embedding model (`gemini*` → Gemini `text-embedding-004`; `ollama*`/`local*` → Ollama `nomic-embed-text`; otherwise OpenAI `text-embedding-3-small` when an OpenAI key exists; otherwise `MockLmProvider`).
- **Retrieval** does not use that resolution. `retrieval/pipeline.rs:536` calls `lm.embed("text-embedding-3-small", …)` on `lm_client`. Every caller (`actx-cli/src/engine.rs:177`, `:615`, `:678`) passes the **chat** provider built by `resolve_lm_provider` (`engine.rs:10`) as `lm_client`.
- How each chat provider handles that call:
  - Anthropic and Gemma ONNX use the default `LmProvider::embed` (`actx-lm/src/traits.rs:28`), which returns `LmError::Unsupported`.
  - Gemini requests `models/text-embedding-3-small`, which does not exist on the Gemini API.
  - Ollama requests a model it does not serve.
  - OpenAI matches ingestion only by coincidence.
- The error is discarded (`_ => None`, `pipeline.rs:538`), so dense search is skipped silently.

Root cause: the query side and the ingestion side resolve the embedding model independently, and the query side resolves it from the wrong input (the chat provider plus a hard-coded model name).

Consequences:

- Dense retrieval works only when the workspace chat model is an OpenAI model.
- In multi-workspace queries (the active workspace plus `Global`), every workspace gets the same query vector, even when they were indexed differently.

Hypotheses (not confirmed, out of scope here):

- Changing a workspace's chat model later changes its embedding model at the next incremental sync, which mixes vector spaces inside one workspace. Captured as BUG-002.
- 768-dimension vectors (Gemini, Ollama) are written into a table created for 1536 dimensions. This is covered by IMP-001 (Frente C).

## 3. Fix design (`software-design`)

Smallest correct change: one embedding resolution, shared by ingestion and retrieval and applied per workspace.

1. **Single resolver in the Core.** Move `resolve_embedding_provider` out of `IngestionOrchestrator` into a new Core module `crates/any-context-core-rs/src/embedding.rs`. The new function is `resolve_embedding_target(db: &NativeConfigDb, model_override: Option<&str>, workspace: &str) -> EmbeddingTarget { provider: Arc<dyn LmProvider>, model: String }`, with the same rules and the same order as today. The orchestrator calls it. Ingestion behavior is unchanged.
2. **Query embedding port.** Add a `QueryEmbedder` trait in `retrieval` with `async fn embed_query(&self, workspace: &str, text: &str) -> Result<Vec<f32>, String>`. The default implementation is `WorkspaceQueryEmbedder` (Core): it holds `Arc<NativeConfigDb>` and uses `resolve_embedding_target`. It does not take a model override, because a query must match what was indexed.
3. **Pipeline embeds per workspace.**
   - `NativeHybridPipeline` holds `Option<Arc<dyn QueryEmbedder>>` in place of `lm_client`.
   - Step 5/6 of `search_single_async` computes one query vector per target workspace and searches each workspace with its own vector. Workspaces that resolve to the same `(provider_id, model)` share one embedding call.
   - `req.query_vector` still takes precedence when set.
   - `new(lance_store, query_embedder)` replaces the `LmProvider` parameter, and `set_lm_client` becomes `set_query_embedder`. The only callers are the three adapters in `actx-cli/src/engine.rs`.
4. **Honest fallback (ADR-110 kept).** If the query embedding fails for a workspace, that workspace falls back to BM25 only, as today. The failure is no longer invisible: the pipeline records the last query-embedding outcome per process (`last_query_embedding_status()`: model, or error text), following the `last_rerank_latency_ms()` pattern. Surfacing it in a command is left to Frente C (`/embed`).

Deliberately left out:

- Recording the model and dimension per workspace (Frente C goal 3).
- LanceDB dimension per table (IMP-001).
- The mixed-space problem after a model change (BUG-002).
- The `Global` workspace, which is bootstrapped with zero vectors (`engine.rs:482`) and stays BM25-only.

No ADR: this restores the documented intent (query and ingestion use the same model) without a new architectural decision. The retrieval section of `TECDOC.md` is corrected in place.

## 4. Affected surfaces and files

- Core:
  - `src/embedding.rs` (new), `src/lib.rs` (module export).
  - `src/ingestion/orchestrator.rs` (calls the shared resolver).
  - `src/retrieval/pipeline.rs` (`QueryEmbedder`, per-workspace dense search, status).
  - `src/retrieval/mod.rs` (exports).
- Adapters: `actx-cli/src/engine.rs`. The `search_workspace` tool, `PipelineBatchRetriever` and `PipelineDeterministicRetriever` build `WorkspaceQueryEmbedder` instead of passing the chat provider.
- Surfaces: `cli` and `tui` (behavior only, no interface change). `mcp` and `python-binding` do not construct the pipeline (verified by search). No command, flag or file format changes.

## 5. Tests (`quality-assurance`)

Automated tests (`cargo test --workspace`):

- **Regression** (`pipeline.rs` tests):
  - Two workspaces in a temporary LanceDB, each indexed with its own fake embedding space. A fake `QueryEmbedder` returns a different vector per workspace.
  - Assert that dense matches come from each workspace's own vector.
  - Assert that a failing embedder for one workspace leaves that workspace on BM25 without failing the query.
  - Before the fix, the same scenario with a provider that returns `Unsupported` yields no dense candidates.
- **Resolver** (`embedding.rs` tests): the workspace-model → `(provider_id, model)` mapping for `gemini*`, `ollama/*`, `claude*` with and without an OpenAI key, `mock*`, and the env overrides. These lock the behavior moved from the orchestrator.
- **Status**: `last_query_embedding_status()` reports the model on success and the error on failure.

Manual test: prepend **Scenario 46 (v0.45.1)** to `tests/MANUAL_TESTS.md`.

1. Workspace with a `claude-…` chat model and an OpenAI key: a semantic question (no keyword overlap) finds the right file.
2. The same with a `gemini-…` model and a Gemini key.
3. Without embedding keys, the answer still works through BM25.

Frente C's manual scenario moves to 47.

## 6. Documentation (`architecture-docs`, `user-docs`)

- `TECDOC.md`, retrieval/RAG section: the query embedding follows the workspace's embedding resolution (the same function as ingestion), there is one vector per target workspace, and the BM25 fallback is recorded.
- README: no change. The user-visible behavior is "semantic search works with any chat provider", which the README already promises.

## 7. Release, migration and rollback (`devops-release`)

- **Version**: patch `v0.45.1`. Bump every crate and `version.txt`, release candidate `v0.45.1-rc.1`, user acceptance, and the user merges `dev` into `main`.
- **Data**: no schema or data change. Vectors already indexed were produced by the ingestion resolver, which is unchanged, so the fix makes queries match existing data without a reindex.
- **Rollback**: revert the pull request on `main` and release the next patch. No data to restore.

## 8. Execution steps and milestones

- **M1 Shared resolver**: `embedding.rs`, orchestrator migration, resolver tests.
- **M2 Query embedder and pipeline**: `QueryEmbedder`, `WorkspaceQueryEmbedder`, per-workspace dense search, status, regression tests.
- **M3 Adapters**: `engine.rs` call sites, full `cargo test --workspace` with ONNX Runtime.
- **M4 Docs and manual test**: `TECDOC.md`, Scenario 46.
- **M5 Release**: PR into `dev`, CI green, version bump, `v0.45.1-rc.1`, acceptance, release.
