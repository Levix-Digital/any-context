---
id: 133
title: Offline and privacy presets require a working local model
status: accepted
date: '2026-10-16'
version: null
subsystems: [commands]
supersedes: []
superseded_by: []
amends: [125]
amended_by: []
---

# ADR-0133: Offline and privacy presets require a working local model

## Context

ADR-0125 made `/preset offline` and `/preset privacy` switch the workspace to the recommended local Gemma 4 tier. ADR-0130 later established that local Gemma 4 generation is planned: the Gemma provider always delegates to the offline mock provider. The presets therefore reported a fully local configuration while every answer came from the mock provider (`Mock response to: ...`).

## Decision

- `offline` and `privacy` no longer change the model. They apply strict grounding, fast search and web search off only when the active model already generates locally (an `ollama/<model>` or `local/<model>` id).
- Otherwise the command fails with an explanation (local Gemma 4 is planned; select a local Ollama model first, for example `/model ollama/qwen2.5`) and changes nothing.
- `balanced` and `frontier` are unchanged.

## Alternatives Considered

- **Keep switching to Gemma 4**: rejected; it silently replaces real answers with mock output.
- **Pick an Ollama model automatically**: rejected; AnyContext cannot know which models the user has pulled.
- **Apply the settings but keep a cloud model**: rejected; the preset would claim to be offline while sending prompts to the cloud.

## Consequences

- An offline preset is only reported when answers really come from the local machine.
- Users without Ollama get an explicit instruction instead of an apparently working configuration.
- When local Gemma 4 generation ships, the preset can select it again in a new ADR.
