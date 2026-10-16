---
id: 104
title: CI/CD Release Pipeline Optimization via Unified Workspace Compilation and Fast Multi-Threaded Packaging
status: accepted
date: '2026-10-08'
version: null
subsystems: [ci-release]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0104: CI/CD Release Pipeline Optimization via Unified Workspace Compilation and Fast Multi-Threaded Packaging

## Context

Release workflows on GitHub Actions Windows runners took over 72 minutes because of duplicated compilation across Maturin and Cargo, single-threaded archive compression and redundant PyInstaller `--onefile` builds.

## Decision

1. Unify all Rust compilation into a single `cargo build --release -p actx-cli -p actx-installer -p any-context-core-rs` invocation that runs before `maturin build`.
2. Speed up archive packaging with multi-threaded `7z -mx=3` and native `tar -a -cf`, with an automatic PowerShell fallback.
3. Drop the legacy `--onefile` build passes and replace the standalone fallback release asset (`actx-windows-x86_64.exe` / `actx-linux-x86_64`) with the lightweight native installer binary.

## Alternatives Considered

Not recorded.

## Consequences

- Windows release time projected to drop from about 1 h 12 min to about 18-20 minutes (around 70% less).
- Linux release time projected to drop from about 11 min 30 s to about 6-7 minutes.
- All 12 release asset URLs and file names remain backward-compatible.
