---
plan_id: any_context_embedding_failures_are_hidden_pseudo_vectors_at_ingestion_and_silent_bm25_only_queries
project: any-context
title: 'Embedding failures are hidden: pseudo-vectors at ingestion and silent BM25-only queries'
kind: feature
version: v0.45.1
status: in_progress
author: Claude
created_at: '2026-10-18T19:32:46'
updated_at: '2026-10-18T21:44:04'
session_id: null
tags: []
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- BUG-014
- IMP-003
milestones:
- name: M1 Ingestion fails the file
  done: true
  done_at: '2026-10-18'
  by: Claude
- name: M2 Breaker and rate limit
  done: true
  done_at: '2026-10-18'
  by: Claude
- name: M3 Reporting
  done: false
- name: M4 Query outcome (IMP-003)
  done: false
- name: M5 Docs and manual tests
  done: false
status_log:
- at: '2026-10-18T19:32:46'
  by: Claude
  status: draft
  note: Plan created
- at: '2026-10-18T19:43:53'
  by: Claude
  status: active
  note: Plan updated
- at: '2026-10-18T20:10:18'
  by: Claude
  note: 'Milestone ''M1 Ingestion fails the file'' completed: Branch ccr-fad47706-3528p3; 6 regression tests in ingestion/orchestrator.rs'
- at: '2026-10-18T21:44:04'
  by: Claude
  note: 'Milestone ''M2 Breaker and rate limit'' completed: Branch ccr-fad47706-3528p3; 5 tests in ingestion/orchestrator.rs'
---

# Embedding failures are hidden: pseudo-vectors at ingestion and silent BM25-only queries

Plan for the backlog items below (`dev-cycle-protocol` Phases 1 and 2). Discovery done and open questions decided on 2026-10-18 on `dev` at `80f47f7` (v0.45.0 plus the unreleased v0.45.1 work).

## Backlog items

### BUG-014: Failed embedding calls store pseudo-vectors next to real ones
- **Severity**: medium
- **Found**: 2026-10-17, 2026-10-17, BUG-003 discovery
- **Description**: embed_and_upsert (ingestion/orchestrator.rs) replaces a failed provider.embed call with SHA-256 pseudo-vectors (generate_fallback_vectors) and writes them to the same table as real vectors, without telling the user or marking the file. A transient API error (rate limit, network) leaves those files unreachable by dense search until they change on disk. Decide between failing the file so the next sync retries it, or marking it for re-embedding; keep the ADR-110 no-key fallback. Related to IMP-003.

### IMP-003: Report query embedding failures instead of skipping dense search silently
- **Priority**: medium
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Description**: NativeHybridPipeline::search_single_async discards the query embedding error (retrieval/pipeline.rs, '_ => None'), so a missing key or unsupported provider silently leaves only BM25. Keep the ADR-110 fallback but record the outcome (for example a last-status accessor like last_rerank_latency_ms) and surface it in the planned /embed status.

## 1. Goal and decisions

Embedding failures must never be hidden, neither at ingestion (BUG-014) nor at query time (IMP-003).

### 1.1 Confirmed facts (discovery)

Ingestion (`crates/any-context-core-rs/src/ingestion/orchestrator.rs`):

- `embed_and_upsert_chunks` (line ~629) maps `Err(_)` from `provider.embed` to `generate_fallback_vectors` (SHA-256 pseudo-vectors, `DEFAULT_VECTOR_DIM`) and upserts them into `workspace_chunks` next to real vectors. The error is discarded.
- A short vector list is also hidden: `vectors.get(i).unwrap_or_else(|| vec![0.0; DEFAULT_VECTOR_DIM])` writes zero vectors for missing positions.
- The `lm_provider == None` branch is unreachable from the sync: `resolve_embedding_provider` (`ingestion/embedding.rs:92`) always returns `Some`, falling back to OpenAI and then to `MockLmProvider` (target `mock/mock-embed`) when no key exists. That mock provider is today's no-key fallback; the pseudo-vectors are only reached through a failed call.
- The sync loop already retries failed files: when `embed_and_upsert_chunks` returns `Err` (line ~492), the SQLite stat cache is not updated, so the next sync sees the file as new or modified again. Failing the file therefore needs no new state.
- Modified files are purged from LanceDB and BM25 before the loop (step 10, Purge-Before-Embed). A failed modified file therefore loses its old chunks.
- BM25 receives a file's chunks before the embedding call (line ~614). `BM25Index::add_chunk` replaces an existing id, and ids are `<content hash prefix>_<index>`, so a retry of unchanged content does not duplicate. A new file that fails and is edited before the retry keeps orphan BM25 chunks under the old hash (not purged by step 10, since it is not in the cache).
- A BM25-only hit without a LanceDB row is already handled: it falls back to the BM25 text with the envelope stripped (ADR-0143). It has no provenance (lines, header path).
- Web pages: `index_web_page` propagates the error, but `sync_web_portals` discards it (`let _ =`). The page's old chunks were already deleted and its retry works (no LanceDB row with the new hash).
- **Sync errors reach no surface.** `SyncResult.errors` is collected but never printed (`actx-cli/src/cli/oneshot.rs` prints only counts), and the final `update_sync_status(..., "idle", None, None)` writes `error = NULL`. The `workspace_sync_status.error` column exists and is read by `get_sync_status`, but nothing displays it. After a sync with failures, the TUI footer shows "Changes detected (/sync)" at best (failed files are missing from the stat cache), never the cause.
- `LmError` distinguishes `Auth`, `RateLimit { retry_after_secs }`, `ProviderError { status }`, `Network` and `Serialization`. No provider retries.

Query (`crates/any-context-core-rs/src/retrieval/pipeline.rs`):

- `search_single_async` (line ~599): `Ok(vecs) if !vecs.is_empty() => Some(..)`, `_ => None`. An error, an empty answer or no provider all silently skip dense search.
- Three callers in `actx-cli/src/engine.rs` (the `search_db` tool, the one-hop retriever, Deep Search) build the pipeline with `resolve_query_embedding`.
- Pattern to reuse: `LAST_RERANK_LATENCY_MS` + `last_rerank_latency_ms()`, a process-wide "last outcome" accessor shown by the rerank status view.
- There is no `/embed` command yet (it belongs to FEAT-005). The surfaces available today are `/status` (`commands/engine.rs` `execute_status`) and the live status report (`actx-cli/src/engine.rs` `format_status_report`, used by the `system_status` tool and the status route, ADR-0145).

Out of scope, captured as **BUG-019**: on the mock target every vector is the same constant, so dense search ranks arbitrary chunks. IMP-003's description says a missing key leaves only BM25; in fact it leaves mock dense search, which is BUG-019.

### 1.2 Decisions

1. **A failed embedding call fails the file** (user's decision, 2026-10-18). No pseudo-vector or zero vector is written to LanceDB; the file is not recorded in the stat cache and the next sync retries it. Alternative rejected: marking the file for re-embedding, which needs new state (a flag or a schema change, the latter colliding with IMP-004) and still has to decide which vector to store meanwhile.
2. **The file stays reachable by keyword while it waits.** Its chunks stay in BM25; to avoid orphans, the loop removes the file's BM25 chunks (`remove_by_exact_file`) before adding the new ones. BM25-only hits already work without a LanceDB row (ADR-0143).
3. **The ADR-110 no-key fallback is unchanged.** It is the mock provider today; the unreachable `None` branch keeps `generate_fallback_vectors`. The mock target's own problem is BUG-019.
4. **Provider breaker per sync pass.** After an `Auth` error, or after 3 consecutive failed files, the pass stops calling the provider: remaining files are chunked into BM25 and fail with "embedding skipped: provider unavailable", so a bad key costs one call instead of one per file, and modified files already purged still stay findable by keyword.
5. **Sync failures are reported.** The pass writes a summary into `workspace_sync_status.error` (e.g. `12 files not embedded (rate limit from 'gemini': ...); they are retried on the next sync`), the CLI prints the errors, and the TUI and the status report show it.
6. **The query embedding outcome is recorded and shown.** The pipeline records the last outcome (dense used / no provider / failed with reason) and `/status` and the status report show it. The ADR-110 behavior stays: a failed query still answers with BM25.

### 1.3 Decisions on the open questions (user, 2026-10-18)

- **Q1. Rate limit: retry once.** One retry of the same batch after `retry_after_secs` (default 2 s, capped at 30 s), only for `LmError::RateLimit`, so a large first sync on a free-tier key does not trip the breaker.
- **Q2. Pseudo-vectors already stored: no automatic cleanup.** The release notes tell users who saw sync problems to run a forced sync (`/sync --force`). Detection (the pattern repeats every 32 values) would mean reading every vector of every workspace once.
- **Q3. Breaker threshold: 3 consecutive failed files**; any `Auth` error trips it at once.

### 1.4 ADR

ADR-0146 "A failed embedding fails the file, and embedding outcomes are reported" (amends ADR-0110 on the fallback scope, relates to ADR-0132 and ADR-0143).

## 2. Design (`software-design`)

### 2.1 Ingestion

- `embed_and_upsert_chunks` returns a typed error instead of `String` for the embedding step, so the loop can drive the breaker:
  ```rust
  enum EmbedFailure {
      Provider(LmError),       // the call failed
      ShortAnswer { expected: usize, got: usize },
      Skipped,                 // breaker open: no call made
      Store(String),           // LanceDB upsert failed (unchanged meaning)
  }
  ```
  The function gains a `skip_embedding: bool` parameter (breaker open); BM25 population stays first, as today.
- `Err` from `provider.embed` → `EmbedFailure::Provider`; `vectors.len() != chunks.len()` → `ShortAnswer`. The `unwrap_or_else(zero vector)` is removed.
- Main loop: before indexing a file, `bm25.remove_by_exact_file(file_path)`. On `Provider`/`ShortAnswer`: push a per-file error, increment the consecutive-failure counter (reset on success), open the breaker on `Auth` or at the threshold. On `Skipped`: push the per-file error. The stat cache is never written on failure (unchanged).
- Q1: one retry inside `embed_and_upsert_chunks` for `LmError::RateLimit`, sleeping `min(retry_after_secs.unwrap_or(2), 30)` seconds.
- `sync_web_portals`/`index_web_page` use the same breaker state and push page errors to the pass errors instead of discarding them.
- End of pass: `update_sync_status(..., "idle", None, Some(&summary))` when there were embedding failures, `None` otherwise; a later clean pass clears it. `SyncResult` gains `embedding_failures: usize`.
- The per-file errors keep the current format (`Embedding/upsert error on '<path>': <reason>`); the summary counts them and quotes the first reason.

### 2.2 Query

- In `pipeline.rs`, next to `LAST_RERANK_LATENCY_MS`:
  ```rust
  pub enum QueryEmbeddingOutcome { Dense, NoProvider, Failed(String) }
  static LAST_QUERY_EMBEDDING: Mutex<Option<QueryEmbeddingOutcome>>;
  pub fn last_query_embedding_outcome() -> Option<QueryEmbeddingOutcome>;
  ```
  `search_single_async` records it at step 5 (`Failed` carries the `LmError` text, or "empty embedding answer"). A caller-supplied `query_vector` records `Dense`.
- `execute_status` adds `• Dense Search: dense + BM25 (last query)` / `BM25 only (last query): <reason>` / `n/a (no query yet in this session)`; `format_status_report` adds the same line and the sync error summary.
- Deliberately not done: a new `/embed` command (FEAT-005 owns it and will reuse the accessor), a per-answer warning in the chat (the status line is enough for now), a retry for query embeddings.

### 2.3 TUI and CLI

- `oneshot.rs` (`--sync` and `--sync-worker`): print `⚠ N files not embedded` and up to 10 error lines, then "they are retried on the next sync". The worker output goes to its log file.
- TUI footer and header (`tui/ui.rs`): when the sync is idle and `status.error` is set, show `⚠ N files not embedded (/sync)` in the error style, with priority over "Changes detected" and "Up to date".

### 2.4 Architecture rules

Core stays free of CLI/TUI types (`forbidden_core_imports`); `LmError` already comes from `actx-lm`, a core dependency. No LanceDB or BM25 format change; no SQLite schema change (the `error` column exists).

## 3. Affected surfaces and files

- Surfaces: `cli` (sync output, `/status`), `tui` (footer/header sync indicator).
- `crates/any-context-core-rs/src/ingestion/orchestrator.rs`: failure handling, breaker, BM25 purge per file, web page errors, status summary, `SyncResult` field.
- `crates/any-context-core-rs/src/ingestion/mod.rs`: `SyncResult.embedding_failures`.
- `crates/any-context-core-rs/src/retrieval/pipeline.rs`: query outcome accessor.
- `crates/any-context-core-rs/src/commands/engine.rs`: `/status` line.
- `crates/actx-cli/src/engine.rs`: `format_status_report`.
- `crates/actx-cli/src/cli/oneshot.rs`: print sync errors.
- `crates/actx-cli/src/tui/ui.rs`: sync error indicator.

## 4. Tests (`quality-assurance`)

Automated (`cargo test --workspace`), with a `FailingLmProvider` test double returning a chosen `LmError` (and one returning fewer vectors):

- `test_failed_embedding_writes_no_vectors_and_skips_the_stat_cache`: LanceDB has no row for the file, the stat cache has no entry, BM25 has the chunks, `SyncResult.errors` names the file.
- `test_failed_file_is_retried_on_the_next_sync`: the next pass with a working provider indexes it with real vectors.
- `test_short_embedding_answer_fails_the_file` (no zero vectors).
- `test_new_file_edited_before_retry_leaves_no_orphan_bm25_chunks`.
- `test_auth_error_opens_the_breaker` (one provider call for N files; the rest skipped but in BM25) and `test_three_consecutive_failures_open_the_breaker`; a success resets the counter.
- `test_failed_web_page_is_reported`.
- `test_sync_status_error_summary_set_and_cleared`.
- `test_rate_limit_is_retried_once` and `test_second_rate_limit_fails_the_file` (zero-delay `retry_after`).
- `pipeline.rs`: `test_query_embedding_failure_is_recorded`, `test_query_without_provider_is_recorded`, `test_query_dense_outcome_is_recorded`; existing BM25 fallback tests keep passing.
- `test_status_report_shows_dense_search_and_sync_errors` (`actx-cli`).
- Remove `test_generate_fallback_vectors` only if the function goes (it stays for the `None` branch).

Manual (`tests/MANUAL_TESTS.md`):

- New **MT-SYNC-008: An embedding failure is reported and retried**: sync with an invalid key → the TUI shows the warning, `/status` shows it, keyword search still finds the files; fix the key and `/sync` → warning cleared, semantic questions answered.
- New **MT-SEARCH (or extend the status case): `/status` shows BM25 only after a failing query** (invalid key set after indexing).

Acceptance: no code path writes pseudo- or zero vectors after a failed call; every embedding failure is visible in the CLI output, the TUI and `/status`; the files retry on the next sync.

## 5. Documentation (`architecture-docs`, `user-docs`)

- ADR-0146 (section 1.4) and the TECDOC ADR table.
- `docs/architecture/ingestion.md`: failure handling, breaker, retry on next sync.
- `docs/architecture/retrieval.md`: query embedding outcome.
- `docs/architecture/tui.md`, `docs/architecture/cli.md`: sync error indicator and output.
- README (sync section): what the warning means and that a sync retries the files; release notes recommend `/sync --force` to users who saw sync problems (Q2).

## 6. CI, release and observability (`devops-release`)

- No workflow change. Patch release, v0.45.1 (with the unreleased v0.45.1 work on `dev`).
- Observability is the point of the plan: sync error summary in SQLite, CLI and TUI; last query embedding outcome in `/status` and the status report. Tested by the automated tests above.

## 7. Security, migration and rollback

- Error messages from providers can echo request details; they are shown locally only, and API keys are never part of `LmError` messages (to verify in `actx-lm` error mapping during implementation).
- No migration: no schema or index format change. Existing pseudo-vectors are not cleaned automatically; a forced sync removes them (Q2).
- Rollback: revert the commits; a downgrade sees failed files as unindexed and indexes them its own way on the next sync.

## 8. Execution steps and milestones

1. **M1 Ingestion fails the file**: typed embedding failure, no fallback vectors on error, short answers fail, per-file BM25 purge, web page errors; tests.
2. **M2 Breaker and rate limit**: breaker per pass (Q3), one rate-limit retry (Q1); tests.
3. **M3 Reporting**: status summary, CLI output, TUI indicator, status report; tests.
4. **M4 Query outcome (IMP-003)**: pipeline accessor, `/status` and status report lines; tests.
5. **M5 Docs and manual tests**: ADR-0146, architecture pages, README, MT-SYNC-008.
