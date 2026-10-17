---
id: 138
title: Record the embedding target per workspace and reindex on change
status: accepted
date: '2026-10-17'
version: v0.45.1
subsystems: [ingestion, retrieval, storage, lm-providers]
supersedes: []
superseded_by: []
amends: [132]
amended_by: []
---

# ADR-0138: Record the embedding target per workspace and reindex on change

## Context

ADR-0132 made chunks and queries use one resolver, `resolve_embedding_provider`, so both are embedded with the same provider and model at any given moment. The resolver derives the embedding model from the workspace **chat** model (after the explicit override, `ACTX_EMBEDDING_MODEL` and `ACTX_MODEL`), and nothing stored which target built the vectors already in LanceDB.

- After `/model` changed the chat model, the next incremental sync re-embedded only new and modified files. Unchanged files kept vectors from the old target, and queries moved to the new one, so part of the workspace became unreachable by dense search (BUG-003).
- With no API key the resolver fell back to the mock provider while still reporting `text-embedding-3-small`, so adding a key later mixed pseudo-vectors with real ones.
- Google shut down `text-embedding-004` on 2026-01-14. Every Gemini workspace has held failed or pseudo-vectors since then (BUG-013), and its successor `gemini-embedding-2` returns 3072 values by default, while every LanceDB table has a fixed 1536 dimension.

## Decision

- **Embedding target.** `ingestion::embedding::EmbeddingTarget { provider, model }`, stored as the key `provider/model` (for example `openai/text-embedding-3-small`). `resolve_embedding_provider` returns it instead of a bare model id. Gemini resolves to `gemini/gemini-embedding-2`, and the no-key fallback and explicit `mock*` models resolve to `mock/mock-embed`.
- **Record.** The nullable column `workspaces.embedding_target` in `settings.db` holds the target that built the workspace's vectors. The sync writes it only after a pass completes; a cancelled pass leaves the old value.
- **Reindex rule** (`decide_reindex`): recorded equal to current → keep; recorded different → reindex; nothing recorded and nothing indexed → adopt the current target. Nothing recorded but chunks stored (a workspace from before this decision) → compare with `legacy_target`, the pre-fix resolver's answer for the same inputs, which differs only for Gemini (`text-embedding-004`). Equal → adopt, different → reindex.
- **Automatic full reindex.** A reindex turns the pass into a forced pass over the whole workspace (vectors, BM25 and stat cache purged), even when the sync was scoped to one folder. It needs no confirmation. The sync status, the verbose output and `SyncResult.embedding_reindex` say so, and `/model` and `/sync` warn when the next sync will reindex (`pending_reindex_notice`).
- **Queries** (amends ADR-0132): `resolve_query_embedding_provider` prefers the recorded target when its provider can be built, so dense search keeps working between a model change and the reindex. It falls back to the current resolution when nothing is recorded, when the recorded provider has no key, or when the recorded model is the shut-down `text-embedding-004`.
- **Gemini output size.** `GeminiProvider::embed` sends `outputDimensionality: 1536` (`GEMINI_EMBEDDING_DIM`, equal to `DEFAULT_VECTOR_DIM`) for each request item. Gemini Embedding 2 normalizes truncated sizes, so the adapter does not normalize.

## Alternatives Considered

- **Record and block until a forced sync**: the sync refuses to embed incrementally and asks the user to run `/sync --force`. The space is never mixed either, but every model change leaves the workspace half-working until the user acts. Rejected by the user in favor of the automatic reindex.
- **Pin the recorded model**: keep embedding with the recorded target until the user reindexes explicitly. It avoids the reindex cost, but the embedding model then silently stops following the model the user chose, and it doesn't repair Gemini workspaces whose model is shut down.
- **A separate embedding setting** (FEAT-005) and **a vector dimension per table** (IMP-004): the complete fix, planned for Frente C. Both need more surface and, for IMP-004, a LanceDB schema migration. The record kept here is the base they build on.
- **A global record** in `app_settings`: simpler, but each workspace has its own chat model and its own rows in the shared table, so a single value can't describe them.

## Consequences

- A workspace never mixes vectors from two targets after a completed pass. A model change costs one full embedding pass of the workspace on the next sync.
- Workspaces mixed before this decision by an earlier `/model` change can't be detected (their old target was never recorded); the release notes ask users to run one forced sync if they changed models.
- A workspace without a row in `workspaces` can't record a target and is left as it is.
- Ollama `nomic-embed-text` vectors (768 values) are still zero-padded to 1536 (IMP-004). A failed embedding call still stores pseudo-vectors (BUG-014).
- Verified by `test_embedding_target_change_reindexes_whole_workspace` and `test_scoped_sync_with_target_change_reindexes_all_folders` (`crates/any-context-core-rs/tests/orchestrator_tests.rs`), the unit tests in `ingestion/embedding.rs` and the manual tests MT-SYNC-006 and MT-SYNC-007.
