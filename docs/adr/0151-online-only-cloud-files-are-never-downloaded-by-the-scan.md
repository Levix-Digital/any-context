---
id: 151
title: Online-only cloud files are never downloaded by the scan
status: accepted
date: '2026-10-25'
version: v1.0.0
subsystems: [ingestion]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0151: Online-only cloud files are never downloaded by the scan

## Context

OneDrive, Dropbox and iCloud (and Google Drive on macOS) keep online-only files as placeholders. They look like normal files in the folder, but their data is downloaded the moment a program reads them. The scanner put every supported file in the diff, and the ingestion router read it, so syncing a cloud folder downloaded every online-only file without the user asking (BUG-028). That breaks the "explicit action" rule of `docs/architecture/connectors-v2.md` (decision 21).

- Windows (Cloud Files API) marks placeholders whose data is not local with `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS` (0x00400000). Some providers use `FILE_ATTRIBUTE_RECALL_ON_OPEN` (0x00040000) or `FILE_ATTRIBUTE_OFFLINE` (0x00001000). These attributes come from the directory entry, without opening the file.
- macOS (File Provider) marks dataless files with `SF_DATALESS` (0x40000000) in `st_flags`, which `lstat` returns without downloading.
- Linux has no placeholder standard.

## Decision

- `ingestion::scanner::is_online_only(path, metadata)` reads those bits from the metadata the walk already has. The bit tests are the pure helpers `online_only_windows_attributes` and `online_only_macos_flags`, and the function returns false on other platforms.
- `WorkspaceScanner::scan_and_diff_native` never puts an online-only file in `disk_files`, so nothing reads it. It goes into `DiffResult::online_only_files`.
- A cached file that is online-only now (for example after OneDrive's "Free up space") is not deleted: its index is kept, the same as offline sources (ADR-0150). Once the user marks it "Always keep on this device", the next sync indexes it normally.
- `SyncResult::online_only_files` lists the files. The CLI prints `☁ N online-only file(s) not indexed: they stay in the cloud. Mark them "Always keep on this device" to index them.`, and verbose syncs list the paths.
- The check is a probe on `WorkspaceScanner` (`with_online_only_probe`), and `NativeSyncOrchestrator::with_scanner` sets it, so tests can simulate placeholders on any platform.
- There is no setting that allows downloads.

## Alternatives Considered

- **Download and index anyway, then free the space**: rejected. It still downloads data the user never asked for and can fill the disk during the sync.
- **Skip whole cloud folders**: rejected. The files already on disk are the ones the user works with, and they should stay searchable.
- **Delete the index of a file freed to the cloud**: rejected. Its content did not change, and reindexing it later would need the same download.

## Consequences

- Syncing a OneDrive, Dropbox or iCloud folder no longer downloads anything, and the CLI line tells the user how to include more files.
- A file freed to the cloud keeps its last indexed version, which can go stale if it changes in the cloud. It is indexed again once it is kept on the device.
- Google Drive for desktop on Windows mounts a virtual drive that may not set these attributes. Manual test MT-SYNC-011 checks it, and a gap there is a documented limitation.
- Regression tests: `test_online_only_files_are_never_read` and `test_file_freed_to_the_cloud_keeps_its_index` in `crates/any-context-core-rs/tests/orchestrator_tests.rs`, plus the attribute bit unit test in `scanner.rs`.
