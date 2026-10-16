---
id: 87
title: OpenAI Function Tool Call Protocol Alignment
status: accepted
date: '2026-10-02'
version: v0.32.5
subsystems: [lm-providers]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0087: OpenAI Function Tool Call Protocol Alignment

## Context

A flattened serialization of `ToolCall` made OpenAI reject requests with HTTP 400 because the `type: "function"` field was missing.

## Decision

Restructure `ChatMessage` serialization in `actx-lm::providers::openai` to follow the OpenAI function tool call specification exactly.

## Alternatives Considered

Not recorded.

## Consequences

- Multi-step tool calls work against OpenAI-compatible endpoints.
