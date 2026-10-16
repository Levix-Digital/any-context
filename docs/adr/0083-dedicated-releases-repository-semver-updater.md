---
id: 83
title: Dedicated Releases Repository & Semver Updater
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0083: Dedicated Releases Repository & Semver Updater

- **Status**: Accepted & Implemented (`v0.32.3`).
- **Decision**: Route `actx --check-update` and `actx --update` to `Levix-Digital/any-context-releases`. Implement semantic version tuple comparisons to prevent false update alerts when running local development builds.

---
