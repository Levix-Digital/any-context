---
plan_id: anycontext_blueprint_v0390_conector_web_jina_reader_rjinaai_roteamento_source_aware_atalhos_de_chat_fetch_fast_deep
project: any-context
title: 'Blueprint v0.39.0: Jina Reader Web Connector (r.jina.ai), Source-Aware Routing & Chat Shortcuts (/fetch, /fast, /deep)'
version: v0.39.0
status: superseded
author: Levi Silveira
created_at: '2026-10-14T12:57:11'
updated_at: '2026-10-24T14:16:59'
session_id: 01fd77a2-4853-4967-b572-f0ce69495ea0
tags:
- jina_reader
- web_fetch
- source_aware_routing
- slash_commands
- deep_search
- v0390
supersedes: null
superseded_by: null
parent_plan_id: null
milestones:
- name: 'Milestone 1: NativeJinaReader'
  done: false
- name: 'Milestone 2: web_fetch Tool'
  done: false
- name: 'Milestone 3: /fetch Command'
  done: false
- name: 'Milestone 4: Source-Aware Routing'
  done: false
- name: 'Milestone 5: Dual-Doc and Release v0.39.0'
  done: false
status_log:
- at: '2026-10-16T10:47:35'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
- at: '2026-10-24T14:16:59'
  by: Levi Silveira
  status: superseded
  note: Superseded by the architecture v2 redesign and roadmap v2 (docs/ROADMAP.md, docs/plans/anycontext_roadmap_v2.md, decided 2026-10-23); the remaining work was rebuilt as backlog items.
---

# 📐 Engineering Blueprint: Jina Reader Web Connector (r.jina.ai), Source-Aware Routing & Chat Shortcuts (/fetch, /fast, /deep) (v0.39.0)

## 📌 Plan Metadata
- **Project**: AnyContext (`actx`)
- **Target Version**: `v0.39.0` (successor of `v0.38.0`)
- **Components**: `crates/any-context-core-rs`, `crates/actx-agent`, `crates/actx-cli`
- **Creation Date**: 2026-10-14
- **Status**: Proposed (`proposed` / awaiting Gate 3)
- **Conversation ID**: `01fd77a2-4853-4967-b572-f0ce69495ea0`
- **Reference Specifications**: Master Roadmap (`anycontext_master_roadmap_v0_30_38_rfc042.md`), RFC-042 (§2.3 & §4.1).

---

## 🏛️ 1. Overview and Architectural Motivation

With the local intelligence infrastructure consolidated in version `v0.38.0` (a 5-tier hardware matrix, ONNX visual/document perception models and a local generative brain with the Gemma 4 family), AnyContext reaches full inference autonomy.

However, two essential capabilities of **Milestone 3 & RFC-042** remain pending to close the agentic research cycle:
1. **Direct Consumption of Web Content as Pure Markdown (Jina Reader - `r.jina.ai`)**:
   - Today web search (`NativeWebSearchEngine`) returns titles and short page snippets. When the ReAct agent or the Reflective Deep Search needs to consult full technical documentation, online RFCs, framework manuals or long articles, 200-character snippets are not enough.
   - Local headless browsers (e.g. Chromium / Puppeteer) need between 300MB and 1GB of RAM, violating AnyContext's principle of lightness and instant startup.
   - The **Jina Reader connector (`https://r.jina.ai/{url}`)** solves this bottleneck by acting as a proxy that converts dynamic web pages and SPAs into clean Markdown, without ads, menus or scripts, with zero local RAM consumption.
   - For restricted, disconnected or offline environments (ADR-110), there must be a native Rust fallback that parses directly without panics.

2. **Source-Aware Routing & Chat Shortcut Activation**:
   - The `ChatModelRouter` in `actx-agent` classifies intent into 4 domains (`Code`, `Architecture`, `Document`, `General`), but the answer flow does not yet recommend or select the optimal context profile based on this typology.
   - The `/fast`, `/deep`, `/search` commands and the new `/fetch <url>` must give the user immediate manual control in the terminal and the TUI.

```mermaid
graph TD
    subgraph "Presentation Layer (Multi-Surface Presentation)"
        TUI["Ratatui TUI (/fetch, /fast, /deep, /search)"]
        CLI["actx Headless CLI (--fast, --deep, fetch <url>)"]
    end

    subgraph "Core Domain & Commands (any-context-core-rs)"
        CE["CommandEngine (/fetch & /search & /router)"]
        NJR["NativeJinaReader (HTTP Client with Fallback)"]
    end

    subgraph "Agentic Orchestration (actx-agent)"
        CMR["ChatModelRouter (Source-Aware Routing)"]
        TF["Tool: web_fetch (r.jina.ai / native)"]
        TS["Tool: web_search (Tavily / Serper / DDG)"]
        DSO["Reflective Deep Search (RFC-042)"]
    end

    subgraph "Web Providers & External Networks"
        JINA["r.jina.ai (Markdown API)"]
        DIRECT["Direct HTTP Fallback (Sanitized HTML -> MD)"]
    end

    TUI --> CE
    CLI --> CE
    CE --> NJR
    DSO --> TF
    DSO --> TS
    TF --> NJR
    NJR -->|Online| JINA
    NJR -->|Failure / Offline| DIRECT
    CMR -->|Telemetry & Routing| TUI
```

---

## 🔒 2. Inviolable Engineering Principles (Dev-Cycle Compliance)

1. **Inviolability of the ADR-110 Guarantee**:
   - Network failures, `r.jina.ai` unavailability, timeouts or CORS/WAF blocks never cause panics or break the agent loop.
   - On an HTTP error (e.g. 403, 429, 502) or no connection, the reader immediately triggers the native fallback with a clear message on the event channel.
2. **Absolute Multi-Surface Parity**:
   - Fetching pages through Jina Reader and configuring search depth must be available in the headless CLI (`actx fetch <url>`, `actx --fast "..."`, `actx --deep "..."`), in the chat slash commands (`/fetch`, `/fast`, `/deep`, `/search`) and in the TUI menu.
3. **Strict Token & Character Budget Control**:
   - Content returned by Jina Reader is truncated deterministically at the configured ceiling (`max_chars`, default: 30,000 characters, ~7,500 tokens), avoiding context window overflow of compact local SLMs (e.g. Gemma 4 Compact).
4. **Dual-Doc Pattern & Non-Truncated Test Script**:
   - Synchronized documentation in `README.md` (UserDoc) and `TECDOC.md` (TecDoc).
   - Scenario 39 added to `tests/MANUAL_TESTS.md` with step-by-step tests.

---

## 🛠️ 3. Detailed Technical Specification of the Components

### 3.1 `NativeJinaReader` Module (`crates/any-context-core-rs/src/retrieval/jina_reader.rs`)
- **Data Structures**:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct JinaFetchResult {
      pub url: String,
      pub title: String,
      pub content: String,
      pub provider: String, // "jina" or "native_fallback"
      pub char_count: usize,
      pub is_truncated: bool,
  }

  #[derive(Debug, Clone)]
  pub struct JinaReaderConfig {
      pub timeout_secs: u64,
      pub max_chars: usize,
      pub api_key: Option<String>,
      pub with_generated_alt: bool,
  }
  ```
- **Execution Logic**:
  1. Sanitizes the target URL (ensures an `http://` or `https://` scheme).
  2. Builds the destination URL: `https://r.jina.ai/{sanitized_url}`.
  3. Injects headers:
     - `Accept: text/markdown`
     - `X-With-Generated-Alt: true` (if configured)
     - `Authorization: Bearer <token>` (if `JINA_API_KEY` is present in the env or in the SQLite `NativeConfigDb` database).
  4. On an HTTP 200 OK response, sanitizes the returned Markdown, extracts the title and applies safe truncation by the character limit (`max_chars`).
  5. On a network failure or an error status, it calls the `fetch_direct_fallback` method:
     - Makes a direct request through `reqwest::Client`.
     - Runs a quick HTML sanitization (removing `<script>`, `<style>`, navigation tags and extracting the readable text with basic formatting).

### 3.2 Native `web_fetch` Tool in the ReAct Agent & Deep Search (`crates/actx-cli/src/engine.rs`)
- Injected into `Agent::builder()`:
  - **Tool Name**: `web_fetch`
  - **Description**: `"Fetches the full text of any public web page, technical documentation, or online article as clean structured Markdown using Jina Reader (r.jina.ai) with native fallback."`
  - **Schema**:
    ```json
    {
      "type": "object",
      "properties": {
        "url": {
          "type": "string",
          "description": "The target public web URL to fetch and read"
        },
        "max_chars": {
          "type": "integer",
          "description": "Optional maximum character limit to retrieve (default: 30000)"
        }
      },
      "required": ["url"]
    }
    ```
- The ReAct agent and the Deep Search engine can now automatically follow relevant links found by `web_search` to extract in-depth details.

### 3.3 `CommandEngine` Extension with `/fetch` (`crates/any-context-core-rs/src/commands/engine.rs`)
- New `/fetch <url>` command:
  - Validates the arguments.
  - Instantiates `NativeJinaReader`.
  - Fetches the page asynchronously.
  - Outputs the result with metadata: title, character count, provider status and a formatted Markdown preview of the content.

### 3.4 Source-Aware Routing Enrichment (`crates/actx-agent/src/router.rs`)
- `RoutingDecision` now carries contextual metadata:
  - `suggested_profile`: indicates whether the typology (`Code` vs `Architecture` vs `Document`) benefits from models with high mathematical/syntactic specialization or a wide reasoning window.
  - Easy-to-read telemetry shown in the TUI reasoning accordion.

---

## 📅 4. Implementation Milestone Schedule

| Milestone | Component | Responsibility |
|---|---|---|
| **Milestone 1** | `crates/any-context-core-rs/src/retrieval/jina_reader.rs` | `NativeJinaReader` with support for `r.jina.ai`, optional authentication, headers and a native HTML->Markdown fallback |
| **Milestone 2** | `crates/actx-cli/src/engine.rs` | Registration of the native `web_fetch` tool in the `Agent::builder()` of the ReAct orchestrator and Deep Search |
| **Milestone 3** | `crates/any-context-core-rs/src/commands/engine.rs` | `/fetch <url>` slash command and integration with `/web` and `/search` |
| **Milestone 4** | `crates/actx-agent/src/router.rs` & `tui/app.rs` | Source-Aware Routing with enriched telemetry in the accordion |
| **Milestone 5** | Tests, Dual-Doc & Release v0.39.0 | ADR-124 in `TECDOC.md`, `README.md`, Scenario 39 in `MANUAL_TESTS.md`, 100% green tests and bump to `v0.39.0` |

---

## 🧪 5. Manual Test Script (Scenario 39)

- **Scenario 39.1 (Web Fetch with Jina Reader)**:
  - Run `/fetch https://docs.rs/tokio/latest/tokio/` and validate that structured Markdown comes back with a title and character count.
- **Scenario 39.2 (Fetch Tool in the ReAct Loop)**:
  - Ask the agent: `"Read the page https://example.com and tell me the main heading"` with web search on. Confirm in the logs that the `web_fetch` tool fired.
- **Scenario 39.3 (Resilience and ADR-110 Fallback)**:
  - Run `/fetch http://127.0.0.1:9999/invalid` or disconnect the internet. Validate that the error is handled smoothly, without panics.
- **Scenario 39.4 (/fast and /deep Shortcuts)**:
  - Test `/fast What is the initialization function?` and confirm the forced single-turn routing.
  - Test `/deep Do a complete architectural audit` and confirm that Deep Search with reflection fires.
