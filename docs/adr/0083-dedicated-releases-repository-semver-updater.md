---
id: 83
title: Dedicated Releases Repository and Semver Updater
status: accepted
date: '2026-10-02'
version: v0.32.3
subsystems: [installer]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0083: Dedicated Releases Repository and Semver Updater

## Context

Windows installations stayed on an older version while Linux had moved on, because `actx --check-update` and `actx --update` were stubs that targeted the main source repository instead of the dedicated release repository.

## Decision

Route `actx --check-update` and `actx --update` to `Levix-Digital/any-context-releases`. Compare versions as semantic version tuples to prevent false update alerts when running local development builds.

## Alternatives Considered

Not recorded.

## Consequences

- All platforms update from the same published release assets.
- A development build newer than the latest release is not offered a downgrade.
