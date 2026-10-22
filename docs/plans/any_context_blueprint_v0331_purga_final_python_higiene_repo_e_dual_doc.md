---
plan_id: any_context_blueprint_v0331_purga_final_python_higiene_repo_e_dual_doc
project: any-context
title: 'Blueprint v0.33.1: Final Purge of the Python Legacy, Repository Hygiene and Full Dual-Doc Update'
version: v0.33.1
status: superseded
author: Levi Silveira
created_at: '2026-10-09T17:07:42'
updated_at: '2026-10-09T18:15:14'
session_id: 4f45b7d4-9ceb-4257-9375-7a0a27e74b3e
tags:
- rust
- purge
- cleanup
- hygiene
- dual-doc
- v0.33.1
supersedes: null
superseded_by: any_context_blueprint_fase2_rfc042_deep_search_engine_rust_native
parent_plan_id: null
milestones:
- name: 'M1: Removal of Python Code and Orphaned Artifacts'
  done: false
- name: 'M2: gitignore and Environment Sanitization'
  done: false
- name: 'M3: Dual-Doc README and TECDOC Update to 100% Pure Rust'
  done: false
- name: 'M4: Scenario 22 Recorded in MANUAL_TESTS.md'
  done: false
- name: 'M5: Rust Test Suite 100% Green'
  done: false
- name: 'M6: v0.33.1 Versioning, Release CI Watch and Publication'
  done: false
status_log:
- at: '2026-10-09T18:15:14'
  note: Superseded by any_context_blueprint_fase2_rfc042_deep_search_engine_rust_native
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.33.1: Final Purge of the Python Legacy, Repository Hygiene and Full Dual-Doc Update

## 1. Context and Architecture Decision (ADR)
With release v0.33.0 published and PyO3 fully removed from any-context-core-rs, AnyContext runs 100% in native Rust. This plan performs the final cleanup of the last orphaned Python files (config/app_settings.py), removes leftover folders from disk, sanitizes .gitignore and rewrites the documentation sections (README.md and TECDOC.md) that still mentioned Python runtimes, PyO3, pip or _internal pre-extractions.

## 2. Parity and Architecture Scope
- **Zero Python Runtime**: no Python dependency in the production monorepo.
- **Single Source of Truth**: settings managed only by NativeConfigDb in SQLite.
- **Rigorous Dual-Doc**: detailed README and TECDOC update reflecting the pure-Rust monorepo architecture.

## 3. Execution Milestones
- **M1**: Removal of config/app_settings.py and of leftover local folders (.pytest_cache, .langgraph_api, build/, .coverage).
- **M2**: .gitignore sanitized for the Cargo/Rust-only profile and local databases.
- **M3**: Complete Dual-Doc update in README.md and TECDOC.md.
- **M4**: Cumulative, non-truncated recording of Scenario 22 in tests/MANUAL_TESTS.md.
- **M5**: Workspace test suite run (cargo test --workspace) with 100% success.
- **M6**: Cycle closure, v0.33.1 versioning, uninterrupted CI/CD follow-up and asset publication.
