---
title: Command-Line Interface
summary: The actx binary with argument parsing, one-shot questions, the background sync worker and agent assembly.
code: [crates/actx-cli/src/main.rs, crates/actx-cli/src/cli, crates/actx-cli/src/engine.rs, crates/actx-cli/src/prompt.rs, crates/actx-cli/src/commands]
---

# Command-Line Interface

The actx binary with argument parsing, one-shot questions, the background sync worker and agent assembly.

## Responsibilities

- Provide the `actx` executable: decide between the headless mode and the interactive [TUI](tui.md), and parse every flag and subcommand.
- Run headless operations: one-shot questions streamed to stdout, sync, the background sync worker, diagnostics, model downloads, update checks and updates.
- Assemble the agent for a workspace: resolve the language model provider, create the re-ranker, build the system prompt, register the tools and plug in the retrievers (`engine.rs`).
- Bootstrap the system documentation into the `Global` workspace on first run and after upgrades.
- Hold the slash command registry used for TUI autocompletion and route slash commands to the [command engine](commands.md).

## Structure

| File | Role |
|---|---|
| `main.rs` | Rewrites `--update@<v>`, `-u@<v>`, `update@<v>`, `upgrade@<v>`, `--update=<v>` and `-u=<v>` into Clap-compatible arguments, parses `CliArgs`, then runs `oneshot::run_headless` or `tui::run_tui`. |
| `cli/args.rs` | `CliArgs`, `CliCommand`, `is_headless`, `resolved_query`. |
| `cli/oneshot.rs` | Headless dispatcher, streaming of agent events to stdout, `handle_download`, `handle_update`, `handle_check_update`. |
| `engine.rs` | `resolve_lm_provider`, `create_default_reranker`, `build_agent_sync`, `ensure_global_knowledge_bootstrap`, `refresh_global_knowledge_vectors`, `QueryEmbedding`, `is_first_run_or_upgrade`, `run_startup_splash_bootstrap`, `PipelineDeterministicRetriever`, `PipelineBatchRetriever`. |
| `prompt.rs` | System prompt builder and the embedded `AGENT.md`, `README.md` and `SYSTEM_KNOWLEDGE.md` (see [Agent Runtime](agent.md)). |
| `commands/registry.rs` | `SlashCommand` table with descriptions and aliases, `find_command`, `autocomplete_commands`. |
| `commands/dispatcher.rs` | `dispatch_slash_command`: builds the `ExecutionContext` from the TUI state, calls `CommandEngine::execute` and applies the result. |
| `theme.rs` | `UiTheme` adapter (see [Terminal UI](tui.md)). |

## Behavior

### Arguments

| Flag or subcommand | Effect |
|---|---|
| `actx` | Opens the TUI (when stdin is a terminal and no headless option is given). |
| `actx "<question>"`, `-q/--query/--prompt` | One-shot question streamed to stdout. Piped stdin also selects headless mode. |
| `-w/--workspace <name>` | Workspace to use (default `Default`). |
| `-m/--model <id>` | Model override for this run. |
| `-s/--sync`, `actx sync [--force] [--folder <path>]` | Runs a sync in the foreground with verbose output. Errors are printed after the summary (first 10, then a count), with a note that files not embedded are retried on the next sync. Then embeds the system documentation (`Global`) when needed and says so (see [Ingestion and Sync](ingestion.md)). |
| `--sync-worker [--workspace] [--force] [--folder]` | Hidden flag used by background workers spawned by the command engine; prints the same errors to the worker log and embeds `Global` the same way. |
| `-f/--force`, `--folder <path>` | Full re-index; folder-scoped sync. |
| `-d/--diagnostics` (`diag`, `health`) | Prints version, platform, workspaces, the embedding model and its source, hardware, accelerators and database paths. |
| `--check-update`, `-u/--update`, `actx update [--check] [version]` | Checks for or installs a release (see [Installer and Updater](installer.md)). |
| `actx embedding [id] [--yes]` (`embeddings`) | Without an id: the embedding model in use, why it was chosen, its vector space and the catalog. With an id: a preview of the switch (workspaces reindexed on their next sync, a notice when document text goes to a cloud provider) that exits with code 2, or the switch itself with `--yes` (exit 0). An unknown or refused id exits with code 1. The chat model (`-m`, `/model`) never changes it (see [Ingestion and Sync](ingestion.md)). |
| `actx download [tier] [--force] [--check]` | Downloads a model with a progress bar, MB/s, ETA and Ctrl+C cancellation, or lists the model store with hardware verdicts. |
| `--rerank <mode>` (`auto`, `on`, `off`) | Persists the re-rank mode. |
| `--rerank-model <id>` | Uses another re-ranker for this run only (sets `ACTX_RERANKER_MODEL`); unknown ids exit with code 2. |
| `--device <name>` (`auto`, `dml`, `metal`, `cuda`, `vulkan`, `cpu`) | Passed to the TUI state; inference runs on the CPU in this build. |
| `-v/-V/--version` | Prints the version. |
| `actx serve`, `actx mcp`, `actx rpc` | Reserved: they print a start message and exit. There is no REST, MCP or JSON-RPC server in this build. |

### One-shot questions

The headless path resolves grounding mode, search mode and web search from the workspace settings, resolves the provider (`resolve_lm_provider`, described in [Language Model Providers](lm-providers.md)), builds the agent and calls `Agent::stream` with session `ws_<workspace>`. Events are printed as they arrive: routing badge with the detected domain and confidence, Deep Search decomposition and iterations, tool calls, reasoning in a dimmed `<think>` block, and answer tokens.

### Agent assembly

`build_agent_sync(provider, model, workspace, grounding_mode, search_mode, web_search_enabled)`:

1. Opens `SqliteSessionStore` on `settings.db` with a history of 50 messages.
2. Builds the retrieved-context system prompt (`prompt::build_system_prompt` with `PromptPath::RetrievedContext`) for one-hop RAG and the Deep Search synthesis.
3. Creates the re-ranker for the active re-ranker model (`create_default_reranker`); its ONNX session loads lazily on first use.
4. Builds `search_db` and `system_status` (its report comes from `workspace_status_report`), and `web_search` when web search is enabled, derives the ReAct system prompt from their names (`PromptPath::Tools`) and registers them (see [Agent Runtime](agent.md)).
5. Sets the execution mode (`DeepSearch` for `/search deep`, otherwise `ReAct`), the search policy and `max_turns(10)`.
6. Resolves the query embedding once (`resolve_query_embedding`, which calls `ingestion::resolve_query_embedding` and `ingestion::global_dense_enabled` and returns a `QueryEmbedding`: provider, target, query prefix, and whether `Global` takes part in dense search) and plugs in `PipelineBatchRetriever` (Deep Search), `PipelineDeterministicRetriever` (one-hop RAG) and `PipelineStatusRetriever` (status questions: the same search plus the live status report, ADR-0145) when LanceDB opens; they and `search_db` use the [retrieval pipeline](retrieval.md) with that embedding (`QueryEmbedding::apply`) and the `rerank_mode` setting.
7. Attaches the `ChatModelRouter` with the query classifier adapter.

### Startup and self-knowledge

`is_first_run_or_upgrade` compares the `global_knowledge_version` setting with the binary version through SQLite only (no LanceDB or BM25 access). When it differs, `run_startup_splash_bootstrap` shows a three-step splash (verify stores, index system knowledge, warm up), runs `ensure_global_knowledge_bootstrap` and always writes the current version. The TUI also runs the bootstrap in a background task on every launch; it returns immediately when the `Global` chunks exist and the version matches. The bootstrap writes the `Global` rows with placeholder vectors and clears `global_knowledge_embedding_target`; the next sync embeds them ([ADR-0147](../adr/0147-constant-vectors-never-reach-dense-search-the-mock-target-is.md)).

## Interfaces

- **Calls**: [Agent Runtime](agent.md), [Command Engine](commands.md), [Ingestion and Sync](ingestion.md), [Retrieval Pipeline](retrieval.md), [Language Model Providers](lm-providers.md), [ONNX Inference](onnx.md), [Model Catalog and Hardware Profiling](model-catalog.md), [Installer and Updater](installer.md) (update, paths).
- **Used by**: users and scripts; the command engine spawns the same binary as the sync worker.
- **Environment**: `ACTX_MODEL`, `ACTX_AGENT_MD`, `ACTX_RERANKER_MODEL`, plus the storage and provider variables listed on their pages.

## Decisions

- [ADR-0077](../adr/0077-native-ratatui-cli-tui.md): a single native `actx` executable with Ratatui and Crossterm.
- [ADR-0091](../adr/0091-hermetic-workspace-isolation-atomic-exe-swap.md): `/sync` runs in a dedicated `--sync-worker` process.
- [ADR-0093](../adr/0093-sync-worker-stdio-isolation-chain-of-responsibility.md): worker output isolated in log files.
- [ADR-0103](../adr/0103-windows-multi-binary-self-update.md): `@` version delimiters preprocessed before Clap.
- [ADR-0105](../adr/0105-native-rust-ingestion-sync-pipeline.md): `--sync-worker` for headless background synchronization.
- [ADR-0114](../adr/0114-aurora-design-tokens-startup-fast-path.md): SQLite startup fast-path and splash on upgrade.
- [ADR-0119](../adr/0119-english-interface-spinner-release-notes.md): splash bootstrap always persists the knowledge version.
- [ADR-0122](../adr/0122-ci-fast-path-timing-resilience.md): warmup and 1500 ms ceiling in the startup fast-path test.
- [ADR-0125](../adr/0125-local-gemma-activation-hardware-guardrails.md): Gemma model routing in `resolve_lm_provider`.
- [ADR-0126](../adr/0126-assisted-gemma-weights-downloader.md): `actx download`.
- [ADR-0127](../adr/0127-cross-platform-hardware-acceleration-matrix.md): `--device` and accelerator diagnostics.
- [ADR-0129](../adr/0129-two-stage-retrieval-cross-encoder-reranking.md) and [ADR-0130](../adr/0130-real-onnx-runtime-multilingual-cross-encoder.md): `--rerank`, `--rerank-model` and lazy re-ranker loading.
- [ADR-0144](../adr/0144-system-prompt-per-answer-path-with-the-tool-section-generate.md): one system prompt per answer path, the ReAct prompt derived from the registered tools.
- [ADR-0145](../adr/0145-live-status-reaches-the-routed-paths-as-retrieved-context.md): `PipelineStatusRetriever` and the shared `workspace_status_report`.
- [ADR-0146](../adr/0146-a-failed-embedding-fails-the-file-and-embedding-outcomes-are.md): sync errors printed after `--sync` and `--sync-worker`; `workspace_status_report` shows the last sync failures and the dense search outcome.
- [ADR-0147](../adr/0147-constant-vectors-never-reach-dense-search-the-mock-target-is.md): `QueryEmbedding` (no dense search on the mock target, `Global` gate) and the `Global` embedding after `--sync` and `--sync-worker`.
