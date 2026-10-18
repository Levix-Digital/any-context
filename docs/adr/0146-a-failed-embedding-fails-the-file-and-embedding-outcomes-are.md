---
id: 146
title: A failed embedding fails the file, and embedding outcomes are reported
status: accepted
date: '2026-10-18'
version: v0.45.1
subsystems: [ingestion, retrieval, tui, cli]
supersedes: []
superseded_by: []
amends: [138]
amended_by: []
---

# ADR-0146: A failed embedding fails the file, and embedding outcomes are reported

## Context

Up to v0.45.0 embedding failures were invisible at both ends (BUG-014, IMP-003):

- `embed_and_upsert_chunks` (`ingestion/orchestrator.rs`) replaced a failed `provider.embed` call with SHA-256 pseudo-vectors and wrote them to `workspace_chunks` next to real vectors; a short answer was padded with zero vectors. The file was recorded as indexed, so a transient error (rate limit, network) left it out of dense search until it changed on disk. [ADR-0138](0138-record-the-embedding-target-per-workspace-and-reindex-on-change.md) listed this as open.
- `resolve_embedding_provider` always returns a provider (falling back to the mock provider without a key), so the pseudo-vectors were only reached through a failed call.
- The sync loop already leaves a file out of the SQLite stat cache when `embed_and_upsert_chunks` returns an error, so the next sync retries it. Modified files are purged from LanceDB and BM25 before the loop.
- `SyncResult.errors` was collected but never printed, the final sync status always wrote `error = NULL`, and web page errors were discarded. The TUI showed "Up to date" or "Changes detected" after a failed pass.
- `search_single_async` mapped any query embedding error or empty answer to "no dense search" without a trace.

## Decision

- **A failed embedding fails the file.** `embed_and_upsert_chunks` returns `EmbedFailure` (`Provider(LmError)`, `ShortAnswer`, `Store`, `Skipped`). Nothing is written to LanceDB for a failed file; it is not recorded in the stat cache, so the next sync retries it. The `None` provider branch keeps `generate_fallback_vectors`.
- **The file stays reachable by keyword.** Its chunks stay in BM25 (BM25-only hits without a LanceDB row already work, [ADR-0143](0143-bm25-only-hits-take-their-text-from-lancedb-batch-deduplicat.md)). The loop drops a file's BM25 chunks (`remove_by_exact_file`) before re-adding them, so a retry after an edit leaves no orphans.
- **Embedding errors are typed.** The OpenAI-compatible and Gemini `embed` implementations map HTTP errors with `LmError::from_http_status` (401/403 → `Auth`, 429 → `RateLimit`), as chat already did, and read a numeric `Retry-After` header (`with_retry_after`). Before, every embedding error was a `ProviderError`, so neither the breaker's `Auth` rule nor the rate-limit retry could trigger. The Gemini key moves from the `?key=` query string to the `x-goog-api-key` header on every request, because a transport error message carries the request URL and sync errors are now stored and shown (the `system_status` report also reaches the chat model).
- **Rate limits are retried once** after `retry_after_secs` (2 s when absent, at most 30 s).
- **Provider breaker per pass** (`EmbedBreaker`): an `Auth` error opens it at once, otherwise 3 consecutive failed files do (a short answer counts, a success resets the count). While open, files and pages are still chunked into BM25 and fail as `Skipped` without a provider call.
- **Sync failures are reported.** `SyncResult.embedding_failures` counts files and pages that failed; the final status writes `embedding_failure_summary` into `workspace_sync_status.error` ("N files not embedded; they are retried on the next sync; first error: ..."), cleared by a clean pass. `actx --sync` and the sync worker print the errors (first 10); the TUI header and footer show `⚠ N files not embedded (/sync)` (`sync_error_headline`); the `system_status` report shows `LAST SYNC INCOMPLETE` and `/status` a `Last Sync` line. Web page and crawler start-up errors join the sync errors.
- **Query outcome.** `search_single_async` records a `QueryEmbeddingOutcome` (`Dense`, `NoProvider`, `Failed(reason)`) on the pipeline and process-wide (`last_query_embedding_outcome`, `dense_search_status`); `/status` and the `system_status` report show a `Dense Search` line. A failed query still answers from BM25.

## Alternatives Considered

- **Mark the file for re-embedding** instead of failing it: needs new state (a flag in SQLite or a LanceDB column, colliding with the IMP-004 schema change) and still has to choose what to store meanwhile; the stat cache already provides the retry.
- **Keep pseudo-vectors but flag them**: they still compete in dense search as arbitrary vectors.
- **Remove the failed file from BM25 too**: consistent, but the file would be unreachable until the retry.
- **Unlimited retries with backoff**: a bad key or an exhausted quota would stall a sync for minutes; one retry plus the breaker bounds the cost.
- **Clean up stored pseudo-vectors automatically**: needs reading every vector of every workspace; the release notes recommend `/sync --force` instead.

## Consequences

- A transient provider error no longer leaves silent gaps: the file is retried on the next sync and the user sees why it failed.
- A pass with a bad key costs one embedding call; a large first sync on a rate-limited key may leave files to the next sync, reported as such.
- `/status` in a separate process shows `n/a` for dense search: the outcome is per process.
- Pseudo-vectors written by earlier versions stay until the file changes or `/sync --force`.
- Gemini answers an invalid key with HTTP 400 (`ProviderError`), so a Gemini pass with a bad key opens the breaker after 3 files instead of 1.
- Without any embedding key the mock target still produces identical vectors (BUG-019).
- Verified by the `orchestrator.rs` tests (`test_failed_embedding_writes_no_vectors_and_keeps_bm25`, `test_failed_file_is_retried_on_the_next_sync`, `test_new_file_edited_before_retry_leaves_no_orphan_bm25_chunks`, `test_auth_error_opens_the_breaker`, `test_three_consecutive_failures_open_the_breaker`, `test_rate_limit_is_retried_once`, `test_sync_status_error_summary_set_and_cleared`), the `pipeline.rs` query outcome tests, the `actx-lm` provider tests (`embed_maps_rate_limit_with_retry_after_and_auth_errors`, `embed_sends_the_key_in_a_header_and_maps_auth_errors`), and the CLI and TUI tests (`sync_error_lines_list_errors_and_the_retry_note`, `idle_sync_warning_shows_the_headline_of_the_last_sync_error`, `status_report_shows_the_embedding_failures_of_the_last_sync`).
