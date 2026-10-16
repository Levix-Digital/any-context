---
id: 86
title: Scoped Non-Blocking LanceDB Invocation and Safe Runtime Destruction
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0086: Scoped Non-Blocking LanceDB Invocation and Safe Runtime Destruction

- **Status**: Accepted & Implemented (`v0.32.5`).
- **Context**: Calling synchronous LanceDB methods or dropping `NativeLanceStore` from within async tasks triggered Tokio runtime panics.
- **Decision**: Introduce `safe_block_on` with `std::thread::scope` for Tokio-safe synchronous calls, wrap `search_db` in `tokio::task::spawn_blocking`, and implement custom `Drop` to offload runtime destruction to dedicated OS threads.
