---
id: 150
title: Offline sources keep their index and folders are never redirected
status: accepted
date: '2026-10-24'
version: v1.0.0
subsystems: [ingestion, storage]
supersedes: []
superseded_by: []
amends: [116]
amended_by: []
---

# ADR-0150: Offline sources keep their index and folders are never redirected

## Context

The sync pass decides what to delete by comparing the stat cache of the workspace with what the scanner finds on disk. Four bugs came from that comparison and from the path healing of ADR-0116:

- **BUG-022**: a scoped sync (`/sync <folder>`, and the sync started by `/folder --add`) scanned only its folder but compared it with the cache of the whole workspace, so every file of the other folders looked deleted and was purged from LanceDB and BM25.
- **BUG-025**: a folder that could not be read (an unplugged drive, a network share that dropped, a cloud drive not mounted) produced no files, so its whole index was purged. The next sync, once the folder was back, had to read and embed everything again.
- **BUG-026**: a subfolder that could not be read (permissions, a broken mount point inside the tree) was skipped by the walk, and its files were purged in the same way.
- **BUG-027**: `try_heal_path` guessed another existing path for a missing folder (removing or adding `Documentos/`, `Documents/` or `OneDrive/`, or searching for the leaf folder name in the parent or at the drive root) and wrote it back to `workspace_folders`, and a workspace with no folders indexed the current working directory. Both index sources the user never chose, and a guess by folder name can point at a different folder with the same name.

## Decision

- **Scoped diff**: when a sync has a target folder and is not forced, the orchestrator keeps only the cache entries inside that folder (`scanner::is_within_folder`, which compares whole path components, so `/a/docs` does not contain `/a/docs-old`) before the diff. The other folders are not read and not touched.
- **Offline sources keep their index**: `WorkspaceScanner::scan_and_diff_native` reports in `DiffResult::offline_paths` every folder it cannot list (`fs::read_dir` fails) and every subfolder or file the walk cannot read. A cached file is reported deleted only when it is inside a folder the scanner read and not inside an offline path. Offline folders are also listed in `DiffResult::missing_folders`, which no longer makes a workspace "not up to date".
- **Removed folders are purged on a full sync**: a sync without a target folder deletes the cached files that are not inside any registered folder, so `/folder --remove` followed by `/sync` still cleans the index.
- **Folders are never redirected**: `storage/path_healer.rs` is removed. `NativeConfigDb::get_workspace_folders` returns the saved paths as they are, `/folder --add` refuses a path that is not an existing directory (`❌ Directory does not exist: <path>`), and the TUI counts a folder that does not exist as offline. A folder that moved is relocated by the user: `/folder --remove <old>` and `/folder --add <new>`.
- **No implicit source**: a workspace with no registered folder indexes no folder. Its web sources still sync.
- **Reporting**: `SyncResult::offline_folders` lists the offline paths and `SyncResult::no_sources` is true when a workspace has no folder and no web source. On every sync, the CLI prints `⚠ N source(s) offline, their index is kept: <paths>. If a folder moved, remove it with /folder --remove <path> and add the new path.` and `ℹ Workspace '<ws>' has no sources: add a folder with /folder <path> or a site with /web <url>.`; the same offline message is the sync status message, and the TUI header shows `⚠️ N source(s) offline, their index is kept`.

This amends part 1 (Smart Path Healing) and the "unreachable folders" part of part 5 of ADR-0116. The temporal metadata, web change checks and TUI change detection of ADR-0116 are unchanged.

## Alternatives Considered

- **Keep path healing behind a confirmation**: rejected. The heuristics are guesses by folder name, and asking the user to confirm a guess on every sync is worse than asking them to add the new path once.
- **Purge an offline folder after a grace period**: rejected for R1. A drive can stay unplugged for weeks, and the user removes a folder explicitly with `/folder --remove`. A grace period can be added later with its own setting if users ask for it.
- **Fall back to the current working directory with a warning**: rejected. Indexing a directory the user did not choose cannot be undone by a warning, since its text is already sent to the embedding provider.

## Consequences

- Unplugging a drive or losing a share no longer costs a full reindex, and search keeps answering from the last indexed version of those files, which can be stale until the source is back.
- A moved folder is not followed automatically; the offline message tells the user how to relocate it.
- Regression tests in `crates/any-context-core-rs/tests/orchestrator_tests.rs`: `test_scoped_sync_keeps_the_other_folders`, `test_unreachable_folder_keeps_its_index`, `test_unreadable_subfolder_keeps_its_index` (Unix, skipped as root), `test_removed_folder_is_purged_on_full_sync`, `test_missing_folder_is_never_redirected` and `test_workspace_without_sources_indexes_nothing`. Manual tests MT-SYNC-001, MT-SYNC-004 and MT-SYNC-010.
