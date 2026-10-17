---
plan_id: any_context_retired_model_ids_in_presets_onboarding_and_model_menus
project: any-context
title: Retired model ids in presets, onboarding and model menus
kind: bug
version: v0.45.1
status: completed
author: Levi Silveira
created_at: '2026-10-17T10:38:09'
updated_at: '2026-10-17T12:03:35'
session_id: null
tags:
- models
- providers
- presets
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- BUG-002
milestones:
- name: M1 Evidence
  done: true
  done_at: '2026-10-17'
  by: Levi Silveira
- name: M2 Catalog
  done: true
  done_at: '2026-10-17'
  by: Levi Silveira
- name: M3 Resolution and adapters
  done: true
  done_at: '2026-10-17'
  by: Levi Silveira
- name: M4 Surfaces
  done: true
  done_at: '2026-10-17'
  by: Levi Silveira
- name: M5 Documentation and manual tests
  done: true
  done_at: '2026-10-17'
  by: Levi Silveira
- name: M6 Review and pull request
  done: true
  done_at: '2026-10-17'
  by: Levi Silveira
status_log:
- at: '2026-10-17T10:38:09'
  by: Levi Silveira
  status: draft
  note: Plan created
- at: '2026-10-17T10:40:22'
  by: Levi Silveira
  status: planned
  note: Plan updated
- at: '2026-10-17T10:40:41'
  by: Levi Silveira
  status: active
  note: Plan updated
- at: '2026-10-17T11:36:34'
  by: Levi Silveira
  note: Milestone 'M1 Evidence' completed
- at: '2026-10-17T11:36:35'
  by: Levi Silveira
  note: Milestone 'M2 Catalog' completed
- at: '2026-10-17T11:36:37'
  by: Levi Silveira
  note: Milestone 'M3 Resolution and adapters' completed
- at: '2026-10-17T11:36:39'
  by: Levi Silveira
  note: Milestone 'M4 Surfaces' completed
- at: '2026-10-17T11:36:41'
  by: Levi Silveira
  note: Milestone 'M5 Documentation and manual tests' completed
- at: '2026-10-17T11:36:48'
  by: Levi Silveira
  status: in_progress
  note: Implementation on fix/retired-model-ids
- at: '2026-10-17T12:03:33'
  by: Levi Silveira
  note: 'Milestone ''M6 Review and pull request'' completed: PR #345 merged into dev (98b5ef0)'
- at: '2026-10-17T12:03:33'
  by: Levi Silveira
  status: completed
  note: All milestones completed
- at: '2026-10-17T12:03:36'
  by: Levi Silveira
  status: completed
  note: 'Delivered in PR #345'
---

# Retired model ids in presets, onboarding and model menus

Bug plan for BUG-002 (`dev-cycle-protocol` full path, bug plan). Scope agreed with the user on 2026-10-17: every provider's chat model ids, not only `/preset frontier`. Embedding models are out of scope and tracked as BUG-013.

## Backlog items

### BUG-002: Retired model ids in presets, onboarding and model menus
- **Severity**: high (raised from medium on 2026-10-17 by the user)
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Description**: Every Claude id in the product is retired (claude-3-7-sonnet in /preset frontier, claude-3-5-sonnet in onboarding, claude-3-5-* in the models menu and /models), and so are gemini-1.5-pro, llama-3.3-70b-versatile (Groq, shut down 2026-08-16) and deepseek-chat/deepseek-reasoner (2026-07-24); o1 and o3-mini shut down on 2026-10-23 and gpt-4o-mini on 2027-01-14. Current Claude models also reject the temperature/top_p the Anthropic provider always sends, and Groq ids are routed to the wrong provider or sent with the groq/ prefix. Choosing these models fails on the first question.

## 1. Reproduction

Environment: v0.45.0 (dev at `585283c`), any OS, an API key for the provider.

1. `actx`, then `/keys anthropic <key>` and `/preset frontier`.
2. Ask any question.
3. Expected: an answer from the frontier model. Actual: the Anthropic API rejects the request because `claude-3-7-sonnet` is not a valid model id (the 3.7 Sonnet family was retired on 2026-02-19; the bare id never had a dated snapshot behind it).

The same failure happens on the first question after:
- onboarding with the Anthropic provider (`claude-3-5-sonnet`, retired 2025-10-28);
- choosing `claude-3-5-sonnet-20241022` or `claude-3-5-haiku-20241022` in the models menu;
- choosing `gemini-1.5-pro`, `deepseek-chat`, `deepseek-reasoner` or `llama-3.3-70b-versatile` in the models menu, or `groq/llama-3.3-70b` from the `/models` text;
- from 2026-10-23, any saved `o1` or `o3-mini` model.

No local log is needed to confirm the cause: the model id is sent unchanged to the provider (`crates/actx-cli/src/engine.rs:31-85`), and each provider's own deprecation page lists the ids as shut down (sources in section 2). The real API error text is recorded during implementation (step 1 of section 8) and quoted in the pull request.

## 2. Evidence and root cause

### Confirmed facts

**Hard-coded ids, in five places, none shared.**

| Place | File | Ids |
|---|---|---|
| `/preset frontier` and its help text | `crates/any-context-core-rs/src/commands/engine.rs:1430,1471` | `claude-3-7-sonnet` |
| `/preset balanced` | `engine.rs:1428,1445` | `gpt-4o-mini` |
| `/models` catalog text | `engine.rs:1514-1528` | `gemini-3.8-flash`, `gemini-1.5-pro`, `gpt-4o`, `gpt-4o-mini`, `claude-3-5-sonnet`, `claude-3-5-haiku`, `groq/llama-3.3-70b`, `deepseek-chat`, `deepseek-reasoner` |
| Models menu (TUI) | `crates/actx-cli/src/tui/menu.rs:471-489` | the same family plus `claude-3-5-*-20241022`, `llama-3.3-70b-versatile` |
| Onboarding defaults (TUI) | `crates/actx-cli/src/tui/app.rs:853`, `crates/actx-cli/src/tui/ui.rs:768-773` | `gemini-2.5-flash`, `gpt-4o-mini`, `claude-3-5-sonnet`, `llama3` |
| Application default model | `crates/actx-cli/src/engine.rs:17,29`, `crates/actx-cli/src/cli/oneshot.rs:34,126,261`, `crates/any-context-core-rs/src/commands/models.rs:133`, `storage/sqlite.rs:1184` | `gpt-4o-mini` |

**Status of each id** (providers' deprecation pages, read on 2026-10-17):

| Provider | Id | Status | Successor (approved by the user) |
|---|---|---|---|
| Anthropic | `claude-3-7-sonnet*` | retired 2026-02-19 | `claude-opus-5-5` (frontier) |
| Anthropic | `claude-3-5-sonnet*` | retired 2025-10-28 | `claude-sonnet-5-5` |
| Anthropic | `claude-3-5-haiku*` | retired 2026-02-19 | `claude-haiku-5-5` |
| OpenAI | `o1`, `o1-pro`, `o3-mini` | shut down 2026-10-23 | `gpt-6.1-sol` |
| OpenAI | `gpt-4o-mini` | shuts down 2027-01-14 | `gpt-6-luna` (new application default) |
| OpenAI | `gpt-4o` | shuts down 2027-01-14 | `gpt-6.1-sol` |
| Google | `gemini-1.5-pro` | no longer listed (shut down) | `gemini-2.5-pro` (the only stable Pro) |
| Google | `gemini-2.5-flash`, `gemini-3.8-flash` | current | unchanged |
| Groq | `llama-3.3-70b-versatile` | shut down 2026-08-16 | `openai/gpt-oss-120b` |
| DeepSeek | `deepseek-chat`, `deepseek-reasoner` | discontinued 2026-07-24 | `deepseek-flash`, `deepseek-v4-pro` |
| Ollama | `llama3`, `qwen2.5-coder` | local models chosen by the user | unchanged |

Sources: Anthropic models overview (retired models table), developers.openai.com/api/docs/deprecations, ai.google.dev/gemini-api/docs/deprecations, console.groq.com/docs/deprecations, api-docs.deepseek.com (pricing and change log).

**Sampling parameters rejected by current Claude models.** `SamplingParams::default()` sets `temperature: Some(0.7)` and `top_p: Some(1.0)` (`crates/actx-lm/src/types/request.rs:23-24`), the agent and Deep Search set `temperature` to `0.0` (`actx-agent/src/deep_search.rs`, `lib.rs`, `fsm.rs`), and `AnthropicProvider::serialize_request` sends both whenever they are set (`crates/actx-lm/src/providers/anthropic.rs:113-118`). Claude Opus 5.5 rejects any sampling parameter with a 400; Claude Sonnet 5.5 and Claude Haiku 5.5 reject non-default values. Replacing the id alone would still fail.

**Groq routing.** `resolve_lm_provider` sends a model to Groq only when its id starts with `groq` (`crates/actx-cli/src/engine.rs:64`), and passes the id unchanged. So `llama-3.3-70b-versatile` (models menu) falls through to the first provider with a key, usually OpenAI, and `groq/llama-3.3-70b` (`/models` text) reaches the Groq API with the `groq/` prefix, which is not a Groq model id. The Groq successor `openai/gpt-oss-120b` has no `groq` prefix of its own either.

**Already-saved ids.** `/preset`, `/model` and onboarding write the id to the workspace and to the `active_model` setting in `settings.db`. Users who chose a retired model before the fix keep failing after an upgrade unless the id is mapped.

### Hypotheses to confirm before the fix is final

- H1: the GPT-6 models (`gpt-6-luna`, `gpt-6.1-sol`) accept `temperature` and `max_tokens` on the Chat Completions endpoint used by `OpenAiCompatibleProvider`. The models page lists reasoning effort levels for them and documents only the Responses API. If they reject either field, the OpenAI adapter gets the same per-model parameter rule as Anthropic (no `temperature`; `max_completion_tokens` instead of `max_tokens`). Confirmed in step 1 of section 8 from the official API reference; if the reference stays unclear, the release candidate manual test (MT in section 5) is the evidence.
- H2: `deepseek-flash`, `deepseek-v4-pro` and `openai/gpt-oss-120b` (Groq) accept the parameters the OpenAI-compatible adapter sends today. Same confirmation path as H1.

## 3. Fix design (`software-design`)

Hexagonal style: the model catalog is domain knowledge and lives in the core; provider wire rules live in the `actx-lm` adapters; the CLI/TUI only consume the catalog.

### 3.1 One chat model catalog in the core

New module `crates/any-context-core-rs/src/llm_catalog.rs` (no dependency on `actx-lm`, `ratatui` or `clap`), with plain data and two lookups:

```rust
pub struct ChatModel {
    pub id: &'static str,          // exactly what the user types and what is stored
    pub provider: ChatProvider,    // Anthropic, OpenAi, Gemini, Groq, DeepSeek, Ollama, Mock
    pub summary: &'static str,     // one line for /models and the menu
}

pub const DEFAULT_CHAT_MODEL: &str = "gpt-6-luna";
pub const BALANCED_PRESET_MODEL: &str = "gpt-6-luna";
pub const FRONTIER_PRESET_MODEL: &str = "claude-opus-5-5";

pub fn chat_models() -> &'static [ChatModel];                 // menu and /models order
pub fn onboarding_default(provider: ChatProvider) -> &'static str;
pub fn successor_of(id: &str) -> Option<&'static str>;          // retired id -> current id
```

- The retired table maps exact ids and dated snapshots (`claude-3-5-sonnet`, `claude-3-5-sonnet-20241022`, `claude-3-7-sonnet`, `claude-3-7-sonnet-20250219`, `claude-3-5-haiku-20241022`, `gemini-1.5-pro`, `llama-3.3-70b-versatile`, `groq/llama-3.3-70b`, `deepseek-chat`, `deepseek-reasoner`, `o1`, `o1-pro`, `o3-mini` and their dated snapshots) to the successors in section 2. Matching is case-insensitive on the exact id, never by prefix, so a user's custom id is never rewritten by accident.
- `gpt-4o` and `gpt-4o-mini` are not in the retired table yet: they still work until 2027-01-14. Only the defaults move to `gpt-6-luna`. Adding them to the table is a one-line change in a later release.
- Groq ids in the catalog carry the `groq/` routing prefix (`groq/openai/gpt-oss-120b`), consistent with `ollama/` and `local/`.
- The `/models` text and the models menu are generated from `chat_models()`, replacing the two hand-written lists. The Gemma 4 entries and their "Planned" wording stay as they are (generated from the same list, marked local).
- Deliberately left out: provider-side model discovery (`GET /v1/models`), pricing, and the ONNX/installer catalog duplication (IMP-005), which is a different catalog.

### 3.2 Resolution maps retired ids, with a notice

In `resolve_lm_provider` (`crates/actx-cli/src/engine.rs`), after the effective model is chosen and before the provider is picked:

- if `successor_of(model)` returns a successor, use it, log `tracing::warn!(retired = %old, successor = %new, "retired model id mapped to its successor")`, and return a notice so the CLI prints and the TUI shows once per session: "Model `<old>` is no longer offered by `<provider>`; using `<new>`. Choose another with /model." The stored value is not changed (no data migration); `/model` keeps working as today.
- `groq/` is stripped before the id is sent to Groq, like `ollama/` and `local/`.
- The `o1*`/`o3*` prefixes keep routing to OpenAI (`o3` is not shut down until 2026-12-11).

The function's return type changes from `(Arc<dyn LmProvider>, String)` to a small struct with the provider, the effective model and an optional notice; its callers in `actx-cli` (CLI one-shot, TUI agent rebuild, Deep Search wiring) are updated. It is internal to `actx-cli`, not a public interface.

### 3.3 Provider wire rules per model

- `AnthropicProvider::serialize_request`: do not send `temperature` or `top_p` to models that reject them. The rule is a private function `accepts_sampling_params(model: &str) -> bool` in `anthropic.rs`, false for the families that reject them (`claude-opus-5*`, `claude-sonnet-5*`, `claude-haiku-5*`, `claude-fable-*`, `claude-mythos-*`, `claude-opus-4-7*`, `claude-opus-4-8*`), true otherwise, so a custom older id that still accepts them keeps its behavior. The dropped value is logged at debug level.
- `OpenAiCompatibleProvider`: only if H1/H2 show a rejection, the same pattern for the affected model family (omit `temperature`, send `max_completion_tokens`).
- A 404 or `not_found_error` for the model from any provider is reported as "model `<id>` is not available from `<provider>`; choose another with /model" instead of the raw JSON, so the next retirement does not look like a crash. This is the existing `LmError::ProviderError` path with a clearer message, not a new error type.

### 3.4 Error strategy

No new failure modes: unknown ids still pass through unchanged; a retired id is mapped and announced; a provider rejection is reported with the model and provider named. Nothing is swallowed.

### Decision for an ADR

"Chat model catalog in the core with retired-id successors resolved at runtime" — alternatives considered: migrating stored ids in `settings.db` (rejected: permanent, needs rollback handling, and a downgrade would see ids it does not know) and warning only (rejected: users stay broken until they act). Supersedes the frontier model named in ADR-0125.

## 4. Affected surfaces and files

Surfaces (`architecture.surfaces`): `cli` and `tui` — model ids shown and applied by `/preset`, `/models`, the models menu, onboarding, the default model and the retired-id notice. No command, flag, configuration key or file format is added or removed.

| Module | Files |
|---|---|
| Core | new `src/llm_catalog.rs`, `src/lib.rs` (module), `src/commands/engine.rs` (preset, `/models`), `src/commands/models.rs` (default), `src/storage/sqlite.rs:1184` (default) |
| LM adapters | `crates/actx-lm/src/providers/anthropic.rs`, `providers/openai.rs` (only if H1/H2), error message mapping in both and in `gemini.rs` |
| CLI | `crates/actx-cli/src/engine.rs` (resolution, Groq prefix, notice), `src/cli/oneshot.rs` (default and notice) |
| TUI | `crates/actx-cli/src/tui/menu.rs`, `tui/app.rs`, `tui/ui.rs` (onboarding, notice) |
| Comments and examples | `crates/actx-agent/src/config.rs:44` (doc comment) |
| Tests using retired ids as plain strings | `command_tests.rs`, `workspace_isolation_tests.rs`, `cli_tests.rs`, `menu.rs` test, `hardware/mod.rs` test, `sqlite.rs` test — updated to current ids where the id has meaning; left where it is only an opaque string |

Other places with the same defect: the embedding model ids (`text-embedding-004` is shut down) are BUG-013 and are not touched here.

## 5. Tests (`quality-assurance`)

Automated (`cargo test --workspace`, native runner and CI):

- Regression (fails before the fix): `/preset frontier` sets `claude-opus-5-5`, and no id returned by `/preset`, `chat_models()` or `onboarding_default()` appears in the retired table (`command_tests.rs`, new `llm_catalog` unit tests).
- `successor_of`: every retired id and dated snapshot maps to its successor; case-insensitive; an unknown or current id returns `None`; a custom id that only shares a prefix (`claude-3-5-sonnet-custom`) is not mapped.
- Resolution: a stored retired id resolves to the successor and returns the notice; the stored value is unchanged; `groq/openai/gpt-oss-120b` is routed to Groq and sent as `openai/gpt-oss-120b`; `llama-3.3-70b-versatile` maps to the Groq successor.
- Anthropic serialization: for `claude-opus-5-5`, `claude-sonnet-5-5` and `claude-haiku-5-5` the body has no `temperature` or `top_p`; for an older custom id (`claude-sonnet-4-6`) they are kept.
- OpenAI-compatible serialization: only if H1/H2 changed it, the same pair of tests.
- Model-not-found mapping: a 404 body from each adapter produces the "not available" message with model and provider.
- The `/models` text and the menu list the same ids, in the same order, as `chat_models()`.

Manual (`tests/MANUAL_TESTS.md`, curated in place):

- Change the preset test at lines 107-111: `balanced` sets `gpt-6-luna`, `frontier` sets `claude-opus-5-5`.
- Add one test per cloud provider (needs the user's keys): pick the provider's catalog model, ask a question, get an answer. This is the live evidence for H1/H2 in the release candidate run.
- Add a test: a workspace saved with `claude-3-7-sonnet` (set with `/model claude-3-7-sonnet` before upgrading, or with the same command after) answers with `claude-opus-5-5` and shows the retired-id notice once.

## 6. Documentation (`architecture-docs`, `user-docs`)

- `docs/architecture/lm-providers.md`: model resolution (retired-id mapping and notice, `groq/` prefix stripped, default `gpt-6-luna`), per-model sampling rule of the Anthropic adapter, the "not available" message.
- `docs/architecture/commands.md`: `/preset` and `/models` model ids, and that the lists come from the catalog.
- `docs/architecture/tui.md`: onboarding defaults and the models menu source.
- New ADR (next free number when the work starts; ADR-0136 is taken by BUG-007) for the decision in section 3.
- `README.md`: no model id appears today; check the `/preset` row and the quick start with `check_readme.py`, change nothing if they stay accurate.
- `build_index.py` and `check_docs.py` with zero errors.

## 7. Release, migration and rollback (`devops-release`)

- Version: patch, v0.45.1 (with the other bug plans for this version). No version bump in this pull request; the bump happens when the release candidate is cut.
- Data already affected: workspace and default model ids stored in `settings.db` with a retired id. They are not rewritten; they are mapped at resolution with a notice (section 3.2). Nothing to back up.
- Observability: the `warn` log line for a mapped id and the debug line for dropped sampling parameters, both covered by tests that assert the notice and the serialized body; the user-facing notice is the visible signal.
- Rollback: revert the pull request on `dev` (or on `main` followed by v0.45.2). Since no stored data changes, an older binary reads the same settings as before.
- CI: no workflow change.

## 8. Execution steps and milestones

Branch: `fix/retired-model-ids` from `dev`.

1. **M1 Evidence**: record the real API error for `claude-3-7-sonnet` (or the documented error shape) for the pull request; confirm H1/H2 from the official API references of OpenAI, DeepSeek and Groq.
2. **M2 Catalog**: `llm_catalog.rs` with models, defaults, onboarding defaults and the retired table; unit tests.
3. **M3 Resolution and adapters**: retired-id mapping and notice in `resolve_lm_provider`, `groq/` prefix, Anthropic sampling rule, OpenAI-compatible rule if needed, "not available" message; tests.
4. **M4 Surfaces**: `/preset`, `/models`, models menu, onboarding, defaults and the notice in the CLI and TUI read the catalog; update the tests that use the old ids.
5. **M5 Documentation and manual tests**: section 6 and the manual tests in section 5; `check_docs.py`, `check_readme.py`, `manual_tests.py validate`.
6. **M6 Review and pull request**: code review, `software-design` checklist and `check_design.py`; pull request into `dev` naming BUG-002 (and BUG-013 as the related item left out); CI green.

## 9. Implementation notes (2026-10-17)

Changes from the approved plan, found during implementation:

- **OpenAI flagship successor is `gpt-6-sol`, not `gpt-6.1-sol`.** OpenAI's GPT-6 guide states that GPT-6.1 Sol and GPT-6 Astra support Chat Completions but tool calling requires the Responses API, while GPT-6 Sol and GPT-6 Luna support function calling on Chat Completions with `reasoning_effort: "none"`. The agent needs tools, so `o1`, `o1-pro`, `o3-mini` map to `gpt-6-sol` and the catalog offers `gpt-6-sol`. `gpt-4o` stays out of the retired table (served until 2027-01-14).
- **H1 confirmed**: GPT-6 models reject `temperature`/`top_p` unless reasoning effort is `none`; `max_tokens` is deprecated in favour of `max_completion_tokens`. The OpenAI adapter now sends `reasoning_effort: "none"` and `max_completion_tokens` for `gpt-6-luna*`/`gpt-6-sol*`, no sampling parameters for other `gpt-6*` and o-series models, and the classic fields for every other OpenAI-compatible provider. H2 (DeepSeek, Groq) has no documented change; it is covered by MT-ANS-004 in the release candidate run.
- **Model-not-found message**: the six duplicated 401/403/429 chains in the Anthropic, OpenAI-compatible and Gemini chat calls were replaced by `LmError::from_http_status`, which also produces the 404 message.
- **No debug log for dropped sampling parameters**: `actx-lm` has no logging dependency; adding one for a debug line was not worth it. The behavior is covered by the serialization tests.
- **`ResolvedLm.configured`**: besides the provider model, resolution returns the id as the user selects it (successor applied, `groq/` kept), so the TUI header and later rebuilds keep routing correctly.
- **Onboarding Ollama default is `ollama/llama3`** (was `llama3`, which had no routing prefix and fell through to the first provider with a key).
- **Docs**: ADR-0137 amends ADR-0125; MT-CONF-003 updated, MT-ANS-004 extended to the catalog models, MT-CONF-004 added.
