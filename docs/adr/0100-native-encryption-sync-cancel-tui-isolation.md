---
id: 100
title: Native Encryption-at-Rest Interoperability, Background Worker Cancellation, and TUI Screen Isolation
status: accepted
date: '2026-10-08'
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0100: Native Encryption-at-Rest Interoperability, Background Worker Cancellation, and TUI Screen Isolation

- **Status**: Accepted & Implemented (`v0.32.16`).
- **Context**: Encrypted LanceDB payloads remained raw in Rust inspection and retrieval; background sync workers could not be cancelled; and `eprintln!` output corrupted interactive Ratatui prompt buffers.
- **Decision**: Port `SecurityEngine` to native Rust (`NativeSecurityEngine`) with identical PBKDF2 and AES-GCM parameters; add process termination and status reset to `/sync cancel`; introduce `/cd` and `/pwd`; and enforce strict logging isolation away from terminal stderr during interactive sessions.

---
