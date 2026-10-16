---
id: 104
title: CI/CD Release Pipeline Optimization via Unified Workspace Compilation & Fast Multi-Threaded Packaging
status: accepted
date: '2026-10-08'
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0104: CI/CD Release Pipeline Optimization via Unified Workspace Compilation & Fast Multi-Threaded Packaging

- **Status**: Accepted & Implemented.
- **Context**: Release workflows on GitHub Actions Windows runners took over 72 minutes due to duplicated compilation across Maturin and Cargo, single-threaded archive compression, and redundant PyInstaller `--onefile` builds.
- **Decision**:
  1. Unify all Rust compilation into a single `cargo build --release -p actx-cli -p actx-installer -p any-context-core-rs` invocation preceding `maturin build`.
  2. Accelerate archive packaging using multi-threaded `7z -mx=3` and native `tar -a -cf` with automated PowerShell fallback.
  3. Deprecate legacy `--onefile` build passes, replacing the standalone fallback release asset (`actx-windows-x86_64.exe` / `actx-linux-x86_64`) with the lightweight native installer binary.
- **Consequences**:
  - Windows CI release cycle time projected to drop from ~1h 12m to **~18-20 minutes** (~70% reduction).
  - Linux CI release cycle time projected to drop from ~11m 30s to **~6-7 minutes**.
  - All 12 release asset URLs and filenames remain 100% backward-compatible.

---
