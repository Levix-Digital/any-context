---
plan_id: anycontext_v0_32_5_release
project: any-context
title: 'Critical Fix: Tokio Runtime Panic, LanceDB Safe Drop and Release v0.32.5'
version: v0.32.5
status: completed
author: Levi Silveira
created_at: '2026-10-02T13:39:23'
updated_at: '2026-10-02T13:39:42'
session_id: 0a07fe95-3c95-480f-904a-97aa13635137
tags: []
supersedes: null
superseded_by: null
parent_plan_id: null
milestones:
- name: Scoped LanceDB
  done: true
- name: Safe Drop
  done: true
- name: OpenAI ToolCalls
  done: true
- name: Positional Update
  done: true
- name: Dual-Doc
  done: true
- name: Release CI/CD
  done: true
status_log:
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Engineering & Fix Plan: Release v0.32.5

## 🎯 Goal
Fix a critical Tokio concurrency panic during tool execution (`search_db`), guarantee safe destruction of `NativeLanceStore` instances, align the OpenAI function call protocol and unify the update paths on Linux and Windows.

## 🏛️ Architecture and Decisions
- **ADR-086: Scoped Non-Blocking LanceDB Invocation and Safe Runtime Destruction**:
  - `NativeLanceStore::safe_block_on`: uses `tokio::runtime::Handle::try_current()` and `std::thread::scope` with a dedicated single-thread temporary runtime for synchronous execution without conflicting with the parent Tokio runtime.
  - `Drop for NativeLanceStore` implemented with safe asynchronous disposal through `std::thread::spawn`.
- **ADR-087: OpenAI Function Tool Call Protocol Alignment**:
  - `tool_calls` serialized in the JSON payload strictly following the OpenAI specification (`id`, `type: "function"`, `function: { name, arguments }`).
- **Positional Command Routing**:
  - `actx update` and `actx check-update` mapped to the native installer.

## 🏁 Completed Milestones
1. Scoped Non-Blocking LanceDB Engine & Safe Drop (`crates/any-context-core-rs/src/storage/lancedb.rs`)
2. LanceDB offload in the TUI through `tokio::task::spawn_blocking` (`crates/actx-cli/src/engine.rs`)
3. Canonical `tool_calls` formatting for OpenAI (`crates/actx-lm/src/providers/openai.rs`)
4. Positional update command routing (`crates/actx-cli/src/cli/oneshot.rs`)
5. Unit and integration test suite (132 tests passing)
6. Dual-Doc update (`TECDOC.md`, `tests/MANUAL_TESTS.md` Scenario 7)
7. CI/CD GitHub Actions: cross build and publication of the 12 assets to the releases repository (`Levix-Digital/any-context-releases` tag `v0.32.5`).
