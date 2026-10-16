---
id: 87
title: OpenAI Function Tool Call Protocol Alignment
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0087: OpenAI Function Tool Call Protocol Alignment

- **Status**: Accepted & Implemented (`v0.32.5`).
- **Context**: Flattened serialization of `ToolCall` caused OpenAI HTTP 400 rejection due to missing `type: "function"` parameter.
- **Decision**: Restructure `ChatMessage` serialization in `actx-lm::providers::openai` to strictly adhere to the OpenAI function tool call specification.

---
