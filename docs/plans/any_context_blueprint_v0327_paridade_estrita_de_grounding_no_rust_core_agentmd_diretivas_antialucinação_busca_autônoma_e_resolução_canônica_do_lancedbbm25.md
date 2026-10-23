---
plan_id: any_context_blueprint_v0327_paridade_estrita_de_grounding_no_rust_core_agentmd_diretivas_antialucinação_busca_autônoma_e_resolução_canônica_do_lancedbbm25
project: any-context
title: 'Blueprint v0.32.7: Strict Grounding Parity in the Rust Core (AGENT.md, Anti-Hallucination Directives, Autonomous Search and Canonical LanceDB/BM25 Resolution)'
version: v0.32.7
status: superseded
author: Levi Silveira
created_at: '2026-10-02T15:43:59'
updated_at: '2026-10-02T15:53:47'
session_id: 0a07fe95-3c95-480f-904a-97aa13635137
tags:
- grounding
- strict-mode
- react-agent
- lancedb
- bm25
- system-prompt
- citations
- parity
supersedes: any_context_blueprint_v0324_desacoplamento_universal_do_motor_de_comandos_e_separacao_grounding_mode_vs_search_mode
superseded_by: any_context_blueprint_v0327_paridade_estrita_de_grounding_no_rust_core_injeção_de_agentmd_lancedb_canônico_único_sem_legado_streaming_real_e_persistência_de_modos
parent_plan_id: null
milestones:
- name: 'M1: Canonical Resolution of LanceDB and the BM25 Index'
  done: false
- name: 'M2: System Prompt Engine and Grounding Directives with AGENT.md'
  done: false
- name: 'M3: Resilient search_db Tool with Tokenization and BM25'
  done: false
- name: 'M4: Automated Parity and Isolation Tests'
  done: false
- name: 'M5: Dual-Doc (README.md TECDOC.md with ADR-090)'
  done: false
- name: 'M6: Cumulative Manual Tests (MANUAL_TESTS.md Scenario 9)'
  done: false
- name: 'M7: Release v0.32.7 Closure and CI Watch'
  done: false
status_log:
- at: '2026-10-02T15:53:47'
  note: Superseded by any_context_blueprint_v0327_paridade_estrita_de_grounding_no_rust_core_injeção_de_agentmd_lancedb_canônico_único_sem_legado_streaming_real_e_persistência_de_modos
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.32.7: Strict Grounding Parity in the Rust Core (AGENT.md, Anti-Hallucination Directives, Autonomous Search and Canonical LanceDB/BM25 Resolution)

## 1. 🎯 Goal & Architecture Decisions (ADR-090)
### Context and Diagnosis of the Critical Regression:
After refactoring and migrating the CLI/TUI interface to Rust (`actx`), the user found a critical functional regression in the agent's behavior in `STRICT` mode:
1. When asked about immigration programs for an AI Engineer in Canada (`Which immigration programs can I use to go from Temporary Worker to Permanent Resident as an AI Engineer?`), the model answered with long paragraphs of general pre-trained knowledge (Express Entry, PNP, Alberta, etc.) **without calling the `search_db` tool**.
2. When questioned (*"Where did this information come from?"*), the model admitted it had used generic parametric memory and asked passively: *"The information I provided is based on general knowledge... If you need... I can search the documents... Would you like me to do that?"*.
3. **Root Causes Identified**:
   - **Hollowed-Out System Prompt**: in `crates/actx-cli/src/engine.rs`, the system prompt was reduced to a simplified 10-line block that only mentioned `- Grounding Strategy: {grounding_mode}`, without injecting the content of `config/AGENT.md`, without forbidding parametric memory, without requiring a prior search through `search_db` and without fidelity directives.
   - **LanceDB Path Disconnect**: Rust looked for LanceDB in `%LOCALAPPDATA%\AnyContext\lancedb` (an empty folder with 0 records), while the user's whole vector base (34,124 chunks) canonically lives in `%LOCALAPPDATA%\AnyContext\data\context_db\lancedb`.
   - **Search Degradation in `search_db`**: the Rust tool only ran a literal SQL search `text LIKE '%query%'` with the user's whole sentence. Natural sentences or questions in Portuguese about documents in English returned 0 results, ignoring the already existing `BM25Index` (`bm25_index.bin`, 12 MB) in the user's folder.

### Architecture Decisions (ADR-090):
1. **Mandatory AGENT.md Injection and Per-Mode Directives**:
   - `crates/actx-cli` embeds the canonical `AGENT.md` (compiled through `include_str!` with a fallback to reading at runtime) and builds a strict system prompt with 100% parity with Python.
   - In `STRICT` mode: a categorical ban on parametric answers, a mandate of autonomous search through `search_db` before answering, an absolute ban on asking *"Would you like me to search the documents?"*, and a formal statement of absence (`⚠️ This information is not in this workspace's documents.`) when nothing is found.
   - In `HYBRID` mode: priority to local documents with explicitly separated sections (`### 📂 Workspace Information` vs `### 💡 General Model Knowledge` with a declarative label).
   - In `PROACTIVE` mode: fluid integration with explicit source citation.
2. **Canonical Resolution of LanceDB and BM25 Storage**:
   - Create `any_context_core_rs::storage::get_default_lancedb_path() -> PathBuf`, which inspects the OS default paths in sequence (`data/context_db/lancedb` before `lancedb`), detecting where the `.lance` files really exist.
3. **Smart `search_db` Tool (BM25 + LanceDB Metadata)**:
   - The Rust `search_db` tool uses `bm25_index.bin` when available for fast lexical scoring (Okapi BM25) and complements it with a metadata search in LanceDB with term and keyword tokenization, formatting chunks with the file title, score and the relevant passage.

---

## 2. 🏛️ Surface Parity Mapping
- **Core Domain & Storage (`crates/any-context-core-rs`)**:
  - `storage/lancedb.rs`: exposes `get_default_lancedb_path()`.
  - `commands/engine.rs`: every path call updated to the canonical function.
- **Engine & System Prompt (`crates/actx-cli`)**:
  - `src/prompt.rs`: a new module dedicated to building the structured system prompt (AGENT.md + Active Workspace + Grounding Directives + Temporal Recency + Citations).
  - `src/engine.rs`: replacement of the static prompt by `build_system_prompt()`; improvement of the `search_db` tool to use the canonical path and term/BM25 search.
- **One-Shot CLI & Interactive TUI**:
  - Both consume `build_agent_sync()` and immediately get the same strict Grounding directives and the same canonical search engine.

---

## 3. 📁 Affected Files and Lines
1. `crates/any-context-core-rs/src/storage/lancedb.rs`:
   - Implementation of `pub fn get_default_lancedb_path() -> PathBuf`.
2. `crates/any-context-core-rs/src/storage/mod.rs` & `crates/any-context-core-rs/src/lib.rs`:
   - Re-export of `get_default_lancedb_path`.
3. `crates/any-context-core-rs/src/commands/engine.rs`:
   - Use of `crate::storage::get_default_lancedb_path()` instead of the obsolete local function.
4. `crates/actx-cli/src/prompt.rs` (New):
   - A pure module to generate the canonical system prompt with the embedded `AGENT.md` and the Grounding matrix.
5. `crates/actx-cli/src/engine.rs`:
   - Integration of `prompt::build_system_prompt()`.
   - Update of `search_db` with canonical path resolution and tokenized search.
6. `crates/actx-cli/src/lib.rs` / `main.rs`:
   - Declaration of `pub mod prompt;`.
7. `crates/actx-cli/tests/`:
   - Unit tests for prompt generation and the directives in the three modes (`strict`, `hybrid`, `proactive`).
8. `TECDOC.md`:
   - Record of ADR-090 and Mermaid diagrams of the Grounding flow.
9. `tests/MANUAL_TESTS.md`:
   - Addition of Scenario 9 (Strict Grounding Validation without Parametric Hallucination).
10. `Cargo.toml` / `pyproject.toml`:
    - Bump to `v0.32.7`.

---

## 4. 📚 Dual-Doc Update Plan (`UserDoc` + `TecDoc`)
- **`UserDoc` (`README.md`)**:
  - Detail the fidelity guarantee of `STRICT` mode: it never answers with external data when the document does not exist, requiring an autonomous local search.
- **`TecDoc` (`TECDOC.md`)**:
  - ADR-090: Grounding Parity Architecture in the Rust Core and Multi-OS Canonical Resolution of LanceDB/BM25.
  - Mermaid diagram of the prompt injection pipeline and the agent's ReAct cycle.

---

## 5. 🧪 Automated Test Strategy & Regression Protection
- **Rust Unit Tests**:
  - `test_system_prompt_strict_mode`: validates that the prompt generated in `strict` mode contains explicit mandates: no parametric hallucination, an autonomous search mandate through `search_db`, a ban on asking permission to search local documents, and a formal statement of absence.
  - `test_system_prompt_hybrid_mode`: validates the separation into workspace and labeled general knowledge sections.
  - `test_canonical_lancedb_path_resolution`: validates that `get_default_lancedb_path()` prefers existing folders containing `workspace_chunks.lance`.
  - Full run: `cargo test --workspace` (149+ green tests guaranteed).

---

## 6. 📝 Manual Test Scenario Plan (`tests/MANUAL_TESTS.md`)
- **Scenario 9: Strict Grounding Validation without Parametric Hallucination and Canonical Search**
  - Configuration: Grounding STRICT, Web Search OFF.
  - A question about an absent or specific topic.
  - Check: the agent fires `search_db` autonomously in the first round. It does not hallucinate general pre-trained data. If absent, it states the factual absence and does not ask *"Would you like me to search the documents?"*.

---

## 7. 🔒 Secret Hygiene, Database Migration and Rollback Plan
- **Secret Hygiene**: no key or token in code.
- **Database Migration**: compatible and additive; uses the existing schema without destructive changes.
- **Rollback Plan**: revert the commits in git if the build breaks in CI.

---

## 8. ⏱️ Step-by-Step Execution
1. M1: implement the canonical `get_default_lancedb_path()` in `any-context-core-rs`.
2. M2: create the `actx-cli/src/prompt.rs` module with `AGENT.md` injection and Strict/Hybrid/Proactive Grounding rules.
3. M3: update `actx-cli/src/engine.rs` to use `build_system_prompt()` and improve `search_db` with smart/BM25 search.
4. M4: add automated tests and run `cargo test --workspace`.
5. M5: update `TECDOC.md` (ADR-090) and `README.md`.
6. M6: record Scenario 9 in `tests/MANUAL_TESTS.md` without compaction.
7. M7: bump to `0.32.7`, commit, PR `dev` -> `main`, CI monitoring, creation of tag `v0.32.7` and release validation.
