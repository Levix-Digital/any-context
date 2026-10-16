---
id: 107
title: Removal of Orphaned Files and Unified Release Assets for Legacy Updaters
status: accepted
date: '2026-10-09'
version: v0.33.1
subsystems: [installer, ci-release]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0107: Removal of Orphaned Files and Unified Release Assets for Legacy Updaters

## Context

The move from a hybrid Python/Rust ecosystem to pure Rust requires existing installations to upgrade without manual intervention and without breaking their update scripts.

## Decision

1. Temporarily keep in the distribution packages a compatibility duplicate (`actx-core.exe` / `actx-core`) identical to the native `actx` binary.
2. In `crates/actx-installer`, silently remove `actx-core.exe` and `_internal` directories when run by versions 0.33.x and later.
3. Remove the last orphaned Python files and consolidate the Dual-Doc standard.

## Alternatives Considered

Not recorded.

## Consequences

- Every legacy user upgrades transparently with a plain `actx --update`.
- Clean repository with no dead dependencies or files.
- No Python in production and no packaging overhead.
