---
plan_id: any_context_sync_and_web_search_reach_outside_without_being_asked_online_only_cloud_files_are_downloaded_and_web_search_scrapes_duckduckgo
project: any-context
title: 'Sync and web search reach outside without being asked: online-only cloud files are downloaded and web search scrapes DuckDuckGo'
kind: bug
version: v1.0.0
status: active
author: Levi Silveira
created_at: '2026-10-24T23:05:00'
updated_at: '2026-10-24T23:06:23'
session_id: null
tags:
- ingestion
- retrieval
- connectors
- r1
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- BUG-028
- BUG-029
milestones:
- name: M1 Online-only files are never read
  done: false
- name: M2 Web search only through allowed APIs
  done: false
- name: M3 Reporting
  done: false
- name: M4 Documentation and manual tests
  done: false
- name: M5 Review and PR
  done: false
status_log:
- at: '2026-10-24T23:05:00'
  by: Levi Silveira
  status: draft
  note: Plan created
- at: '2026-10-24T23:06:23'
  by: Levi Silveira
  status: active
  note: Plan written, waiting for gate 3
---

# Sync and web search reach outside without being asked: online-only cloud files are downloaded and web search scrapes DuckDuckGo

Bug plan for the two R1 bugs where AnyContext reaches outside the device without the user asking (`dev-cycle-protocol` Phases 1 and 2). Both break the "explicit action" and "official APIs only" rules of `docs/architecture/connectors-v2.md` (decisions 21, 24 and 32).

## Backlog items

### BUG-028: [connectors] The scan downloads online-only cloud placeholders
- **Severity**: medium
- **Found**: 2026-10-23, batch 1 of the v2 backlog rebuild
- **Description**: Design: docs/architecture/connectors-v2.md, decision 21. The scanner does not check the online-only attributes of OneDrive, Google Drive or Dropbox placeholders (Cloud Files on Windows, File Provider on macOS) (ingestion/scanner.rs:275-286), and reading the file (ingestion/router.rs:158) forces its download. Violates "explicit action": downloads and disk use the user never asked for. Expected: online-only files are skipped and counted in the source report; files already on disk are indexed normally.

### BUG-029: [retrieval] Web search without a key scrapes DuckDuckGo result pages
- **Severity**: medium
- **Found**: 2026-10-23, batch 1 of the v2 backlog rebuild
- **Description**: Design: docs/architecture/connectors-v2.md, decisions 24 and 32. Without a Tavily or Serper key, web search falls back to DuckDuckGo (retrieval/web_search.rs:114-115): the Instant Answer API, which is not a search API, and then scraping html.duckduckgo.com (web_search.rs:281-293, parser at 318). DuckDuckGo has no official search API, so this breaks the official-APIs-only rule. Expected: web search uses only official APIs, or third-party APIs that are allowed and worth their cost; with none available it says so instead of scraping.

## 1. Triage

### BUG-028
- **Expected vs. actual**: connectors-v2 decision 21 says the scan never downloads an online-only file: it is skipped and counted, and files already on disk are indexed normally. Today the scanner treats a placeholder like any file and the router reads it, which makes OneDrive, Dropbox or iCloud download it.
- **Verdict**: bug. The expected behavior is defined in the design and the backlog.
- **Suspected code paths**: `WorkspaceScanner::scan_and_diff_native` (`ingestion/scanner.rs`), which puts every supported file in `disk_files` from `DirEntry::metadata`; `IngestionRouter::chunk_file_redacted` (`ingestion/router.rs`, `read_to_string` and the extractors' reads), and the document envelope read in `orchestrator.rs`. Each of those reads hydrates a placeholder.
- **Severity**: medium, confirmed. Nothing is lost, but an "index my OneDrive" sync can download gigabytes and fill the disk without the user asking.

### BUG-029
- **Expected vs. actual**: decisions 24 and 32 say web search uses only official APIs or allowed third-party APIs. With no key it should say so. Today `NativeWebSearchEngine::search` falls back to the DuckDuckGo Instant Answer API and then scrapes `html.duckduckgo.com` with a browser user agent (Chrome on Windows).
- **Verdict**: bug, as defined in the design.
- **Suspected code paths**: `retrieval/web_search.rs` (`search`, `search_duckduckgo`, `parse_duckduckgo_html`, `percent_decode`, and the spoofed user agent in `new`), the export in `retrieval/mod.rs`, and the `web_search` tool in `crates/actx-cli/src/engine.rs`, which turns an error into "Web search failed".
- **Severity**: medium, confirmed.

## 2. Reproduction

- **BUG-028** (Windows 11, OneDrive with Files On-Demand): add `C:\Users\<you>\OneDrive\Documents` with online-only files (cloud icon) and run `actx --sync`. Explorer shows the files turning into "available on this device", and disk use grows by their size. The macOS behavior with File Provider is the same: the files lose the cloud badge. The repro needs a real cloud client, so it is a manual test (MT-SYNC-011). The automated regression test simulates the placeholder (section 6).
- **BUG-029**: with no `TAVILY_API_KEY` or `SERPER_API_KEY` and no stored key, run `/web-search on` and ask a question that needs the web. The answer cites DuckDuckGo results, and a proxy log shows requests to `api.duckduckgo.com` and `html.duckduckgo.com/html/?q=…`.

## 3. Evidence and root cause

- **BUG-028, confirmed**: `scan_and_diff_native` never looks at the file attributes. On Windows, the Cloud Files API marks placeholders whose data is not local with `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS` (0x00400000). Some providers use `FILE_ATTRIBUTE_RECALL_ON_OPEN` (0x00040000) or `FILE_ATTRIBUTE_OFFLINE` (0x00001000). `std::os::windows::fs::MetadataExt::file_attributes()` reads them from the directory entry without opening the file. On macOS, File Provider marks dataless files with `SF_DATALESS` (0x40000000) in `st_flags`, which `lstat` returns without hydrating. Linux has no placeholder standard, so nothing changes there.
- **BUG-028, hypothesis to check in the manual test**: Google Drive for desktop on Windows mounts a virtual drive (`G:`) that may not set these attributes, so its streamed files could still be read. If so, it stays a documented limitation; the Drive connector is out of the personal plans (decision 30).
- **BUG-029, confirmed**: `search()` step 3 calls `search_duckduckgo` unconditionally when no key resolves. `search_duckduckgo` calls the Instant Answer API, then scrapes the HTML page and parses it with `parse_duckduckgo_html`, which decodes the `/l/?uddg=` redirects with `percent_decode`.
- **BUG-029, a second finding**: Serper (`google.serper.dev`) returns Google result pages that it collects for resale. Google has no public web search API for this use, and its terms forbid automated queries, so Serper fails the "origin's terms allow it" test of decision 32. Tavily runs its own search index and API under its own terms, so it passes. **This is the decision for Levi below.**

## 4. Fix design (`software-design`)

### BUG-028
- **Detection in the scanner**: a pure function `is_online_only(meta: &Metadata) -> bool` in `ingestion/scanner.rs`. On Windows it checks the three attribute bits; on macOS it checks `SF_DATALESS`; on other platforms it returns false. The bit tests live in two pure helpers, `online_only_windows_attributes(u32)` and `online_only_macos_flags(u32)`, so they are tested on every platform.
- **Injectable probe for tests**: `WorkspaceScanner` gains an `online_only` probe field, `fn(&Path, &Metadata) -> bool`, which defaults to `is_online_only`. `WorkspaceScanner::with_online_only_probe` and `NativeSyncOrchestrator::with_scanner` let the integration test simulate placeholders on Linux CI.
- **Diff rule**: an online-only file never goes into `disk_files`, so it is never read. It goes into a new `DiffResult::online_only_files`. A cached file that is now online-only is not deleted: its index is kept, the same rule as offline paths (ADR-0150). This covers OneDrive's "Free up space", which turns an indexed file back into a placeholder. When the user marks the file "Always keep on this device", it is indexed normally on the next sync.
- **Left out**: no hydration on demand, no setting to allow downloads, and no Google Drive virtual drive detection beyond the attributes.

### BUG-029
- **Remove the scraping**: delete `search_duckduckgo`, `parse_duckduckgo_html`, `percent_decode` and `urlencoding_encode` (if nothing else uses it) and their tests, the DuckDuckGo export in `retrieval/mod.rs`, and the spoofed browser user agent. The client sends `AnyContext/<version>` instead.
- **No provider means no search**: `search()` returns `Err(WebSearchError::NoProvider)` without any request when no allowed key resolves. The `web_search` tool answers `Web search is off: no search API key. Save a Tavily key with /keys tavily <key>.` and the model answers from local documents.
- **`/web-search on` without a key** still turns the setting on, so it works as soon as a key is saved, and adds the same notice.
- **Serper**: decided by Levi (see the card). The recommendation is to remove it with DuckDuckGo, leaving Tavily as the only provider until FEAT-161 picks the Levix-provided search API (Jina is the cost reference).
- **Key resolution for tests**: `NativeWebSearchEngine::with_key_resolver` so tests can run with no keys without touching process environment variables.
- **ADRs**: ADR-0151 "Online-only cloud files are never downloaded by the scan" and ADR-0152 "Web search uses only allowed search APIs and never scrapes" (amends ADR-0106's multi-provider web search).

## 5. Affected surfaces and files

- **Core**: `ingestion/scanner.rs`, `ingestion/orchestrator.rs` (`with_scanner`, `SyncResult::online_only_files`), `retrieval/web_search.rs` and `retrieval/mod.rs`.
- **CLI**: `crates/actx-cli/src/cli/oneshot.rs` (the notice line) and `crates/actx-cli/src/engine.rs` (the `web_search` tool message).
- **Commands**: `commands/engine.rs` (the `/web-search on` notice).
- **Other places with the same defect**: `NativeWebCrawler` already fetches site sources through Jina Reader, an allowed API (ADR-0124), so it is not affected. Nothing else reads placeholders before the scanner.

## 6. Tests (`quality-assurance`)

- **Regression, BUG-028** (`tests/orchestrator_tests.rs`):
  - `test_online_only_files_are_never_read`: a probe marks one file online-only. The test checks that the file is not indexed, `online_only_files` lists it, and the other files are indexed.
  - `test_file_freed_to_the_cloud_keeps_its_index`: index a file, then mark it online-only. The test checks that its chunks are kept.
- **Unit tests, BUG-028**: the Windows attribute and macOS flag helpers for each bit, and plain files.
- **Regression, BUG-029** (`retrieval/web_search.rs`): `search_without_a_key_makes_no_request_and_says_so` uses a resolver that returns no key and a client pointed at an unroutable proxy. It checks that the result is `NoProvider` and that no connection was tried.
- **Unit test, BUG-029**: if Serper stays, the cascade order is Tavily, then Serper.
- **Manual tests**:
  - add MT-SYNC-011, "Online-only cloud files are not downloaded", for OneDrive on Windows and, if available, a File Provider client on macOS;
  - change the web search manual test to "without a key, web search says it is off";
  - add a line on `/web-search on` without a key.

## 7. Documentation (`architecture-docs`, `user-docs`)

- ADR-0151 and ADR-0152.
- `docs/architecture/ingestion.md`: scanning, online-only files and `SyncResult`.
- `docs/architecture/retrieval.md`: the web search section and the file table.
- README:
  - the `TAVILY_API_KEY` row (no DuckDuckGo);
  - the Privacy section (web search only with your key);
  - a sync note on online-only files;
  - a Troubleshooting row for "online-only file(s) not indexed".
- Regenerate the constitution after the ADRs.

## 8. Release, migration and rollback (`devops-release`)

- Ships in v1.0.0 with plans A and B. There is no data migration: files already downloaded by older versions stay on disk (the user can free them in the cloud client), and their index is kept.
- **Behavior change**: users who relied on keyless DuckDuckGo search lose web search until they save a key. The notice says how to fix it.
- **Rollback**: revert the PR. Nothing is stored in a new format.

## 9. Execution steps and milestones

1. **M1 Online-only files are never read**: write the regression tests first and confirm they fail. Then add the probe, the platform detection and `DiffResult::online_only_files`, and keep the index of cached files.
2. **M2 Web search only through allowed APIs**: write the regression test first and confirm it fails. Then remove DuckDuckGo (and Serper, per Levi's decision), add `NoProvider` and the honest user agent, and update the tool and `/web-search on` messages.
3. **M3 Reporting**: add `SyncResult::online_only_files` and the CLI line `☁ N online-only file(s) not indexed: they stay in the cloud. Mark them "Always keep on this device" to index them.` Verbose syncs list the paths.
4. **M4 Documentation and manual tests**: section 7 and the manual tests in section 6. Then run `manual_tests.py validate`, `check_docs.py`, `check_readme.py` and `constitution.py generate`.
5. **M5 Review and PR**: `cargo test --workspace`, a self-review, and a PR from `fix/cloud-placeholders-and-web-search` into dev with the review comment. Follow CI.
