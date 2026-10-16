---
id: 100
title: Native Encryption-at-Rest Interoperability, Background Worker Cancellation and TUI Screen Isolation
status: accepted
date: '2026-10-08'
version: v0.32.16
subsystems: [storage, commands, tui]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0100: Native Encryption-at-Rest Interoperability, Background Worker Cancellation and TUI Screen Isolation

## Context

Encrypted LanceDB payloads stayed raw (undecrypted) in Rust inspection and retrieval; background sync workers could not be cancelled; and `eprintln!` output corrupted the interactive Ratatui prompt.

## Decision

Port `SecurityEngine` to native Rust (`NativeSecurityEngine`) with identical PBKDF2 and AES-GCM parameters; make `/sync cancel` terminate the worker process and reset its status; add `/cd` and `/pwd`; and keep all logging away from terminal stderr during interactive sessions.

## Alternatives Considered

Not recorded.

## Consequences

- Data encrypted by earlier versions remains readable.
- A running sync can be stopped from the interface.
- Log output never draws over the TUI.
