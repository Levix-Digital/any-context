---
id: 91
title: Hermetic Workspace Isolation, Windows Atomic Executable Swap and Production Installer Canonicalization
status: accepted
date: '2026-10-02'
version: v0.32.9
subsystems: [storage, installer, ingestion]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0091: Hermetic Workspace Isolation, Windows Atomic Executable Swap and Production Installer Canonicalization

## Context

Workspace settings leaked across workspaces through global SQLite fallbacks; Windows file locking prevented in-place self-updates of the binary; `/sync` was an instantaneous mock; and the installer contained fragile assumptions about a developer virtual environment.

## Decision

Enforce strict relational workspace columns with automatic schema migrations, remove every global settings fallback, perform Windows self-updates by atomically renaming the running binary to `.exe.old`, route `/sync` to a dedicated background `--sync-worker`, and remove all developer virtual environment assumptions from the production installer binaries.

## Alternatives Considered

Not recorded.

## Consequences

- Each workspace's settings are independent.
- `actx --update` can replace the running executable on Windows.
- Synchronization runs in a separate background process.
