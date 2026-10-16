---
id: 99
title: Full Core Command Parity, Worker Auto-Spawning and Hexagonal Command Engine Alignment
status: accepted
date: '2026-10-06'
version: v0.32.15
subsystems: [commands]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0099: Full Core Command Parity, Worker Auto-Spawning and Hexagonal Command Engine Alignment

## Context

After the Rust migration several commands behaved differently from the Python version: `/folder` and `/web` did not spawn sync workers; `/keys` did not persist credentials; `/config` had no dashboard; `/purge` did not clear the file metadata cache; `/logs` and `/inspect` were incomplete.

## Decision

Centralize worker spawning in `CommandEngine::spawn_sync_worker`; spawn workers automatically when folders and web sources are added; persist credentials in SQLite and in the process environment; and provide a full configuration dashboard, complete inspection and live disk log retrieval in the core command engine.

## Alternatives Considered

Not recorded.

## Consequences

- Every command works the same way from every surface, with all logic in the core.
- Adding a source starts indexing immediately.
