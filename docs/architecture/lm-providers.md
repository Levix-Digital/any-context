---
title: Language Model Providers
summary: Provider-neutral chat, streaming and embedding API over OpenAI-compatible, Anthropic, Gemini, Gemma and mock backends.
code: [crates/actx-lm, crates/actx-cli/src/engine.rs]
---

# Language Model Providers

Provider-neutral chat, streaming and embedding API over OpenAI-compatible, Anthropic, Gemini, Gemma and mock backends.

## Responsibilities

- Define one contract (`LmProvider`) for chat completion, token streaming and embeddings, and the request, response and stream types shared by every backend.
- Implement the HTTP adapters for the supported providers, including tool calls and reasoning ("thinking") streams.
- Decode Server-Sent Events robustly.
- Provide a deterministic offline mock provider so the application works without credentials or network.

Which provider and model a workspace uses is resolved by the [CLI](cli.md) (`resolve_lm_provider`); credentials come from [storage](storage.md).

## Structure

| Item | File | Role |
|---|---|---|
| `LmProvider` | `traits.rs` | `provider_id`, `chat_complete`, `chat_stream`, `embed` (default: unsupported). |
| `ChatRequest`, `ChatMessage`, `Role`, `SamplingParams` | `types/request.rs`, `types/message.rs` | Request model, including tool definitions and temperature. |
| `ChatResponse`, `FinishReason`, `TokenUsage` | `types/response.rs` | Completion result, including optional thinking and tool calls. |
| `StreamChunk` | `types/stream.rs` | `Token`, `Reasoning`, `ToolCallDelta`, `Completed { finish_reason, usage }`. |
| `ToolCall`, `ToolDefinition` | `types/tool.rs` | Function tool calling. |
| `ProviderKind::build` | `providers/mod.rs` | Factory from provider kind, API key and base URL. |
| `OpenAiCompatibleProvider` | `providers/openai.rs` | OpenAI, Ollama, Groq, DeepSeek, OpenRouter and custom compatible servers (LM Studio, vLLM, LocalAI). |
| `AnthropicProvider` | `providers/anthropic.rs` | Anthropic Messages API with thinking blocks. |
| `GeminiProvider` | `providers/gemini.rs` | Google Gemini API (`generativelanguage.googleapis.com/v1beta`). |
| `GemmaOnnxProvider` | `providers/gemma.rs` | Local Gemma 4 placeholder that delegates to a fallback provider. |
| `MockLmProvider` | `providers/mock.rs` | Deterministic offline provider. |
| `LmClient`, `LmClientBuilder` | `client.rs` | Facade over a provider with a default model. |
| SSE decoder | `sse.rs` | Frame parsing for streaming responses. |

## Behavior

### Providers

| Kind | Endpoint | Key required | Embeddings |
|---|---|---|---|
| `OpenAi` | `https://api.openai.com/v1` | yes | yes |
| `Ollama` | `http://localhost:11434/v1` (overridable) | no | yes (OpenAI-compatible) |
| `Groq`, `DeepSeek`, `OpenRouter` | their OpenAI-compatible endpoints | yes | through the OpenAI-compatible adapter |
| `CustomCompatible` | any base URL | optional | through the OpenAI-compatible adapter |
| `Anthropic` | Messages API (`x-api-key`, `anthropic-version`) | yes | no |
| `Gemini` | `streamGenerateContent?alt=sse` | yes | yes |
| `GemmaOnnx` | local | no | no |
| `Mock` | none | no | deterministic vectors |

`ProviderKind::build` returns an authentication error when a provider that needs a key has none; the CLI then falls back to the mock provider so the application never crashes.

### Model resolution in the application

`resolve_lm_provider(model_override, workspace)` picks the model from, in order, the explicit override, `ACTX_MODEL`, the workspace model, the default model and `gpt-4o-mini`, then chooses the provider by prefix:

- `mock*` → mock; `ollama/<model>` or `local/<model>` → Ollama; `gemma*` → Gemma (always wrapped with the mock fallback);
- `claude*` → Anthropic; `gemini*` → Gemini; `deepseek*` → DeepSeek; `groq*` → Groq; `gpt*`, `o1*`, `o3*` → OpenAI;
- anything else → the first provider with a configured key (OpenAI, Anthropic, Gemini, DeepSeek, Groq), otherwise the mock provider.

When the selected provider has no key, answers come from the mock provider, which echoes the request (`Mock response to: ...`).

### Streaming and tools

Streaming adapters decode SSE frames into `StreamChunk`s: answer tokens, reasoning tokens (Anthropic thinking deltas and the `reasoning_content` field of OpenAI-compatible APIs such as DeepSeek), incremental tool call arguments and a final `Completed` chunk with the finish reason and usage. OpenAI-compatible requests serialize tool calls exactly as the OpenAI function-calling specification requires (`type: "function"` with nested `function { name, arguments }`), so multi-step tool conversations are accepted.

### Local Gemma 4

`GemmaOnnxProvider::new(path, tier).with_fallback(provider)` is used for every `gemma*` model. Local Gemma 4 generation is planned (see [ONNX Inference](onnx.md)), so the provider always delegates to its fallback; the application passes the mock provider, which keeps local selections offline and free.

## Interfaces

- **Used by**: the [agent](agent.md) (chat and streaming), the [retrieval pipeline](retrieval.md) (query embeddings), [ingestion](ingestion.md) (chunk embeddings), the ingestion model router (vision prompts when a vision mode is enabled).
- **Depends on**: `reqwest` with `rustls`, and `actx-onnx` for the Gemma tier types.

## Decisions

- [ADR-0087](../adr/0087-openai-tool-call-protocol-alignment.md): OpenAI function tool call serialization.
- [ADR-0090](../adr/0090-grounding-parity-streaming-single-lancedb-path.md): real-time token streaming through `chat_stream`.
- [ADR-0123](../adr/0123-onnx-production-activation-gemma-4-tiers.md): `GemmaOnnxProvider` with fallback delegation.
- [ADR-0125](../adr/0125-local-gemma-activation-hardware-guardrails.md): Gemma models routed by `resolve_lm_provider`, offline fallback when weights are missing.
- [ADR-0130](../adr/0130-real-onnx-runtime-multilingual-cross-encoder.md): Gemma 4 planned; the provider always uses the fallback.
