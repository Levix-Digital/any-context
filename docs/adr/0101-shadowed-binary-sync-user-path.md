---
id: 101
title: Shadowed Binary Synchronization, User PATH Prioritization, and Clean Telemetry
status: accepted
date: '2026-10-08'
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0101: Shadowed Binary Synchronization, User PATH Prioritization, and Clean Telemetry

- **Status**: Accepted & Implemented.
- **Decision**: In `actx-installer`, inspect `current_exe` and `PATH` on self-update to rename and overwrite all active/shadowed instances of `actx.exe`. Prepend `%LOCALAPPDATA%\actx\bin` to User PATH. Strip internal GitHub repository URLs and networking protocols from end-user CLI stdout.

---
