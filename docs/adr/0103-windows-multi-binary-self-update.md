---
id: 103
title: Robust Multi-Binary Windows Self-Updating, Target Version Delimiters and Sync Indicator Parity
status: accepted
date: '2026-10-08'
version: v0.32.18
subsystems: [installer, cli, tui]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0103: Robust Multi-Binary Windows Self-Updating, Target Version Delimiters and Sync Indicator Parity

## Context

`actx --update@<version>` was rejected by Clap's argument parser. The native TUI did not show whether the workspace was up to date. On Windows, the copy after renaming the running binary failed intermittently because of antivirus scanning or directory locks, and the rollback left the old version in place. Update runs were not logged.

## Decision

Preprocess `@` version delimiters before Clap validates the arguments. Always show the workspace sync status in the TUI header and footer. Retry Windows binary replacement with a 15-attempt backoff copy loop and also update `~/.cargo/bin/actx.exe`. Write all updater operations to the canonical `update.log`.

## Alternatives Considered

Not recorded.

## Consequences

- A specific version can be installed with `actx --update@<version>`.
- Updates survive transient Windows file locks, and failures can be diagnosed from `update.log`.
