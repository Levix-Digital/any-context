---
id: 147
title: 'Constant vectors never reach dense search: the mock target is BM25 only and Global is embedded with the default target'
status: accepted
date: '2026-10-19'
version: v0.45.1
subsystems: [retrieval, ingestion, cli]
supersedes: []
superseded_by: []
amends: [94]
amended_by: [154]
---

# ADR-0147: Constant vectors never reach dense search: the mock target is BM25 only and Global is embedded with the default target

## Context

Dense search compares the query vector with stored chunk vectors. Two sources stored vectors that are the same for every text, so dense search ranked an arbitrary pool that RRF fused with BM25, and every dense-only candidate passed `RRF_MIN_SCORE` ([ADR-0139](0139-retrieval-order-follows-relevance.md)) (BUG-019):

- **The mock target.** Without an embedding key, `resolve_embedding_provider` falls back to `MockLmProvider` (target `mock/mock-embed`), whose `embed` returns a vector of `0.1` for every text. Chunk and query vectors were identical, and `/status` reported `dense + BM25`. A reproduction returned six chunks about pasta recipes for a question about kestrel timeouts.
- **The `Global` workspace with any provider.** `ensure_global_knowledge_bootstrap` ([ADR-0094](0094-test-sandbox-option-hinting-global-self-knowledge.md)) stores the sections of the embedded `SYSTEM_KNOWLEDGE.md` and `README.md` with zero vectors, and `search_vector_async` adds `OR workspace = 'Global'` to every workspace filter. With LanceDB's default L2 metric, a zero vector is at distance `‖q‖²` from the query: 1.0 for the normalized OpenAI and Gemini vectors, the distance of a real chunk with cosine similarity 0.5. `Global` rows tied with each other and outranked most moderately related user chunks.

The `Global` rows are also read by BM25-only hits ([ADR-0143](0143-bm25-only-hits-take-their-text-from-lancedb-batch-deduplicat.md)) and by file name matches, so they must stay in LanceDB. `Global` has no row in the SQLite workspace table, so its embedding target cannot be recorded like a workspace's ([ADR-0138](0138-record-the-embedding-target-per-workspace-and-reindex-on-change.md)).

## Decision

- **The mock target is searched by BM25 only.** `EmbeddingTarget::is_mock`; `resolve_query_embedding_provider` returns no provider on the mock target. `NativeHybridPipeline::set_query_embedding(provider, &target)` records the new `QueryEmbeddingOutcome::NoEmbeddingKey`, shown as `BM25 only (last query): no embedding API key (/keys)`. Ingestion still writes the mock vectors: the rows are needed and the vectors are never searched; adding a key changes the target, which reindexes the workspace.
- **`Global` is embedded with the default target.** One copy, embedded with `resolve_embedding_provider(db, None, "Global")`, the target of the default model. The target is recorded in the setting `global_knowledge_embedding_target` (`GLOBAL_KNOWLEDGE_TARGET_SETTING`).
- **`Global` joins dense search only for queries on its recorded target.** `ingestion::global_dense_enabled(db, query_target)` compares the setting with the query target (never on the mock target); the CLI passes it to `NativeHybridPipeline::set_global_dense`. Otherwise dense search skips the `Global` target and filters `workspace <> 'Global'`, on a named workspace and on the unfiltered `Default` path; BM25 and metadata matches are unchanged.
- **The sync embeds `Global`; the startup bootstrap stays offline.** The bootstrap (`bootstrap_global_knowledge`) writes the rows (`global_knowledge_records`) with placeholder vectors and BM25 entries as before, then clears the setting. After each pass of `actx --sync` and the sync worker, `refresh_global_knowledge_vectors` embeds the rows when the recorded target differs from the default target. It does nothing on the mock target, before this version's bootstrap (`global_knowledge_version`), or when the pass was cancelled or its provider breaker opened (`SyncResult.embedding_provider_unavailable`).
- **A failed refresh records nothing** ([ADR-0146](0146-a-failed-embedding-fails-the-file-and-embedding-outcomes-are.md)): a provider error, a short answer or a vector size other than the table's leaves the placeholders and the setting empty, and the next sync retries. All vectors are obtained before the `Global` rows are replaced, because `upsert_records` appends rows instead of replacing them by id.

## Alternatives Considered

- **Leave `Global` out of dense search for good**: a smaller change, but questions about AnyContext would be matched by keyword only. Rejected by the user.
- **Skip dense search inside the pipeline when `provider_id()` is `mock`**: puts the knowledge of which targets are meaningful in retrieval instead of in the embedding resolution.
- **Follow the target of the workspace synced last**: re-embeds `Global` back and forth between workspaces on different models.
- **One `Global` copy per target**: more rows, more embedding calls and a target column or table per target, for a case FEAT-005 makes rare.
- **Embed `Global` in the startup bootstrap**: adds network calls and their failures to startup and to the `search_db` tool.

## Consequences

- Without a key, answers use keyword matches only and `/status` says why; unrelated chunks no longer reach the prompt through dense search.
- Existing installations are fixed with the update: their `Global` rows have no recorded target, so they leave dense search at once and enter it after the next sync. Mock vectors of no-key workspaces stay and are never searched.
- After an upgrade, `Global` is found by keyword until the first sync. A workspace whose embedding target differs from the default sees `Global` by keyword only. Ollama's 768-dimension vectors fail the refresh (IMP-004).
- The `Global` texts sent to the provider are AnyContext's public documentation: one batch of calls per version and target.
- Verified by `test_constant_mock_vectors_do_not_rank_unrelated_chunks`, `test_global_rows_are_excluded_from_dense_search_without_a_matching_target`, `test_global_rows_take_part_in_dense_search_when_enabled` (`retrieval/pipeline.rs`), `query_provider_is_none_on_the_mock_target`, `global_dense_requires_the_same_target` (`ingestion/embedding.rs`), the breaker assertions in `ingestion/orchestrator.rs`, and `global_knowledge_records_are_stable`, `bootstrap_clears_the_global_target`, `refresh_embeds_global_and_records_the_target`, `refresh_is_skipped_on_the_mock_target_and_before_the_bootstrap`, `failed_refresh_records_nothing` (`actx-cli` `engine.rs`).
