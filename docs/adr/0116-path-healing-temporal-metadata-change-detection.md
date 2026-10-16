---
id: 116
title: Smart Path Healing, Restored Temporal Metadata Rules (Freshness Cascade and HTTP 304) and Active Source Change Detection
status: accepted
date: '2026-10-10'
version: v0.34.8
subsystems: [storage, ingestion, retrieval, tui]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0116: Smart Path Healing, Restored Temporal Metadata Rules (Freshness Cascade and HTTP 304) and Active Source Change Detection

## Context

1. Paths to Google Drive (`G:/`) or local folders failed to resolve when there were small mount differences (for example an extra `/Documentos/` when the folder is directly at the root of the drive).
2. The "Universal Temporal Recency Rule" in the prompt could not be applied faithfully by the LLM, because the chunks returned by the `search_db` tool dropped the date and time from their headers (`Modified: ...`), although LanceDB stored `last_modified`.
3. The web crawler indexed pages with `mtime: 0.0` (the 1970 UNIX epoch) and had no lightweight change check and no metadata extraction from HTML pages and HTTP.
4. The TUI always showed `✔ Up to date`, even when monitored folders were unreachable or files had been moved, created or deleted.

## Decision

1. **Smart Path Healing engine (`any_context_core_rs::storage::path_healer`)**:
   - A pure function `try_heal_path(p: &Path) -> Option<PathBuf>` tries resolution heuristics: removing and adding `/Documentos/` and `/Documents/`, removing `/OneDrive/`, normalizing `/` and `\` separators, and searching for the leaf directory at the root or in the folder tree of the mounted volume.
   - Transparent self-healing in `NativeConfigDb::get_workspace_folders`: when the stored path does not exist but the folder can be healed, the `workspace_folders` table in SQLite is updated automatically.
   - Validation and auto-resolution in the `/folder` command, with a clear notice to the user.
2. **Restored temporal metadata cascade for the web (`extract_web_metadata`)**, ported to native Rust:
   - Tier 1: HTML meta tags (`article:modified_time`, `dcterms.modified`, `dc.date.modified`, `article:published_time`, `dcterms.issued`), JSON-LD schema.org (`dateModified`, `datePublished`) and `<time datetime="...">`.
   - Tier 2: footer patterns (`Date modified:`, `Last modified:`, `Last updated:`, `Page details`).
   - Tier 3: date patterns in the URL (`/YYYY/MM/DD/`).
   - Tier 4: the `<lastmod>` field of XML sitemaps.
   - Tier 5: the HTTP `Last-Modified` header (RFC 2822 / 7231).
   - Tier 6: the current crawl timestamp `Utc::now()` (never a hard-coded `0.0`).
3. **Lightweight web source check without downloading the body (RFC 7232)**: `check_url_modified` sends HTTP `HEAD` with the conditional headers `If-None-Match` (`ETag`) and `If-Modified-Since` (`Last-Modified`); a `304 Not Modified` answer, evaluated in under 50 ms, confirms `Up to date` without data transfer.
4. **End-to-end `last_modified` propagation and LLM formatting**:
   - Add `pub last_modified: Option<String>` to `DocRecord` (BM25) and `HybridSearchResult`.
   - Inject `| Modified: <timestamp>` into the header of every chunk returned by `search_db`: `• [{file_name}] (Score: {score:.2}, Source: {file_path}, Type: {content_type} | Modified: {timestamp}):`
   - Add a recency micro-boost to the ranking to break score ties in favor of newer versions.
5. **Active change detection and source health in the TUI**:
   - `DiffResult` in `WorkspaceScanner` tracks `missing_folders: Vec<String>`.
   - A periodic lightweight check in the TUI (`check_source_health`) shows `⚠️ [Sync: X folder(s) unreachable]` for unreachable folders, `⚡ [Sync: Changes detected (/sync)]` when changes are detected, and `✔ Up to date` when sources are in sync, in both header and footer.
   - Files that were moved or deleted are purged from LanceDB and BM25 during `/sync`.

## Alternatives Considered

Not recorded.

## Consequences

- No more "folder not found" errors when the folder exists on Google Drive.
- The LLM sees the date and time of every document and applies the recency rule consistently.
- The user gets immediate visual feedback on source health and changes.
