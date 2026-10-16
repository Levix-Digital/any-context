---
id: 75
title: Native Rust ReAct Agent with an Explicit FSM (actx-agent)
status: accepted
date: '2026-09-25'
version: v0.30.38
subsystems: [agent]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0075: Native Rust ReAct Agent with an Explicit FSM (actx-agent)

## Context

LangGraph introduced opaque checkpoints, memory bloat and dependency complexity.

## Decision

Build `actx-agent` as a native Rust crate with an explicit finite state machine loop, no third-party graph frameworks, and PyO3 bindings for backward compatibility with Python callers.

## Alternatives Considered

Keeping the LangGraph-based agent was the status quo being replaced; no other alternative was recorded.

## Consequences

- Memory footprint below 40 MB, turn dispatch latency under 1 ms, fully deterministic turn limits and a clean SQL schema without zlib.
- Existing Python callers kept working through `NativeAgentWrapper` with no regressions (370/370 tests passing).
