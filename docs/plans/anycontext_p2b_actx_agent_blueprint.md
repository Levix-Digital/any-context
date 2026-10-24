---
plan_id: anycontext_p2b_actx_agent_blueprint
project: any-context
title: 'Priority 2B Blueprint: Native ReAct Orchestrator in Rust (actx-agent) with RFC-042 Preparation'
version: v0.30.38
status: superseded
author: Levi Silveira
created_at: '2026-09-25T13:39:43'
updated_at: '2026-10-24T14:17:00'
session_id: 9f488dbf-db54-4ecb-8ed2-b7a6ac1b2372
tags:
- blueprint
- prioridade_2b
- actx_agent
- react
- fsm
- rfc042
supersedes: null
superseded_by: null
parent_plan_id: anycontext_master_roadmap_v0_30_38_rfc042
milestones:
- name: actx-agent Crate
  done: false
- name: ReAct FSM
  done: false
- name: ToolRegistry
  done: false
- name: PyO3 Bindings
  done: false
- name: Dual-Doc
  done: false
- name: Tests
  done: false
status_log:
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
- at: '2026-10-24T14:17:00'
  by: Levi Silveira
  status: superseded
  note: Superseded by the architecture v2 redesign and roadmap v2 (docs/ROADMAP.md, docs/plans/anycontext_roadmap_v2.md, decided 2026-10-23); the remaining work was rebuilt as backlog items.
---

# 📋 Technical Blueprint - Priority 2B: Native ReAct Orchestrator in Rust (`actx-agent`)

## 1. 🎯 Goal & Architecture Decisions (ADR)
- **Goal**: replace the Python ReAct graph built on LangGraph with a native Rust Finite State Machine (FSM) in the new independent crate [`crates/actx-agent`](file:///C:/Users/guilh/source/repos/any-context/crates/actx-agent), consuming [`actx-lm`](file:///C:/Users/guilh/source/repos/any-context/crates/actx-lm) and managing a polymorphic tool catalog (`ToolRegistry`), preparing the engine for deep agentic research (RFC-042 Deep Search).
- **Design Decisions**:
  1. **New Decoupled Crate (`crates/actx-agent`)**: a standalone crate in the Cargo workspace, without heavy framework dependencies, reusable in any Rust application.
  2. **Deterministic ReAct FSM**: finite states: `Idle` $\rightarrow$ `CallModel` $\rightarrow$ `ExecuteTools` $\rightarrow$ `CheckBudget` $\rightarrow$ `Finished`.
  3. **Absolute Anti-Recursion Protection**: a strict configurable limit (`max_turns: 10`, default) with a forced cut and synthesis of the accumulated context, permanently removing infinite recursion errors.
  4. **Hybrid Polymorphic Catalog (`ToolRegistry`)**:
     - `NativeTool`: direct execution in the Rust process (e.g. vector/lexical search in `NativeLanceStore` and SQLite without crossing the PyO3 bridge).
     - `DynamicPyTool`: asynchronous callbacks through PyO3 for legacy tools or external scripts.
     - Native support for the **MCP** (*Model Context Protocol*) through Stdio/SSE adapters.
  5. **Self-Correction and Defensive Tool Execution**: JSON schema validation errors or execution failures are caught and returned as a friendly observation so the LLM can correct itself on the next turn.
  6. **Granular Event Streaming (`AgentEvent`)**: real-time emission through a Tokio async channel (`Thinking`, `ToolStart`, `ToolEnd`, `Delta`, `Done`, `Error`).
  7. **Preparation for RFC-042 (Deep Search)**: modeling of `SearchMode` (`Auto`, `Fast`, `Deep`), the `BatchQueryItem` and `BatchRetrievalResult` structures, and tree events for the future `<DeepSearchTree />`.
  8. **SessionStore in Native SQLite**: clean history stored as JSON without zlib, with a deterministic rolling window of 15 turns.

---

## 2. 🏛️ Surface Parity Mapping
- **Terminal CLI (`src/any_context/cli/`)**: keeps commands, prompts and history, consuming the new native `AgentEngine` through a transparent Python wrapper.
- **OpenTUI Desktop (`src/any_context/tui/`)**: receives streaming chunks and events directly through the Stdio RPC Bridge (`chat_stream`).
- **REST Web API (`src/any_context/server/api.py`)**: the `/v1/chat/completions` endpoint dispatches calls to the native engine.
- **MCP Protocol Server (`src/any_context/server/mcp.py`)**: the MCP server's tools and prompts interact with the same canonical `SessionStore`.

---

## 3. 📁 Affected Files and Lines
- **New Files**:
  - `crates/actx-agent/Cargo.toml`: crate definition, dependencies (`actx-lm`, `tokio`, `serde`, `serde_json`, `async-trait`, `rusqlite`, `thiserror`).
  - `crates/actx-agent/src/lib.rs`: the agent's public façade (`Agent`, `AgentBuilder`, `AgentConfig`).
  - `crates/actx-agent/src/fsm.rs`: asynchronous ReAct state machine.
  - `crates/actx-agent/src/tool.rs`: `Tool` trait, `ToolRegistry`, `NativeTool`, `ToolResult`, JSON validation.
  - `crates/actx-agent/src/session.rs`: `SessionStore` based on SQLite and memory.
  - `crates/actx-agent/src/events.rs`: `AgentEvent` enum and asynchronous receiver.
  - `crates/actx-agent/tests/agent_tests.rs`: Rust unit tests (FSM, tools, circuit breaker, streaming).
  - `src/any_context/core/deep_search_models.py`: RFC-042 Pydantic models (`SearchMode`, `SubQuery`, `QueryPlan`, `GapAnalysisResult`, etc.).
  - `tests/unit/core/test_native_agent.py`: PyO3 integration tests in Python.
- **Modified Files**:
  - `Cargo.toml`: add `crates/actx-agent` to the workspace members.
  - `crates/any-context-core-rs/Cargo.toml`: add the dependency `actx-agent = { path = "../actx-agent" }`.
  - `crates/any-context-core-rs/src/lib.rs`: expose PyO3 classes (`PyAgent`, `PyAgentEngine`, `PyAgentEvent`).
  - `src/any_context/core/agent.py`: redirect `create_anycontext_agent` and `LazyAgentProxy` to the native Rust engine with a safe fallback.

---

## 4. 📚 Dual-Doc Update Plan (`UserDoc` + `TecDoc`)
- **`UserDoc` (`README.md`)**:
  - Architecture diagram updated to include `crates/actx-agent`.
  - Explanation of the native ReAct engine in Rust (no LangChain/LangGraph dependency, faster cold boot).
  - Documentation of the RFC-042 `/fast` and `/deep` modes.
- **`TecDoc` (`TECDOC.md`)**:
  - **Section 75**: deep dive into the `actx-agent` architecture.
  - Mermaid state diagram of the ReAct FSM.
  - Table of algorithmic complexity and memory consumption.
  - Specification of the streaming events and the tool catalog.
  - ADR detailing the LangGraph purge and the integration with `actx-lm`.

---

## 5. 🧪 Automated Test Strategy & Regression Protection
- **Rust Unit Tests (`crates/actx-agent/tests/`)**:
  - `test_direct_response_without_tools`: a single turn without tool calls.
  - `test_single_tool_execution`: a tool call, an observation and a final answer.
  - `test_tool_argument_validation_recovery`: self-correction after an argument error.
  - `test_recursion_limit_enforcement`: guaranteed interruption on reaching `max_turns: 10`.
  - `test_streaming_events_emission`: ordered emission of `ToolStart`, `ToolEnd`, `Delta`.
- **PyO3 Integration Tests (`tests/unit/core/test_native_agent.py`)**:
  - Instantiation of `PyAgent` from Python.
  - Execution with native tools and Python callbacks.
- **Master E2E Runner**:
  - Run `python tests/run_all_e2e.py`, guaranteeing that the 362+ tests stay 100% green.

---

## 6. 📝 Manual Test Scenario Plan (`tests/MANUAL_TESTS.md`)
- A new scenario added at the top of `tests/MANUAL_TESTS.md` without truncating the previous ones:
  - **Scenario 1: Native ReAct Agent Execution with actx-agent**:
    - Prerequisites: AnyContext built with the new `actx-agent` crate.
    - Step 1: run a simple query ("Hi, who are you?") $\rightarrow$ check for a direct answer without tool calls.
    - Step 2: run a query that requires searching the workspace ("How does the authentication system handle token expiration?") $\rightarrow$ watch the `search_db` tool call event in the terminal and the synthesized answer with citations.
    - Step 3: force a complex query with a turn ceiling to validate the safe cut.

---

## 7. 🔒 Secret Hygiene, Database Migration and Rollback Plan
- **Secret Hygiene**: validation run through `verify_gate.py --gate 3` (approved, no exposed credentials).
- **Database Migration**:
  - Non-destructive addition of the `search_mode TEXT DEFAULT 'auto'` column to the SQLite `workspaces` table (`PRAGMA table_info` before `ALTER TABLE`).
- **Atomic Rollback Plan**:
  - In `src/any_context/core/agent.py`, the implementation keeps a feature flag (`ACTX_NATIVE_AGENT=1`). If it is disabled or loading the binary fails, there is a transparent safe fallback.

---

## 8. ⏱️ Step-by-Step Execution
1. **Step 1**: create the new `crates/actx-agent` crate with modular source code (`fsm.rs`, `tool.rs`, `session.rs`, `events.rs`, `lib.rs`).
2. **Step 2**: implement Rust unit tests and validate the MSVC x64 build through `cargo test -p actx-agent`.
3. **Step 3**: integrate `actx-agent` into `crates/any-context-core-rs` and expose the PyO3 classes (`PyAgentEngine`, `PyAgentEvent`).
4. **Step 4**: create `src/any_context/core/deep_search_models.py` (RFC-042 models) and update `src/any_context/core/agent.py`.
5. **Step 5**: run the full automated test suite (`cargo test`, `pytest`, `run_all_e2e.py`).
6. **Step 6**: update the Dual-Doc documentation (`README.md` and Section 75 of `TECDOC.md`).
7. **Step 7**: record the manual test scenario in `tests/MANUAL_TESTS.md`.
8. **Step 8**: validate Gate 8 with `verify_gate.py --gate 8` and prepare closure and release.
