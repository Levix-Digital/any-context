---
plan_id: any_context_system_status_and_web_search_never_reach_the_model_on_the_routed_paths
project: any-context
title: system_status and web_search never reach the model on the routed paths
kind: bug
version: 0.45.1
status: planned
author: Claude
created_at: '2026-10-18T17:14:09'
updated_at: '2026-10-18T17:44:20'
session_id: null
tags: []
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- BUG-017
milestones:
- name: M1 Status route
  done: true
  done_at: '2026-10-18'
  by: Claude
- name: M2 Docs and manual tests
  done: true
  done_at: '2026-10-18'
  by: Claude
- name: M3 Review and PR
  done: false
status_log:
- at: '2026-10-18T17:14:09'
  by: Claude
  status: draft
  note: Plan created
- at: '2026-10-18T17:15:18'
  by: Claude
  status: planned
  note: Plan updated
- at: '2026-10-18T17:28:57'
  by: Claude
  status: planned
  note: Plan updated
- at: '2026-10-18T17:44:17'
  by: Claude
  note: 'Milestone ''M1 Status route'' completed: router intent, status retriever, live_status prompt; cargo test --workspace green'
- at: '2026-10-18T17:44:20'
  by: Claude
  note: 'Milestone ''M2 Docs and manual tests'' completed: ADR-0145, agent/cli/retrieval pages, SYSTEM_KNOWLEDGE.md, MT-MEM-002; checkers clean'
---

# system_status and web_search never reach the model on the routed paths

Bug plan: discovery and planning of BUG-017 (`dev-cycle-protocol` Phases 1 and 2). Decisions taken with the user on 2026-10-18:

- Status questions are detected by the router and answered on a one-hop path that carries the live status as retrieved context.
- **Inference is never done directly with web data**: AnyContext is a RAG platform, and web content, like any other, is indexed before it reaches the model. The web search half of BUG-017 is therefore split into **FEAT-019** (index web search results before answering), and the ReAct `web_search` tool, which hands raw snippets to the model, is captured as **BUG-018**. This plan changes nothing about web search: routed answers stay workspace-only and the prompt keeps saying so.

## Backlog items

### BUG-017: system_status and web_search never reach the model on the routed paths
- **Severity**: medium
- **Found**: 2026-10-18, v0.45.0, BUG-012 discovery
- **Description**: LanceDB always opens (NativeLanceStore::open creates the directory), so build_agent_sync always sets the deterministic retriever and Agent::run/stream send Fast and Auto-fast questions to the one-hop path and Deep and Auto-deep questions to Deep Search; neither offers tools. ReAct, the only path with search_db, system_status and web_search, runs only when LanceDB fails to open. Effects: /web-search on does not change answers, and status questions (MT-MEM-002) and live web search (MT-WEB-003) cannot be answered. Decide how status and web results reach the routed paths. Related to IMP-011.

Scope of this plan: the status half. The web half is FEAT-019; BUG-017 closes with this plan.

## 1. Reproduction

No log is involved: routing is decided in memory and the routed paths never call the tools. Reproduced by reading the code on `dev` at 1a64e12.

Workspace in `/search auto`, a sync running, question "is everything indexed yet?". The router classifies it as `Fast`, `Agent::stream` takes the one-hop path, `PipelineDeterministicRetriever` runs a hybrid search that finds no viable chunk, and the Zero Hallucination Gate returns "No information or documents matching the query ... were found" without calling the model. When some chunk is found, the `RetrievedContext` prompt (ADR-0144) tells the model that the live status is not visible and points to `/status`. In `/search deep` the question goes to Deep Search, which only reads LanceDB. MT-MEM-002 step 2 fails.

## 2. Evidence and root cause

Confirmed facts:

- `NativeLanceStore::open` creates the directory, so in `build_agent_sync` (`crates/actx-cli/src/engine.rs`) `lance_store_for_retrieval` is always `Some` and both `PipelineDeterministicRetriever` and `PipelineBatchRetriever` are always set.
- `Agent::run` and `Agent::stream` (`crates/actx-agent/src/lib.rs`) send `DeepSearch` decisions to `DeepSearchOrchestrator` and every fast decision (`/search fast`, or `Fast` complexity) to the deterministic retriever. `ReActOrchestrator` is reached only when neither is configured.
- The live status exists only as the `system_status` `NativeTool`, which only ReAct offers.
- `ChatModelRouter::evaluate` (`crates/actx-agent/src/router.rs`) has no notion of a status question; `QueryIntent` has seven domains, none for status.
- `ContextFormatter::evaluate_zero_hallucination` (`crates/any-context-core-rs/src/retrieval/prompt.rs`) returns the negative message whenever no workspace chunk is viable, which also blocks a status answer.

Root cause: the tool is the only carrier of the status, and the routed paths, which answer every question in practice, take no tools by design (ADR-0144). The status must reach those paths as retrieved context instead.

## 3. Fix design (`software-design`)

- **Router** (`actx-agent/src/router.rs`): new `QueryIntent::SystemStatus` (`"system_status"`, "System Status"). `ChatModelRouter::evaluate` runs a deterministic `is_status_query(query)` check after the classifier, so it applies with any classifier (the application's ONNX adapter included); on a match the intent becomes `SystemStatus` and the reason says so. Terms, PT and EN, tied to state rather than to how-to: "indexed", "indexing", "indexado", "indexação", "syncing", "sync status", "sincronizando", "sincronização", "status", "progress", "progresso", "how many chunks", "quantos chunks". Mode and complexity are unchanged, so telemetry and badges keep their meaning.
- **Agent** (`actx-agent/src/lib.rs`): new optional `status_retriever: Arc<dyn DeterministicRetriever>` (builder method `status_retriever`). When the intent is `SystemStatus` and it is set, `run` and `stream` use it in place of the deterministic retriever, whatever the search mode (Deep Search cannot answer status). The one-hop execution of `run` and `stream` is extracted into one private function per entry point that takes the retriever, so both retrievers share the same code (session history, persistence, events).
- **CLI** (`actx-cli/src/engine.rs`): the body of the `system_status` tool moves into `workspace_status_report(workspace) -> String`, used by the tool and by a new `PipelineStatusRetriever`. It runs the same hybrid search as `PipelineDeterministicRetriever` (so a misdetected how-to question still gets the manual excerpts) and always returns `ContextReady`: the Zero Hallucination Gate is skipped because the status block is context. The hybrid search request is shared with `PipelineDeterministicRetriever` through one helper.
- **Formatter** (`any-context-core-rs/src/retrieval/prompt.rs`): `format_deterministic_prompt` takes an optional status report and renders it as `<system_status>` after `<retrieved_context>`. `system_status` is added to `RESERVED_PROMPT_TAGS` and the block content is neutralized like chunk text (folder names and URLs come from the user's configuration).
- **Prompt** (`actx-cli/src/prompt.rs`, `RetrievedContext` section): the status line becomes: the live synchronization and indexing status, when the question asks for it, is in `<system_status>`; otherwise point to the status bar and `/status`. The web search line ("no live web search runs for this answer") stays.

The status block is local telemetry of AnyContext itself (SQLite sync state, LanceDB chunk count, configured folders and web source URLs), not external content, so it does not conflict with the indexing rule.

Deliberately left out: web search on the routed paths (FEAT-019) and the raw-snippet `web_search` tool (BUG-018); tools on the routed paths and any change to ReAct (ADR-0144 stays); prompt reorganization (IMP-011).

## 4. Affected surfaces and files

- Surfaces: chat answers in the TUI and in one-shot mode; the routing badge shows the new `system_status` intent.
- `crates/actx-agent/src/router.rs`: `QueryIntent::SystemStatus`, `is_status_query`, `evaluate`.
- `crates/actx-agent/src/lib.rs`: `status_retriever`, shared one-hop execution, routing in `run` and `stream`.
- `crates/any-context-core-rs/src/retrieval/prompt.rs`: `<system_status>` rendering, reserved tag.
- `crates/actx-cli/src/engine.rs`: `workspace_status_report`, `PipelineStatusRetriever`, wiring in `build_agent_sync`.
- `crates/actx-cli/src/prompt.rs`: `RetrievedContext` status line.
- `crates/any-context-core-rs/src/ingestion/onnx_adapter.rs`: no change (status detection runs in the router after any classifier).

## 5. Tests (`quality-assurance`)

Regression tests (fail on `dev`, pass after the fix):

- `actx-agent/tests/router_tests.rs`: status questions in PT and EN get `QueryIntent::SystemStatus` in `auto`, `fast` and `deep`; how-to questions ("how do I sync a folder?", "como adicionar um site?") do not.
- `actx-agent/tests/agent_tests.rs`: with a deterministic retriever and a status retriever (mocks recording calls), a status question goes to the status retriever in `run` and in `stream`, in fast and deep search modes; other questions keep their path; without a status retriever the previous routing is kept.
- `any-context-core-rs` `retrieval/prompt.rs` unit tests: `<system_status>` rendered after the context with neutralized content (a folder name containing `</system_status>` or `</user_query>` cannot close the envelope); no block without a report.
- `actx-cli/src/prompt.rs`: the `RetrievedContext` status line mentions `<system_status>`; the existing "names no tools" tests keep passing.
- `actx-cli/src/engine.rs`: `workspace_status_report` content from a temporary config database.

Manual tests: MT-MEM-002 keeps its steps and becomes the acceptance test of this fix, with a note that it must pass in `/search fast`, `auto` and `deep`. MT-WEB-003 is unchanged and stays failing until FEAT-019; MT-ANS-008 (prompt per path) is reviewed for the changed line.

Acceptance criteria: "is everything indexed yet?" during a sync reports the live progress and chunk count in every search mode; other questions answer as before; `cargo test --workspace` green.

## 6. Documentation (`architecture-docs`, `user-docs`)

- New ADR "Live status reaches the routed paths as retrieved context", amending ADR-0144 (its consequence "stay reachable only on ReAct; bringing them to the routed paths is BUG-017"); it records the rule that web content is indexed before inference and points to FEAT-019 and BUG-018.
- `docs/architecture/agent.md`: Routing (status intent), One-hop deterministic RAG (status retriever), Tools, System prompt (`RetrievedContext` line).
- `docs/architecture/retrieval.md`: `<system_status>` block and reserved tag.
- `docs/architecture/cli.md`: `build_agent_sync` steps.
- `config/SYSTEM_KNOWLEDGE.md`: the "Tool loop fallback" line no longer implies status needs the tool loop.
- `README.md`: no change expected (`/status` and the status bar are unchanged); confirmed with `check_readme.py`.

## 7. Release, migration and rollback (`devops-release`)

- Version: part of 0.45.1 (patch, bug fix). No data, schema or configuration change; no migration.
- Observability: `tracing::debug!` when a question takes the status route.
- Rollback: revert the pull request; no persisted state depends on it.

## 8. Execution steps and milestones

1. **M1 Status route**: router intent and detection, agent status retriever and shared one-hop execution, `workspace_status_report`, `PipelineStatusRetriever`, `<system_status>` rendering, prompt line, tests.
2. **M2 Docs and manual tests**: ADR, architecture pages, `SYSTEM_KNOWLEDGE.md`, MT-MEM-002, MT-ANS-008; doc and manual test checkers.
3. **M3 Review and PR**: review of the diff, `check_design.py`, pull request into `dev`, CI green.
