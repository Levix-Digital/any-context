---
id: 105
title: Full Native Rust Ingestion and Sync Pipeline (Deprecation of Python Worker Runtimes)
status: accepted
date: '2026-10-08'
version: v0.32.19
subsystems: [ingestion, cli]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0105: Full Native Rust Ingestion and Sync Pipeline (Deprecation of Python Worker Runtimes)

## Context

AnyContext depended on PyInstaller-packaged Python runtimes for folder scanning, chunking and vector ingestion. This meant a 380 MB distribution payload, 1.5-3 s startup latency, Windows path divergence bugs and domain models duplicated across two languages.

## Decision

1. Deprecate and remove every Python ingestion runtime invocation (`actx-core.exe`, `python main.py`, `unified_sync.py`, `local_folder_ingestor.py`).
2. Implement `NativeSyncOrchestrator` in `crates/any-context-core-rs`, orchestrating differential scanning, tree-sitter AST chunking, Markdown/PDF/tabular parsing, LanceDB Arrow columnar writes and Okapi BM25 disk persistence.
3. Add the `--sync-worker` flag to `actx-cli` for headless background synchronization, spawned directly by the TUI slash commands (`/sync`, `/folder`).
4. Normalize paths to forward slashes (`/`) on every operating system in the SQLite stat cache and in the scanner diff.

## Alternatives Considered

Not recorded.

## Consequences

- No Python runtime is required on user machines.
- Incremental scans of unchanged workspaces take under 30 ms.
- The 380 MB PyInstaller payload is gone.
- `actx --sync` and background workers start instantly.
- Verified by 156 workspace tests.
