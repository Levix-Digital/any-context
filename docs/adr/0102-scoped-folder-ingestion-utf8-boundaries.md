---
id: 102
title: Scoped Directory Ingestion and Multi-Byte UTF-8 Boundary Clamping
status: accepted
date: '2026-10-08'
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0102: Scoped Directory Ingestion and Multi-Byte UTF-8 Boundary Clamping

- **Status**: Accepted & Implemented.
- **Decision**: Scope `/folder <path>` sync invocations exclusively to the added folder via `--folder <path>`. Restrict stat caching and diffing to the target prefix. Enforce `safe_floor_char_boundary` on all string slices in the native code splitter.

---
