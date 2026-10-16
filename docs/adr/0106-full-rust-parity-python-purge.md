---
id: 106
title: Full Ingestion and Retrieval Parity in Rust and Purge of Legacy Python Code
status: accepted
date: '2026-10-09'
version: v0.33.0
subsystems: [ingestion, retrieval, ci-release]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0106: Full Ingestion and Retrieval Parity in Rust and Purge of Legacy Python Code

## Context

The temporary coexistence of the Rust core and legacy Python code meant duplicated maintenance, technical debt, dependence on fragile packagers (PyInstaller) and a risk of features drifting apart.

## Decision

1. Port every remaining capability (RAG presets, Office docx/pptx chunker, Contextual Retrieval envelopes, sitemap web crawler, multi-provider web search) to native Rust in the `any-context-core-rs` and `actx-cli` crates.
2. Permanently remove every Python reference, script, test and file from the repository.
3. Align the CI/CD pipeline to a strictly native Rust build.

## Alternatives Considered

Not recorded.

## Consequences

- The whole stack is modern Rust (2021 edition).
- Much smaller repository and much shorter GitHub Actions build time (under 2 minutes).
- No more runtime failures caused by Python interpreters or package incompatibilities.
- Workspace test suite green with 170+ native unit and integration tests.
