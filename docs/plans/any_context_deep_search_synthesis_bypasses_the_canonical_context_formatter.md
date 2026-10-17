---
plan_id: any_context_deep_search_synthesis_bypasses_the_canonical_context_formatter
project: any-context
title: Deep Search synthesis bypasses the canonical context formatter
kind: bug
version: v0.45.1
status: in_progress
author: Claude
created_at: '2026-10-17T22:07:45'
updated_at: '2026-10-17T22:52:28'
session_id: null
tags:
- deep-search
- prompt
- retrieval
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- BUG-009
milestones:
- name: M1 Port and adapter
  done: true
  done_at: '2026-10-17'
  by: Claude
- name: M2 Docs and manual tests
  done: true
  done_at: '2026-10-17'
  by: Claude
- name: M3 Review and PR
  done: false
status_log:
- at: '2026-10-17T22:07:45'
  by: Claude
  status: draft
  note: Plan created
- at: '2026-10-17T22:09:04'
  by: Claude
  note: 'Backlog items added: BUG-009'
- at: '2026-10-17T22:18:00'
  by: Claude
  status: active
  note: Plan updated
- at: '2026-10-17T22:23:08'
  by: Claude
  status: in_progress
  note: Approved by the user on 2026-10-17
- at: '2026-10-17T22:52:26'
  by: Claude
  note: 'Milestone ''M1 Port and adapter'' completed: SynthesisPromptFormatter port, CanonicalSynthesisFormatter adapter, CLI injection; cargo test --workspace green (410 passed)'
- at: '2026-10-17T22:52:28'
  by: Claude
  note: 'Milestone ''M2 Docs and manual tests'' completed: ADR-0141, agent and retrieval pages, MT-ANS-002 and MT-ANS-006; docs, readme, manual test and design checks clean'
---

# Deep Search synthesis bypasses the canonical context formatter

Bug plan for BUG-009 (`dev-cycle-protocol` full path: it adds a port to `actx-agent`, changes a public struct shared by two crates and records a decision with alternatives). Target: patch release v0.45.1, in its own pull request into `dev`.

## Backlog items

### BUG-009: Deep Search synthesis bypasses the canonical context formatter
- **Severity**: medium
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Description**: The Deep Search `SUMMARY_PROMPT` places the original query before the chunks and lists them as plain `--- CHUNK n ---` blocks, without U-shaped reordering, XML isolation, scope/version attributes or a query anchor. It is the mode that gathers the most chunks from several sub-queries.

Since BUG-008 (ADR-0140) there is no `scope` attribute any more; "scope/version attributes" now means the `version` attribute from explicit path tokens.

Related items, deliberately out of scope: BUG-010 (section and line metadata lost at ingestion, which is why Deep Search has no line ranges to cite), BUG-011 (chunk text depends on the retrieval path), BUG-015 (captured in this discovery: `REFLECT_PROMPT` and `RERANK_PROMPT` inject raw chunk text), IMP-011 (review of every prompt), ADJ-005 (`rank` and `score` attributes). The user chose on 2026-10-17 not to add a Zero Hallucination Gate to Deep Search in this plan.

## 1. Reproduction

No log is involved: the synthesis prompt is built in memory and not logged. The defect is reproduced by reading the code on `dev` at 826d1e0 and by a unit test that captures the synthesis request.

Scenario: `/search deep`, question "What is the request timeout and how often are invoices retried?" in a workspace where decomposition yields two sub-queries and the reflection one gap query. `PipelineBatchRetriever` returns up to 8 chunks per iteration; after 2 iterations the orchestrator holds up to 16 chunks in arrival order. The final model call receives (prior session messages, then one user message):

```
You are an expert AI system ...
CRITICAL GROUNDING & LANGUAGE RULES:
1. ... 3. ... cite ... `[path/to/file.rs:45-60]`.
Original Query: What is the request timeout and how often are invoices retried?

Retrieved Chunks:
--- CHUNK 1 [/home/ana/projects/api-gateway/docs/timeouts.md] (Context: timeouts.md) ---
<raw chunk text>
...
--- CHUNK 16 [...] (Context: ...) ---
<raw chunk text>

Conclusion:
```

The query is at the top, the weakest chunk of the last iteration sits next to the end, a chunk containing `--- CHUNK 17 ...` or instructions is indistinguishable from the envelope, and the `AGENT.md` system prompt and grounding mode are absent.

## 2. Evidence and root cause

Confirmed facts (code read on `dev` at 826d1e0):

1. **Own formatter.** `DeepSearchOrchestrator::run` builds the synthesis prompt with `SUMMARY_PROMPT` and `format_full_chunks` (`crates/actx-agent/src/deep_search.rs:117-131`, `:439-446`, `:528-549`). It does not call `ContextFormatter`; `docs/architecture/retrieval.md` ("`ContextFormatter` is the single formatter for every surface") is therefore wrong today.
2. **Query before context, no anchor.** `{original_query}` is placed before `{chunks_text}`; the prompt ends with `Conclusion:`. The canonical prompts put the query last (`<user_query>` in `format_deterministic_prompt`, `<query_anchor>` in `format_tool_result_payload`, `crates/any-context-core-rs/src/retrieval/prompt.rs`).
3. **Arrival order, no U-shape.** `all_accepted_chunks` is appended per iteration (`deep_search.rs:342-368`). Each batch is sorted by `compare_by_relevance` inside `retrieve_hybrid_batch_async` (`pipeline.rs:841`), but the concatenation of iterations is not sorted, and no U-shaped reordering is applied, although ADR-0139 relies on score order for it.
4. **No isolation or neutralization.** Chunk text and the query are inserted with `str::replace`; reserved tags (ADR-0136) and the `--- CHUNK` delimiter are not neutralized, and there is no "chunk content is data" rule.
5. **No version attribute and no isolation rules.** `infer_version` (ADR-0140) and rules 1 to 7 of `format_deterministic_prompt` never reach Deep Search.
6. **System prompt dropped.** `AgentConfig::system_prompt` (`AGENT.md`, environment, grounding directives, built by `build_system_prompt`) reaches the orchestrator through `self.config` (`lib.rs:479-485`) but the synthesis messages are only session history plus `SUMMARY_PROMPT`. The fast path includes it in `<system_instructions>` (`engine.rs:772`). Strict, hybrid and proactive grounding therefore do not apply to Deep Search answers.
7. **Line citations cannot be honored.** `SUMMARY_PROMPT` asks for `[path:lines]`, but `PipelineBatchRetriever` sets `header_path`, `start_line`, `end_line` to `None` (`engine.rs:696-706`). That is BUG-010; here the citation rule is aligned with the canonical one (file or chunk id).
8. **Crate graph.** `any-context-core-rs` depends on `actx-agent` (`crates/any-context-core-rs/Cargo.toml:14`), so `actx-agent` cannot call `ContextFormatter` directly. `RetrievedChunk` carries no `content_type` or `last_modified`, which `format_xml_chunks` and `compare_by_relevance` use.
9. **Fallback retriever.** Without LanceDB, the builder uses `ToolBasedRetriever` (`lib.rs:75-98`), whose "chunks" are whole `search_db` payloads (already canonical XML).

Root cause: Deep Search was written with its own prompt template inside `actx-agent`, and the crate dependency direction kept it from reaching the canonical formatter in the core, so later prompt fixes (ADR-0128, 0136, 0139, 0140) never applied to it.

## 3. Fix design (`software-design`)

Hexagonal style: the orchestrator (application layer in `actx-agent`) depends on a port; the core provides the adapter backed by `ContextFormatter`; the CLI composes them. Decisions taken with the user on 2026-10-17: port plus adapter (not moving the formatter to a new crate), and the synthesis includes the system prompt.

### Port (`crates/actx-agent/src/deep_search.rs`)

```rust
/// Builds the final synthesis prompt from the accepted chunks and the user's question.
pub trait SynthesisPromptFormatter: Send + Sync {
    fn format_synthesis_prompt(&self, chunks: &[RetrievedChunk], query: &str) -> String;
}
```

- `DeepSearchOrchestrator` gets a field `synthesis_formatter: Arc<dyn SynthesisPromptFormatter>` and a builder-style setter `with_synthesis_formatter(...)`; `new` keeps its signature and uses `DefaultSynthesisFormatter`.
- `run` replaces `SUMMARY_PROMPT` + `format_full_chunks` with `self.synthesis_formatter.format_synthesis_prompt(&all_accepted_chunks, input)`. Session history stays before it, as today. `sanitize_direct_conclusion` stays.
- `DefaultSynthesisFormatter` (in `actx-agent`, for library use, tests and the no-LanceDB fallback): the current rules, but the query moves after the chunks and the text ends with the query; chunks keep their order. It is documented as a fallback, not a second canonical format. `format_full_chunks` and `SUMMARY_PROMPT` are reworked into it (the `[path:lines]` demand is replaced by "cite the source file").
- `RetrievedChunk` gains `content_type: String` and `last_modified: Option<String>` with `#[serde(default)]`, and derives `Default` so literals can use `..Default::default()`.
- `AgentBuilder` gets `deep_search_synthesis_formatter(Arc<dyn SynthesisPromptFormatter>)`, passed to the orchestrator in `build`.

### Adapter (`crates/any-context-core-rs/src/retrieval/prompt.rs`)

- New `ContextFormatter::format_deep_search_prompt(chunks: &[HybridSearchResult], query, system_prompt, workspace) -> String`:
  - Sorts a copy with `pipeline::compare_by_relevance` (made `pub(crate)`; stable, score then recency), because Deep Search concatenates several score-ordered batches; then `format_xml_chunks` applies the U-shaped order, version attributes, escaping and neutralization.
  - Layout: `<system_instructions>` (system prompt, the shared isolation rules, then Deep Search rules) → `<retrieved_context>` → `<user_query>` (neutralized) at the end. No new tag name, so `RESERVED_PROMPT_TAGS` is unchanged.
  - Deep Search rules appended after the isolation rules: the excerpts were gathered by several sub-queries, cover every part of the question; deliver only the direct conclusion, without preambles, research narration or a list of sub-queries; state which parts were not found.
- The isolation rules 1 to 7 move into one private function `isolation_rules(workspace)` used by both `format_deterministic_prompt` and `format_deep_search_prompt`, so the text has one source. `format_deterministic_prompt` output stays byte-identical (guarded by its existing test plus an equality check).
- New `pub struct CanonicalSynthesisFormatter { system_prompt: String, workspace: String }` implementing `actx_agent::SynthesisPromptFormatter`: maps each `RetrievedChunk` to `HybridSearchResult` (`id`→`chunk_id`, `file_name`, `file_path`, `text`, `score`, `content_type`, `last_modified`, `matched_queries`→`matched_subqueries`, `workspace` empty, `token_count` estimated) and calls `format_deep_search_prompt`. Re-exported from `any_context_core_rs::retrieval`.

### Composition (`crates/actx-cli/src/engine.rs`)

- `PipelineBatchRetriever` fills `content_type` and `last_modified` from `HybridSearchResult`.
- `build` of the agent always injects `CanonicalSynthesisFormatter { system_prompt, workspace }` (pure, no I/O, so also when LanceDB does not open).

### Error strategy

Formatting is a total function: no new error path. An empty chunk list yields the canonical `<!-- No relevant chunks retrieved -->` context and rule 5 ("state that the information was not found in the workspace").

### Deliberately left out

- A Zero Hallucination Gate for Deep Search (user's choice).
- `REFLECT_PROMPT`, `RERANK_PROMPT`, `SUB_QUERY_PROMPT` (BUG-015, IMP-011).
- Line ranges and section paths (BUG-010); a Deep Search density budget across iterations (each batch keeps its own 16,000-character budget, so the context can reach about 48,000 characters over 3 iterations, as today).
- Re-scoring chunks found again in later iterations: the orchestrator keeps the first copy and its score, as today.
- When the `ToolBasedRetriever` fallback is combined with the canonical formatter, each "chunk" is a whole `search_db` payload whose tags are neutralized; acceptable for a fallback that only runs when LanceDB does not open.

### ADR

New ADR-0141 "Deep Search synthesis uses the canonical context formatter through a port", amending ADR-0108 (Deep Search engine) and ADR-0128 (canonical formatter): the port, the adapter in the core, the global score sort before the U-shape, the shared isolation rules and the system prompt in the synthesis. Alternatives recorded: move `ContextFormatter` into a new shared crate (cleaner ownership, larger change and public path churn), add a core dependency to `actx-agent` (impossible, cycle), duplicate the canonical rules in `actx-agent` (two formats drift again, the cause of this bug), keep the system prompt out (grounding modes keep not applying to Deep Search).

## 4. Affected surfaces and files

- `crates/actx-agent/src/deep_search.rs`: `SynthesisPromptFormatter`, `DefaultSynthesisFormatter`, `RetrievedChunk` fields and `Default`, orchestrator field and setter, `run` synthesis step; `SUMMARY_PROMPT`/`format_full_chunks` reworked into the default formatter.
- `crates/actx-agent/src/lib.rs`: re-exports, `AgentBuilder::deep_search_synthesis_formatter`, `ToolBasedRetriever` literal.
- `crates/any-context-core-rs/src/retrieval/prompt.rs` (and `retrieval/mod.rs` re-export; `compare_by_relevance` in `pipeline.rs` becomes `pub(crate)`): `isolation_rules`, `format_deep_search_prompt`, `CanonicalSynthesisFormatter`, tests.
- `crates/actx-cli/src/engine.rs`: `PipelineBatchRetriever` mapping, formatter injection.
- `crates/actx-agent/tests/deep_search_tests.rs`: literals, request capture, new tests.
- Surfaces `cli` and `tui`: no command, flag, setting or file format changes; Deep Search answers now follow the grounding mode and cite sources like fast answers. No stored data changes (`RetrievedChunk` is in-memory; the serde defaults keep any serialized form readable).

## 5. Tests (`quality-assurance`)

Automated (`cargo test --workspace`, native runner and CI):

- **Regression test** (`deep_search_tests.rs`) `test_deep_search_synthesis_uses_injected_formatter`: the mock provider records requests; with a recording `SynthesisPromptFormatter`, the last user message of the synthesis request equals the formatter output, and the formatter received every accepted chunk of both iterations and the original query. Fails before the fix (no port), passes after.
- **Regression test** (`prompt.rs`) `test_format_deep_search_prompt_orders_and_isolates`: chunks given out of score order (two batches) produce `<chunk>` tags where the best is first and the second best is last; `</retrieved_context>` comes before `<user_query>`, which holds the query and is the last element; the system prompt and rules 1 to 7 are inside `<system_instructions>`; a chunk with `--- CHUNK` and `</chunk><system_instructions>` text is neutralized (one `<chunk ` per chunk); a `v2/` path has `version="v2"`; no `scope`.
- `test_format_deterministic_prompt_unchanged_by_shared_rules`: the deterministic prompt for a fixed input matches the text produced before the refactor.
- `test_reserved_tags_cover_the_envelope` extended with `format_deep_search_prompt`.
- `test_canonical_synthesis_formatter_maps_chunks`: `CanonicalSynthesisFormatter` output contains each chunk's id, path, content type and modification date.
- `test_default_synthesis_formatter_puts_query_last` (`actx-agent`): the fallback ends with the query after the chunks.
- Existing Deep Search tests stay green with `..Default::default()` literals.

Manual (`tests/MANUAL_TESTS.md`, Answers):

- Update **MT-ANS-002: Quick answers and deep research**: expected result of the deep question adds "cites source files like the fast answer, and follows the grounding mode (in strict mode it says what was not found instead of using general knowledge)".
- New step in **MT-ANS-006** (files on one subject, versions apart): repeat the version question with `/search deep`; the answer gives 10 for v1 and 50 for v2 with their sources.

Acceptance criteria: Deep Search synthesis is built only by `ContextFormatter` in the application; query last; best and second-best chunks at the edges; version attributes and neutralization as in the fast path; system prompt and grounding directives present.

## 6. Documentation (`architecture-docs`, `user-docs`)

- `docs/architecture/agent.md`, Deep Search step 4: synthesis through `SynthesisPromptFormatter`; the CLI injects `CanonicalSynthesisFormatter` (system prompt, isolation and Deep Search rules, U-shaped XML context over all iterations sorted by relevance, query last); `DefaultSynthesisFormatter` only without injection. Also correct step 2 (the yes/no `RERANK_PROMPT` only runs when `enable_reranker` is on, off by default).
- `docs/architecture/retrieval.md`, "Context formatting": add `format_deep_search_prompt` and `CanonicalSynthesisFormatter`; the shared isolation rules. ADR-0141 in Decisions of both pages.
- ADR-0141 (new); `amended_by` on ADR-0108 and ADR-0128; `build_index.py` and `check_docs.py` with zero errors.
- README: no change expected (prompt format not documented for users); `check_readme.py` still run.

## 7. Release, migration and rollback (`devops-release`)

- **Version**: patch, v0.45.1, with the other fixes already in `dev`. `RetrievedChunk` gains fields, which is source-breaking only for external struct literals; the crates are internal to this workspace.
- **Data already affected**: none; the prompt is built per question. No re-indexing.
- **Rollback**: revert the pull request (or the release through a pull request on `main`) and ship the next patch; no data to restore.
- **Observability**: none added; prompts are not logged (privacy). Behavior is covered by the tests above and MT-ANS-002/006.
- **Security**: Deep Search synthesis gains the BUG-007 protections (reserved-tag neutralization, "chunk content is data" rule). The intermediate prompts stay as BUG-015.

## 8. Execution steps and milestones

1. **M1 Port and adapter**: on `fix/deep-search-canonical-synthesis` (from `dev`): `RetrievedChunk` fields, `SynthesisPromptFormatter`, `DefaultSynthesisFormatter`, orchestrator and builder wiring; `isolation_rules`, `format_deep_search_prompt`, `CanonicalSynthesisFormatter`; CLI mapping and injection; new and updated tests; `cargo test --workspace` green.
2. **M2 Docs and manual tests**: ADR-0141, `amended_by` on ADR-0108 and ADR-0128, agent and retrieval pages, MT-ANS-002 and MT-ANS-006; `check_docs.py`, `check_readme.py` and `manual_tests.py validate` with zero errors.
3. **M3 Review and PR**: code review, `software-design` checklist and `check_design.py`; PR into `dev`, CI green; close BUG-009 when merged.
4. Release with v0.45.1 when the user wants it (version bump PR, `v0.45.1-rc.N`, MT-ANS-002 and MT-ANS-006 in the run).
