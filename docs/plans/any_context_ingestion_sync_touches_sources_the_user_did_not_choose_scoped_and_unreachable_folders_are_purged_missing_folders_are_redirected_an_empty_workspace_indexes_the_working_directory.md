---
plan_id: any_context_ingestion_sync_touches_sources_the_user_did_not_choose_scoped_and_unreachable_folders_are_purged_missing_folders_are_redirected_an_empty_workspace_indexes_the_working_directory
project: any-context
title: '[ingestion] Sync touches sources the user did not choose'
kind: bug
version: v1.0.0
status: active
author: Levi Silveira
created_at: '2026-10-24T20:07:24'
updated_at: '2026-10-24T20:09:18'
session_id: null
tags:
- ingestion
- sync
- r1
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- BUG-022
- BUG-025
- BUG-026
- BUG-027
milestones:
- name: M1 Scoped and reachable diff
  done: false
- name: M2 No guessing and no implicit source
  done: false
- name: M3 Reporting
  done: false
- name: M4 Documentation and manual tests
  done: false
- name: M5 Review and PR
  done: false
status_log:
- at: '2026-10-24T20:07:24'
  by: Levi Silveira
  status: draft
  note: Plan created
- at: '2026-10-24T20:09:18'
  by: Levi Silveira
  status: active
  note: Plan updated
---

# [ingestion] Sync touches sources the user did not choose

Bug plan for BUG-022, BUG-025, BUG-026 and BUG-027 (all high), group B of the R1 bugs of release v1.0.0 Personal. The four bugs share one cause: the scan and the diff of `WorkspaceScanner::scan_and_diff_native` and `NativeSyncOrchestrator::run` decide which files exist from something other than the sources the user explicitly chose and can reach. Discovery and planning follow `dev-cycle-protocol` Phases 1 and 2.

## Backlog items

### BUG-022: [ingestion] Folder-scoped sync purges the other folders' index
- **Severity**: high
- **Description**: A sync with a target folder scans only that folder (ingestion/orchestrator.rs) but diffs it against the stat cache of the whole workspace, so every file of the other folders counts as deleted and its chunks are purged. Adding a folder starts such a sync (commands/engine.rs), so adding a second folder silently empties the index of the first. A fix (`is_within_folder` in scanner.rs, with a regression test) exists only on the unmerged branch feat/imp-004-embedding-model (commit 4042b89). Expected: a scoped sync only touches files inside its folder.

### BUG-025: [ingestion] An unreachable folder's files are purged from the index
- **Severity**: high
- **Description**: A missing folder is skipped (ingestion/scanner.rs), so its cached files count as deleted and their chunks are purged; the warning comes after the purge and only in verbose mode. Walk errors are dropped too (`filter_map(|e| e.ok())`), so a subfolder without read permission is purged the same way. Violates "nothing disappears silently". Expected: an unreachable folder or subfolder is reported as offline, its index is kept, and it is reconciled when it comes back.

### BUG-026: [ingestion] Path healer silently redirects folders and rewrites the saved path
- **Severity**: high
- **Description**: When a folder is missing, storage/path_healer.rs guesses another one; its last step takes any folder with the same name in the grandparent, so a missing `/a/projX/docs` becomes an unrelated `/a/docs`. The scan uses the guess (ingestion/scanner.rs) and `get_workspace_folders` writes it over the saved folder (storage/sqlite.rs), with no message. Content the user never chose can be indexed and sent to a cloud model. Violates "explicit action" and "nothing disappears silently". Expected: no automatic redirection; a missing folder is reported and the user relocates it explicitly.

### BUG-027: [ingestion] Sync indexes the working directory when a workspace has no folders
- **Severity**: high
- **Description**: When a workspace has no folders, sync indexes the process's current directory (ingestion/orchestrator.rs), which can be the home folder, and its content can reach a cloud embedding model. Violates "every source comes from an explicit user action". Expected: a workspace without sources indexes nothing and says so.

## 1. Triage

- **Expected**: `docs/architecture/ingestion-pipeline-v2.md`, section 1 "Source selection" (every source comes from an explicit user action) and section 3 "Incremental diff" (an unreachable source goes offline, it is not deleted; no silent path healing; nothing disappears silently).
- **Verdict**: four bugs. Each one deletes indexed content or indexes content the user did not choose, with no message.
- **Severity**: high confirmed for all four. BUG-022 and BUG-025 lose the user's index silently; BUG-026 and BUG-027 can send unchosen content to a cloud model.
- **Code paths** (confirmed in section 3): `ingestion/scanner.rs` `scan_and_diff_native` (missing folder skip, `filter_map(|e| e.ok())`, metadata failures, the healer call); `ingestion/orchestrator.rs` `run` steps 2 and 6 (folder list, stat cache); `storage/sqlite.rs` `get_workspace_folders` (healer and rewrite of the saved path); `storage/path_healer.rs`; `commands/engine.rs` `/folder` add (healer at add time); `actx-cli/src/tui/app.rs` `check_source_health` (healer in the health check).

## 2. Reproduction

On `dev` 2a53a16, each with the `mock` target in a sandbox (the regression tests of section 6 do exactly this and must fail on `dev` before the fix):

1. **BUG-022**: a workspace with folders `alpha` and `beta`, a full sync, then a sync scoped to `alpha` (what `/folder <path>` starts): `beta`'s files are reported as deleted and its chunks and stat cache entries are gone.
2. **BUG-025**: index a folder, rename it (or remove read permission from one of its subfolders), sync: its chunks are purged; the warning is printed only with `verbose`, after the purge.
3. **BUG-026**: register `/tmp/x/projX/docs`, index it, delete it and create `/tmp/x/docs`: the next `get_workspace_folders` returns `/tmp/x/docs`, rewrites the saved path, and the sync indexes it.
4. **BUG-027**: a workspace with no folder, sync from a directory with files: those files are indexed.

## 3. Evidence and root cause

- `scanner.rs:265-273`: a folder that does not exist is passed to `try_heal_path`; when no guess is found it goes to `missing_folders` and is skipped, but its cached files stay in `cached_files`, so the diff at `scanner.rs:308-312` lists them in `deleted_files`.
- `scanner.rs:276`: `walker.filter_map(|e| e.ok())` drops walk errors (a subfolder without permission, an I/O error) and `entry.metadata()` failures skip the file, so those files are missing from `disk_files` and count as deleted too.
- `orchestrator.rs` step 6: the stat cache of the **whole workspace** is diffed against a scan of **one** folder when `target_folder` is set.
- `orchestrator.rs` step 2: no folders falls back to `std::env::current_dir()`.
- `path_healer.rs:17-69`: candidates are tried in order and the last one (same name in the grandparent) matches unrelated folders. It is called from the scan, from `get_workspace_folders` (which also writes the guess over the saved folder), from `/folder` add and from the TUI health check.

**Root cause**: the diff treats "not seen in this scan" as "deleted", without knowing which part of the workspace the scan covered or could reach, and the folder list is not limited to what the user registered.

## 4. Fix design (`software-design`)

### Scan scope and reachability (BUG-022, BUG-025)

`WorkspaceScanner::scan_and_diff_native` keeps its signature and gains two facts in `DiffResult`:

- `missing_folders` (existing): registered folders that do not exist or cannot be read at their root.
- `offline_paths` (new): the missing folders plus every directory or file under a reachable folder that the walk could not read (walk errors and metadata failures, with the path from the error).

A cached file is listed in `deleted_files` **only** when it lies inside a folder that was scanned and is not inside any offline path (`is_within_folder`, salvaged from commit 4042b89 with its unit test). A cached file outside every scanned folder is left alone by the scanner; the orchestrator decides what it means:

- **Scoped sync** (BUG-022): `run` passes only the target folder's cache entries to the scanner (the 4042b89 fix: `cached.retain(|p, _| is_within_folder(p, tf))`), so files of the other folders are never compared.
- **Full sync**: cached files outside every **registered** folder belong to a folder the user removed, and are purged as today (that is how removing a folder cleans its index). The orchestrator computes them from the registered folder list, not from the scan.

An offline folder keeps its chunks and stat cache, so when it comes back the normal diff reconciles it (new, modified and deleted files as usual). `SyncResult` gains `offline_folders: Vec<String>` (missing folders and unreadable subfolders), printed by every CLI sync (not only verbose) as `⚠ N source(s) offline, their index is kept: <paths>`, and written to the sync status so the TUI and `/status` show it. The TUI footer already shows `⚠️ N folder(s) unreachable` from `check_source_health`.

### No guessing (BUG-026)

`storage/path_healer.rs` is deleted with its tests, and its four callers change:

- the scan reports a missing folder as offline (above);
- `get_workspace_folders` returns the saved paths as they are and never writes them;
- `/folder <path>` with a path that does not exist answers `❌ Directory does not exist: <path>` (the existing error) instead of adding a guess;
- the TUI health check counts folders that do not exist.

Relocation stays explicit with the existing commands: `/folder --remove <old>` and `/folder <new>`. The offline message says so. A one-step relocate command is not part of this fix.

### No implicit source (BUG-027)

With no registered folder, `run` scans nothing: the step-2 fallback to the current directory is removed. A workspace with no folders and no web sources returns an up-to-date result with `no_sources: true`; the CLI prints `ℹ Workspace '<ws>' has no sources: add a folder with /folder <path> or a site with /web <url>.` Web sources keep working without folders. `actx --sync --folder .` stays valid: it is an explicit target folder.

### Deliberately left out

- A relocate command and an offline state per source in the UI (the source report view is FEAT-184, R1; source states for connectors are FEAT-156).
- Cloud placeholders (BUG-028, plan C).
- Changes to rename detection (it only pairs deleted and new files, which are now scoped correctly).

## 5. Affected surfaces and files

- Core: `ingestion/scanner.rs` (`is_within_folder`, `DiffResult::offline_paths`, deletion scope, walk and metadata errors), `ingestion/orchestrator.rs` (scoped cache, removed-folder purge, no cwd fallback, `SyncResult::offline_folders` and `no_sources`, sync status message), `storage/sqlite.rs` (`get_workspace_folders` without healing), `storage/path_healer.rs` and `storage/mod.rs` (removed), `commands/engine.rs` (`/folder` add without healing).
- Surfaces (`cli`, `tui`): CLI sync summary lines; TUI health check without healing. No command, flag or file format changes; `/folder` with a missing path now fails instead of guessing.
- Public API: `storage::try_heal_path` and `storage::path_healer` are removed (only internal callers).

## 6. Tests (`quality-assurance`)

Regression tests in `crates/any-context-core-rs/tests/orchestrator_tests.rs`, written first and failing on `dev`:

- `test_scoped_sync_keeps_the_other_folders` (BUG-022, salvaged from 4042b89): two folders, full sync, sync scoped to the first: no deletion, same chunk count and stat cache.
- `test_unreachable_folder_keeps_its_index` (BUG-025): index two folders, rename one, sync: `deleted_files == 0`, its chunks and cache stay, `offline_folders` names it; rename it back and add a file, sync: only the new file is indexed.
- `test_unreadable_subfolder_keeps_its_index` (BUG-025, Unix only, skipped when running as root, where permissions are not enforced): remove read permission from a subfolder, sync: its files stay indexed and the subfolder is listed offline.
- `test_removed_folder_is_purged_on_full_sync`: removing a folder from the workspace still purges its files (no behavior lost).
- `test_missing_folder_is_never_redirected` (BUG-026): register `<tmp>/projX/docs`, index, delete it and create `<tmp>/docs` with a file: the saved folder path is unchanged, `<tmp>/docs` is not indexed, the folder is reported offline.
- `test_workspace_without_sources_indexes_nothing` (BUG-027): no folders, sync with the process in a directory with files: nothing indexed, `no_sources` is true.

Unit tests: `scanner.rs` (`is_within_folder` on whole components; deletion only inside scanned, reachable folders); the `/folder` command with a missing path (`command_tests.rs`); the CLI summary lines.

Manual (`tests/MANUAL_TESTS.md`): rewrite **MT-SYNC-004** as "Unreachable and moved folders keep their index": rename an indexed folder, sync, check the offline message and that answers still use it; rename it back, sync, check it is reconciled; register a path that does not exist and check it is refused. Extend **MT-SYNC-001** with "adding a second folder keeps the first folder's index". New **MT-SYNC-010** "A workspace without sources indexes nothing".

## 7. Documentation (`architecture-docs`, `user-docs`)

- `docs/architecture/ingestion.md`: sync pass steps 2 and 4, "Scanning and change detection" (scope of deletions, offline paths, no healing), `SyncResult` fields.
- `docs/architecture/storage.md`: remove path healing.
- New ADR-0150 "Offline sources keep their index and folders are never redirected": supersedes the path-healing part of ADR-0116 (marked amended).
- `README.md`: replace "Folders that moved slightly ... are found again automatically" with the offline behavior and explicit relocation; a Troubleshooting row for "N source(s) offline". Then `build_index.py`, `check_docs.py`, `check_readme.py`.

## 8. Release, migration and rollback (`devops-release`)

- **Version impact**: behavior fix inside v1.0.0; no bump in this PR.
- **Data already affected**: chunks purged by these bugs cannot be restored; they come back with the next sync of their folder once it is reachable, at the cost of one embedding pass for those files. Folder paths that the healer already rewrote in `workspace_folders` cannot be told apart from paths the user saved; they stay as they are, and the user removes them if they are wrong (noted in the release notes). No schema change.
- **Rollback**: revert the PR on `dev`; the stat cache and folders are unchanged in format, so older versions keep working.
- **Observability**: `offline_folders` and `no_sources` in `SyncResult`, CLI lines on every sync, the sync status message for the TUI and `/status`, verbose lines with the offline paths and the walk error kind.

## 9. Execution steps and milestones

1. **M1 Scoped and reachable diff**: `is_within_folder` (salvaged), `offline_paths`, deletion scope, scoped cache, removed-folder purge; BUG-022 and BUG-025 regression tests.
2. **M2 No guessing and no implicit source**: remove the path healer and its callers, no cwd fallback, `no_sources`; BUG-026 and BUG-027 regression tests.
3. **M3 Reporting**: `SyncResult::offline_folders`, CLI lines, sync status message; unit tests.
4. **M4 Documentation and manual tests**: ingestion and storage pages, ADR-0150, README, MT-SYNC-001/004/010.
5. **M5 Review and PR**: `software-design` checklist and `check_design.py`, PR `fix/ingestion-source-safety` into `dev`, protocol review, CI green.
