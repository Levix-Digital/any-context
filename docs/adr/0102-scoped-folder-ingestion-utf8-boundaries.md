---
id: 102
title: Scoped Directory Ingestion and Multi-Byte UTF-8 Boundary Clamping
status: accepted
date: '2026-10-08'
version: v0.32.17
subsystems: [ingestion, chunkers]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0102: Scoped Directory Ingestion and Multi-Byte UTF-8 Boundary Clamping

## Context

Adding a folder with `/folder <path>` spawned the sync worker without a folder target, so it re-crawled every root of the workspace (one had 268,000 files) and all web sources. Separately, the code splitter panicked when `&remaining[..max_chunk_chars]` cut through a multi-byte UTF-8 character (for example `…`), crashing the worker.

## Decision

Scope the sync triggered by `/folder <path>` to the added folder only, via `--folder <path>`. Restrict stat caching and diffing to the target prefix. Use `safe_floor_char_boundary` for every string slice in the native code splitter.

## Alternatives Considered

Not recorded.

## Consequences

- Adding a folder indexes only that folder.
- Non-ASCII text can no longer crash chunking.
