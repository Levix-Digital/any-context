---
id: 103
title: Robust Multi-Binary Windows Self-Updating, Target Version Delimiters & Sync Indicator Parity
status: accepted
date: '2026-10-08'
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0103: Robust Multi-Binary Windows Self-Updating, Target Version Delimiters & Sync Indicator Parity

- **Status**: Accepted & Implemented.
- **Decision**: Preprocess CLI `@` version delimiters prior to Clap argument validation. Guarantee continuous display of workspace sync status in TUI header and footer. Enforce 15-attempt backoff copy loops on Windows binary replacements and synchronize `~/.cargo/bin/actx.exe`. Route all updater operations to canonical `update.log`.

---
