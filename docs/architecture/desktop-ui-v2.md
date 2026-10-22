# Desktop UI (Tauri 2) — layout reference

Guide to the desktop UI layout, from the mockup and the prototype and visual token
blueprints (2026-10). **What counts is the layout.** Values, labels and options in the
mockup (e.g. Chunk Size 512, Score Threshold 0.75, Qdrant, top-K) are illustrative and will
be replaced by the real ones, following the ingestion, retrieval and deployment designs.

## Three-column structure

| column | content | width (min · default · max) | collapse |
|---|---|---|---|
| **Context Explorer** (left) | tree of workspaces and, inside each, its sources (expandable folders, sites expandable into pages, connectors), with the status of each source; "+ New" button; footer with the user profile and shortcuts | 200 · 260 · 420 px | `Cmd/Ctrl + B` or drag |
| **Canvas Hub** (center) | tab bar: one **chat tab per workspace** and one tab per **open source** (reader); content of the active tab | 480 px · flexible | — |
| **Workspace Inspector** (right) | settings for the context of the active tab (workspace or document): basic section exposed, advanced section in an accordion | 240 · 280 · 480 px | `Cmd/Ctrl + I` or drag |

Fixed top bar (38 px) with the app name, quick search (`Cmd/Ctrl + K`) and the global menu.

## Behavior

- **Clicking a workspace** opens or focuses its chat tab; **clicking a source** opens or
  focuses a reader tab. Tabs never repeat: opening something already open brings its tab
  into focus.
- **Chat tab:** streaming history with clickable citations (they open the source at the
  cited point); multiline composer with a fixed bar of quick options (model, retrieval
  mode, web search, sync, send), saved per workspace.
- **Reader tab:** read-only document (syntax highlighting, line numbers, indexing metadata,
  reprocess the document). Opening files, browsing folders and viewing sites without
  leaving AnyContext is in scope for this UI.
- **Inspector:** follows the active tab and shows only what belongs to the user (minimal
  configuration principle). The mockup's "Advanced Settings" — Chunk Size, Overlap, Top-K,
  Score Threshold, Neural Reranker, Max Depth — **are removed**: they are architecture
  decisions, tuned automatically. What stays, for example: name and description, sources
  and their filters, chat model, domain pack, privacy policies, crawl ceiling of a site.

## Design system

Visual tokens (semantic colors, typography, spacing, radii, elevations) defined in the
token specification: dark theme (surfaces from `#0B0C0E` to `#1E2330`), primary accent
`#2563EB`, source status (synced, syncing, error), native sans font and mono for code,
4 px base spacing.

## Rules inherited from the architecture decisions

- One chat per workspace, individual per user; no query crosses workspaces.
- AnyContext's documentation is the system help, not a workspace in the tree.
- Shared and organization workspaces appear in the tree with the user's role (owner,
  editor, reader).
- Connectors are plugged explicitly per source (`connectors-v2.md`); a source the
  plan no longer covers shows as deactivated, never disappears (`connectors-v2.md`,
  decisions 26 and 29).
