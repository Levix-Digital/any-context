<!-- dev-cycle-constitution
generated-by: dev-cycle-protocol/constitution.py (do not edit; change a source and regenerate)
sources-sha256: 16e0540c3cc6c88b101b2cc158aed2f6e1db88468903f0cda6386f96818728cf
content-sha256: 975a9a448f1f3dfaecc50b8e510e84b0c0a3cccc5e809956e8e7d0d984896b60
-->
# any-context Constitution

The non-negotiable rules every specification, plan and task of this project follows. It is generated from the project's sources of truth by `dev-cycle-protocol/constitution.py`; change a source and regenerate it, never edit it.

## Core Principles

### I. Design

*Source: software-design*

#### Modularity and separation of responsibilities
- One module, type or function has **one reason to change**. When a description needs "and", split it.
- Group code by what it is about (a domain concept or a feature), not by what kind of file it is, unless the architecture style says otherwise.
- Business rules never live in the user interface, in transport code (HTTP handlers, CLI argument parsing) or in persistence code.
- Dependencies point toward stable code: details (frameworks, databases, network, UI) depend on the core, never the other way around.

#### SOLID, in practice
| Principle | What it means when writing code | Smell that it is violated |
|---|---|---|
| **S**ingle responsibility | Each unit does one job and can be named without "and". | A 600-line file mixing parsing, rules and output. |
| **O**pen/closed | Add behavior by adding a new implementation, not by growing an `if/else` chain in stable code. | Every new provider edits the same `match` in five places. |
| **L**iskov substitution | Any implementation of an interface can replace another without surprises. | An implementation that panics or ignores half the contract. |
| **I**nterface segregation | Small interfaces shaped by what the caller needs. | Callers implementing methods they never use. |
| **D**ependency inversion | High-level code depends on abstractions it owns; adapters implement them. | Core code constructing an HTTP client or a database driver. |

#### Clean Code
- **Names** say what something is or does; no abbreviations a newcomer cannot decode; no `data`, `info`, `manager`, `util` as whole names.
- **Small functions** that do one thing at one level of abstraction; early returns instead of deep nesting.
- **No magic values**: named constants with units in the name (`TIMEOUT_MS`).
- **Comments explain why**, never what; delete commented-out code and dead code.
- **Consistency over preference**: follow the conventions already in the codebase (naming, error types, layout) unless the change is an agreed refactor.
- **Duplication**: remove it when the same rule exists twice; tolerate it when two pieces only look alike today.

#### Explicit error handling
- Never swallow an error (`except: pass`, `let _ =` on a meaningful result, an empty `catch`, a blind `.unwrap()` on input that can fail).
- Errors carry context (what was being done, with which input) and use the project's canonical error types.
- Decide at each boundary: recover, translate into the caller's error type, or propagate. Report to the user once, at the edge.
- Fallbacks are allowed only when they are explicit, logged and visible in status output; a silent fallback that changes behavior is a defect.

#### Testability
- Pure logic is separated from I/O so it can be unit-tested without mocks.
- External services sit behind an interface the tests can replace.
- A structure that is hard to test is a design problem, not a testing problem.

### II. Simplicity First

*Source: software-design*

Principles applied mechanically produce over-engineering. The rule: **the simplest solution that keeps responsibilities separate.**

- Introduce an abstraction (trait, interface, base class, factory) only when at least one of these is true:
  1. there is a **second implementation** now (not "maybe later");
  2. it creates a **test seam** you need today;
  3. it isolates an **external dependency** (framework, service, database, OS API).
- No speculative configuration, plugin systems or generic frameworks for a single use case.
- Prefer composition over inheritance; prefer plain data and functions over class hierarchies.
- Three similar lines are better than a premature helper with five parameters.

#### Design patterns: only when they solve a real problem
| Problem actually present | Pattern that fits |
|---|---|
| Several interchangeable algorithms or providers chosen at runtime | Strategy |
| A third-party or legacy interface that does not match yours | Adapter |
| Construction is complex or depends on configuration | Factory / Builder |
| Behavior changes with an explicit lifecycle state | State machine |
| Actions must be queued, logged, undone or dispatched uniformly | Command |
| Persistence must be swappable or mocked | Repository |
| Several parts react to an event without knowing each other | Observer / event channel |

If you cannot point to the problem in the left column, do not use the pattern. Name the pattern in code or docs only when it helps the reader.

### III. Architecture Style: hexagonal

*Source: software-design/styles/hexagonal.md*

##### Structure

- **Core (domain + application services)**: entities, business rules, use cases and the **ports** they need (interfaces/traits for storage, providers, clocks, file systems). No framework, UI, transport or driver code.
- **Driving adapters** (input): CLI, TUI, HTTP handlers, RPC, MCP servers. They translate requests into core calls and core results into output.
- **Driven adapters** (output): database repositories, HTTP clients, model providers, file system access. They implement the core's ports.
- **Composition root**: one place per executable that builds the adapters and wires them into the core.

##### Dependency rule

Adapters depend on the core; the core depends on nothing outside itself except plain libraries (serialization, math, date handling). List the adapter and framework modules in `architecture.forbidden_core_imports` so `check_design.py` enforces the rule.

##### Where things go

| Piece | Location |
|---|---|
| A business rule or validation | Core service or entity |
| A new user command | Core command/use case, then one thin handler per surface |
| Database schema or query | Driven adapter implementing a core repository port |
| A new LLM, payment or storage provider | Driven adapter implementing an existing port |
| Output formatting, colors, layout | Driving adapter |

##### Common mistakes

- Business logic in a CLI handler or TUI event loop "because it was quicker".
- The core constructing its own HTTP client, database connection or UI widget.
- Ports shaped like the first adapter's API (leaking driver types into the core).
- A port for something with one implementation and no test need (see "Simplicity First").

### IV. Development Cycle

*Source: dev-cycle-protocol*

1. **Design**: modular code with clear responsibilities, SOLID, Clean Code, explicit errors, patterns only when needed, in the project's architecture style (`software-design`).
2. **Evidence**: every change is proven by automated tests; manual tests cover only what cannot be automated, in one curated suite (`quality-assurance`).
3. **Documentation describes today**: technical docs and README are edited in place; history lives in Git and release notes (`architecture-docs`, `user-docs`).
4. **Plans are shared and complete**: approved plans are versioned files in the project (`plan-memory`).
5. **Safe delivery**: short-lived branches on the project's branching model, CI on every pull request, release candidates, the user's release gate (`devops-release`).
6. **Security**: no secrets in Git, backward-compatible data migrations, a rollback path for every release.
7. **Bugs first**: open `critical` and `high` bugs are planned and fixed before new features, improvements and adjustments, unless the user explicitly accepts the risk; nothing found is lost, it is fixed now or captured in the backlog.
8. **Follow-through**: the agent follows CI and releases to the end and fixes what fails; the user's decisions (plan approval, candidate acceptance, the release gate) are never taken by the agent.
9. **No AI attribution, anywhere**: no artifact names an AI, a model or a coding agent as author, co-author, reviewer or source: commits and their trailers, tags, branches, pull and merge requests, reviews and review comments, issues, plans, documentation, release notes and code comments. Everything is authored by the repository's Git identity (`devops-release`). Configuration a project needs for its tools (an agent's settings folder, an integration name) is technical, not attribution.

## Architecture Decisions

Accepted decisions; a plan that contradicts one needs a new ADR that supersedes it.

- ADR-0075: Native Rust ReAct Agent with an Explicit FSM (actx-agent) (`docs/adr/0075-native-rust-agent-fsm.md`)
- ADR-0076: Native Rust Hybrid RAG Pipeline and RFC-042 Batch Retrieval Engine (`docs/adr/0076-native-hybrid-rag-pipeline.md`)
- ADR-0077: Native Rust CLI and Full-Screen Interactive TUI (actx-cli with Ratatui) (`docs/adr/0077-native-ratatui-cli-tui.md`)
- ADR-0078: Normalized Relational Folder Storage and Legacy Schema Elimination (`docs/adr/0078-relational-workspace-folders.md`)
- ADR-0079: Multi-Byte UTF-8 Character Boundary Safety, Visual Cursor Alignment and Universal Release Protocol (`docs/adr/0079-utf8-safe-input-auto-scroll.md`)
- ADR-0080: Workspace-Isolated Prompt History Navigation (`docs/adr/0080-workspace-isolated-prompt-history.md`)
- ADR-0081: TUI Top Header Quad-Status Telemetry (`docs/adr/0081-tui-header-status-telemetry.md`)
- ADR-0082: SQLite Session Memory and Canonical Identity Injection (`docs/adr/0082-sqlite-session-memory-identity-injection.md`)
- ADR-0083: Dedicated Releases Repository and Semver Updater (`docs/adr/0083-dedicated-releases-repository-semver-updater.md`)
- ADR-0084: Universal UI-Agnostic Command Engine (Hexagonal Architecture) (`docs/adr/0084-ui-agnostic-command-engine.md`)
- ADR-0085: Grounding Strategy vs Search Depth Mode Decoupling and Quint-Status Header (`docs/adr/0085-grounding-vs-search-depth-decoupling.md`)
- ADR-0086: Scoped Non-Blocking LanceDB Invocation and Safe Runtime Destruction (`docs/adr/0086-non-blocking-lancedb-safe-runtime-drop.md`)
- ADR-0087: OpenAI Function Tool Call Protocol Alignment (`docs/adr/0087-openai-tool-call-protocol-alignment.md`)
- ADR-0088: High-Level Conversational Session Boundary and Orphaned Tool Scrubbing (`docs/adr/0088-conversational-session-boundary.md`)
- ADR-0089: TUI Error Surfacing Guarantee and Temporary Search Depth UI Concealment (`docs/adr/0089-tui-error-surfacing-search-depth-concealment.md`)
- ADR-0090: Grounding Parity, Real-Time Token Streaming and Single Canonical LanceDB Path (`docs/adr/0090-grounding-parity-streaming-single-lancedb-path.md`)
- ADR-0091: Hermetic Workspace Isolation, Windows Atomic Executable Swap and Production Installer Canonicalization (`docs/adr/0091-hermetic-workspace-isolation-atomic-exe-swap.md`)
- ADR-0092: Virtual Tab Workspace Chat Buffer Isolation and Viewport Lifecycle (`docs/adr/0092-virtual-tab-chat-buffer-isolation.md`)
- ADR-0093: Sync Worker Background Stdio Isolation and Chain of Responsibility Ingestion Engine (`docs/adr/0093-sync-worker-stdio-isolation-chain-of-responsibility.md`)
- ADR-0094: Test Isolation Sandbox, Universal Contextual Option Hinting and Skill-Based Self-Knowledge via Global LanceDB Workspace (`docs/adr/0094-test-sandbox-option-hinting-global-self-knowledge.md`)
- ADR-0095: Clean Screen Buffer Initialization, Long-Term Memory Decoupling and Virtual Tab Lifecycle (`docs/adr/0095-clean-screen-buffer-long-term-memory-decoupling.md`)
- ADR-0096: Bounded BM25 Deserialization, Atomic Temp-Rename and Self-Healing Index Recovery (`docs/adr/0096-bounded-bm25-deserialization-self-healing-index.md`)
- ADR-0097: Canonical SQLite Sync Telemetry, Live TUI Footer Progress Bar and Native Agent Background Awareness (`docs/adr/0097-sqlite-sync-telemetry-agent-background-awareness.md`)
- ADR-0098: Parameter Order Inversion Resolution in Native LanceStore (count_records) and Live Telemetry Unfreezing (`docs/adr/0098-lancestore-count-records-parameter-fix.md`)
- ADR-0099: Full Core Command Parity, Worker Auto-Spawning and Hexagonal Command Engine Alignment (`docs/adr/0099-core-command-parity-worker-auto-spawn.md`)
- ADR-0100: Native Encryption-at-Rest Interoperability, Background Worker Cancellation and TUI Screen Isolation (`docs/adr/0100-native-encryption-sync-cancel-tui-isolation.md`)
- ADR-0101: Shadowed Binary Synchronization, User PATH Prioritization and Clean Telemetry (`docs/adr/0101-shadowed-binary-sync-user-path.md`)
- ADR-0102: Scoped Directory Ingestion and Multi-Byte UTF-8 Boundary Clamping (`docs/adr/0102-scoped-folder-ingestion-utf8-boundaries.md`)
- ADR-0103: Robust Multi-Binary Windows Self-Updating, Target Version Delimiters and Sync Indicator Parity (`docs/adr/0103-windows-multi-binary-self-update.md`)
- ADR-0104: CI/CD Release Pipeline Optimization via Unified Workspace Compilation and Fast Multi-Threaded Packaging (`docs/adr/0104-unified-release-compilation-fast-packaging.md`)
- ADR-0105: Full Native Rust Ingestion and Sync Pipeline (Deprecation of Python Worker Runtimes) (`docs/adr/0105-native-rust-ingestion-sync-pipeline.md`)
- ADR-0106: Full Ingestion and Retrieval Parity in Rust and Purge of Legacy Python Code (`docs/adr/0106-full-rust-parity-python-purge.md`)
- ADR-0107: Removal of Orphaned Files and Unified Release Assets for Legacy Updaters (`docs/adr/0107-orphan-cleanup-legacy-updater-compatibility.md`)
- ADR-0108: Reflective Agentic Deep Search Engine and Adaptive Routing (RFC-042) (`docs/adr/0108-reflective-deep-search-engine.md`)
- ADR-0109: CI/CD Acceleration with Sccache and Release Profile Optimizations (`docs/adr/0109-ci-sccache-release-profile.md`)
- ADR-0110: Ingestion ModelRouter, Deterministic Quality Gate and Spatial Form Reconstruction (RFC-043) (`docs/adr/0110-ingestion-model-router-quality-gate.md`)
- ADR-0112: UX Stabilization, Deep Search Language Parity, Multi-Line Prompt and Transparent Self-Knowledge (`docs/adr/0112-ux-stabilization-deep-search-multiline-prompt.md`)
- ADR-0113: Removal of Heuristic Inversion in the Tabular Chunker and Unconditional Preservation of 2D Markdown Formats (`docs/adr/0113-tabular-chunker-markdown-table-preservation.md`)
- ADR-0114: Modular Aurora Borealis Design Tokens and Hybrid Startup Fast-Path (Option C) (`docs/adr/0114-aurora-design-tokens-startup-fast-path.md`)
- ADR-0115: Native ONNX Weights Repository, Resilient Fallbacks, Search Mode Header Indicator and Human Conversational Tone (`docs/adr/0115-onnx-weights-repository-fallbacks.md`)
- ADR-0116: Smart Path Healing, Restored Temporal Metadata Rules (Freshness Cascade and HTTP 304) and Active Source Change Detection (`docs/adr/0116-path-healing-temporal-metadata-change-detection.md`)
- ADR-0117: Aurora Borealis Color Harmonization, Borderless Canvas, ReAct Auto-Scroll, Friendly Update Messages, Graceful Download Cancellation and Neutral Onboarding Wizard (`docs/adr/0117-aurora-palette-borderless-canvas-onboarding.md`)
- ADR-0118: Canonical Catalog of Verified Standalone ONNX Models and HTTP Resilience (`docs/adr/0118-verified-single-file-onnx-model-catalog.md`)
- ADR-0119: Global English Internationalization, Point-by-Point Braille Spinner, Default Collapsed Reasoning Accordion and Standardized Release Notes (`docs/adr/0119-english-interface-spinner-release-notes.md`)
- ADR-0120: Intelligent Source Naming, Decoupled SQLite Alias, Shared Sources Deprecation and Canonical System Knowledge Manual (`docs/adr/0120-source-display-names-shared-sources-removal.md`)
- ADR-0121: Modular ONNX Crate (actx-onnx), Hardware Pre-Flight Check and Semaphore-Tiered Installation Profiles (`docs/adr/0121-modular-onnx-crate-hardware-profiles.md`)
- ADR-0122: CI Fast-Path Timing Resilience and SQLite Warmup under Virtualized Runners (`docs/adr/0122-ci-fast-path-timing-resilience.md`)
- ADR-0123: Activation of ONNX Models in Production Flows and Gemma 4 Family Support with Five Hardware Tiers (`docs/adr/0123-onnx-production-activation-gemma-4-tiers.md`)
- ADR-0124: Web Ingestion with Jina Reader, Pure Grounded RAG and Seven Semantic Domain Matrix (`docs/adr/0124-jina-reader-web-ingestion-seven-domains.md`)
- ADR-0125: Operational Activation of Local Gemma 4 ONNX in the CLI Engine, LM Studio-Style Hardware Guardrails and Safe Workspace Presets (`docs/adr/0125-local-gemma-activation-hardware-guardrails.md`)
- ADR-0126: Assisted Gemma 4 ONNX Weights Downloader in the CLI and TUI with Pre-Flight Checks and Real-Time Telemetry (`docs/adr/0126-assisted-gemma-weights-downloader.md`)
- ADR-0127: Tri-Platform Hardware Acceleration Matrix in actx-onnx with DirectML, Metal, CUDA, Vulkan and Graceful CPU SIMD Fallback (`docs/adr/0127-cross-platform-hardware-acceleration-matrix.md`)
- ADR-0128: Dual RAG Architecture with Backend-First Deterministic Retrieval, U-Shaped Reordering and XML Structural Isolation (`docs/adr/0128-backend-first-deterministic-dual-rag.md`)
- ADR-0129: Two-Stage Neural Retrieval with Local ONNX Cross-Encoder Re-Ranking, Hardware Acceleration and Graceful ADR-110 Fallback (`docs/adr/0129-two-stage-retrieval-cross-encoder-reranking.md`)
- ADR-0130: Real ONNX Runtime (ort load-dynamic), Multilingual mmarco Cross-Encoder and Rectification of ADRs 118/123/125/127/129 (`docs/adr/0130-real-onnx-runtime-multilingual-cross-encoder.md`)
- ADR-0131: Field-level encryption on every LanceDB write (`docs/adr/0131-field-level-encryption-on-every-lancedb-write.md`)
- ADR-0132: Query vectors use the ingestion embedding provider and model (`docs/adr/0132-query-vectors-use-the-ingestion-embedding-provider-and-model.md`)
- ADR-0133: Offline and privacy presets require a working local model (`docs/adr/0133-offline-and-privacy-presets-require-a-working-local-model.md`)
- ADR-0134: Release candidates and tagging on merge into main (`docs/adr/0134-release-candidates-and-tagging-on-merge-into-main.md`)
- ADR-0135: Automatic encryption of LanceDB rows stored in plaintext (`docs/adr/0135-automatic-encryption-of-lancedb-rows-stored-in-plaintext.md`)
- ADR-0136: Neutralizing reserved prompt tags in retrieved text (`docs/adr/0136-neutralizing-reserved-prompt-tags-in-retrieved-text.md`)
- ADR-0137: Chat model catalog with retired model ids resolved to successors (`docs/adr/0137-chat-model-catalog-with-retired-model-ids-resolved-to-succes.md`)
- ADR-0138: Record the embedding target per workspace and reindex on change (`docs/adr/0138-record-the-embedding-target-per-workspace-and-reindex-on-change.md`)
- ADR-0139: Retrieval order follows relevance (`docs/adr/0139-retrieval-order-follows-relevance.md`)
- ADR-0140: Chunk version attribute from explicit path tokens, no inferred scope (`docs/adr/0140-chunk-version-attribute-from-explicit-path-tokens-no-inferre.md`)
- ADR-0141: Deep Search synthesis uses the canonical context formatter through a port (`docs/adr/0141-deep-search-synthesis-uses-the-canonical-context-formatter-t.md`)
- ADR-0142: Chunk provenance columns in LanceDB, added in place (`docs/adr/0142-chunk-provenance-columns-in-lancedb-added-in-place.md`)
- ADR-0143: BM25-only hits take their text from LanceDB; batch deduplication by chunk id (`docs/adr/0143-bm25-only-hits-take-their-text-from-lancedb-batch-deduplicat.md`)
- ADR-0144: System prompt per answer path with the tool section generated from the registered tools (`docs/adr/0144-system-prompt-per-answer-path-with-the-tool-section-generate.md`)
- ADR-0145: Live status reaches the routed paths as retrieved context (`docs/adr/0145-live-status-reaches-the-routed-paths-as-retrieved-context.md`)
- ADR-0146: A failed embedding fails the file, and embedding outcomes are reported (`docs/adr/0146-a-failed-embedding-fails-the-file-and-embedding-outcomes-are.md`)
- ADR-0147: Constant vectors never reach dense search: the mock target is BM25 only and Global is embedded with the default target (`docs/adr/0147-constant-vectors-never-reach-dense-search-the-mock-target-is.md`)
- ADR-0148: Shared prompt safety primitives and the Deep Search intermediate envelope (`docs/adr/0148-shared-prompt-safety-primitives-and-the-deep-search-intermed.md`)
- ADR-0149: Secret values are redacted before chunking, with no exception (`docs/adr/0149-secret-values-are-redacted-before-chunking-with-no-exception.md`)

## Governance

- This constitution supersedes other practices for specifications, plans and tasks. A plan passes the Constitution Check or records each justified violation in its Complexity Tracking table; a lasting exception becomes an ADR.
- Amendments change the source, not this file: an ADR for decisions, the technical specification for requirements and constraints, `.dev-cycle.json` for the architecture style, the skills for principles. Then run `constitution.py generate` and commit the result with the change.
- Gate 3 of the development cycle blocks a plan while this file is stale or edited by hand.

**Version**: 1.1.0 | **Ratified**: 2026-10-24 | **Last Amended**: 2026-10-24
