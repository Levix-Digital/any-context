---
plan_id: any_context_data_indexed_before_field_encryption_stays_in_plaintext
project: any-context
title: Data indexed before field encryption stays in plaintext
kind: bug
version: v0.45.1
status: planned
author: Levi Silveira
created_at: '2026-10-17T09:25:05'
updated_at: '2026-10-17T09:26:53'
session_id: null
tags:
- storage
- encryption
- lancedb
- migration
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- BUG-001
milestones:
- name: M1 Store migration
  done: false
- name: M2 Triggers
  done: false
- name: M3 Docs and manual tests
  done: false
- name: M4 Review and PR
  done: false
status_log:
- at: '2026-10-17T09:25:05'
  by: Levi Silveira
  status: draft
  note: Plan created
- at: '2026-10-17T09:26:53'
  by: Levi Silveira
  status: planned
  note: Plan updated
---

# Data indexed before field encryption stays in plaintext

Bug plan for BUG-001 (`dev-cycle-protocol` full path: it is a data migration). Target: patch release v0.45.1.

## Backlog items

### BUG-001: Data indexed before field encryption stays in plaintext
- **Severity**: high
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Description**: Chunks written to LanceDB before field-level encryption on every write (ADR-0131) remain unencrypted on disk until the workspace is re-indexed with actx --sync --force. Nothing tells the user, while the README states that indexed text is encrypted.

Related item captured during this planning: IMP-012 (encrypt the BM25 index file at rest), deliberately out of scope.

## 1. Reproduction

1. With v0.44.0 (or any version before ADR-0131), index a folder containing a file with the unique phrase `ZEBRA-CANARY-4711`.
2. Upgrade to v0.45.0, open the TUI and run an incremental `/sync` (the file is unchanged).
3. Search the LanceDB files: `Select-String -Path "$env:LOCALAPPDATA\AnyContext\data\context_db\lancedb\workspace_chunks.lance\data\*" -Pattern "ZEBRA-CANARY-4711" -SimpleMatch`.
4. Result: the phrase is found in plaintext. Running `actx sync --force` and repeating step 3 still finds it (see root cause, fact 3).

No log is involved: the defect is silent by construction (nothing logs or reports plaintext rows).

## 2. Evidence and root cause

Confirmed facts (code read on `dev` at 2a52e90):

1. **Encryption only on the write path.** `NativeLanceStore::upsert_records` (`storage/lancedb.rs:305-326`) encrypts `text`, `document_summary` and `keywords` for records written from v0.45.0 on. `extract_scored_results` (`lancedb.rs:716-728`) decrypts on read, and `NativeSecurityEngine::decrypt_text` passes non-`enc::` values through unchanged, so legacy rows keep working and nobody notices.
2. **Incremental sync never rewrites unchanged files.** The differential sync uses `workspace_files_stat_cache`; unchanged files are skipped, so their chunks are never re-upserted and stay plaintext indefinitely. ADR-0131 explicitly deferred the migration ("Alternatives Considered").
3. **The documented workaround does not remove plaintext from disk either.** LanceDB/Lance datasets are versioned: `delete` writes deletion vectors and `add` writes new fragments, but the old data files (with the plaintext) stay on disk, referenced by older manifests and by the current fragments' deletion vectors. Nothing in the code base calls `Table::optimize` (no compaction, no pruning of old versions), so neither a forced reindex nor any future rewrite physically removes the old plaintext.
4. **Chunk ids are not unique.** The orchestrator builds ids as `"{content_hash[..12]}_{i}"` (`ingestion/orchestrator.rs:556`); the same content in two paths or workspaces yields the same id. Any row rewrite keyed only on `id` would be wrong.
5. **The README overclaims.** README.md:219 says "The indexed text is encrypted"; the BM25 index file `bm25_index.bin` keeps chunk text in plaintext by design (ADR-0131 consequence). Captured as IMP-012; this plan only corrects the wording.

lancedb 0.39.0 (the pinned version) provides `Table::merge_insert(&[..])` with `when_matched_update_all` and `Table::optimize(OptimizeAction::{Compact, Prune { older_than, delete_unverified, error_if_tagged_old_versions }, All})`.

## 3. Fix design (`software-design`)

Hexagonal style: the migration is a storage concern and lives in the store (core); adapters only trigger it.

### Store (core, `storage/lancedb.rs`)

- New `pub fn encrypt_plaintext_rows(&self, table_name: Option<&str>) -> Result<EncryptionMigrationReport, String>` with `pub struct EncryptionMigrationReport { pub rows_encrypted: usize, pub versions_pruned: bool }`.
  1. Returns an empty report if the table does not exist.
  2. Counts legacy rows with one predicate (private `const PLAINTEXT_ROWS_FILTER`): a row is legacy when any of `text`, `document_summary`, `keywords` is non-null, non-empty and `NOT LIKE 'enc::%'`. Zero rows: return immediately (cheap, idempotent no-op; this is the steady state).
  3. Reads the legacy rows in bounded batches (all columns including `vector`), builds the Arrow batch through a shared private helper extracted from `upsert_records` (`build_record_batch(records, dim)`, which already encrypts), and writes each batch with `merge_insert(&["id", "workspace", "file_path"]).when_matched_update_all(None)` (one commit per batch; no `when_not_matched_insert_all`, so it can never add rows). The vector dimension is read from the table schema, not assumed.
  4. Re-counts; only when zero legacy rows remain, runs `optimize(OptimizeAction::Compact {..})` then `optimize(OptimizeAction::Prune { older_than: Some(Duration::zero()), delete_unverified: Some(false), error_if_tagged_old_versions: Some(false) })`, so the old fragments and versions that hold plaintext are physically removed.
- `upsert_records` reuses `build_record_batch`; its behavior does not change.
- Explicit errors: every failure returns `Err(String)` with context (count, read, merge, compact, prune); no partial state is hidden. A failed merge leaves rows plaintext (the previous commit stays valid) and the next run retries; prune runs only after the verification count.

### Triggers (adapters)

- **Startup**: the background task that already runs `ensure_global_knowledge_bootstrap` (`actx-cli/src/tui/mod.rs:65-70`, and the equivalent one-shot/engine startup path) calls `encrypt_plaintext_rows(None)` first, inside the same `spawn_blocking`. Errors are logged, never shown as a crash.
- **Sync**: `NativeSyncOrchestrator::run` calls it once at the start (before scanning) and records the count in a new `SyncResult::chunks_encrypted_legacy: usize`; the sync summary prints "Encrypted N chunks stored before encryption at rest" when N > 0.
- Concurrency: Lance uses optimistic commits; a conflicting concurrent write makes the migration fail cleanly and it retries on the next trigger. Prune uses `delete_unverified: false`, so files of in-flight writes are never removed.

### Deliberately left out

- BM25 file encryption (IMP-012).
- A settings flag recording "migration done": the count query is the source of truth and costs one `count_rows`.
- Periodic compaction/pruning of normal writes and deletes (general storage hygiene); only the migration prunes. If the review shows forced reindexes must also prune to honor the encryption promise, it is a backlog item, not this fix.

### ADR

New ADR-0135 "Automatic migration of plaintext LanceDB rows to encryption at rest", amending ADR-0131 (which rejected automatic migration): in-place `merge_insert` rewrite keyed on `(id, workspace, file_path)` instead of a forced reindex, plus compaction and pruning so the plaintext leaves the disk. Alternatives: warn and require `actx sync --force` (rejected: re-embeds everything and still leaves old versions on disk); explicit command (rejected: users would not know to run it).

## 4. Affected surfaces and files

- `crates/any-context-core-rs/src/storage/lancedb.rs`: `encrypt_plaintext_rows`, `EncryptionMigrationReport`, `build_record_batch`, filter constant, tests.
- `crates/any-context-core-rs/src/storage/mod.rs`: export `EncryptionMigrationReport` if needed.
- `crates/any-context-core-rs/src/ingestion/orchestrator.rs`: call at sync start; `SyncResult::chunks_encrypted_legacy`.
- `crates/actx-cli/src/tui/mod.rs`, `crates/actx-cli/src/engine.rs` (and the one-shot startup in `cli/oneshot.rs` if it bootstraps): startup trigger; sync summary line where `SyncResult` is printed.
- User-visible surface: one new line in the sync summary. No command, flag, file format or setting changes. The on-disk format stays the same (`enc::` payloads already read by v0.45.0).

## 5. Tests (`quality-assurance`)

Automated (`cargo test --workspace`, native runner and CI):

- **Regression test** `test_encrypt_plaintext_rows_migrates_legacy_rows` in `lancedb.rs`: writes rows in plaintext directly through the `lancedb` API (bypassing `upsert_records`), including two rows with the same `id` in different `file_path`s and one row with an empty summary; then:
  - before the fix the assertions fail (plaintext `text LIKE '%canary%'` matches);
  - after `encrypt_plaintext_rows`: report counts the legacy rows; `text LIKE` finds nothing; reads return the original decrypted text, summary and keywords; ids, paths and vectors are unchanged; row count unchanged;
  - no file under the table directory contains the canary bytes (proves compaction + prune);
  - a second call returns `rows_encrypted == 0` (idempotent).
- `test_encrypt_plaintext_rows_missing_table_is_noop`.
- `test_encrypt_plaintext_rows_keeps_encrypted_rows`: rows written by `upsert_records` are not rewritten (count 0, ciphertext unchanged).
- Orchestrator test: a sync over a store seeded with plaintext rows reports `chunks_encrypted_legacy > 0`.

Manual (`tests/MANUAL_TESTS.md`, Privacy and Data):

- New **MT-DATA-004: Data indexed by an older version is encrypted after upgrading**: index the canary file with v0.44.0, install the candidate, open the TUI (no sync), then search the `workspace_chunks.lance` folder (all subfolders, not only `data`) for the canary: no match; `/inspect ZEBRA` still shows the decrypted chunk; a later `/sync` reports nothing to encrypt.
- **MT-DATA-001**: widen the search step to the whole `workspace_chunks.lance` folder.

Acceptance criteria: after upgrading, no plaintext canary anywhere in the LanceDB table folder without any user action; answers and `/inspect` unchanged; no re-embedding.

## 6. Documentation (`architecture-docs`, `user-docs`)

- `docs/architecture/storage.md`, "Encryption at rest": replace "Rows written before encryption was enabled stay plaintext until their source is re-indexed" with the automatic migration (triggers, filter, merge key, compaction and pruning); add `encrypt_plaintext_rows` to the LanceDB bullet list; add ADR-0135 to Decisions.
- `docs/architecture/ingestion.md`: sync start step and `chunks_encrypted_legacy`.
- ADR-0135 (new); `build_index.py` and `check_docs.py` with zero errors.
- README.md:219: state precisely what is encrypted ("The chunk text, summaries and keywords stored in the vector index are encrypted with a key bound to this machine; the keyword search index keeps the words in plaintext."); `check_readme.py` with zero errors.

## 7. Release, migration and rollback (`devops-release`)

- **Version**: patch, v0.45.1 (bug fix, no interface change).
- **Data already affected**: repaired automatically on the first startup or sync after upgrading; no re-embedding, no user action.
- **Backward compatibility**: the result is the same `enc::` format v0.45.0 already reads. v0.44.0 and older also decrypt `enc::` (ADR-0100 engine), so a rollback to an older binary keeps reading the data on the same machine.
- **Irreversibility**: pruning removes old dataset versions, so a Lance-level time travel to pre-migration versions is no longer possible; this is the intent (the plaintext must leave the disk). The data itself is not lost: the current version holds every row.
- **Rollback**: revert the release through a pull request on `main` and ship the next patch; already migrated data stays valid for every version.
- **Observability**: `tracing` info line `encryption migration: encrypted N legacy rows, pruned old versions` with the duration; warn line with the error context on failure; the sync summary line for the user. Covered by the orchestrator test (count) and by the manual test.
- **Security**: no secrets involved; the key derivation is unchanged.

## 8. Execution steps and milestones

1. **M1 Store migration**: branch `fix/legacy-plaintext-encryption` from `dev`; extract `build_record_batch`; implement `encrypt_plaintext_rows` with compaction and pruning; regression and unit tests green.
2. **M2 Triggers**: startup background task and sync start; `SyncResult::chunks_encrypted_legacy` and summary line; orchestrator test green; `cargo test --workspace` green.
3. **M3 Docs and manual tests**: ADR-0135, storage and ingestion pages, README line, MT-DATA-001/004; all checkers with zero errors.
4. **M4 Review and PR**: code review, `software-design` checklist and `check_design.py`; PR into `dev`, CI green; close BUG-001 when merged (`backlog.py close BUG-001 --ref "PR #N"` or plan completion).
5. Release v0.45.1 when the user wants it (version bump PR, `v0.45.1-rc.1`, MT-DATA-001/004 in the run).
