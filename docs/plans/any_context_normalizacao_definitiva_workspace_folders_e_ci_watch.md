---
plan_id: any_context_normalizacao_definitiva_workspace_folders_e_ci_watch
project: any-context
title: Definitive workspace_folders Normalization and CI Watch
version: v0.32.0
status: superseded
author: Levi Silveira
created_at: '2026-09-26T11:24:18'
updated_at: '2026-10-04T13:28:33'
session_id: 9f488dbf-db54-4ecb-8ed2-b7a6ac1b2372
tags:
- sqlite
- migration
- workspace_folders
- dev-cycle-protocol
- ci-watch
supersedes: null
superseded_by: any_context_blueprint_v03210_isolamento_hermetico_de_buffers_de_chat_e_historico_por_workspace_na_tui_nativa
parent_plan_id: null
milestones:
- name: 'M1: Dev-Cycle Protocol CI Rule'
  done: true
- name: 'M2: Version Bump v0.32.0'
  done: true
- name: 'M3: SQLite Migration to workspace_folders'
  done: true
- name: 'M4: Drop Legacy paths_json Column'
  done: true
- name: 'M5: Pure Single Table Folder Methods'
  done: true
- name: 'M6: Automated Tests'
  done: true
- name: 'M7: Dual-Doc & Manual Tests'
  done: true
- name: 'M8: Production Release & CI Watch'
  done: true
status_log:
- at: '2026-09-26T13:52:06'
  note: 'Milestone ''M1: Dev-Cycle Protocol CI Rule'': added Pillar 7 and strict CI and SemVer follow-up rules to the dev-cycle-protocol'
- at: '2026-09-26T13:53:09'
  note: 'Milestone ''M2: Version Bump v0.32.0'': every manifest updated to v0.32.0 (Cargo.toml, pyproject.toml, version.txt)'
- at: '2026-09-26T13:56:26'
  note: 'Milestone ''M3: SQLite Migration to workspace_folders'': data migration from workspaces.paths_json to workspace_folders implemented'
- at: '2026-09-26T13:56:31'
  note: 'Milestone ''M4: Drop Legacy paths_json Column'': legacy paths_json column removed through ALTER TABLE DROP COLUMN'
- at: '2026-09-26T13:56:34'
  note: 'Milestone ''M5: Pure Single Table Folder Methods'': get, add and remove_workspace_folder simplified to work only on the workspace_folders table'
- at: '2026-09-26T13:56:38'
  note: 'Milestone ''M6: Automated Tests'': 125 unit and integration tests green (0 failures) in cargo test --workspace'
- at: '2026-09-26T13:59:24'
  note: 'Milestone ''M7: Dual-Doc & Manual Tests'': TECDOC.md (Section 78 & ADR-078) and MANUAL_TESTS.md (Scenario 3 v0.32.0) updated'
- at: '2026-10-02T23:03:46'
  note: 'Milestone ''M8: Production Release & CI Watch'': release v0.32.9 published successfully in Levix-Digital/any-context-releases'
- at: '2026-10-04T13:28:33'
  note: Superseded by any_context_blueprint_v03210_isolamento_hermetico_de_buffers_de_chat_e_historico_por_workspace_na_tui_nativa
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Architecture Blueprint: Definitive Migration and Normalization of workspace_folders (v0.32.0)

## 1. 🎯 Goal & Architecture Decisions (ADR)
- **ADR-032-01: Relational `workspace_folders` Table as the Single Source of Truth**:
  - Remove the double-sync overhead and data redundancy between the legacy `workspaces.paths_json` column and the relational `workspace_folders` table.
  - Atomically migrate 100% of the existing records in `workspaces.paths_json` to the normalized `workspace_folders` table.
  - Physically remove the legacy column (`ALTER TABLE workspaces DROP COLUMN paths_json`), guaranteeing strict relational integrity and removing technical debt from the Python -> Rust transition.
- **ADR-032-02: Inviolable CI/CD Rule in the `dev-cycle-protocol`**:
  - Add to the Pillars and Stage 8 of the `dev-cycle-protocol` the requirement that GitHub Actions CI and Release workflows be followed without interruption until the binaries are published and everything is complete.

## 2. 🏛️ Surface Parity Mapping
- **Core Domain & Storage (`crates/any-context-core-rs`)**:
  - Automatic migration run in the `ensure_tables()` and `migrate_legacy_paths_json_if_needed()` methods.
  - The `get_workspace_folders`, `add_workspace_folder` and `remove_workspace_folder` methods work only on `workspace_folders`.
- **Presentation Adapters (CLI, TUI, Stdio RPC, MCP, REST API)**:
  - They consume the Core transparently, receiving the normalized folder lists without any change to their contracts.
- **Governance Protocol (`dev-cycle-protocol`)**:
  - Update of the `C:\Users\guilh\.gemini\config\skills\dev-cycle-protocol\SKILL.md` file with the mandatory rule of uninterrupted CI/CD follow-up.

## 3. 📁 Affected Files and Lines
- `crates/any-context-core-rs/src/storage/sqlite.rs`: atomic migration, removal of the legacy `paths_json` column, simplification of `get/add/remove_workspace_folder`.
- `crates/actx-cli/Cargo.toml`, `crates/actx-agent/Cargo.toml`, `crates/actx-installer/Cargo.toml`, `crates/actx-lm/Cargo.toml`, `crates/any-context-core-rs/Cargo.toml`, `crates/any-context-core-rs/pyproject.toml`, `pyproject.toml`, `crates/actx-cli/src/cli/args.rs`, `version.txt`: bump to `v0.32.0`.
- `C:\Users\guilh\.gemini\config\skills\dev-cycle-protocol\SKILL.md`: update of the Pillars and Stage 8.
- `TECDOC.md` and `README.md`: technical documentation of the new relational schema and ER diagram.
- `tests/MANUAL_TESTS.md`: a new detailed Scenario verifying the single normalized table.

## 4. 📚 Dual-Doc Update Plan (UserDoc + TecDoc)
- **UserDoc (`README.md`)**:
  - Documentation of version v0.32.0, highlighting source performance and integrity.
- **TecDoc (`TECDOC.md`)**:
  - Updated SQLite database schema without legacy columns and a Mermaid relational diagram of `workspace_folders`.

## 5. 🧪 Automated Test Strategy & Regression Protection
- Unit test in `sqlite.rs` validating that databases with the legacy `paths_json` column are automatically migrated to `workspace_folders` and the old column is removed with no data loss.
- Full suite run with `cargo test --workspace` (100% green across the 125+ tests).

## 6. 📝 Manual Test Scenario Plan (`tests/MANUAL_TESTS.md`)
- Addition of **Scenario 3 (v0.32.0): Validation of the Single Normalized workspace_folders Table and Uninterrupted CI/CD Monitoring**:
  - SQL inspection checking that the `paths_json` column is absent from the `workspaces` table.
  - Query of the `workspace_folders` table confirming every folder mapped to its workspace.
  - Test of adding and removing sources through `/folder --add` and `/folder --remove`.

## 7. 🔒 Secret Hygiene, Database Migration and Rollback Plan
- Automatic backup of the `settings.db` database before running `ALTER TABLE DROP COLUMN`.
- Atomic transaction (`BEGIN TRANSACTION ... COMMIT`) during the migration.
- Verification through `verify_gate.py` (Gates 3 and 8).

## 8. ⏱️ Step-by-Step Execution
1. Update the `dev-cycle-protocol` with the uninterrupted CI/CD follow-up rule.
2. Run the semantic version bump to `v0.32.0` in every manifest.
3. Implement the `paths_json` migration and removal routine in `sqlite.rs`.
4. Simplify the folder manipulation methods to use only `workspace_folders`.
5. Run `cargo test --workspace` and validate 100% success.
6. Build the release binary and deploy it to the local environment.
7. Update `TECDOC.md`, `README.md` and `tests/MANUAL_TESTS.md`.
8. Commit, push, open a PR on GitHub and follow the GitHub Actions until the final release is published.
