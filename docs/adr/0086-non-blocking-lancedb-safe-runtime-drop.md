---
id: 86
title: Scoped Non-Blocking LanceDB Invocation and Safe Runtime Destruction
status: accepted
date: '2026-10-02'
version: v0.32.5
subsystems: [storage, agent]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0086: Scoped Non-Blocking LanceDB Invocation and Safe Runtime Destruction

## Context

Calling synchronous LanceDB methods, or dropping `NativeLanceStore`, from inside async tasks triggered Tokio runtime panics.

## Decision

Introduce `safe_block_on` using `std::thread::scope` for Tokio-safe synchronous calls, wrap `search_db` in `tokio::task::spawn_blocking`, and implement a custom `Drop` that offloads runtime destruction to a dedicated OS thread.

## Alternatives Considered

Not recorded.

## Consequences

- The synchronous storage API can be called from both sync and async contexts without panics.
- Dropping a store never destroys a Tokio runtime from within another runtime.
