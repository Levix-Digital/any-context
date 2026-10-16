---
id: 78
title: Normalized Relational Folder Storage and Legacy Schema Elimination
status: accepted
date: '2026-09-26'
version: v0.32.0
subsystems: [storage]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0078: Normalized Relational Folder Storage and Legacy Schema Elimination

## Context

Reading two tables and synchronizing `workspace_folders` and `workspaces.paths_json` in both directions added maintenance overhead and query complexity, and risked the data drifting apart between interfaces.

## Decision

Make `workspace_folders` the single canonical source of folder sources for every surface (TUI, headless CLI, stdio RPC, MCP and REST API). Migrate existing records once and drop the legacy column.

## Alternatives Considered

Not recorded.

## Consequences

- Clean relational schema with no redundant columns and no two-way synchronization.
- Backward compatibility preserved: legacy databases upgrade on first run.
- The native Rust engine runs with full speed and type safety.
