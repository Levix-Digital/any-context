---
id: 137
title: Chat model catalog with retired model ids resolved to successors
status: accepted
date: '2026-10-17'
version: v0.45.1
subsystems: [lm-providers, commands, tui]
supersedes: []
superseded_by: []
amends: [125]
amended_by: []
---

# ADR-0137: Chat model catalog with retired model ids resolved to successors

## Context

Chat model ids were hard-coded in five places that did not share a list: `/preset`, the `/model list` text, the TUI models menu, the onboarding wizard and the application default (`gpt-4o-mini`). By 2026-10-17 most of these ids were no longer served by their providers. Examples: `claude-3-7-sonnet` in `/preset frontier` ([ADR-0125](0125-local-gemma-activation-hardware-guardrails.md)), `claude-3-5-sonnet` in onboarding, `gemini-1.5-pro`, `llama-3.3-70b-versatile` (Groq), `deepseek-chat` and `deepseek-reasoner`. `o1` and `o3-mini` shut down on 2026-10-23. Choosing any of them failed on the first question. The id is sent to the provider unchanged, and the ids users already chose are stored in `settings.db` (BUG-002).

The current models also changed their request rules:

- Claude Opus 5.5 rejects `temperature` and `top_p`; Claude Sonnet 5.5 and Haiku 5.5 reject non-default values. The Anthropic adapter always sent both.
- On Chat Completions, the GPT-6 models reject sampling parameters unless `reasoning_effort` is `none`. Only GPT-6 Luna and GPT-6 Sol accept `none` and function calling. GPT-6.1 Sol and GPT-6 Astra need the Responses API for tools.
- Groq ids were routed only by a `groq` prefix that was then sent to the API.

## Decision

- `crates/any-context-core-rs/src/llm_catalog.rs` is the single list of offered chat models (`chat_models()`), the defaults (`DEFAULT_CHAT_MODEL` and `BALANCED_PRESET_MODEL` = `gpt-6-luna`, `FRONTIER_PRESET_MODEL` = `claude-opus-5-5`), the onboarding model per provider (`onboarding_default`) and a table of retired ids with their successors (`successor_of`). `/preset`, `/model list`, the models menu, the onboarding wizard and the defaults read it.
- `successor_of` matches exact ids only (case and surrounding spaces ignored), never prefixes, so custom ids are never rewritten.
- `resolve_lm_provider` (`crates/actx-cli/src/engine.rs`) replaces a retired id with its successor and logs a `warn` line. It returns a notice that the CLI prints and the TUI shows as a system message. The stored settings are not changed. It returns `ResolvedLm { provider, model, configured, notice }`: `model` is sent to the provider, and `configured` keeps the routing prefix for display and later resolution.
- Groq models use the `groq/` routing prefix (`groq/openai/gpt-oss-120b`), which is removed before the request, like `ollama/`.
- Provider wire rules per model: the Anthropic adapter leaves out `temperature`/`top_p` for the Claude families that reject them. For GPT-6 Luna and Sol, the OpenAI adapter sends `reasoning_effort: "none"` and `max_completion_tokens`. For the other GPT-6 models and the o-series it sends no sampling parameters and uses `max_completion_tokens`. Other OpenAI-compatible providers keep the classic fields.
- A 404 from a provider chat API becomes "model `<id>` is not available from `<provider>`; choose another with /model" (`LmError::from_http_status`).
- `gpt-4o` and `gpt-4o-mini` are not in the retired table while OpenAI still serves them (until 2027-01-14); only the defaults moved.

## Alternatives Considered

- **Rewrite the stored ids in `settings.db` with a migration**: rejected. The change would be permanent and need its own rollback, and an older binary would then read ids it does not know.
- **Only report the error and ask the user to choose another model**: rejected. Every affected user would stay broken until they act, and onboarding and presets would keep offering dead ids.
- **Discover models from each provider's models endpoint**: rejected for this fix. It needs a key and network access before the first question, and it does not answer which successor a retired id should get.
- **GPT-6.1 Sol as the OpenAI flagship**: rejected because tool calling on Chat Completions is not available for it, and the agent depends on tools. GPT-6 Sol is used instead.

## Consequences

- When a provider retires the next model, the fix is one line in the retired table, one in the catalog and their tests. `offered_models_are_never_retired` and `every_successor_is_offered_or_a_default` keep the two lists consistent.
- The notice tells users which model is now used; their stored choice keeps working without a migration, and rolling back the release is a plain revert.
- The provider wire rules are prefix-based and need an update when a provider changes request rules for a family.
- Embedding model ids are not covered (BUG-013).
- Verification: `cargo test --workspace` (catalog, resolution, adapter serialization and the `/preset` regression) and the manual test per cloud provider in the release candidate run.
