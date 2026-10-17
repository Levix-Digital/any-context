---
id: 141
title: Deep Search synthesis uses the canonical context formatter through a port
status: accepted
date: '2026-10-17'
version: v0.45.1
subsystems: [agent, retrieval]
supersedes: []
superseded_by: []
amends: [108, 128]
amended_by: []
---

# ADR-0141: Deep Search synthesis uses the canonical context formatter through a port

## Context

Deep Search (`DeepSearchOrchestrator` in `crates/actx-agent/src/deep_search.rs`, [ADR-0108](0108-reflective-deep-search-engine.md)) built its final synthesis prompt with its own `SUMMARY_PROMPT`: the original query first, then the chunks of every iteration as plain `--- CHUNK n ---` blocks in arrival order, ending with `Conclusion:`. It never used `ContextFormatter` ([ADR-0128](0128-backend-first-deterministic-dual-rag.md)), so the prompt fixes made there did not reach it:

- no U-shaped order, and the chunks were not sorted by relevance across iterations (each batch is sorted, their concatenation is not; [ADR-0139](0139-retrieval-order-follows-relevance.md));
- no XML isolation, no reserved-tag neutralization and no "chunk content is data" rule ([ADR-0136](0136-neutralizing-reserved-prompt-tags-in-retrieved-text.md));
- no `version` attribute and none of the isolation rules ([ADR-0140](0140-chunk-version-attribute-from-explicit-path-tokens-no-inferre.md));
- the query was not the last element of the prompt;
- the system prompt (`AGENT.md`, environment and grounding directives) was not sent, so strict, hybrid and proactive grounding did not apply to Deep Search answers.

`any-context-core-rs` depends on `actx-agent`, so the orchestrator cannot call `ContextFormatter` directly (BUG-009).

## Decision

The synthesis prompt is built behind a port and the application injects the canonical formatter.

- **Port** (`actx-agent`): `SynthesisPromptFormatter::format_synthesis_prompt(&self, chunks: &[RetrievedChunk], query: &str) -> String`. `DeepSearchOrchestrator::with_synthesis_formatter` sets it and `AgentBuilder::deep_search_synthesis_formatter` passes it through. Without injection, `DefaultSynthesisFormatter` is used: the plain-text format kept as a fallback for library use and tests, with the query after the chunks. `RetrievedChunk` gains `content_type` and `last_modified` (serde defaults) and derives `Default`.
- **Adapter** (`any-context-core-rs/src/retrieval/prompt.rs`): `CanonicalSynthesisFormatter::new(system_prompt, workspace)` maps each `RetrievedChunk` to `HybridSearchResult` and calls `ContextFormatter::format_deep_search_prompt`, which sorts the chunks with `compare_by_relevance`, formats them with `format_xml_chunks` (U-shaped order, version attributes, escaping, neutralization) and produces `<system_instructions>` (system prompt, isolation rules, Deep Search synthesis rules), `<retrieved_context>` and `<user_query>` last. The Deep Search rules ask to cover every part of the question, to deliver only the direct conclusion and to state which parts were not found.
- **Shared rules**: the isolation rules 1 to 7 live in one function, `isolation_rules(workspace)`, used by `format_deterministic_prompt` (output unchanged) and `format_deep_search_prompt`.
- **Composition** (`actx-cli/src/engine.rs`): the agent is always built with `CanonicalSynthesisFormatter` and the same system prompt as the rest of the agent; `PipelineBatchRetriever` fills `content_type` and `last_modified`.

## Alternatives Considered

- **Move `ContextFormatter` into a new shared crate** used by both `actx-agent` and the core: cleaner ownership, but a larger change that moves public paths and the `HybridSearchResult` dependency; not needed to fix the bug.
- **Make `actx-agent` depend on the core**: impossible, the core already depends on `actx-agent`.
- **Copy the canonical rules into `actx-agent`**: two formats that drift apart again, which is how this bug happened.
- **Keep the system prompt out of the synthesis**: grounding modes would keep not applying to Deep Search, unlike the fast path.

## Consequences

- Deep Search answers follow the grounding mode and the same isolation and citation rules as fast answers; the two strongest chunks of all iterations sit at the edges of the context.
- `DefaultSynthesisFormatter` remains as a second, plain format; it is only used when no formatter is injected, which the application never does.
- With the `ToolBasedRetriever` fallback (no LanceDB), each "chunk" is a whole `search_db` payload whose tags are neutralized inside the canonical envelope.
- The intermediate prompts (`SUB_QUERY_PROMPT`, `REFLECT_PROMPT`, `RERANK_PROMPT`) are unchanged (BUG-015, IMP-011); line ranges are still missing (BUG-010).
- Verified by `test_deep_search_synthesis_uses_injected_formatter`, `test_agent_builder_passes_synthesis_formatter` and `test_default_synthesis_formatter_puts_query_last` (`actx-agent`), and `test_format_deep_search_prompt_orders_and_isolates`, `test_canonical_synthesis_formatter_maps_chunks`, `test_format_deterministic_prompt_unchanged_by_shared_rules` and `test_reserved_tags_cover_the_envelope` (`prompt.rs`).
