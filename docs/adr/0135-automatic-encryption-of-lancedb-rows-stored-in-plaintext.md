---
id: 135
title: Automatic encryption of LanceDB rows stored in plaintext
status: accepted
date: '2026-10-17'
version: v0.45.1
subsystems: [storage, ingestion]
supersedes: []
superseded_by: []
amends: [131]
amended_by: []
---

# ADR-0135: Automatic encryption of LanceDB rows stored in plaintext

## Context

ADR-0131 made every LanceDB write encrypt `text`, `document_summary` and `keywords`, but left the rows written by earlier versions in plaintext until their files were re-indexed (`actx sync --force`). The differential sync never rewrites unchanged files, so those rows stayed plaintext indefinitely, with no message to the user (BUG-001).

A forced re-index did not fix it either. Lance datasets are versioned: a delete only writes deletion markers and an add writes new fragments, so the old data files, with the plaintext, stay on disk, referenced by older versions and by the current fragments' deletion markers. Nothing compacted the table or pruned old versions.

Chunk ids are not unique (`<content hash prefix>_<index>`): the same content in two paths or workspaces has the same id.

## Decision

- `NativeLanceStore::encrypt_plaintext_rows` finds the rows where any protected field is non-empty and does not start with `enc::`, reads them with their vectors in batches of 1,000, and rewrites them encrypted with `merge_insert` keyed on `(id, workspace, file_path)` (update only, never insert). Nothing is re-embedded. Duplicate source keys are dropped before the merge; a batch that encrypts nothing stops the migration with an error instead of looping.
- When no plaintext row is left, the table is compacted with `materialize_deletions_threshold = 0.0` (every fragment with deleted rows is rewritten) and every version except the current one is pruned (`older_than = 0`, `delete_unverified = false`), so the plaintext is physically removed. Files of in-progress writes are never deleted.
- It runs at startup (TUI background task and CLI startup, before the `Global` bootstrap) and at the start of every sync. `SyncResult::chunks_encrypted_legacy` reports the count, the verbose sync prints it, and a failure is added to `SyncResult::errors` and retried on the next pass.
- With no plaintext row it costs one filtered `count_rows`, so no "migration done" flag is stored.

## Alternatives Considered

- **Warn and ask for `actx sync --force`**: rejected; it re-embeds every chunk and still leaves the old versions with the plaintext on disk.
- **An explicit command**: rejected; users would not know they have to run it.
- **Delete and re-add the rows**: rejected; two commits, so a crash between them loses chunks. `merge_insert` is one commit per batch.
- **Merge on `id` alone**: rejected; ids repeat across paths and workspaces.

## Consequences

- Data indexed by any older version is encrypted on disk after the first startup or sync with this version, without user action.
- Pruning removes old dataset versions, so Lance time travel to versions before the migration is no longer possible; the current version keeps every row.
- The result uses the same `enc::` format, which every Rust version reads on the same machine, so rolling back to an older binary keeps working.
- Regular deletes and forced re-indexes still leave old versions on disk until the next migration; general compaction and pruning is separate work. The BM25 index file still keeps chunk text in plaintext (IMP-012).
- Verified by `test_encrypt_plaintext_rows_migrates_legacy_rows` in `storage/lancedb.rs`, which also checks that no file of the table contains the plaintext after the migration (it fails if pruning is skipped), and by `test_sync_encrypts_chunks_stored_in_plaintext` in `ingestion/orchestrator.rs`.
