---
id: 144
title: System prompt per answer path with the tool section generated from the registered tools
status: accepted
date: '2026-10-18'
version: v0.45.1
subsystems: [agent, cli]
supersedes: []
superseded_by: []
amends: [82, 90]
amended_by: []
---

# ADR-0144: System prompt per answer path with the tool section generated from the registered tools

## Context

`build_agent_sync` (`crates/actx-cli/src/engine.rs`) built one system prompt and gave it to the three answer paths: the ReAct loop, one-hop RAG (`PipelineDeterministicRetriever`, inside `<system_instructions>`) and the Deep Search synthesis (`CanonicalSynthesisFormatter`, [ADR-0141](0141-deep-search-synthesis-uses-the-canonical-context-formatter-t.md)). Only ReAct sends tool definitions, and with LanceDB available (always, since it is created on open) the router sends fast questions to one-hop RAG and deep ones to Deep Search, so the model almost always ran without tools.

The prompt still read as written for a ReAct agent with a larger tool set:

- `config/AGENT.md` required `search_db` on every first turn and named `add_web_source`, `list_web_sources`, `remove_web_source` and a `search_session_memory` parameter, none of which exists (BUG-012);
- `build_system_prompt` required `search_db` or `system_status` on every query in strict mode ([ADR-0090](0090-grounding-parity-streaming-single-lancedb-path.md)), told the model to call `system_status` for indexing questions and claimed real-time web search when it was enabled;
- `config/SYSTEM_KNOWLEDGE.md`, indexed into `Global`, repeated that the model must invoke `search_db` first.

A model told to call tools it does not have can print fake calls, claim to have added a source, or refuse to answer.

## Decision

The system prompt is built per answer path, and only the ReAct path names tools, taken from the registered tools.

- `crate::prompt::PromptPath` has two values: `Tools(&[&str])` (the ReAct loop may call exactly these tools) and `RetrievedContext` (one-hop RAG and Deep Search synthesis: the context is already in the prompt and no tool is offered). `build_system_prompt(workspace, grounding_mode, search_mode, web_search_enabled, path)` takes it.
- **Shared part**, which names no tool: `AGENT.md`, the operational environment, persistent memory, the grounding mode directives, the disabled web search block, the recency and language rules. Strict mode answers from "the workspace context you were given (retrieved excerpts or tool results)"; citations refer to files and URLs present in that context.
- **`Tools` section**: lists the allowed tools and adds one rule per known tool present (`search_db`: search before answering, at most once, bilingual keywords; `system_status`: indexing questions; `web_search`: the permission-gated web directives). No other tool is mentioned.
- **`RetrievedContext` section**: the excerpts were already retrieved into `<retrieved_context>`; no tools are available, so the model never writes tool calls or claims to have searched or changed anything; live sync status is pointed to the status bar and `/status`; when web search is enabled, no live web search runs for this answer.
- `AGENT.md` drops every tool name: the assistant cannot change sources and explains `/folder`, `/web` and `/sources`; past conversations are answered from the earlier turns, which every path already sends.
- `build_agent_sync` builds the tools first, derives the ReAct prompt from their `name()`, and gives the `RetrievedContext` prompt to the deterministic retriever and the synthesis formatter.

## Alternatives Considered

- **Only remove the missing tools from `AGENT.md`**: the rules that require `search_db` and `system_status` would still contradict the two paths without tools.
- **One prompt saying "if tools are available"**: ambiguous for the model and impossible to test per path.
- **Move the tool rules into the tool descriptions only**: the grounding rules would still name the tools, and the descriptions never reach the paths without tools.

## Consequences

- The prompt cannot name an unregistered tool on the ReAct path, and names none on the paths without tools.
- `system_status` and `web_search` stay reachable only on ReAct; bringing them to the routed paths is BUG-017.
- A user's own `AGENT.md` copy (`ACTX_AGENT_MD`, working directory, user configuration directory) keeps its old text; the path section that follows it still tells the model which tools exist.
- Verified by `test_retrieved_context_prompt_names_no_tools`, `test_tool_prompt_names_only_registered_tools`, `test_embedded_agent_md_names_no_tools` and `test_embedded_system_knowledge_md_names_no_phantom_tools` (`crates/actx-cli/src/prompt.rs`), and manual test MT-ANS-008.
