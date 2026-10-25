---
plan_id: any_context_agent_prompt_names_tools_that_do_not_exist
project: any-context
title: Agent prompt names tools that do not exist
kind: bug
version: 0.45.1
status: completed
author: Levi Silveira
created_at: '2026-10-18T15:17:32'
updated_at: '2026-10-18T16:32:24'
session_id: null
tags: []
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- BUG-012
milestones:
- name: M1 Prompt per path
  done: true
  done_at: '2026-10-18'
  by: Levi Silveira
- name: M2 Docs and manual tests
  done: true
  done_at: '2026-10-18'
  by: Levi Silveira
- name: M3 Review and PR
  done: true
  done_at: '2026-10-18'
  by: Levi Silveira
status_log:
- at: '2026-10-18T15:17:32'
  by: Levi Silveira
  status: draft
  note: Plan created
- at: '2026-10-18T15:25:01'
  by: Levi Silveira
  status: planned
  note: Plan updated
- at: '2026-10-18T15:57:19'
  by: Levi Silveira
  note: Milestone 'M1 Prompt per path' completed
- at: '2026-10-18T15:57:21'
  by: Levi Silveira
  note: Milestone 'M2 Docs and manual tests' completed
- at: '2026-10-18T16:14:47'
  by: Levi Silveira
  note: 'Milestone ''M3 Review and PR'' completed: PR #359, CI green'
- at: '2026-10-18T16:32:24'
  by: Levi Silveira
  status: completed
  note: 'Delivered in PR #359'
---

# Agent prompt names tools that do not exist

Bug plan: discovery and planning of BUG-012 (`dev-cycle-protocol` Phases 1 and 2). Decisions taken with the user on 2026-10-18: fix the prompt per execution path and capture the unreachable tools as a separate bug (BUG-017); requests to manage sources are answered by pointing to the commands.

## Backlog items

### BUG-012: Agent prompt names tools that do not exist
- **Severity**: medium
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Description**: `config/AGENT.md` tells the model to call `add_web_source`, `list_web_sources`, `remove_web_source` and `search_db(search_session_memory=True)`, but only `search_db(query)` and `system_status` are registered. It also says the model must always call `search_db` first, while the deterministic path sends the retrieved chunks with no tools available.

Captured during discovery (not in this plan): **BUG-017** "system_status and web_search never reach the model on the routed paths" (medium, proposed; triage is the user's).

## 1. Reproduction

No log is involved: the system prompt is built in memory and not logged. The defect is reproduced by reading the code on `dev` at 2985832 and by unit tests on the built prompts.

Scenario: workspace in `/search fast` (or `auto` with a fast question), strict grounding, question "add https://docs.example.com as a web source". The deterministic path sends one user message:

```
<system_instructions>
... You **MUST ALWAYS** call the `search_db` tool on your first turn ...
... If the user asks to index, scrape, or add a website ... call `add_web_source(url=..., workspace=...)`.
... MANDATORY AUTONOMOUS RETRIEVAL: You MUST call `search_db` or `system_status` immediately on EVERY user query ...
</system_instructions>
<retrieved_context> ... </retrieved_context>
<user_query>add https://docs.example.com as a web source</user_query>
```

No tool definitions are sent with the request. The model is told to call four tools that do not exist on this request, so it can (a) print a fake call such as `add_web_source(url=...)` or a JSON tool call as the answer, (b) claim to have added the source, or (c) refuse because it "must call search_db first". The same text reaches the Deep Search synthesis.

## 2. Evidence and root cause

Confirmed facts (code read on `dev` at 2985832):

1. **Phantom tools in `AGENT.md`.** `config/AGENT.md:7-14` requires `search_db` on every first turn, `search_db(search_session_memory=True)` for past conversations and `add_web_source`, `list_web_sources`, `remove_web_source` for web sources. `build_agent_sync` registers only `search_db(query)` (no `search_session_memory` parameter), `system_status(workspace?)` and, when web search is on, `web_search(query)` (`crates/actx-cli/src/engine.rs:179-450`). None of the three web source tools exists anywhere in the code.
2. **Session memory needs no tool.** Prior turns are loaded from `SqliteSessionStore` and placed before the new message on every path (`crates/actx-agent/src/lib.rs:163-168`, `:266-272`; `fsm.rs`), so the history is already in the messages.
3. **Tool rules also live in `prompt.rs`.** `build_system_prompt` adds tool-dependent text for every path: the `system-knowledge` skill ("you MUST call `search_db`") and `system-status` skill ("you MUST call the tool `system_status`") (`crates/actx-cli/src/prompt.rs:117-118`), strict mode "MANDATORY AUTONOMOUS RETRIEVAL ... call `search_db` or `system_status`" and "if `search_db` returns no relevant chunks" (`:126-138`), hybrid "Query `search_db` first" (`:145`), and "You have access to real-time web search capabilities" when web search is on.
4. **One prompt for three paths.** `build_agent_sync` builds one `system_prompt` and gives it to the ReAct orchestrator (`builder.system_prompt`, `engine.rs:378`), to `PipelineDeterministicRetriever` (placed in `<system_instructions>` by `format_deterministic_prompt`) and to `CanonicalSynthesisFormatter` (Deep Search synthesis). Only ReAct sends tool definitions.
5. **ReAct is almost never used.** `NativeLanceStore::open` creates the directory and connects (`storage`, "Opens or creates"), so the deterministic retriever is always set. `ChatModelRouter::evaluate` returns `ReAct` with `Fast` complexity for `/search fast` and for fast `auto` questions, and `Agent::run`/`stream` send those to the deterministic path (`lib.rs:140-143`, `:231-234`); deep questions go to Deep Search. The prompt that names tools is therefore read almost only by paths with no tools. (The consequence for `system_status` and `web_search` is BUG-017.)
6. **Self-knowledge repeats the rule.** `config/SYSTEM_KNOWLEDGE.md:46` and `:61-62` state that the model must invoke `search_db` / `system_status` first; this manual is indexed into `Global` and reaches the model as retrieved chunks.
7. **Tests pin the wrong text.** `prompt.rs` tests assert that `AGENT.md` contains `search_db` and that the strict prompt contains `system_status` and "MANDATORY AUTONOMOUS RETRIEVAL".

Root cause: the agent prompt was written for a ReAct agent with a larger tool set (the Python version), then shared unchanged by the one-hop and Deep Search paths, which receive the context already retrieved and no tools. The text is not derived from the registered tools, so nothing stops it from drifting.

## 3. Fix design (`software-design`)

Hexagonal style: the prompt builder stays in the CLI composition layer (`actx-cli/src/prompt.rs`); no change in `actx-agent` or the core. The fix separates what every path shares from what depends on how the context arrives.

### Prompt path (`crates/actx-cli/src/prompt.rs`)

```rust
/// How the model receives its context, which decides the retrieval section of the system prompt.
pub enum PromptPath<'a> {
    /// ReAct loop: the model may call exactly these registered tools.
    Tools(&'a [&'a str]),
    /// One-hop RAG and Deep Search synthesis: the context is already in the prompt and no tool is offered.
    RetrievedContext,
}

pub fn build_system_prompt(
    workspace: &str,
    grounding_mode: &str,
    search_mode: &str,
    web_search_enabled: bool,
    path: PromptPath<'_>,
) -> String
```

- **Shared part** (both paths), tool-neutral: `AGENT.md`, the operational environment, persistent memory, the grounding mode directives, the recency and language rules. Every mention of a tool is removed from it: strict mode says "answer solely from the workspace context you were given (retrieved excerpts or tool results)" and "if that context has no relevant excerpt ..."; hybrid says "present workspace facts first"; citations refer to "files or URLs present in the context".
- **`RetrievedContext` section**: "The workspace excerpts for this question were already retrieved and are in `<retrieved_context>`. No tools are available in this answer: never write tool calls, function calls or JSON requests, and never say you searched, added or changed anything." Self-knowledge: questions about AnyContext are answered from the AnyContext manual excerpts when present. Live sync status is not visible in this answer: point to the status bar and `/status`. When web search is enabled for the workspace, say that no live web search runs for this answer (the setting stays shown in the environment line; making it work is BUG-017).
- **`Tools(names)` section**, built only from `names`: one rule per known tool that is present (`search_db`: search before answering, at most once per question, cross-lingual query keywords; `system_status`: indexing and sync questions; `web_search`: the existing permission-gated web directives). A name without a rule is listed as available, nothing more. No tool outside `names` is ever mentioned.
- **Source management** (shared, in `AGENT.md`): the assistant cannot add, list, rename or remove folders or web sources itself; it explains the commands `/folder`, `/web` and `/sources` and never claims to have changed a source.
- **Past conversations** (shared, in `AGENT.md`): the earlier turns are in the conversation above; answer from them, and say so when they hold no record.

### `config/AGENT.md`

Rewritten without tool names: "Mandatory Retrieval Strategy" becomes "Workspace grounding" (answer from the workspace context; the per-path retrieval rules are appended by the application), plus the source management and past conversation rules above. Sections 2 to 4 (grounding and dialogue, citations, language) keep their content; "returned by `search_db`" becomes "present in the workspace context". It stays the overridable base (`ACTX_AGENT_MD`, working directory, user configuration directory, embedded copy).

### Composition (`crates/actx-cli/src/engine.rs`)

- The tools (`search_tool`, `status_tool`, the optional `web_search_tool`) are built first into `Vec<Arc<dyn Tool>>`.
- `tool_prompt = build_system_prompt(..., PromptPath::Tools(&names))` with `names` taken from `tool.name()` of that vector, so the prompt and the registry cannot disagree; it goes to `builder.system_prompt`.
- `context_prompt = build_system_prompt(..., PromptPath::RetrievedContext)` goes to `PipelineDeterministicRetriever` and `CanonicalSynthesisFormatter`.
- The tools are then registered on the builder from the same vector.

### `config/SYSTEM_KNOWLEDGE.md`

Lines 46 and 61-62 describe the real behavior: fast and deep answers retrieve the workspace context before the model is called; the tool loop is used only when the vector store is unavailable. The `global_knowledge_version` bump that comes with the release re-indexes it.

### Error strategy

Prompt building is a total function; no new error path.

### Deliberately left out

- Bringing `system_status` and `web_search` to the routed paths (BUG-017).
- A skill mechanism and the review of every prompt, including the Deep Search intermediate prompts and the citation formats (IMP-011, BUG-015).
- Moving the deterministic instructions into a system message (IMP-010).
- New tools to list or read sources (FEAT-015).
- Users' own `AGENT.md` copies (`ACTX_AGENT_MD`, `config/AGENT.md`, user configuration directory) are not touched; the installer does not write them. The path section still comes from the code, so phantom tool names in an old copy are contradicted by the "no tools" rule; the release notes tell users to refresh their copy.

### ADR

New ADR-0144 "System prompt per execution path, with the tool section generated from the registered tools", amending ADR-0090 (mandatory retrieval in strict mode) and ADR-0082 (memory directives and the `search_db` tool). Alternatives recorded: only remove the phantom names from `AGENT.md` (the "must call `search_db`" rules still contradict the paths without tools); one prompt that says "if tools are available" (ambiguous for the model, untestable); move the tool rules into the tool descriptions only (the grounding rules still name the tools and the descriptions do not reach the paths without tools).

## 4. Affected surfaces and files

- `config/AGENT.md`: rewritten without tool names; source management and past conversation rules.
- `crates/actx-cli/src/prompt.rs`: `PromptPath`, `build_system_prompt` signature, shared and per-path sections, module doc comment, tests.
- `crates/actx-cli/src/engine.rs`: tools built first, two prompts, distribution to ReAct, the deterministic retriever and the synthesis formatter.
- `config/SYSTEM_KNOWLEDGE.md`: retrieval behavior lines.
- Other places with the same defect: none in the core prompts (`retrieval/prompt.rs` names `search_db` only as the tool result tag, which is correct on the ReAct path).
- Surfaces `cli` and `tui`: no command, flag, setting or file format changes. Answers stop printing tool calls or claiming source changes, and point to the commands. No stored data changes.

## 5. Tests (`quality-assurance`)

Automated (`cargo test --workspace`, native runner and CI), in `crates/actx-cli/src/prompt.rs`:

- **Regression test** `test_retrieved_context_prompt_names_no_tools`: for every grounding mode and web search on and off, the `RetrievedContext` prompt contains none of `search_db`, `system_status`, `web_search`, `add_web_source`, `list_web_sources`, `remove_web_source`, `search_session_memory`, and contains the "already retrieved" and "no tools" rules. Fails before the fix (the prompt contains `search_db` and `add_web_source`).
- **Regression test** `test_tool_prompt_names_only_registered_tools`: `Tools(&["search_db", "system_status"])` mentions both and not `web_search` or any phantom name; adding `web_search` adds its directives.
- `test_embedded_agent_md_names_no_tools`: the embedded `AGENT.md` contains no tool name and contains the `/web`, `/folder` and `/sources` rule; replaces the assertion that it contains `search_db`.
- `test_embedded_system_knowledge_md_names_no_phantom_tools`: the manual names none of the phantom tools and no longer says the model must invoke `search_db` first.
- Existing mode tests (`strict`, `hybrid`, `proactive`, natural conversation) keep their assertions on the shared text, run for both paths; the strict assertions on `system_status` and "MANDATORY AUTONOMOUS RETRIEVAL" move to the `Tools` path.

Manual tests (`tests/MANUAL_TESTS.md`):

- New **MT-ANS-008: Source management requests are answered with the commands**: in `/search fast` and `/search deep`, ask "add https://docs.example.com as a web source", "list my web sources" and "what did we talk about earlier?". Expected: the first two explain `/web` and `/sources`, never claim the change was made and contain no tool call text or JSON; the third answers from the earlier turns.
- MT-MEM-002 and MT-WEB-003 unchanged; their failure belongs to BUG-017 and is noted there.

Acceptance criteria: the regression tests fail on `dev` and pass after the fix; `cargo test --workspace` green; MT-ANS-008 Pass on the release candidate.

## 6. Documentation (`architecture-docs`, `user-docs`)

- `docs/architecture/agent.md`: "System prompt" lists the shared part and the two path sections, with the tool section built from the registered tools; "Tools" notes that only the ReAct path offers them; decisions list ADR-0144.
- `docs/architecture/cli.md`: the build steps mention the two prompts.
- `docs/adr/0144-...md`: the ADR above; `build_index.py` and `check_docs.py` with zero errors.
- README: no change expected (it does not describe the prompt); confirmed with `check_readme.py`.

## 7. Release, migration and rollback (`devops-release`)

- Version: patch (v0.45.1 or the next patch on `dev`); release notes: answers no longer print tool calls or claim to change sources; users with their own `AGENT.md` copy should refresh it from the new default.
- Data: none; `SYSTEM_KNOWLEDGE.md` is re-indexed into `Global` by the existing version bump.
- CI: no workflow change.
- Observability: none added; the prompt is pure and covered by unit tests.
- Security: no secrets; the change narrows what the model is told it can do.
- Rollback: revert the pull request on `dev` (or on `main` followed by the next patch).

## 8. Execution steps and milestones

- **M1 Prompt per path**: branch `fix/agent-prompt-tools` from `dev`; regression tests first (failing); `PromptPath` and the split in `prompt.rs`; `AGENT.md` and `SYSTEM_KNOWLEDGE.md` rewrite; two prompts in `engine.rs`; `cargo test --workspace` green.
- **M2 Docs and manual tests**: ADR-0144, `agent.md`, `cli.md`, index and doc checks; MT-ANS-008 and `manual_tests.py validate`.
- **M3 Review and PR**: code review and `check_design.py`; Conventional Commits; PR into `dev` linking BUG-012 (and mentioning BUG-017); CI green; BUG-012 closed with the PR reference when the plan completes.
