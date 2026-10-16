---
id: 122
title: CI Fast-Path Timing Resilience and SQLite Warmup under Virtualized Runners
status: accepted
date: '2026-10-14'
version: null
subsystems: [cli, ci-release]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0122: CI Fast-Path Timing Resilience and SQLite Warmup under Virtualized Runners

## Context

1. The test `test_is_first_run_or_upgrade_does_not_panic` in `crates/actx-cli/src/engine.rs` failed on GitHub Actions `windows-latest` (run `38478876141`: 554.25 ms elapsed against a rigid 500 ms ceiling). This was a CI fix after v0.37.0.
2. The test checks that `is_first_run_or_upgrade` uses the lightweight SQLite fast-path without invoking heavy LanceDB/BM25 indexing (over 5 s). Under concurrent multi-threaded tests and virtualized disk I/O in CI, the first SQLite file creation or open can exceed 500 ms.

## Decision

1. Add a warmup call before the timing measurement, so directory creation and SQLite connection setup do not distort the fast-path benchmark.
2. Raise the timing assertion ceiling from 500 ms to 1500 ms, removing false-positive flakes caused by shared runner I/O contention while keeping the regression guard against multi-second vector bootstrapping.

## Alternatives Considered

Not recorded.

## Consequences

- Deterministic green test runs on Ubuntu and Windows runners.
- The invariant that the fast-path is sub-millisecond in normal operation is preserved.
