---
plan_id: any_context_blueprint_v0327_paridade_estrita_de_grounding_no_rust_core_injeção_de_agentmd_lancedb_canônico_único_sem_legado_streaming_real_e_persistência_de_modos
project: any-context
title: 'Blueprint v0.32.7: Strict Grounding Parity in the Rust Core (AGENT.md Injection, Single Canonical LanceDB without Legacy, Real Streaming and Mode Persistence)'
version: v0.32.7
status: superseded
author: Levi Silveira
created_at: '2026-10-02T15:53:47'
updated_at: '2026-10-06T09:18:38'
session_id: 0a07fe95-3c95-480f-904a-97aa13635137
tags:
- grounding
- strict-mode
- react-agent
- lancedb
- streaming
- query-flag
- sqlite-persistence
- parity
supersedes: any_context_blueprint_v0327_paridade_estrita_de_grounding_no_rust_core_agentmd_diretivas_antialucinação_busca_autônoma_e_resolução_canônica_do_lancedbbm25
superseded_by: any_context_blueprint_v03212_test_isolation_sandbox_universal_workspace_deletion_native_self_knowledge_user_guide
parent_plan_id: null
milestones:
- name: 'M1: Single Canonical LanceDB Path'
  done: false
- name: 'M2: Grounding Persistence per Workspace'
  done: false
- name: 'M3: System Prompt and Grounding with AGENT.md'
  done: false
- name: 'M4: Real Streaming and -q Flag'
  done: false
- name: 'M5: search_db Tool with BM25 and Real Chunks'
  done: false
- name: 'M6: Automated Tests and Dual-Doc'
  done: false
- name: 'M7: Release v0.32.7 and CI Watch'
  done: false
status_log:
- at: '2026-10-06T09:18:38'
  note: Superseded by any_context_blueprint_v03212_test_isolation_sandbox_universal_workspace_deletion_native_self_knowledge_user_guide
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.32.7: Strict Grounding Parity in the Rust Core (AGENT.md Injection, Single Canonical LanceDB without Legacy, Real Streaming and Mode Persistence)

## 1. 🎯 Goal & Architecture Decisions (ADR-090)
### Context and Resolution of All Reported Problems:
1. **LanceDB with a Single Canonical Path (Zero Legacy)**:
   - Completely forgets any legacy path or search in old directories. The system has a **single official canonical path**:
     - Windows: `%LOCALAPPDATA%\AnyContext\data\context_db\lancedb`
     - Linux: `~/.local/share/any-context/data/context_db/lancedb`
     - macOS: `~/Library/Application Support/AnyContext/data/context_db/lancedb`
     - (With an optional override through the `ACTX_CONTEXT_DB` environment variable).
   - No legacy heuristics, no fallback to obsolete directories. A single source of truth where the 34,124 chunks already live.

2. **Strict Grounding Parity & AGENT.md Injection**:
   - Embed `config/AGENT.md` natively, compiled into the Rust binary.
   - In `STRICT` mode:
     - An imperative mandate to search through `search_db` before answering any factual question.
     - A categorical ban on using pre-trained parametric weights.
     - An absolute ban on passively asking *"Would you like me to search the documents?"*. The agent MUST run `search_db` autonomously.
     - If no document is found, state categorically: `⚠️ This information is not in this workspace's documents.`
   - In `HYBRID` mode: prioritize the workspace with visually separated sections (`### 📂 Workspace Information` vs a properly labeled `### 💡 General Model Knowledge`).
   - In `PROACTIVE` mode: broad integration with rigorous source labeling.
   - Mandatory sources footer (`📄 Sources Consulted:`).

3. **Persistence and Isolation of the Grounding Mode in SQLite**:
   - Fix how `grounding_mode` is stored: save it directly in the `grounding_mode` column of the `workspaces` table in SQLite (`UPDATE workspaces SET grounding_mode = ? WHERE name = ?`), besides updating `settings`.
   - Implement `get_workspace_grounding_mode(workspace)` and `set_workspace_grounding_mode(workspace, mode)` in `NativeConfigDb`.
   - Guarantee that both at boot and on a workspace switch (`/switch`), the configured mode is read faithfully from the database.

4. **Real-Time Token Streaming (Removing the "Dumped Text")**:
   - `actx-agent` was using `chat_complete()` and dumping the whole block at once in a single `Delta` event.
   - Implement token-by-token streaming using `chat_stream()` from `actx-lm`, emitting `AgentEvent::Delta(token)` in real time as tokens arrive from the API/provider.

5. **Active ReAct & `<think>` Reasoning Window**:
   - Guarantee that tool events (`ToolStart`, `ToolEnd`) and reasoning tokens appear in real time in the accordion buffer.
   - When rendering the accordion, inspect the last message in the history that contains `thinking` (`chat_history.iter().rev().find_map(...)`), instead of only `chat_history.last()`.

6. **Global Flag Standardization: `-q, --query` (with `-p, --prompt` as an alias)**:
   - Standardize `-q, --query` as the main global one-shot question flag, keeping `-p, --prompt` as a compatible alias.
   - Highlight the support for direct positional queries: `actx "your question here"`.

---

## 2. 🏛️ Surface Parity Mapping
- **`any-context-core-rs`**:
  - `storage/lancedb.rs`: a single canonical function `get_default_lancedb_path() -> PathBuf` without legacy.
  - `storage/sqlite.rs`: `get_workspace_grounding_mode` and `set_workspace_grounding_mode` methods.
  - `commands/engine.rs`: connect the `/mode` and `/workspace` commands to real persistence in the `workspaces` table.
- **`actx-lm` / `actx-agent`**:
  - `fsm.rs`: real token-by-token streaming of the final answer through `chat_stream()`.
- **`actx-cli`**:
  - `src/prompt.rs`: `AGENT.md` injection and strict Grounding rules.
  - `src/cli/args.rs`: default `-q, --query` flag (with the `-p, --prompt` alias).
  - `src/tui/app.rs` & `src/tui/ui.rs`: mode persistence and retroactive history inspection for `<think>`.

---

## 3. 📁 Affected Files and Lines
1. `crates/any-context-core-rs/src/storage/lancedb.rs`: single canonical path.
2. `crates/any-context-core-rs/src/storage/sqlite.rs`: grounding persistence methods per workspace.
3. `crates/any-context-core-rs/src/commands/engine.rs`: real persistence in `execute_grounding_mode`.
4. `crates/actx-agent/src/fsm.rs`: token-by-token streaming through `chat_stream()`.
5. `crates/actx-cli/src/prompt.rs` (New): building the system prompt with `AGENT.md`.
6. `crates/actx-cli/src/engine.rs`: connecting the new prompt and `search_db` with the canonical path and BM25/terms.
7. `crates/actx-cli/src/cli/args.rs`: priority `-q, --query` flag.
8. `crates/actx-cli/src/tui/ui.rs`: resilient `<think>` accordion.
9. `TECDOC.md`: ADR-090 recorded.
10. `tests/MANUAL_TESTS.md`: cumulative Scenario 9.
11. `Cargo.toml` / `pyproject.toml`: version `v0.32.7`.

---

## 4. 📚 Dual-Doc Plan (`UserDoc` + `TecDoc`)
- **`UserDoc` (`README.md`)**:
  - Documentation of the `-q, --query` flag (or direct execution `actx "query"`).
  - Explanation of the strict Grounding behavior (mandatory search, no hallucinations).
- **`TecDoc` (`TECDOC.md`)**:
  - ADR-090: Grounding Parity Architecture in Rust, Reactive Streaming and the Single Canonical LanceDB Path.

---

## 5. 🧪 Automated Test Strategy & Regression Protection
- Unit tests for `grounding_mode` persistence in SQLite per workspace.
- Unit tests for prompt generation in `strict`, `hybrid` and `proactive`.
- Test of the single canonical LanceDB path resolution.
- CLI flag parsing test (`-q`, `-p` and positional).
- Guarantee of 100% green tests (`cargo test --workspace`).

---

## 6. 📝 Manual Test Scenario Plan (`tests/MANUAL_TESTS.md`)
- **Scenario 9: Strict Grounding Validation, Mode Persistence, Streaming and Canonical Resolution**
  - Step 1: run `actx -q "Which immigration programs are there for an AI Engineer in Canada?"`
  - Step 2: validate that `search_db` fires autonomously and reads the real chunks from LanceDB.
  - Step 3: open the TUI, change `/mode proactive`, restart and validate that the mode persists.
  - Step 4: watch token-by-token streaming and activity in `<think>`.

---

## 7. 🔒 Secret Hygiene, Migration and Rollback
- No credentials in code. A compatible SQLite schema with no breaking changes. Atomic rollback if CI fails.

---

## 8. ⏱️ Step-by-Step Execution
1. `M1`: implement the single canonical path in `any-context-core-rs` (zero legacy).
2. `M2`: implement `grounding_mode` persistence per workspace in SQLite.
3. `M3`: implement the prompt module with `AGENT.md` and strict Grounding rules.
4. `M4`: implement real token-by-token streaming in `actx-agent` and standardize `-q, --query`.
5. `M5`: improve `search_db` with the canonical path and tokenized/BM25 search.
6. `M6`: automated tests (`cargo test --workspace`), Dual-Doc and Scenario 9 in `MANUAL_TESTS.md`.
7. `M7`: version bump `v0.32.7`, commit, PR, CI Watch on GitHub Actions, tag and publication to the releases repository.
