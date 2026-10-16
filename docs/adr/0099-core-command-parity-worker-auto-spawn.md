---
id: 99
title: 100% Core Command Parity, Worker Auto-Spawning & Hexagonal Command Engine Alignment
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0099: 100% Core Command Parity, Worker Auto-Spawning & Hexagonal Command Engine Alignment

- **Status**: Accepted & Implemented (`v0.32.15`).
- **Context**: Following the Rust migration, several commands had discrepancies compared to Python: `/folder` and `/web` did not auto-spawn workers; `/keys` did not persist credentials; `/config` lacked a dashboard; `/purge` did not clear file metadata cache; `/logs` and `/inspect` had incomplete implementations.
- **Decision**: Centralize worker spawning in `CommandEngine::spawn_sync_worker`; auto-spawn workers on folder and web additions; persist credentials in SQLite and process environment; provide full configuration dashboard, comprehensive inspection, and live disk log retrieval in the core command engine.

---
