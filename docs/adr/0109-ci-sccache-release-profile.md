---
id: 109
title: CI/CD Acceleration with Sccache and Release Profile Optimizations
status: accepted
date: '2026-10-09'
version: v0.34.1
subsystems: [ci-release]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0109: CI/CD Acceleration with Sccache and Release Profile Optimizations

## Context

A clean build of the Rust workspace (LanceDB, Apache Arrow, DataFusion and the C-based Tree-sitter grammars) took about 50 minutes on the GitHub Actions Windows runner, slowing down the release cycle.

## Decision

1. Configure `mozilla-actions/sccache-action@v0.0.11` with `SCCACHE_GHA_ENABLED: "true"` and `RUSTC_WRAPPER: "sccache"` in the `release.yml` and `e2e-tests.yml` workflows.
2. Set `swatinem/rust-cache` to `cache-targets: "false"`, caching only the crate registry.
3. Grant the workflows the explicit `actions: write` permission.
4. Add a `[profile.release]` to `Cargo.toml` with `opt-level = 3`, `codegen-units = 16`, `lto = "off"`, `strip = true` and `debug = false`.

## Alternatives Considered

Not recorded.

## Consequences

- External dependencies are no longer recompiled after version bumps.
- Much shorter pipeline run times on GitHub Actions.
- Cache effectiveness is visible in the logs through `${SCCACHE_PATH} --show-stats`.
