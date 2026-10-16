---
id: 124
title: Web Ingestion with Jina Reader, Pure Grounded RAG and Seven Semantic Domain Matrix
status: accepted
date: '2026-10-14'
version: v0.39.0
subsystems: [ingestion, agent]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0124: Web Ingestion with Jina Reader, Pure Grounded RAG and Seven Semantic Domain Matrix

## Context

1. The website and documentation crawler used basic HTML tag stripping (`strip_html_tags`), which fails on documentation portals built as single-page applications (React, Next.js, Docusaurus, GitBook), producing empty pages or garbled text without code blocks and tables.
2. Injecting live web content directly into the LLM context window without indexing it first would violate the core philosophy of AnyContext, which is exclusively a **grounded RAG** system.
3. The previous intent classification (four domains: `Code`, `Architecture`, `Document`, `General`) did not cover critical software engineering areas (DevOps/infrastructure, databases/schemas and security/audit).

## Decision

1. **Jina Reader used only at ingestion time (`crates/any-context-core-rs/src/ingestion/jina.rs`)**:
   - Add the `NativeJinaExtractor`, integrated with `NativeWebCrawler` (`crawler.rs`).
   - When `/sync` runs over the workspace's web sources (`/web add <url>`), the crawler calls `https://r.jina.ai/{url}` with `Accept: text/markdown`, obtaining high-fidelity structured Markdown with no RAM cost on the operator's machine.
   - Incremental control through RFC 7232 (`ETag`, `If-Modified-Since`) and SHA-256 hashing is preserved.
   - ADR-110 guarantee: on an HTTP error or without network access, the crawler falls back transparently to direct extraction of sanitized HTML without interrupting the sync.
2. **Pure RAG preserved**: the inference model (local Gemma 4 family or cloud) reads **only from the native RAG index** (`NativeHybridPipeline`: LanceDB + BM25). Unindexed web content never enters the prompt.
3. **Seven semantic domain matrix (`actx-agent::router`)**: extend the `QueryIntent` and `OnnxIntent` enums to seven canonical domains:
   - `Code`: syntax, algorithms, bugs, refactoring, panics.
   - `Architecture`: topology, diagrams, microservices, FSM, ADRs.
   - `DevOpsInfra`: CI/CD, Docker, shell/PowerShell scripts, ports, environment variables.
   - `DataSchema`: SQL, SQLite, LanceDB, migrations, tables, tabular data.
   - `SecurityAudit`: vulnerabilities, credentials, authentication, sanitization, LGPD.
   - `Document`: contracts, invoices, reports, tax terms, compliance.
   - `General`: open conceptual questions, greetings, translations.
   - Contextual telemetry with precise badges in the TUI and in headless mode.

## Alternatives Considered

Not recorded.

## Consequences

- Single-page documentation portals are ingested with code blocks and Markdown hierarchy preserved.
- No risk of hallucination or loss of vector grounding.
- Fine-grained semantic routing with clear telemetry for the operator.
