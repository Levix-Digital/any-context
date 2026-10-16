---
id: 98
title: Parameter Order Inversion Resolution in Native LanceStore (`count_records`) & Live Telemetry Unfreezing
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0098: Parameter Order Inversion Resolution in Native LanceStore (`count_records`) & Live Telemetry Unfreezing

- **Status**: Accepted & Implemented (`v0.32.15`).
- **Context**: An inverted parameter order in `NativeLanceStore::count_records` caused the agent to report 0 indexed chunks across workspaces with tens of thousands of chunks, and the TUI footer remained frozen during long scanning stages.
- **Decision**: Fix the parameter order to `(Some(&ws), Some("workspace_chunks"))`; add live scanning counts to crawler and ingestor progress callbacks; integrate a 250ms Braille spinner in the TUI footer; and implement defensive auto-recalculation in SQLite telemetry.
