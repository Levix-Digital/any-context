---
id: 132
title: Query vectors use the ingestion embedding provider and model
status: accepted
date: '2026-10-16'
version: null
subsystems: [retrieval, ingestion, cli]
supersedes: []
superseded_by: []
amends: []
amended_by: [138]
---

# ADR-0132: Query vectors use the ingestion embedding provider and model

## Context

The sync worker embeds chunks with the provider and model chosen by `resolve_embedding_provider` (explicit override, `ACTX_EMBEDDING_MODEL`, `ACTX_MODEL`, workspace model, default model; for example Gemini `text-embedding-004`, Ollama `nomic-embed-text` or OpenAI `text-embedding-3-small`). Retrieval embedded the query with the chat provider of the session and the hard-coded model name `text-embedding-3-small`. When the two differed, query vectors were in another vector space than the chunk vectors (meaningless dense scores), and with a provider that cannot embed (Anthropic) dense search was skipped entirely.

## Decision

- `resolve_embedding_provider(db, model_override, workspace)` becomes a public function in `ingestion::orchestrator` (re-exported from `ingestion`) and is the single resolver for both chunk and query vectors.
- `NativeHybridPipeline` gains `set_embedding(provider, model)` and an `embedding_model` field (default `text-embedding-3-small`), used instead of the hard-coded name.
- The CLI resolves the query embedding once per agent (`engine::resolve_query_embedding`) and passes it to the `search_db` tool, the one-hop deterministic retriever and the Deep Search batch retriever; the chat provider is no longer used for embeddings.

## Alternatives Considered

- **Keep using the chat provider and only fix the model name**: rejected; the chat provider may not support embeddings at all, and a different provider produces incompatible vectors.
- **Store the embedding model in the index and read it back**: more robust when settings change after indexing, but needs a schema change; left for later.

## Consequences

- Query and chunk vectors come from the same model, so dense search works with every chat provider, including Anthropic.
- If the workspace model changes after indexing, both sides follow the new setting and a re-index (`actx sync --force`) is needed to rebuild the chunk vectors.
