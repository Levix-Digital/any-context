---
id: 91
title: Hermetic Workspace Isolation, Windows Atomic Executable Swap & Production Installer Canonicalization
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0091: Hermetic Workspace Isolation, Windows Atomic Executable Swap & Production Installer Canonicalization

- **Status**: Accepted & Implemented (`v0.32.9`).
- **Context**: Workspace settings leaked across workspaces via global SQLite fallbacks; Windows file locking prevented in-place binary self-updates; `/sync` was an instantaneous mock; and installer code contained fragile developer venv assumptions.
- **Decision**: Enforce strict relational workspace columns with automated schema migrations, eliminate all global settings fallbacks, execute Windows self-updates via atomic `.exe.old` renaming, route `/sync` to a dedicated background `--sync-worker`, and eradicate all developer virtual environment assumptions from production installer binaries.

---
