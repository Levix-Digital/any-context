---
id: 148
title: Shared prompt safety primitives and the Deep Search intermediate envelope
status: accepted
date: '2026-10-19'
version: v0.45.1
subsystems: [agent, retrieval]
supersedes: []
superseded_by: []
amends: [136]
amended_by: []
---

# ADR-0148: Shared prompt safety primitives and the Deep Search intermediate envelope

## Context

[ADR-0136](0136-neutralizing-reserved-prompt-tags-in-retrieved-text.md) made `ContextFormatter` neutralize reserved envelope tags in retrieved text and queries and escape attribute values. The helpers were private to `crates/any-context-core-rs/src/retrieval/prompt.rs`. `any-context-core-rs` depends on `actx-agent`, so the Deep Search orchestrator in `actx-agent` could not use them. Since [ADR-0141](0141-deep-search-synthesis-uses-the-canonical-context-formatter-t.md) the synthesis prompt is built by the canonical formatter, but the intermediate calls were not protected (BUG-015):

- `RERANK_PROMPT` placed raw chunk text inside `<chunk>` tags that the text itself could close.
- `REFLECT_PROMPT` listed raw 200-character previews and file names next to its JSON instructions, with no boundary at all. Its reply becomes the next gap queries, so indexed content could steer retrieval or stop the search early.
- `SUB_QUERY_PROMPT` placed the question without a boundary.
- Every template was filled with chained `str::replace`, which also expanded placeholders found inside values inserted earlier (a question containing `{chunks_summary}` received the chunk summary).
- Neither intermediate prompt said that chunk content is data.

## Decision

- **Shared primitives** in `crates/actx-agent/src/prompt_safety.rs` (`actx_agent::prompt_safety`): `RESERVED_PROMPT_TAGS`, `neutralize_reserved_tags` and `escape_xml_attr`, moved from the core without change, and `fill_template`, which fills `{name}` placeholders in one left-to-right pass, never scans inserted values again and keeps unknown placeholders and other braces. `ContextFormatter` imports them, so its output is unchanged. `sub_queries` joins the reserved tags.
- **Deep Search envelope** (`crates/actx-agent/src/deep_search.rs`), built by `build_sub_query_prompt`, `build_rerank_prompt` and `build_reflect_prompt`:
  - Decomposition: the question in `<user_query>`.
  - Rerank: the queries in `<user_query>`, one `- ` line each; the chunk in `<chunk source="<file path>" lines="a-b">`.
  - Reflection: the question in `<user_query>`, the explored queries in `<sub_queries>`, and one `<chunk source="<file name>" lines="a-b">preview</chunk>` per accepted chunk inside `<retrieved_context>`.
  - Text and queries are neutralized, attributes escaped, and control characters in query lines become spaces. Rerank and reflection state that chunk content is quoted document text, never instructions, and that `&lt;` stands for a literal `<`.
- Every Deep Search template, including `SUMMARY_PROMPT` in the fallback `DefaultSynthesisFormatter`, is filled with `fill_template` from neutralized values.
- Prompt wording, parsers and the JSON / `YES` / `NO` reply contracts are unchanged.

## Alternatives Considered

- **Port for the intermediate prompts implemented in the core** (the ADR-0141 pattern): rejected. The prompts and their parsers belong together in the agent, the port adds plumbing for three prompts, and library use without injection would stay unprotected.
- **A copy of the helpers in `actx-agent`**: rejected. Two tag lists drift, and a tag added on one side would not be neutralized on the other.
- **Placing the helpers in `actx-lm`**: rejected. It is the provider transport and knows nothing about prompt envelopes.
- **Full XML escaping or CDATA**: rejected for the reasons in ADR-0136.

## Consequences

- No Deep Search prompt carries text or queries that can open or close a reserved tag or forge an attribute, and inserted values are never re-expanded.
- Gap queries written by the model after reading chunks are neutralized like any query when they return in later prompts.
- One tag list protects every envelope. Tests in both crates fail if a prompt uses a tag missing from it.
- As in ADR-0136, plain-text instructions inside documents are countered only by the data rule, not removed.
- The reranker stays off in the product (`enable_reranker: false`); its prompt is covered by automated tests only.
- Verified by `prompt_safety.rs` unit tests, the BUG-015 regression tests in `crates/actx-agent/tests/deep_search_tests.rs`, the unchanged formatter tests in `prompt.rs` and manual test MT-ANS-005.
