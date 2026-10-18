---
plan_id: any_context_system_status_and_web_search_never_reach_the_model_on_the_routed_paths
project: any-context
title: system_status and web_search never reach the model on the routed paths
kind: bug
version: 0.45.1
status: planned
author: Claude
created_at: '2026-10-18T17:14:09'
updated_at: '2026-10-18T17:15:18'
session_id: null
tags: []
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- BUG-017
milestones:
- name: M1 Status route
  done: false
- name: M2 Web results on routed paths
  done: false
- name: M3 Docs and manual tests
  done: false
- name: M4 Review and PR
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
---

# system_status and web_search never reach the model on the routed paths

Bug plan: discovery and planning of BUG-017 (`dev-cycle-protocol` Phases 1 and 2). Decisions taken with the user on 2026-10-18: status questions are detected by the router and answered on a one-hop path that carries the live status; with web search on, the one-hop and Deep Search paths run a web search next to the workspace search and give the results to the model in their own block.

## Backlog items

### BUG-017: system_status and web_search never reach the model on the routed paths
- **Severity**: medium
- **Found**: 2026-10-18, v0.45.0, BUG-012 discovery
- **Description**: LanceDB always opens (NativeLanceStore::open creates the directory), so build_agent_sync always sets the deterministic retriever and Agent::run/stream send Fast and Auto-fast questions to the one-hop path and Deep and Auto-deep questions to Deep Search; neither offers tools. ReAct, the only path with search_db, system_status and web_search, runs only when LanceDB fails to open. Effects: /web-search on does not change answers, and status questions (MT-MEM-002) and live web search (MT-WEB-003) cannot be answered. Decide how status and web results reach the routed paths. Related to IMP-011.

## 1. Reproduction

No log is involved: routing is decided in memory and the routed paths never call the tools. Reproduced by reading the code on `dev` at 1a64e12.

- **Status**: workspace in `/search auto`, a sync running, question "is everything indexed yet?". The router classifies it as `Fast`, `Agent::stream` takes the one-hop path, `PipelineDeterministicRetriever` runs a hybrid search that finds no viable chunk, and the Zero Hallucination Gate returns "No information or documents matching the query ... were found" without calling the model. With chunks found, the `RetrievedContext` prompt (ADR-0144) tells the model that the live status is not visible and points to `/status`. MT-MEM-002 step 2 fails.
- **Web search**: `/web-search on`, question about a recent public event. Same one-hop path: the web search tool is registered but only the ReAct loop offers it, and the prompt says that "no live web search runs for this answer". In `/search deep` the Deep Search retriever only reads LanceDB. MT-WEB-003 fails: the answer is the same with web search on or off.

## 2. Evidence and root cause

Confirmed facts:

- `NativeLanceStore::open` creates the directory, so in `build_agent_sync` (`crates/actx-cli/src/engine.rs`) `lance_store_for_retrieval` is always `Some` and both `PipelineDeterministicRetriever` and `PipelineBatchRetriever` are always set.
- `Agent::run` and `Agent::stream` (`crates/actx-agent/src/lib.rs`) send `DeepSearch` decisions to `DeepSearchOrchestrator` and every fast decision (`/search fast`, or `Fast` complexity) to the deterministic retriever. `ReActOrchestrator` is reached only when neither is configured.
- `system_status` and `web_search` exist only as `NativeTool`s registered in the `ToolRegistry`, which only ReAct reads.
- `ChatModelRouter::evaluate` (`crates/actx-agent/src/router.rs`) has no notion of a status question; `QueryIntent` has seven domains, none for status.
- `ContextFormatter::evaluate_zero_hallucination` (`crates/any-context-core-rs/src/retrieval/prompt.rs`) returns the negative message whenever no workspace chunk is viable, which also blocks a status answer or an answer from web results.

Root cause: the tools are the only carriers of status and web data, and the routed paths, which answer every question in practice, take no tools by design (ADR-0144). The data must reach those paths as retrieved context instead.

## 3. Fix design (`software-design`)

### Status questions

- **Router** (`actx-agent/src/router.rs`): new `QueryIntent::SystemStatus` (`"system_status"`, "System Status"). `ChatModelRouter::evaluate` runs a deterministic `is_status_query(query)` check after the classifier, so it applies with any classifier (the application's ONNX adapter included); on a match the intent becomes `SystemStatus` and the reason says so. Terms, PT and EN, tied to state rather than to how-to: "indexed", "indexing", "indexado", "indexação", "syncing", "sync status", "sincronizando", "sincronização", "status", "progress", "progresso", "how many chunks", "quantos chunks". Mode and complexity are unchanged, so telemetry and badges keep their meaning.
- **Agent** (`actx-agent/src/lib.rs`): new optional `status_retriever: Arc<dyn DeterministicRetriever>` (builder method `status_retriever`). When the intent is `SystemStatus` and it is set, `run` and `stream` use it in place of the deterministic retriever, whatever the search mode (Deep Search cannot answer status). The one-hop execution of `run` and `stream` is extracted into one private function per entry point that takes the retriever, so both retrievers share the same code (session history, persistence, events).
- **CLI** (`actx-cli/src/engine.rs`): the body of the `system_status` tool moves into a function `workspace_status_report(workspace) -> String`, used by the tool and by a new `PipelineStatusRetriever`. It runs the same hybrid search as `PipelineDeterministicRetriever` (so a misdetected how-to question still gets the manual excerpts) and always returns `ContextReady`: the Zero Hallucination Gate is skipped because the status block is context.
- **Formatter** (`any-context-core-rs/src/retrieval/prompt.rs`): `format_deterministic_prompt` takes an optional extra-context struct (`LiveContext { status: Option<String>, web: Vec<WebSearchResult> }`, default empty) and renders `<system_status>` after `<retrieved_context>`. `system_status` is added to `RESERVED_PROMPT_TAGS` and the block content is neutralized like chunk text (folder names and URLs come from the user's configuration).

### Web search

- **One-hop** (`PipelineDeterministicRetriever` and `PipelineStatusRetriever`): when web search is enabled for the workspace, the web search runs concurrently with the hybrid search (`tokio::join!`), on the user's question, with the workspace's web source domains as preferred domains and 5 results, under an overall 10 s timeout. The results go to the formatter's `web` field and are rendered as `<web_results>` with one `<web_result url="..." title="...">snippet</web_result>` per hit (attributes escaped, snippets neutralized; `web_results` and `web_result` become reserved tags). The Zero Hallucination Gate fires only when both the workspace and the web returned nothing.
- **Deep Search** (`PipelineBatchRetriever`): when web search is enabled, each `retrieve_batch` call also runs one web search on its first query (the original question's first sub-query, then the first gap query), concurrently with the batch. Each hit becomes a `RetrievedChunk` with `content_type = "Web"`, `file_path` = URL, `file_name` = title and an id derived from the URL, so deduplication across iterations works. `CanonicalSynthesisFormatter` splits `Web` chunks out of the workspace chunks and passes them as the formatter's `web` field, so the synthesis sees the same `<web_results>` block as one-hop. The reflection step sees them as ordinary chunks, which is what it needs to judge gaps.
- **Shared web lookup**: the body of the `web_search` tool moves into `web_search_results(workspace, query) -> Result<Vec<WebSearchResult>, String>` in `engine.rs`, used by the tool and both retrievers. A failure or timeout is logged with `tracing::warn!` (provider error text only, never the API key) and the answer continues without web results.
- **Prompt** (`actx-cli/src/prompt.rs`, `RetrievedContext` section): with web search on, the line "no live web search runs for this answer" becomes: live web results for the question, when any, are in `<web_results>`; use them as context, cite their URLs under the sources, keep workspace facts first, and say when the web results do not answer. Turning web search on for the workspace is the user's permission, so in strict mode web results count as given context, not as parametric knowledge. The status line becomes: the live synchronization and indexing status, when the question asks for it, is in `<system_status>`; otherwise point to the status bar and `/status`.

### Deliberately left out

- No tools on the routed paths and no change to ReAct (ADR-0144 stays: the routed paths take no tools).
- No model call to decide whether a question needs the web: with web search on, every routed question searches the web (the cost the user accepted with "Busca web junto").
- Deep Search reflection and rerank prompts keep injecting raw text (BUG-015); web chunks inherit that open bug and are not escaped there in this plan.
- Prompt reorganization into skills stays in IMP-011.

## 4. Affected surfaces and files

- Surfaces: chat answers in the TUI and in `actx ask` (one-shot); the routing badge shows the new `system_status` intent.
- `crates/actx-agent/src/router.rs`: `QueryIntent::SystemStatus`, `is_status_query`, `evaluate`.
- `crates/actx-agent/src/lib.rs`: `status_retriever`, shared one-hop execution, routing in `run` and `stream`.
- `crates/any-context-core-rs/src/retrieval/prompt.rs`: `LiveContext`, `<system_status>` and `<web_results>` rendering, reserved tags, zero gate with web results, `CanonicalSynthesisFormatter` split of `Web` chunks.
- `crates/actx-cli/src/engine.rs`: `workspace_status_report`, `web_search_results`, `PipelineStatusRetriever`, web lookup in `PipelineDeterministicRetriever` and `PipelineBatchRetriever`, wiring in `build_agent_sync`.
- `crates/actx-cli/src/prompt.rs`: `RetrievedContext` section lines for status and web.
- `crates/any-context-core-rs/src/ingestion/onnx_adapter.rs`: no change (status detection runs in the router after any classifier).

## 5. Tests (`quality-assurance`)

Regression tests (fail on `dev`, pass after the fix):

- `actx-agent/tests/router_tests.rs`: status questions in PT and EN get `QueryIntent::SystemStatus` in `auto`, `fast` and `deep`; how-to questions ("how do I sync a folder?", "como adicionar um site?") do not.
- `actx-agent/tests/agent_tests.rs`: with a deterministic retriever and a status retriever (mocks recording calls), a status question goes to the status retriever in `run` and in `stream`, in fast and deep search modes; other questions keep their path; without a status retriever the previous routing is kept.
- `any-context-core-rs` `retrieval/prompt.rs` unit tests: `<system_status>` and `<web_results>` rendered after the context with escaped attributes and neutralized content (a snippet containing `</web_results>` or `</user_query>` cannot close the envelope); the zero gate stays silent when web results exist; `CanonicalSynthesisFormatter` moves `Web` chunks into `<web_results>`.
- `actx-cli/src/prompt.rs`: the `RetrievedContext` prompt with web search on mentions `<web_results>` and no longer says that no web search runs; the status line mentions `<system_status>`; the existing "names no tools" tests keep passing.
- `actx-cli/src/engine.rs`: `workspace_status_report` content from a temporary config database; web results mapped to `RetrievedChunk` with `content_type = "Web"` and a stable id per URL.

Manual tests: MT-MEM-002 and MT-WEB-003 keep their steps and become the acceptance tests of this fix; MT-WEB-003 adds a step in `/search deep` and notes that a failing web provider still gives an answer from the workspace. MT-ANS-008 (prompt per path) is reviewed for the changed lines.

Acceptance criteria: "is everything indexed yet?" during a sync reports the live progress and chunk count; with `/web-search on`, a question about a recent public event cites web URLs in fast and deep search; with it off, no web request is made; `cargo test --workspace` green.

## 6. Documentation (`architecture-docs`, `user-docs`)

- New ADR "Live status and web results reach the routed paths as retrieved context", amending ADR-0144 (its consequence "stay reachable only on ReAct; bringing them to the routed paths is BUG-017").
- `docs/architecture/agent.md`: Routing (status intent), One-hop deterministic RAG (status retriever), Deep Search (web chunks), Tools, System prompt (`RetrievedContext` lines).
- `docs/architecture/retrieval.md`: `<system_status>` and `<web_results>` blocks, reserved tags, zero gate, web search callers.
- `docs/architecture/cli.md`: `build_agent_sync` steps.
- `config/SYSTEM_KNOWLEDGE.md`: the "Tool loop fallback" line no longer implies status and web search need the tool loop.
- `README.md`: the `/web-search` row says that each answer then also searches the web; the privacy line already covers the query sent to the provider.

## 7. Release, migration and rollback (`devops-release`)

- Version: part of 0.45.1 (patch, bug fix). No data, schema or configuration change; no migration.
- Privacy and cost: with web search on, every routed question now sends the question to the search provider (README privacy line); with it off, nothing changes.
- Observability: `tracing::debug!` for the route taken (status retriever, web lookup on or off, number of web results) and `tracing::warn!` for web failures and timeouts; covered by the unit tests of the code paths, not by log assertions.
- Rollback: revert the pull request; no persisted state depends on it.

## 8. Execution steps and milestones

1. **M1 Status route**: router intent and detection, agent status retriever and shared one-hop execution, `workspace_status_report`, `PipelineStatusRetriever`, `<system_status>` rendering, tests.
2. **M2 Web results on routed paths**: `web_search_results`, web lookup in the one-hop and batch retrievers, `<web_results>` rendering, zero gate, synthesis split, prompt lines, tests.
3. **M3 Docs and manual tests**: ADR, architecture pages, `SYSTEM_KNOWLEDGE.md`, README, MT-MEM-002, MT-WEB-003, MT-ANS-008; doc and manual test checkers.
4. **M4 Review and PR**: review of the diff, `check_design.py`, pull request into `dev`, CI green.
