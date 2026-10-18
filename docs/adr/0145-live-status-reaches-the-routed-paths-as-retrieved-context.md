---
id: 145
title: Live status reaches the routed paths as retrieved context
status: accepted
date: '2026-10-18'
version: v0.45.1
subsystems: [agent, cli, retrieval]
supersedes: []
superseded_by: []
amends: [144]
amended_by: []
---

# ADR-0145: Live status reaches the routed paths as retrieved context

## Context

`build_agent_sync` (`crates/actx-cli/src/engine.rs`) always configures the deterministic retriever, because `NativeLanceStore::open` creates the LanceDB directory. `Agent::run` and `Agent::stream` therefore send fast questions to one-hop RAG and deep ones to Deep Search, and the ReAct loop, the only path that offers tools, runs only when LanceDB fails to open. [ADR-0144](0144-system-prompt-per-answer-path-with-the-tool-section-generate.md) made the prompts of the routed paths name no tool and left the `system_status` and `web_search` tools reachable only on ReAct (BUG-017).

Effects: a question such as "is everything indexed yet?" finds no excerpt, so the Zero Hallucination Gate answers "No information or documents matching the query ..." without calling the model; with excerpts, the prompt can only point to `/status`. Manual test MT-MEM-002 fails.

AnyContext is a RAG platform: the model answers from indexed content. Web content, like any other, is indexed before it reaches the model, so web search results cannot be handed to the model directly.

## Decision

The live status reaches the routed paths as retrieved context, chosen by the router; web search on the routed paths is left to indexing.

- `QueryIntent::SystemStatus` (`crates/actx-agent/src/router.rs`): `ChatModelRouter::evaluate` applies `is_status_query` after any classifier (PT and EN terms about state: indexed, indexing, syncing, sync status, status, progress, chunk counts, and their Portuguese forms). A match sets the intent and prefixes the reason with "live status question"; mode and complexity are unchanged.
- `AgentBuilder::status_retriever` (`crates/actx-agent/src/lib.rs`): with the `SystemStatus` intent, `run` and `stream` use this `DeterministicRetriever` in every search mode, before Deep Search and the fast path. Both retrievers share the one-hop execution (`run_one_hop`, `stream_one_hop`).
- `PipelineStatusRetriever` (`crates/actx-cli/src/engine.rs`): runs the same hybrid search as `PipelineDeterministicRetriever`, adds `workspace_status_report` (the report the `system_status` tool returns: sync state, chunk count, monitored folders and web sources) and always returns `ContextReady`; the Zero Hallucination Gate is skipped because the report is context. A failed search leaves the report to answer from.
- `ContextFormatter::format_status_prompt` (`crates/any-context-core-rs/src/retrieval/prompt.rs`): the deterministic prompt plus `LIVE STATUS RULES` and a `<live_status>` block after `<retrieved_context>`. `live_status` is a reserved prompt tag and the report is neutralized like chunk text. The tag is not named after the tool, so the prompts without tools still name no tool.
- The `RetrievedContext` prompt says that a status report, when present, is in `<live_status>`, and otherwise points to the status bar and `/status`.
- Web search: routed answers stay workspace-only and the prompt keeps saying that no live web search runs. Indexing web search results before answering is FEAT-019; the ReAct `web_search` tool, which returns raw snippets to the model, is BUG-018.

## Alternatives Considered

- **Live status in every prompt**: costs tokens on every question and still needs the gate changed for status questions.
- **Route status questions to the ReAct loop**: reuses the tool, but takes two or more model calls and depends on the model calling it.
- **Web search results in the prompt next to the excerpts** (a `<web_results>` block on one-hop and Deep Search): rejected, because the model would answer from web content that was never indexed.

## Consequences

- Status questions are answered from the live report in fast, auto and deep search; a how-to question misread as a status question still gets its excerpts.
- The status detection is a keyword list: a status question phrased without those terms takes its normal route, and a how-to question containing them costs one status read.
- `/web-search on` still does not change routed answers until FEAT-019; MT-WEB-003 keeps failing until then.
- Verified by the router tests `test_router_detects_status_questions_in_every_search_mode`, `test_router_keeps_how_to_questions_out_of_the_status_route` and `test_status_intent_keeps_mode_and_complexity`, the agent tests `test_status_question_takes_the_status_retriever_in_run` and `..._in_stream`, the formatter tests `test_format_status_prompt_*`, the engine tests `status_report_*`, and manual test MT-MEM-002.
