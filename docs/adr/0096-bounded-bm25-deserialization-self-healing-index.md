---
id: 96
title: Bounded BM25 Deserialization, Atomic Temp-Rename & Self-Healing Index Recovery
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0096: Bounded BM25 Deserialization, Atomic Temp-Rename & Self-Healing Index Recovery

- **Status**: Accepted & Implemented (`v0.32.14`).
- **Context**: Concurrently written BM25 binary index files suffered corrupt length headers, causing unbounded `bincode` deserialization to request 7.9 exabytes of RAM and trigger unrecoverable OOM aborts.
- **Decision**: Impose a 150MB allocation bound on bincode deserialization; enclose deserialization inside `panic::catch_unwind`; enforce atomic file writes via temporary file and atomic rename; quarantine corrupt index files automatically to `.corrupt.<timestamp>`; and provide graceful in-memory fallback index recovery.

---
