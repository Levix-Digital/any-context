---
id: 96
title: Bounded BM25 Deserialization, Atomic Temp-Rename and Self-Healing Index Recovery
status: accepted
date: '2026-10-06'
version: v0.32.14
subsystems: [retrieval]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0096: Bounded BM25 Deserialization, Atomic Temp-Rename and Self-Healing Index Recovery

## Context

BM25 binary index files written concurrently ended up with corrupt length headers. Unbounded `bincode` deserialization then requested 7.9 exabytes of RAM and caused unrecoverable out-of-memory aborts.

## Decision

- Bound bincode deserialization to 150 MB.
- Run deserialization inside `panic::catch_unwind`.
- Write index files atomically through a temporary file and an atomic rename.
- Quarantine corrupt index files automatically as `.corrupt.<timestamp>`.
- Fall back gracefully to an in-memory index.

## Alternatives Considered

Not recorded.

## Consequences

- A damaged BM25 file can no longer crash the process; it is set aside and rebuilt.
