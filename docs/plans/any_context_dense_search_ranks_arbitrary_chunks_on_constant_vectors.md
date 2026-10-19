---
plan_id: any_context_dense_search_ranks_arbitrary_chunks_on_constant_vectors
project: any-context
title: Dense search ranks arbitrary chunks on constant vectors
kind: bug
version: v0.45.1
status: completed
author: Claude
created_at: '2026-10-19T08:22:38'
updated_at: '2026-10-19T10:51:12'
session_id: null
tags:
- retrieval
- embedding
- global-knowledge
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- BUG-019
milestones:
- name: M1 Mock target is BM25 only
  done: true
  done_at: '2026-10-19'
  by: Claude
- name: M2 Global dense gate
  done: true
  done_at: '2026-10-19'
  by: Claude
- name: M3 Global embedding
  done: true
  done_at: '2026-10-19'
  by: Claude
- name: M4 Docs and manual tests
  done: true
  done_at: '2026-10-19'
  by: Claude
status_log:
- at: '2026-10-19T08:22:38'
  by: Claude
  status: draft
  note: Plan created
- at: '2026-10-19T08:45:46'
  by: Claude
  status: active
  note: Plan updated
- at: '2026-10-19T09:53:44'
  by: Claude
  note: 'Milestone ''M1 Mock target is BM25 only'' completed: Branch ccr-6c7c51f6-2tckxi; regression tests in ingestion/embedding.rs and retrieval/pipeline.rs'
- at: '2026-10-19T10:14:40'
  by: Claude
  note: 'Milestone ''M2 Global dense gate'' completed: Branch ccr-6c7c51f6-2tckxi; regression tests in retrieval/pipeline.rs and ingestion/embedding.rs'
- at: '2026-10-19T10:22:33'
  by: Claude
  note: 'Milestone ''M3 Global embedding'' completed: Branch ccr-6c7c51f6-2tckxi; tests in actx-cli engine.rs and ingestion/orchestrator.rs'
- at: '2026-10-19T10:32:07'
  by: Claude
  note: 'Milestone ''M4 Docs and manual tests'' completed: Branch ccr-6c7c51f6-2tckxi; ADR-0147, retrieval/ingestion/cli pages, README, MT-ANS-010, MT-ANS-011'
- at: '2026-10-19T10:32:07'
  by: Claude
  status: completed
  note: All milestones completed
- at: '2026-10-19T10:32:12'
  by: Claude
  status: in_progress
  note: All milestones done on the branch; completed when the pull request is merged
- at: '2026-10-19T10:51:12'
  by: Claude
  status: completed
  note: 'Merged in PR #365'
---

# Dense search ranks arbitrary chunks on constant vectors

Bug plan for the backlog item below (`dev-cycle-protocol` Phases 1 and 2). Discovery done on 2026-10-19 on `dev` at `5da3a40` (v0.45.0 plus the unreleased v0.45.1 work). The discovery found a second source of constant vectors, the `Global` workspace; the user decided to fix it in this plan by embedding it with the real provider (section 1.3).

## Backlog items

### BUG-019: Dense search ranks arbitrary chunks on the mock embedding target
- **Severity**: medium
- **Found**: 2026-10-18, 2026-10-18, v0.45.0, BUG-014/IMP-003 discovery
- **Description**: Without an embedding key, resolve_embedding_provider falls back to MockLmProvider (target mock/mock-embed), whose embed returns the same constant vector (0.1 x 1536) for every text. Chunks indexed and queries embedded on that target are all at the same distance, so dense search returns an arbitrary candidate pool that RRF fuses with BM25: a rank-1 dense-only hit scores 1/61, above RRF_MIN_SCORE (0.005), so unrelated chunks can reach the prompt. Found while planning BUG-014/IMP-003; candidate fix: skip dense search on the mock target and report it as BM25-only.

## 1. Reproduction

No logs are involved: the defect is silent ranking, not an error. It was reproduced with two throw-away probe tests in `retrieval/pipeline.rs` (reverted after the run), `cargo test -p any-context-core-rs --lib probe_bug019 -- --nocapture`, request as the CLI callers send it (`top_k` 8, `candidate_pool_k` 30, `min_score` `RRF_MIN_SCORE`), query `kestrel timeouts`.

**Case A, mock target.** Workspace `Proj`: one chunk about kestrel timeouts and six chunks about pasta recipes, all with the mock vector (`0.1` everywhere). Every chunk is in BM25. Result:

```
PROBE mock: k_0 score=0.03154 dense=Some(1.0)
PROBE mock: u_0 score=0.01639 dense=Some(1.0)
PROBE mock: u_3 score=0.01613 dense=Some(1.0)
PROBE mock: u_1 score=0.01587 dense=Some(1.0)
PROBE mock: u_4 score=0.01562 dense=Some(1.0)
PROBE mock: u_5 score=0.01538 dense=Some(1.0)
PROBE mock: u_2 score=0.01493 dense=Some(1.0)
```

All six pasta chunks, which share no word with the query, reach the results, in arbitrary order, as dense-only hits.

**Case B, `Global` workspace with a real key.** Workspace `Proj` with real-looking unit vectors (the kestrel chunk close to the query, the pasta chunks further away), and four `Global` chunks stored as the bootstrap stores them (`vec![0.0; dim]`), with text that shares no word with the query; `target_workspaces = ["Global"]` as in the CLI. Result:

```
PROBE global: k_0 score=0.03178 dense=Some(0.71428573)
PROBE global: u_1 ... u_3 score=0.016..0.015 dense=Some(0.7271057)
PROBE global: global_sys_doc_0 score=0.01515 dense=Some(0.5)
PROBE global: global_sys_doc_1 score=0.01493 dense=Some(0.5)
PROBE global: global_sys_doc_2 score=0.01471 dense=Some(0.5)
```

The `Global` chunks reach the results through dense search alone.

## 2. Evidence and root cause

Confirmed facts:

- `MockLmProvider::embed` (`actx-lm/src/providers/mock.rs`) returns a 1536-dimension vector of `0.1` for every text. `resolve_embedding_provider` (`ingestion/embedding.rs:92`) falls back to it (target `mock/mock-embed`) when neither the preferred provider nor OpenAI has a key; `build_provider` returns it for an explicit `mock*` model. Chunk vectors and query vectors on that target are identical, so every chunk is at distance 0 and LanceDB returns an arbitrary pool.
- `search_single_async` (`retrieval/pipeline.rs`, step 5) records `QueryEmbeddingOutcome::Dense` for the mock answer, so `/status` says `dense + BM25 (last query)` while dense search carries no information.
- RRF gives a rank-1 single-list hit 1/61 and `RRF_MIN_SCORE` is 0.005 (1/200): every dense-only candidate of the pool passes the floor (ADR-0139, by design). The arbitrary pool therefore reaches re-ranking and, when the cross-encoder is not active, the prompt.
- **The `Global` workspace has the same defect with any provider.** `ensure_global_knowledge_bootstrap` (`actx-cli/src/engine.rs:468`) stores the embedded `SYSTEM_KNOWLEDGE.md` and `README.md` sections with `vector: vec![0.0; 1536]`. `NativeLanceStore::search_vector_async` (`storage/lancedb.rs:648`) adds `OR workspace = 'Global'` to every workspace filter, and the three CLI callers also pass `Global` as an extra target, so `Global` rows are in the dense pool of every query. LanceDB uses its default L2 metric (no `distance_type` is set): the distance of a zero vector to the query is `‖q‖²`, the same for every `Global` row, which is 1.0 for the normalized vectors of OpenAI and Gemini, the distance of a real chunk with cosine similarity 0.5. `Global` rows therefore tie with each other and outrank every user chunk with cosine below 0.5, which is most of the moderately related ones.
- The `Global` rows are needed for more than dense search: BM25-only hits read text and provenance from their LanceDB row (ADR-0143), and file name matches use LanceDB metadata search. They must stay in LanceDB.
- Only the CLI builds query embeddings: the `search_db` tool, the one-hop retriever and Deep Search all call `actx-cli/src/engine.rs` `resolve_query_embedding`, which calls `ingestion::resolve_query_embedding_provider`. One change there covers every path.
- `Global` has no row in the SQLite workspace table, so `set_workspace_embedding_target` cannot record its target (`decide_embedding_reindex` leaves such workspaces alone); a setting is needed (`global_knowledge_version` is already stored the same way).
- All cloud providers produce 1536-dimension vectors (Gemini through `outputDimensionality`); Ollama's `nomic-embed-text` produces 768, which is IMP-004 and out of scope.
- No existing test relies on dense search over mock vectors (the orchestrator tests use the mock target for sync only).

## 3. Goal and decisions

No constant vector ever takes part in dense search, and `/status` says when dense search was not used.

### 3.1 Decisions

1. **The mock target is BM25 only at query time.** `resolve_query_embedding_provider` returns no provider when the resolved target is `mock`, so the pipeline skips dense search and records a new outcome, `QueryEmbeddingOutcome::NoEmbeddingKey`, shown as `BM25 only (last query): no embedding API key (/keys)`. Ingestion keeps writing the mock vectors: the rows are needed (section 2), the vectors are never searched, and adding a key changes the target, which already reindexes the workspace (ADR-0138). Alternative rejected: detecting `provider_id() == "mock"` inside the pipeline, which puts the knowledge of which targets are meaningful in retrieval instead of in the embedding resolution that owns it.
2. **`Global` is embedded with the real provider** (user's decision, 2026-10-19). Alternative rejected by the user: leaving `Global` out of dense search (smaller change, but questions about AnyContext itself would be matched by keyword only).
3. **One copy of `Global`, embedded with the default target.** `Global` is shared by every workspace but workspaces can resolve different targets. It is embedded with `resolve_embedding_provider(db, None, "Global")`, which resolves to the default model's target (no workspace model exists for `Global`), and the target is recorded in the setting `global_knowledge_embedding_target`. Alternative rejected: following the target of the workspace synced last, which re-embeds `Global` back and forth between workspaces on different models. Limitation: a workspace whose embedding target differs from the default sees `Global` by keyword only; FEAT-005 (one embedding model for all workspaces) makes that rare.
4. **`Global` takes part in dense search only when its recorded target equals the query target** (and the target is not `mock`). Otherwise the dense query excludes `Global` rows; BM25 and metadata matches are unchanged. Existing installations are fixed immediately: their `Global` rows have no recorded target, so they leave dense search with the update and enter it again once embedded.
5. **The sync worker embeds `Global`, not the startup bootstrap.** The bootstrap stays synchronous and offline (it runs in the startup splash and inside the `search_db` tool): it writes the rows with placeholder vectors and BM25 as today, and clears `global_knowledge_embedding_target` whenever it rewrites them. At the end of each sync pass, the worker embeds `Global` when the recorded target differs from the current default target. Alternative rejected: embedding in the bootstrap, which adds network calls and their failures to startup and to the search tool. Consequence: after an upgrade, `Global` is keyword-only until the first sync.
6. **A failed `Global` embedding follows ADR-0146.** No partial write: on any error the placeholder vectors stay, the target stays unrecorded (so `Global` stays out of dense search) and the next sync retries. It is not counted as a user file: it is logged in the sync log and not shown in the footer. When the pass's provider breaker is open, `Global` is skipped.

### 3.2 ADR

ADR-0147 "Constant vectors never reach dense search: the mock target is BM25 only and `Global` is embedded with the default target" (amends ADR-0094 on how `Global` is stored, relates to ADR-0110, ADR-0138 and ADR-0146).

## 4. Fix design (`software-design`)

### 4.1 Query embedding (core, `ingestion/embedding.rs`, `retrieval/pipeline.rs`)

- `EmbeddingTarget::is_mock()`; `resolve_query_embedding_provider` returns `(None, target)` when `target.is_mock()`. `resolve_embedding_provider` is unchanged (ingestion still needs a provider).
- `QueryEmbeddingOutcome::NoEmbeddingKey`. A missing provider records `NoProvider` today; the pipeline cannot tell that case from the mock target, so it gains one private field, `missing_provider_outcome` (default `NoProvider`), and `set_query_embedding(provider, &EmbeddingTarget)`, which sets the provider and model and records `NoEmbeddingKey` when the target is mock. `set_embedding` is unchanged. (Implementation refinement of M1: the plan first had a public setter for the outcome; taking the target keeps the rule in the core, where it is tested.)
- CLI: `resolve_query_embedding` returns a `QueryEmbedding { provider, target }` whose `apply(&mut pipeline)` replaces the three `set_embedding(provider, model)` calls; M2 adds the `Global` gate to `apply`.
- `HybridSearchRequest` is unchanged; the pipeline gains `global_dense: bool` (default `false`, set by `set_global_dense`). When false, step 6 skips the `Global` target and passes `workspace <> 'Global'` as `filter_expr` to `search_vector_async` (combined with the workspace clause by `AND`; it also covers the unfiltered `Default` path). The storage layer is not changed.

### 4.2 `Global` knowledge (CLI, `actx-cli/src/engine.rs`)

- Extract the section chunking of `ensure_global_knowledge_bootstrap` into `global_knowledge_records() -> Vec<VectorRecord>` (same ids `global_sys_doc_<n>`, same text and metadata) so the bootstrap and the embedding step build identical rows. The bootstrap writes them with placeholder vectors as today and clears `global_knowledge_embedding_target`.
- `global_dense_enabled(db, query_target) -> bool` and the setting name `GLOBAL_KNOWLEDGE_TARGET_SETTING` live in `ingestion/embedding.rs` (core) next to the target they compare, so the rule is tested with an in-memory database: recorded `global_knowledge_embedding_target` equals `query_target.key()` and the target is not mock. `resolve_query_embedding` returns `QueryEmbedding { provider, target, global_dense }` and `apply` calls `set_global_dense`. (Implementation refinement of M2: the plan first placed the rule in the CLI.)
- `refresh_global_knowledge_vectors(db, lance_store, provider, target) -> Result<bool, String>` (async): returns `Ok(false)` when the recorded target already equals `target` or the target is mock; otherwise embeds the texts of `global_knowledge_records()` in batches (the same rate-limit retry as file chunks: `embed_with_retry` in `ingestion/orchestrator.rs` becomes public through `ingestion`), checks the answer length, upserts the rows with real vectors (upsert encrypts as for any row) and then records the target. Any error returns `Err` and records nothing.
- `oneshot.rs` sync worker: after `orchestrator.run`, unless the pass's breaker opened, resolve `resolve_embedding_provider(db, None, "Global")` and call `refresh_global_knowledge_vectors`; log the outcome to the sync log.

### 4.3 Deliberately not done

- No change to ingestion of user files on the mock target (rows stay, vectors are never searched).
- No per-target copies of `Global`, no `/status` line for `Global` (the sync log has it), no change to the LanceDB distance metric, no fix for Ollama's 768-dimension vectors (IMP-004).

### 4.4 Architecture rules

Core stays free of CLI types; the embedded documentation stays in the CLI crate. No LanceDB or BM25 format change, no SQLite schema change (one new setting key).

### 4.5 Found during implementation (M3)

- `NativeLanceStore::upsert_records` appends rows; it does not replace rows with the same id (the orchestrator purges before it writes). A first refresh doubled the `Global` rows, caught by `refresh_embeds_global_and_records_the_target`. The refresh now gets every vector first, then deletes the `Global` rows and inserts the embedded ones. If that insert fails, the next startup bootstrap finds no `Global` rows and writes the placeholders again (BM25 is untouched).
- The breaker of a pass was internal to the orchestrator. `SyncResult` gains `embedding_provider_unavailable` (breaker open), and the worker skips `Global` when it is set or the pass was cancelled.
- Race between a startup bootstrap and a sync worker: the bootstrap clears the recorded target *after* writing its rows, and the refresh does nothing while `global_knowledge_version` is not the running version (the bootstrap for this version has not run), so placeholders are never left with a recorded target.
- A vector whose size is not the table's (Ollama's 768, IMP-004) fails the refresh instead of being padded with zeros by `build_record_batch`.
- The embedding runs in both `actx --sync` and the background `--sync-worker` (`cli/oneshot.rs` `refresh_global_knowledge`).

## 5. Affected surfaces and files

- Surfaces: `cli` (`/status` line text, sync worker log), `tui` (`/status` through the command engine).
- `crates/any-context-core-rs/src/ingestion/embedding.rs`: `is_mock`, query resolution returns no provider on mock.
- `crates/any-context-core-rs/src/ingestion/orchestrator.rs` and `ingestion/mod.rs`: expose the batched embedding with retry for reuse.
- `crates/any-context-core-rs/src/retrieval/pipeline.rs`: `NoEmbeddingKey` outcome, no-provider reason, `global_dense` gate.
- `crates/actx-cli/src/engine.rs`: `global_knowledge_records`, bootstrap clears the target, `global_dense_enabled`, `refresh_global_knowledge_vectors`, the three pipeline builders, `resolve_query_embedding`.
- `crates/actx-cli/src/cli/oneshot.rs`: the sync worker refreshes `Global`.
- Same defect elsewhere: none found; the only other writers of constant vectors are `generate_fallback_vectors` (unreachable `None` branch, ADR-0146) and test fixtures.

## 6. Tests (`quality-assurance`)

Automated (`cargo test --workspace`), regression tests first (they fail before the fix):

- `pipeline.rs` `test_constant_mock_vectors_do_not_rank_unrelated_chunks` (case A as a test: with the CLI's resolution on the mock target, only the BM25 match is returned and the outcome is `NoEmbeddingKey`).
- `pipeline.rs` `test_global_rows_are_excluded_from_dense_search_without_a_matching_target` (case B: placeholder `Global` rows never appear as dense hits; still found by BM25) and `test_global_rows_take_part_in_dense_search_when_enabled`, including the unfiltered `Default` path.
- `embedding.rs` `test_query_provider_is_none_on_the_mock_target` (explicit `mock` model and the no-key fallback); the real targets still return a provider.
- `pipeline.rs` `test_no_embedding_key_outcome_is_described`.
- `actx-cli` `engine.rs`: `test_global_knowledge_records_are_stable` (same ids and texts as the bootstrap), `test_refresh_embeds_global_and_records_the_target` (fixed embedder: rows get its vectors, setting recorded), `test_refresh_is_skipped_on_the_mock_target_and_when_up_to_date`, `test_failed_refresh_records_nothing` (error and short answer), `test_bootstrap_clears_the_global_target`; `global_dense_requires_the_same_target` is in `ingestion/embedding.rs`.

Manual (`tests/MANUAL_TESTS.md`):

- New **MT-ANS-010: Without an embedding key, answers use keyword matches only**: no API key, a synced workspace; ask about a phrase in one file → answer from that file, no unrelated sources cited; `/status` shows `Dense Search: BM25 only (last query): no embedding API key (/keys)`.
- New **MT-ANS-011: Questions about AnyContext are matched by meaning after a sync**: with a working key, upgrade or first start, `/sync`; ask about a command using words not in the documentation (e.g. "how do I forget a folder?") → answered from the system documentation; questions about the user's files cite no system documentation unless relevant.
- MT-ANS-009 step 1 to 3 unchanged; check that its expected text still matches.

Acceptance: no dense candidate comes from a constant vector (mock target or placeholder `Global` rows); `/status` reports the no-key case; `Global` is embedded with the default target after a sync and used by dense search only for queries on that target.

## 7. Documentation (`architecture-docs`, `user-docs`)

- ADR-0147 (section 3.2) and the TECDOC ADR table.
- `docs/architecture/retrieval.md`: step 5 (mock target, `NoEmbeddingKey`), step 6 (`Global` dense gate).
- `docs/architecture/ingestion.md`: `Global` embedding at the end of a sync pass, its setting and failure handling.
- README: the no-key note (answers use keyword search only until a key is added) and the `/status` troubleshooting row for `no embedding API key`.

## 8. Release, migration and rollback (`devops-release`)

- No workflow change. Patch release, v0.45.1 (with the unreleased v0.45.1 work on `dev`).
- Release notes: "Without an embedding API key, answers now use keyword search only instead of mixing in unrelated passages, and `/status` says so. AnyContext's own documentation is now matched by meaning: it is embedded with your default embedding model on the next `/sync`; until then it is found by keyword."
- Data already affected: existing `Global` rows have placeholder vectors and no recorded target, so the dense gate excludes them from the first query after the update and the next sync embeds them; mock vectors of no-key workspaces stay and are never searched. No migration.
- Security: the `Global` texts sent to the provider are AnyContext's public documentation, no user data; one batch per version and target.
- Rollback: revert the commits. An older version rewrites `Global` with placeholder vectors on its version change and ignores the new setting.

## 9. Execution steps and milestones

1. **M1 Mock target is BM25 only**: `is_mock`, query resolution, `NoEmbeddingKey`; tests.
2. **M2 Global dense gate**: pipeline `global_dense`, `global_dense_enabled`, the three pipeline builders; tests.
3. **M3 Global embedding**: `global_knowledge_records`, bootstrap clears the target, `refresh_global_knowledge_vectors`, sync worker step; tests.
4. **M4 Docs and manual tests**: ADR-0147, architecture pages, README, MT-ANS-010, MT-ANS-011.
