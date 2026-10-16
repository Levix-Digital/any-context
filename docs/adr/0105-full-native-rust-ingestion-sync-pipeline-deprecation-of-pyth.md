---
id: 105
title: Full Native Rust Ingestion & Sync Pipeline (Deprecation of Python Worker Runtimes)
status: accepted
date: '2026-10-08'
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0105: Full Native Rust Ingestion & Sync Pipeline (Deprecation of Python Worker Runtimes)

- **Status**: Accepted & Implemented (`v0.32.19`).
- **Context**: AnyContext previously depended on PyInstaller-packaged Python runtimes for folder scanning, chunking, and vector ingestion. This introduced a 380MB distribution payload, 1.5–3s startup latencies, Windows path divergence bugs, and duplicated domain models across languages.
- **Decision**:
  1. Deprecate and remove all Python ingestion runtime invocations (`actx-core.exe`, `python main.py`, `unified_sync.py`, `local_folder_ingestor.py`).
  2. Implement `NativeSyncOrchestrator` in `crates/any-context-core-rs` orchestrating differential scanning, tree-sitter AST chunking, Markdown/PDF/tabular parsing, LanceDB Arrow columnar writes, and Okapi BM25 disk persistence.
  3. Add the `--sync-worker` CLI flag to `actx-cli` for headless background synchronization spawned directly by TUI slash commands (`/sync`, `/folder`).
  4. Enforce strict forward-slash (`/`) path normalization across all operating systems in SQLite stat cache and scanner diff calculations.
- **Consequences**:
  - Zero Python runtime dependencies required on user machines.
  - Sub-30ms incremental scan passes for unchanged workspaces.
  - Eliminated ~380MB PyInstaller binary payload.
  - Instant cold-start execution for CLI `actx --sync` and background workers.
  - 100% test coverage verified across 156 workspace tests.

---
