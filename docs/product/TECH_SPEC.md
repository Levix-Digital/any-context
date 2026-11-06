# AnyContext: Technical Specification

AnyContext is a local-first application written in Rust: a Cargo workspace with a core domain crate, an agent runtime, language model providers, optional local inference on ONNX Runtime, the `actx` CLI and terminal UI, and the installer; the Tauri desktop app joins in v1.0.0. The architecture style is **hexagonal** (`architecture.style` in `.dev-cycle.json`): the core (`crates/any-context-core-rs`) depends on ports, never on the CLI, the terminal UI or any surface. How each part works is in the technical documentation ([TECDOC.md](../../TECDOC.md)); the v2 target design is in [`docs/architecture/README.md`](../architecture/README.md).

## Platforms and Stack

- **Platforms**: Windows 10/11 x86_64 and Linux x86_64 today; macOS joins with the desktop app in v1.0.0, unsigned until there is revenue (Windows signed with Azure Artifact Signing, Linux with GPG).
- **Language**: Rust (memory-safe, static binaries, no runtime to install); the Python code was purged (ADR-0106).
- **Async runtime**: Tokio.
- **Vector store**: LanceDB, tables per workspace and embedding target (ADR-0138).
- **Lexical index**: BM25 today; Tantivy per workspace in v1.0.0 (roadmap v2, R1 foundations).
- **Relational store**: SQLite `settings.db`, the single source of truth for workspaces, sources, settings, sync telemetry and conversation memory (ADR-0078, ADR-0082).
- **Local inference**: ONNX Runtime (`ort`, dynamic loading) for the cross-encoder re-ranker and the local embedding model (ADR-0121, ADR-0130, ADR-0154).
- **Language models**: OpenAI-compatible APIs, Anthropic, Gemini and Ollama for local models, chosen by the user per workspace.
- **Terminal UI**: Ratatui (ADR-0077). **Desktop UI**: Tauri 2 ([desktop-ui-v2.md](../architecture/desktop-ui-v2.md)).
- **Shared Levix components**: `levix-platform` crates for identity, keys, vault, policies, license and signature verification ([platform-shared-components.md](../architecture/platform-shared-components.md)).

## Architecture Overview

A question flows from a surface (CLI, terminal UI, desktop app, later the MCP server) to the agent runtime, which routes it: Fast runs one hybrid search and one model call; Deep decomposes the question, searches in batches and reflects before answering. Retrieval fuses dense vectors and BM25 with Reciprocal Rank Fusion, re-ranks with the local cross-encoder, reads its scores as relevance bands and selects chunks within a token budget per mode (ADR-0156 to ADR-0159). Settings changes go through the UI-agnostic command engine (ADR-0084), which starts background sync workers; ingestion scans sources, chunks, redacts secrets, embeds and writes encrypted rows. The deployment modes (local, desktop plus Model Server, client plus AnyContext Server) are in [deployment-architecture-v2.md](../architecture/deployment-architecture-v2.md).

## Non-Functional Requirements

### Performance

- No retrieval change is merged if it lowers the scores of the development retrieval benchmark (ADR-0153).
- The re-ranker depth is bounded per mode, hardware profile and time budget (ADR-0156); exact latency targets per profile are an open question.
- Incremental sync reprocesses only changed chunks (per-chunk diff by hash).

### Security and Privacy

- Nothing leaves the machine except the language model call; with a local model, nothing leaves. No telemetry.
- Every LanceDB row is encrypted at field level (ADR-0131, ADR-0135); the v1.0.0 key hierarchy replaces the machine-derived key, and the lexical index is encrypted too ([identity-and-governance-v2.md](../architecture/identity-and-governance-v2.md)).
- Secret values are redacted before chunking, with no exception (ADR-0149); reserved prompt tags in retrieved text are neutralized (ADR-0136, ADR-0148).
- Plan licenses and enterprise policies are signed by Levix or the Admin Hub and verified offline; downloads are signature-verified.
- Offline and privacy presets require a working local model (ADR-0133); web search uses only allowed search APIs (ADR-0152) and is off by policy in isolated mode.

### Availability and Recovery

AnyContext is a local application, not a service: it has no uptime target. Recovery: the index can always be rebuilt from the sources; offline sources keep their index (ADR-0150); a lapsed plan deactivates indexes and never deletes them (connectors-v2, decision 26); the updater swaps binaries atomically (ADR-0091, ADR-0103).

### Accessibility and Localization

The interface is in English (ADR-0119); questions and documents in any language the models support, with multilingual embedding and re-ranking. The desktop app follows the keyboard shortcuts and layout of [desktop-ui-v2.md](../architecture/desktop-ui-v2.md); a formal accessibility standard is an open question.

## Integrations

- **Language model providers** (outbound, prompts and retrieved passages): when unavailable, the answer fails with a clear message; local search keeps working.
- **Web connector** (Free, outbound): Jina Reader for pages and Tavily for web search; when unavailable or over the fair-use ceiling, web search pauses and local search keeps working.
- **GitHub releases** (`Levix-Digital/any-context-releases`, inbound): installer and updater downloads, signature-verified (ADR-0083).
- **Levix Admin Hub** (v2.0.0, both directions, metadata only): licenses, policies, update mirror and auditing; verified offline, so AnyContext keeps working when the Hub is unreachable.
- **MCP server** (exposed): lets the company's agents query workspaces; a machine contract to index and query a repository workspace is FEAT-196.

## Data

- **Workspace**: sources, settings, models and conversation history; isolated from other workspaces.
- **Source**: a folder, share, site or connector target, with its sync state.
- **Chunk**: text, provenance columns and embedding, in LanceDB per workspace and embedding target; BM25 or Tantivy next to it.
- **Conversation memory**: in `settings.db`, a tree of chat history from v1.0.0.
- All data lives on the user's machine (or the customer's server in mode 3). Schema changes are additive (columns added in place, ADR-0142); a change of embedding target reindexes the workspace (ADR-0138).

## Constraints

- One-person team: the design favors one codebase, few moving parts and documented decisions (ADRs).
- Always hosted by the user or the customer: no Levix-hosted service in the data path.
- Official APIs only for every connector and external service (connectors-v2, decision 32).
- Every change to `levix-platform` or `levix-admin-hub` stays generic and goes through a pull request in that repository.
- The core never imports a surface (`forbidden_core_imports` in `.dev-cycle.json`); `settings.db` is the single source of truth (no parallel databases).
- Proprietary license with source access for audit (LICENSE); third-party components keep their own licenses, listed with each release.

## Open Questions

- Latency targets per hardware profile for Fast and Deep answers (IMP-055 weighs the re-ranker latency).
- The accessibility standard for the desktop app.
