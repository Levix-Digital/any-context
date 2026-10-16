---
title: Terminal UI
summary: Full-screen Ratatui interface with chat, header and footer telemetry, menus, onboarding and the Aurora theme.
code: [crates/actx-cli/src/tui, crates/actx-cli/src/theme.rs, crates/any-context-core-rs/src/theme]
---

# Terminal UI

Full-screen Ratatui interface with chat, header and footer telemetry, menus, onboarding and the Aurora theme.

## Responsibilities

- Run the interactive session opened by `actx`: terminal setup, the event loop, keyboard and mouse input.
- Render the conversation, the streaming answer, the reasoning and tool log, the multi-line input, the slash command palette, menus and the onboarding wizard.
- Show live state: workspace, sync progress, model, execution device, grounding, search depth, re-ranker and web search.
- Act as a thin adapter: slash commands go to the [command engine](commands.md) and questions go to the [agent](agent.md); the TUI only applies their results.
- Run model downloads requested by the command engine, with progress and cancellation.

## Structure

| File | Role |
|---|---|
| `tui/mod.rs` | `run_tui`: splash on upgrade, agent creation, background self-knowledge bootstrap, raw mode and alternate screen, the event loop. |
| `tui/app.rs` | `App` state and behavior: chat buffers per workspace, input and history, scrolling, menus, downloads, onboarding, agent events, command results. |
| `tui/ui.rs` | Rendering: header, chat, reasoning accordion, input, slash palette, footer, menus, onboarding. |
| `tui/events.rs` | Keyboard and mouse handling. |
| `tui/menu.rs` | `MenuState` and the menu builders (main, workspaces, models, sync, grounding, search, sources, keys, Document AI and its submenus, profiles, devices). |
| `theme.rs` | `UiTheme`: converts core tokens into Ratatui colors and 24-bit ANSI sequences. |
| `any-context-core-rs/src/theme` | `AuroraTheme` and `RgbColor`: UI-agnostic design tokens, serializable with `serde`. |

## Behavior

### Event loop

`run_tui` shows the startup splash when `is_first_run_or_upgrade()` is true, resolves the provider and builds the agent for the workspace, starts `ensure_global_knowledge_bootstrap` in a blocking background task, enables raw mode, the alternate screen and mouse capture, then loops:

1. `poll_sync_status` (at most every 400 ms) reads `workspace_sync_status` from SQLite; every 4 s `check_source_health` counts unreachable folders (after path healing) and runs a scan diff to detect changes.
2. Draw the frame.
3. Wait for a 125 ms tick (animations and polling), a terminal event or an agent event.

### Layout

From top to bottom: a 3-line header, the borderless chat viewport, the reasoning accordion (7 lines, only when open), the input area (3 to 8 lines, growing with the text) and a 1-line footer.

- **Header**: `AnyContext v<version> ─ [WS: …] ─ [Sync: …] ─ [Model: …] ─ [Acc: …] ─ [Grounding: …] ─ [Search: …] ─ [Rerank: …] ─ [Web: …]` plus the agent status (`IDLE`, `THINKING`, `STREAMING`, `ERROR`). Sync shows a spinner and the progress bar while syncing, `Sync Cancelled`, `Changes detected (/sync)`, unreachable folders, or `Up to date`. The re-rank badge shows the neural model, RRF fallback (also when the model failed to load in this process) or off; it caches the re-ranker state for 3 s.
- **Chat**: user lines prefixed with `[YOU]`, answers with `[AI - <workspace>]`, and system messages for errors and command output.
- **Accordion**: routing decisions, Deep Search decomposition and iterations, tool calls and results, reasoning tokens; collapsed by default and toggled with `Ctrl+T`; auto-scrolls while reasoning is generated.
- **Footer**: sync status with the 10-point Braille spinner (`⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`, one step per tick), download progress (model, MB, percentage, speed, ETA, "Esc to cancel"), a "scroll paused, End for real time" hint when auto-scroll is detached, and key hints.

### Keys

| Keys | Action |
|---|---|
| `Enter` | Submit the input (a slash command or a question). |
| `Shift+Enter`, `Alt+Enter`, `Ctrl+J` | New line in the input. |
| `Up` / `Down` | Prompt history for the active workspace, or navigation in the slash palette. |
| `Tab` | Complete the selected slash command. |
| `PageUp`/`PageDown`, `Home`/`End`, mouse wheel, `Shift+Up`/`Shift+Down` | Scroll the chat; `End` resumes auto-scroll. |
| `Ctrl+PageUp`/`Ctrl+PageDown` | Scroll the reasoning accordion when open. |
| `Ctrl+T` | Toggle the reasoning accordion. |
| `F1`, `Ctrl+M`, `/menu` | Open or close the menu. Menus use `Up`/`Down` (or `k`/`j`), `Enter`/`Tab`/`Right`/`l` to select and `Esc`/`Left`/`h` to go back. |
| `Esc` | Cancel a running model download, close the palette or menu. |
| `Ctrl+C` | Exit. |

### Workspaces and chat buffers

Each workspace keeps its own visible chat buffer (`workspace_chat_buffers`) and prompt history while the application runs. `switch_to_workspace` saves the current buffer, reloads the target's model, grounding mode, search mode and web search from SQLite, resets streaming and scroll state, and restores the target's buffer or starts it with the welcome message. Past conversations are not printed on start; they stay in the session store for the agent and can be shown with `/history`.

### Commands and agent turns

Input starting with `/` goes to `dispatch_slash_command`, which builds an `ExecutionContext` from `App` (`to_execution_context`) and applies the `CommandResult` (`apply_command_result`): it prints the message, applies the state updates and performs the action (`Exit`, `ClearChat`, `OpenMenu`, `SwitchWorkspace`, `RebuildAgent`, `StartDownload`, `SetDevice`). Any other input starts `Agent::stream` for the session `ws_<workspace>`; `handle_agent_event` appends deltas to the stream buffer, routes reasoning, tool and Deep Search events to the accordion, and pushes errors to the chat as system messages so failures are always visible. `finalize_assistant_turn` moves the streamed answer into the chat history.

### Downloads

`StartDownload(id)` makes `start_model_download` spawn a Tokio task that calls `OnnxModelManager::download_model_with_progress` with progress and cancellation channels; `poll_model_download` updates the footer, and `cancel_model_download` (`Esc` or the menu) stops it and removes the partial file. In the model and Document AI menus, pressing `Enter` on a model that is not installed starts the same flow after the pre-flight checks.

### Menus and onboarding

The main menu (`F1`) leads to workspaces, models (with hardware compatibility badges), sync, grounding, search depth, sources, API keys, Document AI (ingestion, scans, query routing, vision, storage summary), hardware profiles and devices. `/onboarding` (or `/setup`) opens a three-step wizard: language model provider and API key, Document AI profile, and an optional first documents folder; it records `onboarding_completed`. The wizard opens automatically at startup when onboarding was never completed and the resolved model is the mock provider (no credentials yet).

### Theme

`AuroraTheme` defines the palette: primary Emerald `#00E5A3`, accent Glacial Cyan `#00F0FF`, reasoning Cosmic Violet `#A855F7`, magenta `#EC4899`, warning Amber `#F59E0B`, error Crimson `#EF4444`, info Sky Blue `#38BDF8`, backgrounds `#0B0F19` and `#161F30`, borders `#00F0FF` and `#334155`, text `#F1F5F9`, `#CBD5E1` and `#64748B`. Warning and error colors are reserved for real problems; routine elements use primary, accent and reasoning. `UiTheme` exposes the same tokens to Ratatui and to the CLI's ANSI output.

## Interfaces

- **Calls**: [Command Engine](commands.md), [Agent Runtime](agent.md), [storage](storage.md) (settings and telemetry), [ingestion](ingestion.md) (scanner for change detection), [Model Catalog and Hardware Profiling](model-catalog.md) (downloads, menus), [ONNX Inference](onnx.md) (device and re-ranker state).
- **Started by**: `actx` without headless options (see [Command-Line Interface](cli.md)).

## Decisions

- [ADR-0077](../adr/0077-native-ratatui-cli-tui.md): Ratatui and Crossterm as the canonical interface.
- [ADR-0079](../adr/0079-utf8-safe-input-auto-scroll.md): UTF-8 safe input editing and the auto-scrolling chat viewport.
- [ADR-0080](../adr/0080-workspace-isolated-prompt-history.md): prompt history on `Up`/`Down`, per workspace.
- [ADR-0081](../adr/0081-tui-header-status-telemetry.md) and [ADR-0085](../adr/0085-grounding-vs-search-depth-decoupling.md): status indicators in the header.
- [ADR-0089](../adr/0089-tui-error-surfacing-search-depth-concealment.md): agent errors are always rendered.
- [ADR-0092](../adr/0092-virtual-tab-chat-buffer-isolation.md): chat buffer per workspace.
- [ADR-0095](../adr/0095-clean-screen-buffer-long-term-memory-decoupling.md): clean screen on start, memory kept in SQLite.
- [ADR-0097](../adr/0097-sqlite-sync-telemetry-agent-background-awareness.md) and [ADR-0098](../adr/0098-lancestore-count-records-parameter-fix.md): live sync progress in the footer.
- [ADR-0100](../adr/0100-native-encryption-sync-cancel-tui-isolation.md): no stderr output during interactive sessions.
- [ADR-0103](../adr/0103-windows-multi-binary-self-update.md): sync status always visible.
- [ADR-0112](../adr/0112-ux-stabilization-deep-search-multiline-prompt.md): 125 ms loop, multi-line input, per-workspace answer prefix.
- [ADR-0114](../adr/0114-aurora-design-tokens-startup-fast-path.md): Aurora design tokens in the core and the UI adapter.
- [ADR-0115](../adr/0115-onnx-weights-repository-fallbacks.md): search mode indicator and reload on workspace switch.
- [ADR-0116](../adr/0116-path-healing-temporal-metadata-change-detection.md): source health and change detection.
- [ADR-0117](../adr/0117-aurora-palette-borderless-canvas-onboarding.md): palette harmonization, borderless chat, reasoning auto-scroll, download cancellation, onboarding.
- [ADR-0119](../adr/0119-english-interface-spinner-release-notes.md): English interface, point-by-point spinner, collapsed accordion.
- [ADR-0126](../adr/0126-assisted-gemma-weights-downloader.md): download telemetry in the footer.
- [ADR-0127](../adr/0127-cross-platform-hardware-acceleration-matrix.md) and [ADR-0129](../adr/0129-two-stage-retrieval-cross-encoder-reranking.md): device and re-rank badges.
