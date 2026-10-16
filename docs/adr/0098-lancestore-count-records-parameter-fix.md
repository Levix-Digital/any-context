---
id: 98
title: Parameter Order Inversion Resolution in Native LanceStore (count_records) and Live Telemetry Unfreezing
status: accepted
date: '2026-10-06'
version: v0.32.15
subsystems: [storage, tui, ingestion]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0098: Parameter Order Inversion Resolution in Native LanceStore (count_records) and Live Telemetry Unfreezing

## Context

An inverted parameter order in `NativeLanceStore::count_records` made the agent report 0 indexed chunks for workspaces with tens of thousands of chunks, and the TUI footer stayed frozen during long scanning stages.

## Decision

Fix the parameter order to `(Some(&ws), Some("workspace_chunks"))`; add live scanning counts to the crawler and ingestor progress callbacks; add a 250 ms Braille spinner to the TUI footer; and add defensive auto-recalculation to the SQLite telemetry.

## Alternatives Considered

Not recorded.

## Consequences

- Chunk counts reported to the agent and the user are correct.
- The footer shows movement during every sync stage.
