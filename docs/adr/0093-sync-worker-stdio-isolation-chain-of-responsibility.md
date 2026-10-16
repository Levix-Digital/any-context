---
id: 93
title: Sync Worker Background Stdio Isolation and Chain of Responsibility Ingestion Engine
status: accepted
date: '2026-10-04'
version: v0.32.11
subsystems: [ingestion, cli]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0093: Sync Worker Background Stdio Isolation and Chain of Responsibility Ingestion Engine

## Context

Background sync child processes wrote raw stdout/stderr into the terminal while the TUI was rendering, and workspaces that only had web sources produced false-positive "local folder" error alerts.

## Decision

Redirect child process stdio to dedicated per-workspace log files, with `CREATE_NO_WINDOW` on Windows, and refactor multi-source ingestion into a Chain of Responsibility in which each handler independently checks whether its source type is present and skips gracefully, without error markers, when it is not.

## Alternatives Considered

Not recorded.

## Consequences

- Background synchronization never corrupts the interactive screen.
- A workspace with only web sources (or only folders) syncs without spurious errors.
