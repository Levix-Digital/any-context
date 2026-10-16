---
id: 101
title: Shadowed Binary Synchronization, User PATH Prioritization and Clean Telemetry
status: accepted
date: '2026-10-08'
version: null
subsystems: [installer]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0101: Shadowed Binary Synchronization, User PATH Prioritization and Clean Telemetry

## Context

On Windows, copies of `actx.exe` placed by Cargo (`~/.cargo/bin`) or other tools often came earlier in `PATH` than `%LOCALAPPDATA%\actx\bin`. `actx --update` updated only the install directory, so the shadowing copy kept running the old version and the update looked broken. The updater output also exposed internal repository names and networking details.

## Decision

In `actx-installer`, inspect `current_exe` and `PATH` during self-update and rename and overwrite every active or shadowed `actx.exe`. Prepend `%LOCALAPPDATA%\actx\bin` to the user `PATH`. Remove internal GitHub repository URLs and networking protocol details from end-user CLI output.

## Alternatives Considered

Not recorded.

## Consequences

- After an update, whichever `actx` the shell resolves is the new version.
- Update messages are customer-facing only.
