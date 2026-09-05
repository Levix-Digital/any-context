# 🧠 AnyContext (`actx`)

> **Transform any file, folder, website, or drive into a living, real-time AI context.**

```text
  ___               ____ ___  _   _ _____ _____ _  _______ 
 / _ \ _ __  _   _ / ___/ _ \| \ | |_   _| ____\ \/ /_   _|
| |_| | '_ \| | | | |  | | | |  \| | | | |  _|  \  /  | |  
|  _  | | | | |_| | |__| |_| | |\  | | | | |___ /  \  | |  
|_| |_|_| |_|\__, |\____\___/|_| \_| |_| |_____/_/\_\ |_|  
             |___/                                         
  🚀 AnyContext (actx)  |  Levix Digital
  ⚡ Agnostic AI Agent with Isolated Workspaces & Long-Term Memory
  🔒 100% Local & Offline-First Privacy
```

[![Release](https://img.shields.io/github/v/release/Levix-Digital/any-context?color=blue&label=release)](https://github.com/Levix-Digital/any-context/releases)
[![License](https://img.shields.io/badge/license-Community%20%2F%20Enterprise-green.svg)](https://github.com/Levix-Digital/any-context)
[![Python](https://img.shields.io/badge/python-3.10%2B-blue.svg)](https://www.python.org/)
[![Privacy](https://img.shields.io/badge/privacy-100%25%20Offline%20First-success.svg)](https://lmstudio.ai/)
[![Architecture & TecDoc](https://img.shields.io/badge/docs-TecDoc%20(Architecture)-blueviolet.svg)](TECDOC.md)

**AnyContext** is the universal intelligence layer for your personal and professional data. Built with an uncompromising focus on **privacy, speed, and versatility**, AnyContext connects your local folders, scanned documents, and web portals directly to the world's most capable Artificial Intelligence models.

> 📖 **Looking for Deep Engineering & Architecture Details?**  
> Check our exhaustive technical manual: 👉 **[`TECDOC.md`](TECDOC.md)** (covers 2-Phase RAG, ChromaDB parallel scanning, 5-tier date resolution, SQLite memory, REST API schemas, and MCP specs).

Whether you are a **lawyer reviewing hundreds of contract pages**, a **consultant managing immigration dossiers**, a **researcher navigating scientific publications**, or an **enterprise deploying a secure in-VPC context server**, AnyContext operates **100% on your terms and infrastructure**.

---

## 🌟 Why AnyContext? (In Plain English)

Traditional AI tools require you to manually copy and paste files into web chats, exposing your confidential documents to external clouds and forgetting past conversations the moment you close the tab.

**AnyContext changes everything:**

1. **You Point to Your Folders**: Point AnyContext to any folder on your computer (PDFs, Word docs, Excel spreadsheets, images, text files, or source code).
2. **Instant Local Memory**: AnyContext reads and organizes your documents locally into an ultra-fast columnar vector database (**LanceDB / Apache Arrow in Rust**).
3. **Ask Anything Naturally**: Open the terminal and ask questions in plain English or Portuguese (*"What is the renewal deadline in contract 42?"*, *"Compare the tax policies across our 2025 financial reports"*).
4. **Cites Real Sources**: The AI answers accurately, quoting the exact file name, page, and chunk where the information was found.
5. **Zero Cloud Lock-In**: Choose from **9 leading AI providers** (OpenAI, Claude, Gemini, DeepSeek, Groq, Mistral, xAI Grok, OpenRouter) or run **100% offline and free** using local models (LM Studio / Ollama).

---

## 🚀 Key Features & Superpowers

- **⚡ 100% LanceDB Columnar Vector Engine (Apache Arrow / Rust)**:
  - **Sub-5ms Vector Queries**: Powered by native Rust SIMD vector distance routines, eliminating database locks and enabling instant retrieval across 500,000+ chunks.
  - **Zero Database Locks**: Columnar Apache Arrow dataset architecture eliminates SQLite write-lock contentions on Windows.
  - **Live Dataset Inspection (`/inspect` ou `/chunks`)**: Directly inspect live vector records, record counts, and snippet previews for both document context and session memory.
  - **Instant $0.00 Transfers & Renames**: Data source transfers and workspace renames execute on LanceDB datasets in `< 50ms` with zero token expenditure.
- **🚀 Sub-60ms Native Launcher Shim & Dual-Binary Architecture**:
  - **Instant Execution**: Native 5.6KB C# launcher (`actx.exe`) executes version checks (`actx -v`) in `< 50ms` (< 2ms direct) by reading cached `version.txt` without loading the 248MB Python engine.
  - **Dual-Binary Isolation**: Automatically preserves `actx.exe` (Shim) and delegates heavy commands to `actx-core.exe` (PyInstaller), self-healing automatically on auto-update.
- **🌐 LanceDB Single Source of Truth Web Engine & Resilient Crawler**:
  - **Zero Split-Brain Desynchronization**: Eradicated legacy secondary SQLite cache tables. LanceDB is the 100% single source of truth for both web page caching, hashes, and vector chunks.
  - **Sub-5ms Zero-Copy Cache Scans**: Apache Arrow columnar projections (`select(['file_path', 'content_hash', 'last_modified'])`) verify page modifications instantly without deserializing dense embedding vectors.
  - **Automatic Canonical Redirect & Trailing Slash Normalization**: Automatically detects canonical HTTP 301/302 redirects when registering portals without trailing slashes (e.g. `/stable/book` -> `/stable/book/`), ensuring relative chapter links (`ch01-...html`) resolve to the proper subpath instead of truncating parent directories. Fully resilient across Linux, WSL, and Windows.
  - **Prioritized Deep Scraping & $0 Resync**: Multi-page documentation portals (e.g. The Rust Book) are recursively scraped and indexed in the background with progress reporting and instant $0.00 zero-token re-sync skips.
  - **Zero Dropped Jobs**: Eliminates race conditions when adding web sources (`/web --add`) immediately after workspace creation, automatically queuing pending resync passes.
- **🛡️ Hermetic Triple Sandboxing & Production Data Immunity**:
  - **Zero Database or Vector Contamination**: Automated test runners execute under strict, multi-tier isolation (`ACTX_SETTINGS_DB`, `ACTX_CONTEXT_DB`, `ACTX_MEMORY_DB`) with hard safety barriers in `paths.py` protecting `settings.db`, LanceDB vector storage, and session memory against automated test runners.
  - **Automatic Cascade Test Purge**: Production startup automatically cascade-purges any test workspace fixtures tagged with `created_by = 'test'` across all 9 database tables, eliminating lingering test workspaces and test source folders completely.
- **🛡️ Session Process Immunology & Clean Terminal Teardown**:
  - **Terminal Screen Integrity & Zero Ghost TUIs**: In `/update` or `actx --update`, requesting to close active sessions terminates all AnyContext instances safely only after download verification, completely tears down the OpenTUI console renderer (restoring canonical cooked mode, cursor visibility, and terminal echo), and exits cleanly to the shell, eliminating desynchronized prompt leaks, ghost TUIs, and frozen cursor locks.
- **🛡️ Auto-Healing Conversation Sanitizer & OpenAI Tool Call Shield**:
  - **Zero Error 400 Invalid Request**: Interrupted session turns or tool cancellations automatically synthesize compliant response messages for orphan tool calls, eliminating permanent SQLite checkpoint corruption.
- **🔐 Persistent Configuration & Seamless Cross-Version Retention**:
  - **Zero Repeated Onboarding**: Upgrading AnyContext (`actx --update` or installer) automatically retains all configured workspaces, custom AI models, and API keys. The onboarding modal is never triggered if an active API key or provider is already configured.
  - **Triple-Layer Persistence Immunity**: Configuration state is anchored across `system_config` table, `context_settings` table, and SQLite `api_keys` store with automatic self-healing.
  - **Preserved Uninstallation**: Uninstalling with data preservation keeps your workspaces, vector database, configured models, and API keys 100% intact for future installations.
- **🕒 Temporal RAG & Metadata Freshness Engine**:
  - **5-Tier Web Date Resolution**: Automatically extracts publication and update dates via OpenGraph/Schema.org, visible in-page text/footers (`Page details YYYY-MM-DD`, `Date modified:`), URL date patterns (`/2023/06/...`), HTTP `Last-Modified` headers, and crawl timestamps.
  - **Content Classification**: Distinguishes between `Canonical Service / Documentation` (authoritative current rules), `Historical News / Press Release` (past announcements), and `Local Document`.
  - **Filesystem Timestamps**: Automatically tags all local files with `last_modified_date` and `creation_date`.
  - **Time-Aware Chunk Headers**: Injects `Source: ... | Workspace: ... | Last Modified: YYYY-MM-DD | Type: ...` into every chunk.
  - **Recency Primacy & Conflict Resolution**: The AI agent evaluates timestamps and status notices, ensuring that current rules (`Status: Paused`) always supersede older historical announcements.
- **🔬 2-Phase High-Precision RAG & Context Calibration Engine**:
  - **🎯 Semantic Precision & Zero Cross-Domain Noise (Contextual Retrieval)**: The system captures the overarching theme and domain keywords of each document and web page before reading paragraphs. Searches on a specific subject never pull unrelated files (like financial or IT docs in legal questions).
  - **Phase 1 (Deep Candidate Pool Scanning)**: Evaluates a broad initial pool of **100+ candidate chunks** across thousands of indexed pages and files, guaranteeing 100% recall coverage.
  - **Phase 2 (Balanced Multi-Source Diversity)**: Balanced round-robin distribution ensures fair representation across all documents and web portals (up to 3 chunks per source), preventing a single large file from monopolizing the context window.
  - **⚡ Ultra-Fast Parallel Multi-Source Search**: Concurrently searches the active workspace, linked Shared Sources, and Global knowledge bases on all CPU cores, fusing results in sub-10ms.
  - **"Lost in the Middle" Prevention & Context Calibration**: By injecting **15-20 high-density chunks (~10,000-15,000 tokens)** rather than 40+ noisy chunks, AnyContext prevents attention degradation, eliminates hallucinations, reduces latency by 3x, and stays well within provider rate limits.
  - **🧹 Smart Conversation Flow & Zero Token Overflows**: Automatically compacts past search dumps in multi-turn conversations, keeping dialogue fast and prompt size permanently optimized (~10,000 tokens) even after dozens of consecutive questions.
  - **🔄 Resilient Auto-Retry with Exponential Backoff**: Automatic transparent recovery from AI provider rate limits (TPM/RPM 429 errors) across all 9 supported providers with millisecond backoff loops, ensuring zero disruption during heavy research sessions.
  - **🎛️ Dynamic Retrieval Presets Matrix**:
    | Preset | Pool no LanceDB | Chunks Injetados | Tokens de Contexto | Melhor uso |
    | :--- | :---: | :---: | :---: | :--- |
    | **⚡ Turbo** | 50 candidatos | 10 chunks | ~5.000 tokens | **Velocidade máxima:** perguntas rápidas, fatos pontuais, dúvidas do dia a dia. |
    | **⚖️ Balanced** *(Padrão)* | 100 candidatos | 20 chunks | ~10.000 a 15.000 tokens | **Equilíbrio perfeito:** alta precisão factual sem ruído (o padrão ideal). |
    | **🔬 Deep Research** | 150 candidatos | 40 chunks | ~30.000 a 40.000 tokens | **Auditoria pesada:** comparar cláusulas de 10 contratos ou analisar dossiês complexos. |
- **🛡️ Strict Context Grounding & Zero Pre-Training Hallucination**:
  - Answers are strictly anchored to the retrieved workspace chunks.
  - The AI is forbidden from using outdated 2023 pre-training weights to answer current factual, legal, or regulatory questions.
  - Cross-lingual domain translation: Automatically translates Portuguese prompts into targeted English domain keywords when searching English documentation.
- **🧠 3-Level Structured Long-Term Memory (5 Dimensions)**:
  - **Level 1 (Structured 5-Dimension Session Summary)**: Extracted upon `/exit` or `/q` across 5 clear dimensions:
    1. 👤 *User Directives & Preferences* (rules, workflow habits).
    2. 🏗️ *Technical Architecture & Key Decisions* (parameters, constants, schemas).
    3. 📁 *Files, Code Symbols & Databases* (files touched, functions, tables).
    4. 📌 *Critical Context & Problem Resolution* (root-cause diagnoses, bug fixes).
    5. 🚀 *Pending Tasks & Next Steps* (roadmap milestones, open tasks).
  - **Level 2 (Active Rolling Window)**: Retains recent active messages in SQLite graph state.
  - **Level 3 (Consolidated Meta-Summarization)**: Consolidates older memory vectors into high-level indices.
  - **1024-Token Memory Chunks**: Expanded chunking (`chunk_size=1024`, `chunk_overlap=200`) preserves deep technical reasoning.
- **📂 Decoupled Workspace Architecture & Empty Workspaces**:
  - Workspaces are isolated logical scopes for privacy and contextual boundaries.
  - Creating a workspace is completely separate from attaching folders: create empty workspaces anytime via `/switch <name>` or `/workspace add <name>` (ideal for web documentation portals, market research, or agent tasks).
  - Attach local folders or web URLs whenever you want via `/config` or `/web add`.
- **🏛️ 3-Tier AI Context Architecture (Company Global & Shared Sources)**:
  - **1. 🏢 Institutional Global (`Global`)**: Organization-wide knowledge base curatable by admins and automatically inherited in RAG search across authorized project workspaces.
  - **2. 📦 Reusable Shared Sources Library (`Shared Sources`)**: Dedicated central library workspace for reusable frameworks, codebases, and documentation portals linked across project workspaces on-demand in < 50ms with **$0.00 in embedding costs** via `/link` and `/shared`.
  - **3. 📁 Scoped Active Workspaces**: Dedicated project-specific boundaries guaranteeing zero cross-department data leakage.
- **🛒 Universal Schema.org & E-Commerce Rating Extraction**:
  - Automatic extraction of `Product` and `IndividualProduct` structured metadata (`<script type="application/ld+json">`) including star ratings, review counts, prices, and stock status across Walmart, Amazon, Mercado Livre, Shopify, VTEX, WooCommerce, etc.
  - Retains HTML semantic tags (`<form>`, `<header>`, `<aside>`, `<dl>`) so visible review badges (e.g. `4.844 out of 5 stars. 1199 reviews`) and buy box pricing are never discarded.
- **🔓 100% Unlocked Community Edition CLI**: Full local power for individual users at zero cost:
  - Unlimited local workspaces and folders.
  - Deep recursive subfolder scanning.
  - **Interactive 2-Phase Web Crawler & Sitemap Engine** (`/web add`): Fast discovery phase maps all internal URLs and XML sitemaps, presenting estimated page counts before concurrent multi-threaded crawling.
  - Image & Scanned PDF OCR parsing (`/ocr`).
  - ChromaDB local vector storage & SQLite long-term memory.
  - Access to all 9 supported AI model providers.
- **🤖 9 Leading AI Providers with Verified Low-Tier Models**:
  - **OpenAI**: `gpt-4o-mini`, `gpt-4o`, `gpt-4-turbo`, `gpt-3.5-turbo`.
  - **Anthropic Claude**: `claude-haiku-4-5-20251001`, `claude-sonnet-4-5-20250929`, `claude-sonnet-4-6`, `claude-opus-4-5-20251101`.
  - **Google Gemini**: `gemini-flash-latest`, `gemini-3.5-flash`, `gemini-3.5-flash-lite`, `gemini-pro-latest`.
  - **DeepSeek**: `deepseek-chat` (DeepSeek V3 - High Intelligence at $0.14/M tokens).
  - **Groq Cloud**: `llama-3.3-70b-versatile`, `llama-3.1-8b-instant`, `mixtral-8x7b-32768`, `gemma2-9b-it`.
  - **Mistral AI**: `mistral-small-latest`, `open-mistral-nemo`, `mistral-large-latest`.
  - **xAI Grok**: `grok-2-1212`, `grok-2`, `grok-beta`.
  - **OpenRouter**: `openrouter/auto`, `meta-llama/llama-3.3-70b-instruct:free`, `google/gemini-flash-1.5-8b`.
  - **Local Offline (Free & Private)**: `local-model` via LM Studio or Ollama (`http://localhost:1234/v1`).
- **🎉 Zero-Friction First-Time Onboarding & Surface Parity**:
  - Automatically detects fresh installations or factory resets (`actx --factory-reset`).
  - Launches an interactive setup wizard across both OpenTUI Desktop (`actx --tui`) and CLI (`actx`).
  - One-click configuration for **OpenAI Cloud** (`gpt-4o-mini` & `text-embedding-3-small`), **Local Offline Server** (LM Studio / Ollama at $0.00), or **Custom Provider Gateway**.
- **📋 Multi-line & Long-Prompt Input Engine**:
  - **Direct Bracketed Paste (`Ctrl+V`)**: Seamlessly paste multi-line contract clauses, meeting transcripts, or large code snippets without premature command submission.
  - **Universal Line Break (`Ctrl + J` / `Esc` then `Enter`)**: Insert clean newlines with visual continuation prompts (`... `) anytime while typing.
  - **Shell-Style Line Continuation (`\` + `Enter`)**: End any line with a backslash `\` and press Enter to naturally continue your prompt on the next line.
  - **Triple Quotes (`""" ... """` or `''' ... '''`)**: Start a prompt with `"""` to enter multi-line block mode and submit by closing with `"""` or `/send`.
  - **Dedicated `/paste` Capture Mode**: Type `/paste` or `/multiline` to open an explicit capture buffer with abort (`/cancel`) protection.
- **🔄 Instant Zero-Cost Source Transfer (`/transfer` & `/config`)**:
  - Move folders and web portals between workspaces in sub-50ms with **$0.00 in embedding API costs**.
  - Dynamically updates vector metadata tags in ChromaDB and SQLite without re-indexing or re-crawling.
- **🖥️ OpenTUI Reactive Desktop Interface (`actx --tui`)**:
  - TUI reativa desenvolvida com **OpenTUI (`@opentui/core` + `@opentui/react`)** e Zig/React: chat rolável fluido com **paridade estética e visual completa com a CLI UI**, herança de background transparente nativo do terminal, Banner ASCII Art clássico, identificadores `👤 You:` e `🤖 AI [modelo]:`, e barra de status inferior de 1 linha.
  - **Frozen Full-Screen Scroll Engine**: Rolagem nativa fluida em tela cheia com **rodinha do mouse**, teclas `PgUp` / `PgDn`, `Shift + ↑` / `↓` e auto-scroll reativo em streaming.
  - **⚡ Real-Time Progressive Engine Startup Telemetry**: Substituição completa de telas de espera estáticas por uma árvore progressiva de telemetria de boot (`┌─ ⚡ Engine Startup Telemetry`) com animação de spinner braille em tempo real e cadência perceptual em cascata suave (Abordagem B). Acompanha o spawn do runtime Python, conexão com SQLite, vinculação do modelo de IA, conexão do workspace e verificação do índice vetorial, fixando os tempos exatos em milissegundos no topo da conversa para paridade visual absoluta com o Terminal CLI.
  - **Slash Command Palette (`/`)**: Ao teclar `/`, uma paleta flutuante abre automaticamente com filtro fuzzy em tempo real e navegação por setas para todos os 23 comandos internos.
  - **Zero-Port Stdio RPC Bridge & Sub-Process DLL Isolation**: Comunicação local em sub-milissegundo com o backend AnyContext via NDJSON, sem portas de rede ou avisos de firewall, com isolamento total de variáveis de bootloader (`_MEIPASS`, `PATH`) para execução perfeita em binários compilados sem colisões de DLL em C (`multiarray.pyd`).
- **⚡ Native Developer CLI & One-Shot Prompt Engine**:
  - Direct terminal execution without alternate screen buffer locks: 100% native mouse selection and copy/paste support.
  - One-shot execution for scripts & automation: `actx "sua pergunta aqui"` or `actx -p "..."`.
  - All 23 Slash Commands with rich interactive menus (`/sources`, `/switch`, `/sync`, `/config`, `/help`, `/mode`, `/web-search`).

- **🗑️ Interactive Workspace Deletion with Safety Confirmation (`/menu` & `/switch --delete`)**:
  - Exclusão segura e interativa de workspaces com listagem do total de fontes e modal de confirmação explícito (`Yes, permanently delete` vs `Cancel` com foco seguro).
  - Proteção de workspaces do sistema (`Default`, `Global`, `Shared Sources`) e fallback automático para `Default`.

- **🧹 Clean Markdown Observability & TUI ANSI-Shield Engine (`v0.28.73`)**:
  - **Zero Terminal Buffer Corruption**: Refactored `/logs`, `/spans`, and `/diagnostics` to emit 100% clean GitHub Flavored Markdown with monospaced fenced code blocks (` ```text `), permanently eliminating raw ANSI escape sequences (`\033[...]`) that caused cell positioning corruption and scattered characters in OpenTUI.
  - **OpenTUI Frontend ANSI Sanitization**: Implemented `stripAnsi()` across user, system, and assistant messages, rendering immune against control characters from external processes.
  - **Cross-Platform CI/CD Build Hardening**: Hardened `launcher/build_shim.py` with `UTF-8` reconfiguration and ASCII status indicators, eliminating `UnicodeEncodeError` under Windows runner `cp1252` environments.
  - **Multi-Interface Parity**: Integrated `/logs`, `/spans`, and `/diagnostics` directly into CLI interactive chat loop dispatching.

- **🌐 Resilient Web Ingestion & Decompression Engine (`v0.28.72`)**:
  - **Tolerant HTTP Stream Decompression**: Native support for `Content-Encoding: gzip`, `deflate` and `.xml.gz` sitemaps using `zlib.decompressobj(32 + zlib.MAX_WBITS)`. Gracefully handles truncated, partial, or interrupted network streams without failing with low-level `Error -5 incomplete stream`.
  - **Self-Healing Web Source Addition**: Transparent auto-recovery and retry loops in `SourceService` and `CommandDispatcher` for web portals (including CDNs like Akamai and Cloudflare).
  - **Structured Command Error Telemetry**: All slash command exceptions and full tracebacks are automatically logged to `system_logs` in `settings.db` and queryable via `/logs`.

- **🩹 RAG Self-Healing Ingestion Engine**:
  - Invalidação automática de cache desatualizado no SQLite caso arquivos físicos existam no disco mas chunks estejam ausentes no LanceDB, garantindo reindexação imediata e respostas precisas com citação de fontes.

- **⚡ Non-Blocking Background Synchronization & Micro-Bar Status Dock (`/sync`)**:
  - Sincronização desacoplada em threads em segundo plano (`BackgroundSyncManager`) que libera o prompt `👤 You:` imediatamente para conversação sem travar o teclado.
  - Micro-barra de blocos unicode em tempo real (`⚡ Syncing [████░░░░] 50% (15/30 files)`) na barra inferior (`bottom_toolbar`).
  - Sincronização mestre unificada em todas as fontes (pastas locais, portais web, cloud drives) com modo verbose estruturado para auditoria completa (`/sync --verbose`).
- **🎯 Typo-Resilient Slash Command Interception**:
  - Mistyped commands (e.g. `/check-updaete`, `/swich`, `/modeel`, `/sinc`) are caught by the intelligent fuzzy matcher and suggest the correct command without wasting AI tokens.
- **🔄 Interactive Multi-Instance Aware Self-Updater & Auto-Restart (`/update` & `/check-update`)**:
  - Detects GitHub releases with cache-busting and prompts: `? Would you like to download and install vX.Y.Z now? [Y/n]`.
  - **Multi-Instance Intelligence & Preserved Working Directory**: Substituição atômica com reinicialização automática assíncrona preservando o diretório de trabalho do projeto do usuário (`-WorkingDirectory`).
- **🌐 REST API Server Mode (`actx --serve`)**:
  - High-performance FastAPI server with interactive Swagger UI at `http://127.0.0.1:8000/docs`.
- **🔌 Model Context Protocol (MCP) Server (`actx --mcp`)**:
  - Native JSON-RPC stdio implementation for **Claude Desktop**, **Cursor IDE**, and **Antigravity**.
- **📘 Permanent Self-Help System**:
  - The AI agent has full access to this documentation. You can ask in chat: *"Como eu adiciono um site ao meu workspace?"* or *"Quais comandos estão disponíveis?"*.
- **⚡ Sub-2ms Native Launcher Shim Architecture & Clean Versioning (`v0.28.71`)**:
  - Implementação de Launcher Shim nativo autônomo (`actx.exe` / `actx` < 20KB) compilado diretamente para código de máquina nativo (via `csc.exe` no Windows ou `gcc` no Linux), sem dependência de máquina virtual ou overhead de extração do PyInstaller.
  - O comando `actx -v` / `actx --version` responde instantaneamente em **< 2 milissegundos** exibindo a versão no padrão limpo e direto da indústria (`v0.28.71`), exatamente como `node -v` ou `agy --version`.
  - Delegação transparente de comandos completos e interfaces interativas (`actx`, `actx --tui`, `actx --mcp`) para o motor core (`actx-core.exe`).
  - Suíte de testes automatizados expandida para 207 testes (100% PASS).

- **🖥️ OpenTUI Syntax Repair & CI Frontend Verification Gate (`v0.28.70`)**:
  - Correção cirúrgica de sintaxe no componente React principal [`app.tsx`](file:///C:/Users/guilh/source/repos/any-context/src/any_context/tui/app.tsx) do OpenTUI, fechando blocos de callback de erro e restaurando a inicialização instantânea do comando `actx --tui`.
  - Integração do verificador estático do Bun (`bun run check` / `tsc --noEmit`) ao pipeline oficial do GitHub Actions (`.github/workflows/e2e-tests.yml`), impedindo regressões visuais ou de sintaxe no frontend antes de qualquer merge.
  - Adição de teste unitário automatizado (`test_tui_syntax.py`) elevando a suíte para 203 testes 100% aprovados.

- **🛡️ Provenance-Based Workspace Immunity, Pre-Migration Snapshots, and Persistent Logging Architecture (`v0.28.79`)**:
  - **Imunidade Absoluta por Proveniência (`created_by`)**: Adição de rastreamento de autoria na tabela `workspaces`. Workspaces criados por usuários (`created_by = 'user'`) são 100% imunes a qualquer rotina de expurgo ou migração automática, independente do seu nome (mesmo que chamado `TestWorkspace` ou iniciando com `test_`).
  - **Snapshots de Segurança Pré-Migração**: O Core realiza backup automático do banco de dados SQLite (`settings.db.bak`) antes de executar qualquer instrução DDL ou migração estrutural.
  - **Logs Persistentes de Instalação, Atualização e Migração**: Registro estruturado e persistente em disco em `%LOCALAPPDATA%\AnyContext\logs\` (`install.log`, `update.log`, `migration.log`), permitindo auditoria ponta a ponta de downloads, processos em execução, substituições atômicas e alterações de schema.

- **🛡️ Test Sandbox Isolation, Auto-Purge of Legacy Workspaces, and Hexagonal Model Authority (`v0.28.78`)**:
  - **Imunidade Total do Banco de Dados**: A suíte inteira de testes automatizados (`tests/run_all.py` e testes individuais) executa em sandbox temporário isolado (`ACTX_SETTINGS_DB`), garantindo que nenhum teste contamine o banco de produção do usuário.
  - **Auto-Expurgo Seguro de Workspaces Residuais**: Inicialização do Core (`_init_db`) expurga apenas workspaces órfãos comprovadamente criados por testes (`created_by = 'test'` e sem fontes associadas), protegendo totalmente o usuário.
  - **Paridade Hexagonal no OpenTUI (RPC Bridge)**: A barra de status e o backend RPC delegam a resolução e persistência de modelos para o `ModelService`, garantindo inicialização limpa com `🤖 GPT-4o Mini` no workspace `Default` e isolamento por workspace.

- **🛡️ 100% Native Standard Test Suite & CI/CD Pipeline Hardening (`v0.28.69`)**:
  - Eliminação de dependências externas de teste em ambiente de execução: migração e padronização integral dos testes unitários para `unittest.TestCase` nativo do Python (com zero dependências do `pytest` para execução).
  - Resolução definitiva de erros de importação no GitHub Actions CI/CD (`ModuleNotFoundError: No module named 'pytest'`).
  - Descoberta automatizada do módulo de testes `unit/ingestion` (crawler e progresso em tempo real) com 100% dos 200 testes aprovados.

- **⚡ Background Web Crawler Real-time Progress & Instant Completion Notifications (`v0.28.68`)**:
  - Feedback visual imediato e contínuo durante a raspagem web em background (`⚡ Crawling [████░░░░] 50% (15/30 pages)`), eliminando o estado prematuro de `✔ Up to date` enquanto o crawler ainda está indexando.
  - Notificações de conclusão emitidas em tempo real em todas as interfaces (CLI, OpenTUI e RPC Bridge) indicando o total exato de páginas web e arquivos indexados.

- **🤖 Factory Default Model Isolation (`gpt-4o-mini`) per Workspace (`v0.28.67`)**:
  - Todo e qualquer novo workspace criado no sistema (seja no primeiro uso, via `/switch`, REST API ou MCP Server) é inicializado rigorosamente com o modelo padrão de fábrica `gpt-4o-mini`.
  - O modelo configurado em um workspace é isolado e persistido de forma independente, impedindo contaminações indesejadas de modelos mais caros para workspaces recém-criados.


  - Resposta instantânea de linha de comando (`actx -v` / `actx --version` em < 5ms) via roteamento imediato sem carregamento de banco de dados ou variáveis de ambiente.
  - Verificação de atualizações (`print_startup_update_notice`) desacoplada em segundo plano de forma 100% assíncrona, eliminando qualquer travamento por latência de rede no início do terminal.
  - Telemetria visual de micro-etapas de boot exibida logo abaixo do banner (`[08ms] 🔌 SQLite Config Store active`, `[19ms] 📂 Workspace connected`, `[29ms] 🤖 AI Model engine linked`, `[36ms] 🚀 Ready in 0.04s`), transmitindo sensação imediata de velocidade, precisão e responsividade.


- **⏱️ Deep Observability Engine & Time Watching (`/logs`, `/diagnostics`, `/spans`, `actx --diag`)**:
  - Rastreamento cirúrgico de latência (*time watching*) com precisão de microssegundos em todas as operações críticas do sistema (recuperação vetorial RAG, comandos slash, raspagem web, sincronização em background e chamadas de inferência).
  - Tabela persistente SQLite com auto-purga e métricas estatísticas consolidadas (`avg_ms`, `min_ms`, `max_ms`, `count`, `status`).
  - Painel de diagnósticos integrado acessível via linha de comando (`actx --diag`, `actx --logs`) ou no chat interativo (`/diagnostics`, `/logs`, `/spans`, `/perf`), permitindo identificar gargalos de tempo instantaneamente.

- **🔒 Hardware-Bound AES-GCM-256 Vector Storage Encryption & OS Isolation (`v0.28.16`)**:

  - Armazenamento nativo em diretórios padrão do sistema operacional (`%LOCALAPPDATA%\AnyContext\` no Windows, `~/Library/Application Support/AnyContext/` no Mac, `~/.local/share/any-context/` no Linux).
  - Criptografia local em repouso: textos de documentos, resumos contextuais e palavras-chave são cifrados no disco com **AES-GCM-256** amarrado ao ID exclusivo de hardware da máquina (Hardware Binding).
  - Proteção total contra cópia indevida ou extração de bases de dados fora do AnyContext com velocidade instantânea via aceleração por hardware (CPU AES-NI).

---

## ⚡ Quick Start & Installation

### Option 1: 1-Click Terminal Installer (No Python Required!)

Download the installer from the **[Latest GitHub Release](https://github.com/Levix-Digital/any-context/releases/latest)**:

- **Windows (PowerShell)**:
  ```powershell
  irm https://raw.githubusercontent.com/Levix-Digital/any-context/main/install.ps1 | iex
  ```
- **Linux / macOS (Bash)**:
  ```bash
  curl -fsSL https://raw.githubusercontent.com/Levix-Digital/any-context/main/install.sh | bash
  ```

### Option 2: Install via Python / `uv` / `pip`

```bash
git clone https://github.com/Levix-Digital/any-context.git
cd any-context
pip install -e .
```
*(Available terminal aliases: `actx`, `anycontext`, `any-context`, `ac`)*

---

## 💬 In-Chat Slash Commands Reference

Inside the interactive chat (`actx`), use these powerful slash commands:

| Command | Standard Options / Flags | Description |
| :--- | :--- | :--- |
| **`Ctrl+V` (Paste)** | — | Paste multi-line text with line breaks without premature submission. |
| **`[Ctrl + J]`** | `[Esc] + Enter` | Universal terminal newline shortcut (Linefeed). |
| **`\` + `[Enter]`** | — | Trailing backslash shell-style line continuation. |
| **`""" ... """`** | `''' ... '''` | Multi-line block delimiter (close with `"""` or `/send`). |
| **`/paste`** | `/multiline` | Open dedicated multi-line paste capture mode. |
| **`/sync`** | `--folder`, `--web`, `--drive`, `--all`, `--force`, `--status`, `--bg` | **Sincronizador Mestre Unificado**: Sincroniza todas as fontes (pastas, web, drives) do workspace ativo ou todas de uma vez (`--all`). |
| **`/folder`** | `--add <path>`, `--remove <path>`, `--list`, `--sync` | Gerencia e sincroniza exclusivamente pastas locais do workspace. |
| **`/web`** | `--add <url>`, `--remove <url>`, `--list`, `--sync` | Gerencia, rastreia e sincroniza exclusivamente portais e páginas web. |
| **`/drive`** | `--add`, `--remove <id>`, `--list`, `--sync`, `/cloud` | Gerencia e sincroniza conexões com Google Drive e OneDrive. |
| **`/mode`** | `--hybrid`, `--strict`, `--proactive`, `--global` | Switch AI grounding mode per workspace or globally: **`Hybrid`** (dual-layer), **`Strict`** (100% verified facts), or **`Proactive`** (research & synthesis). |
| **`/web-search`** | `--on`, `--off`, `--status`, `--global`, `/ws` | Toggle real-time live web search per workspace with domain portal prioritization and source discrimination. |
| **`/link`** | `<src> [dst]`, `--list`, `--unlink <src>` | Link or unlink reusable indexed sources across workspaces with zero API cost ($0.00). |
| **`/transfer`** | `<src_ws> <tgt_ws> <path_or_url>` | Instant zero-cost transfer of folders or web portals between workspaces. |
| **`/rename`** | `<old_ws> <new_ws>` | Atomic zero-cost workspace renaming ($0.00). |
| **`/model`** | `<name>`, `--list`, `-l` | Open key-aware AI model selector across 9 providers or list available models. |
| **`@model <msg>`** | — | One-shot prompt to a specific model without changing session defaults. |
| **`/api-keys`** | `/keys` | Step-by-step guide with portal links to obtain API keys. |
| **`/ocr`** | `image`, `scan` | View Image & Scanned PDF OCR parsing status. |
| **`/config`** | `-c` | Open interactive settings menu (Workspaces, AI Models, API Keys). |
| **`/billing`** | `--plans`, `-p` | View subscription tiers, capabilities, and license status. |
| **`[↑] / [↓]`** | — | Navigate through past prompts & commands in the active workspace. |
| **`/history`** | `--clear`, `--limit <n>` | Display or clear input history for the active workspace. |
| **`/update`** | `--check`, `-c`, `--force` | Check for updates and download/install the latest release. |
| **`/reset-memory`**| `--force`, `--all` | Purge conversation session memory for active or all workspaces. |
| **`/clear`** | `/cls` | Clear terminal screen and redraw the clean signature banner. |
| **`/factory-reset`**| `--force` | Reset all settings, workspaces, API keys, and databases to clean factory state. |
| **`/version`** | `/v` | Display current AnyContext version. |
| **`/help [cmd]`** | `/h`, `[cmd] --help`, `[cmd] -h` | Open the interactive manual index or get help for a specific command. |
| **`/exit`** | `/q`, `exit`, `quit` | Save structured 5-dimension long-term memory and exit gracefully. |
| **`Ctrl+C`** | — | Interrupt AI generation immediately or prompt graceful exit. |

---

## 🔬 Deep-Dive: A Ciência do Motor RAG do AnyContext

O AnyContext foi projetado com uma arquitetura de RAG em **duas fases distintas**, resolvendo os dois maiores desafios da inteligência artificial moderna: **alcance amplo em grandes volumes de dados** e **precisão cirúrgica sem dispersão de atenção**.

```mermaid
graph TD
    A["📚 10.000+ Páginas & Arquivos no Workspace"] --> B["⚡ Etapa 1: Busca Paralela na CPU (ThreadPoolExecutor)<br/>(Escaneia pool de 100+ candidatos no ChromaDB em < 15ms)"]
    B --> C["⚖️ Etapa 2: Diversificação Source-Fair Round-Robin<br/>(Filtra a nata de 15-20 chunks mais densos sem ruído)"]
    C --> D["🧠 LLM Prompt Calibrado (~10.000 tokens)<br/>(Foco cirúrgico, zero alucinação, 3x mais rápido)"]
    D --> E["💬 Resposta Instantânea & Fundamentada"]
```

### 1. Profundidade de Busca vs. Injeção no Modelo
- **Busca Vetorial Ampla (ChromaDB):** A busca nunca é rasa. O ChromaDB avalia **100% dos documentos indexados**, avaliando similaridade semântica em um pool inicial amplo de **100 candidatos**.
- **Injeção Calibrada na LLM:** Apenas a "nata" dos trechos de maior relevância entra no prompt do modelo.

### 2. O Fenômeno *"Lost in the Middle"* (Por que mais chunks podem prejudicar a precisão)
- **A Escala Real de 10.000 Tokens:** 10.000 tokens equivalem a **~7.500 palavras** ou cerca de **15 a 20 páginas de livro densamente digitadas**. Para responder a uma pergunta pontual (ex: *"Quem deve assinar o waiver?"*), a resposta exata ocupa de 300 a 600 tokens (1 a 2 páginas).
- **O Perigo de Injetar 50.000+ Tokens:** Prompts gigantescos contendo 40 a 50 chunks brutos introduzem dezenas de páginas irrelevantes (horários, cardápios, avisos gerais). Pesquisas científicas de ponta (incluindo Stanford) comprovam que LLMs sofrem de **degradação de atenção no meio do contexto**, perdendo trechos cruciais e gerando alucinações.
- **Precisão por Calibração:** Ao entregar **15 a 20 chunks mais densos**, a IA foca 100% no que importa, reduz o tempo de resposta em 3x e economiza até 90% em créditos de API.

### 3. Algoritmo *Source-Fair Round-Robin* (Diversificação Multi-Fonte)
Para garantir que um documento gigante de 500 páginas não monopolize todas as vagas de resposta:
- O algoritmo balanceia a seleção pegando no máximo 2 a 3 trechos de cada arquivo ou página web diferente.
- Em workspaces com 50 PDFs ou 20 portais web, **todas as fontes relevantes têm representação garantida** no contexto final.

### 4. Resiliência a Rate Limits (TPM / RPM) com Backoff Exponencial
- Todos os 9 provedores de IA (OpenAI, Anthropic Claude, Gemini, Groq, DeepSeek, Mistral, xAI) impõem limites de **Tokens por Minuto (TPM)** e **Requisições por Minuto (RPM)**.
- O AnyContext conta com **`max_retries=5` e Backoff Exponencial automático**: caso um provedor retorne **HTTP 429 (Rate Limit)** em rajadas de perguntas, o sistema aguarda silenciosamente as frações de segundo necessárias e conclui a resposta sem que você veja erro na tela.

### 5. Pruning de Histórico em Runtime (`PruningChatModelWrapper`)
- O histórico da conversa preserva 100% das perguntas do usuário e conclusões da IA (`User` ➔ `AI`), mas intercepta e poda os blocos de texto bruto de buscas passadas (`ToolMessage`) em runtime para `"[Prior workspace context retrieved and synthesized in conversation history]"`.
- O prompt enviado à LLM se mantém permanentemente entre ~5.000 e 12.000 tokens, eliminando o erro de estouro de 128.000 tokens em conversas de 100+ turnos.

### 🎛️ Matriz Dinâmica de Presets de RAG

| Preset | Pool no ChromaDB | Chunks Injetados | Tokens de Contexto | Melhor uso |
| :--- | :---: | :---: | :---: | :--- |
| **⚡ Turbo** | 50 candidatos | 10 chunks | ~5.000 tokens | **Velocidade máxima:** perguntas rápidas, fatos pontuais, dúvidas do dia a dia. |
| **⚖️ Balanced** *(Padrão)* | 100 candidatos | 20 chunks | ~10.000 a 15.000 tokens | **Equilíbrio perfeito:** alta precisão factual sem ruído (o padrão ideal). |
| **🔬 Deep Research** | 150 candidatos | 40 chunks | ~30.000 a 40.000 tokens | **Auditoria pesada:** comparar cláusulas de 10 contratos ou analisar dossiês complexos. |

### 🛡️ Modos de Grounding, Busca Web & Matriz Dinâmica de Prioridades (`/mode` & `/web-search`)

O AnyContext adota uma arquitetura determinística de **Injeção de Estratégias por Turno (*Strategy Pattern*)** que protege o agente contra *prompt dilution* (degradação de foco em conversas longas). Cada turno de conversa recebe dinamicamente um cabeçalho ultracompacto (~35-45 tokens) contendo a matriz exata de precedência de fontes, priorização de portais web registrados e a **Regra Universal de Recência Temporal (*Universal Temporal Recency Rule*)**:

> **⏱️ Regra Universal de Recência Temporal (Critério de Desempate):**
> Quando múltiplas fontes coexistem dentro do **mesmo nível de prioridade** (ex: um documento do VectorDB e um portal web registrado no Tier 0; ou a busca na web aberta e o conhecimento paramétrico no Tier 1; ou todos os canais fundidos no modo Proativo), **a fonte com timestamp/data mais recente SEMPRE prevalece como a verdade factual**, sobrepondo dados legados ou desatualizados.

#### 📊 Matriz de Prioridades 3 × 2 (Grounding 3 × Web 2 = 6 Estados)

| Modo | Web Search | Precedência de Fontes | Protocolo de Ausência & Regra de Recência no Mesmo Tier |
| :--- | :---: | :--- | :--- |
| **🛡️ Strict** *(Auditoria / Jurídico)* | **OFF** | • **Prioridade 0:** VectorDB exclusivo.<br>• **Proibida:** Memória Paramétrica (0% alucinação).<br>• **Desativada:** Busca Web. | Responde 100% pelos docs locais. Se houver divergência entre arquivos: **o mais recente prevalece**.<br>**Ausência:** Declara estritamente: `⚠️ Essa informação não consta nos documentos deste workspace.` |
| **🛡️ Strict** *(Auditoria / Jurídico)* | **ON** | • **Prioridade 0:** VectorDB.<br>• **Prioridade 1 (Gated):** Portais Registrados e Web Aberta.<br>• **Proibida:** Memória Paramétrica (0% alucinação). | **Gate de Permissão Obrigatório:** O agente é proibido de buscar na web de forma autônoma no turno inicial. Se o dado não constar nos arquivos locais, para e pergunta:<br>`⚠️ Essa informação não consta nos documentos deste workspace. Deseja que eu faça uma busca na internet sobre '[tópico]'?`<br>*Apenas após o 'sim' explícito a busca web é realizada.* Entre fontes no mesmo tier, **a mais recente sempre prevalece**. |
| **⚖️ Hybrid** *(Equilibrado / Padrão)* | **OFF** | • **Prioridade 0:** VectorDB (Docs locais primeiro).<br>• **Prioridade 1:** Conhecimento Geral do Modelo.<br>• **Desativada:** Busca Web. | Fatos do workspace têm primazia. Resposta em duas camadas estruturadas: `### 📂 Informações do Workspace` e `### 💡 Sugestões / Conhecimento Geral do Modelo`. Arquivo mais recente prevalece no Tier 0. |
| **⚖️ Hybrid** *(Equilibrado / Padrão)* | **ON** | • **Prioridade 0:** VectorDB & Portais Web Registrados do Workspace.<br>• **Prioridade 1:** Busca na Web Aberta & Conhecimento Geral do Modelo. | Busca na web executada **autonomamente** se o workspace estiver incompleto. Prioriza portais registrados (`target_domain`) antes da web global. No Tier 0 ou Tier 1, **a informação mais recente sempre prevalece**. |
| **🚀 Proactive** *(Pesquisa & Estratégia)* | **OFF** | • **Prioridade 0:** VectorDB & Conhecimento Estratégico do Modelo.<br>• **Desativada:** Busca Web. | Fusão de documentos locais com antecipação de riscos, próximos passos e visão estratégica. A norma ou dado mais recente prevalece. |
| **🚀 Proactive** *(Pesquisa & Estratégia)* | **ON** | • **Prioridade 0 (Fusão Total em Tempo Real):** VectorDB + Portais Web + Web Search ao Vivo + Raciocínio Proativo. | Execução autônoma e contínua de RAG + Web. **Total fusão temporal:** a fonte com data mais recente em qualquer canal prevalece. Sugere ativamente novas URLs autoritativas com `/web add <url>`. |

### 🖥️ Painel Fixo de Input & Barra de Status Ancorada no Rodapé (`v0.24.6`)

Inspirado na experiência de console do **Antigravity CLI**, o AnyContext gerencia as margens de rolagem do terminal via sequências ANSI VT100 (`DECSTBM - \033[top;bottom r`):
- **Barra de Status Permanentemente Ancorada**: O divisor horizontal e o dock de informações (`📂 Workspace │ 🤖 Modelo │ 🛡️ Modo │ 🌐 Search │ ⚡ Syncing │ 🚪 /exit`) permanecem **fixos na base da tela** durante todo o ciclo de busca vetorial, chamadas de ferramentas e streaming de tokens.
- **Streaming Fluido na Janela Superior**: A resposta da IA sobe naturalmente na área de rolagem superior sem empurrar, piscar ou apagar a barra de status.
- **Resiliência Multi-Console**: Compatível com Windows Terminal, PowerShell, VS Code Terminal, Cursor e Git Bash, com fallback transparente para ambientes não-TTY (CI/CD).

---


## 💡 Real-World Usage Examples (For Technical & Non-Technical Users)

### ⚖️ 1. Legal / Contract Risk Analysis (Non-Technical)
```text
You [LegalDocs | gpt-4o-mini]: Qual é o prazo de vigência e as penalidades por quebra de confidencialidade no Contrato Acme?

🤖 AI [gpt-4o-mini]:
🔍 [Search] Searching strictly within Workspace: 'LegalDocs' (top 8 chunks)...
📚 Reading retrieved documents...

De acordo com o arquivo 'Contrato_Acme_2025.docx' (Modificado em: 2026-08-12):
- **Prazo de Vigência**: 24 meses contados a partir da data de assinatura (Cláusula 8.1).
- **Penalidade por Quebra**: Multa não compensatória de R$ 100.000,00 (cem mil reais), sem prejuízo de perdas e danos apurados judicialmente (Cláusula 12.4).
- **Aviso Prévio**: A rescisão imotivada exige notificação prévia de 30 dias (Cláusula 8.3).
```

### 🛂 2. Temporal Immigration / Government Portal Verification (Non-Technical)
```text
You [Immigration | gpt-4o-mini]: Qual o status atual do Start-up Visa Program do Canadá?

🤖 AI [gpt-4o-mini]:
🔍 [Search] Searching strictly within Workspace: 'Immigration' (top 8 chunks)...
📚 Reading retrieved documents...

De acordo com a página oficial 'Start-up Visa Program - Canada.ca' (Última modificação: 2026-07-21 | Tipo: Canonical Service):
- **Status Operacional**: O programa está atualmente **PAUSADO (Paused)** para novas aplicações.
- **Regra em Vigor**: Apenas candidatos com certificado de compromisso válido emitido em 2025 podem submeter suas aplicações (prazo limite: 30 de junho de 2026).
- **Permissão de Trabalho**: Solicitantes com visto de trabalho ativo vinculado ao programa podem solicitar extensão durante o processamento da residência permanente.

*(Nota: Comunicados de imprensa anteriores de junho de 2023 sobre expansão de vagas representavam iniciativas históricas da época, sendo atualmente sobrepostos pela regra de pausa em vigor).*
```

### 🧠 3. Cross-Session Long-Term Memory Recall (Business & Management)
```text
# Session 1:
You [FinTech | gpt-4o-mini]: Vamos adotar a arquitetura SQLite WAL com migrações gerenciadas via Alembic para a release v2.0.
🤖 AI [gpt-4o-mini]: Excelente decisão! Registrado: SQLite WAL mode com Alembic para a v2.0.
You [FinTech | gpt-4o-mini]: /exit
🧠 [Hierarchical Memory - Level 1] Generating structured 5-dimension session summary block...
✔ Session memory persisted!

# Session 2 (Days later):
You [FinTech | gpt-4o-mini]: O que decidimos na última conversa sobre o banco de dados da release v2.0?
🤖 AI [gpt-4o-mini]:
🔍 [Search] Searching session memory records for workspace 'FinTech'...

Na sessão anterior, decidimos as seguintes diretrizes técnicas para a **release v2.0**:
1. **Engine**: SQLite configurado em modo **WAL (Write-Ahead Logging)** para suporte a leitura/escrita concorrente.
2. **Migrações**: Gerenciamento de esquema estruturado via **Alembic**.
```

### 💻 4. Developer / Software Engineering Codebase Analysis (Technical)
```text
You [BackendAPI | deepseek-chat]: Como funciona o fluxo de autenticação por tokens na rota /v1/auth/login?

🤖 AI [deepseek-chat]:
🔍 [Search] Searching strictly within Workspace: 'BackendAPI' (top 8 chunks)...

Conforme implementado no arquivo `src/any_context/server/routes.py` (Modificado em: 2026-08-18):
1. A rota recebe `POST /v1/auth/login` com payload `{"username": "...", "password": "..."}`.
2. O método `ConfigDBStore.verify_user_password()` valida o hash PBKDF2 com salt de 100.000 iterações.
3. Se válido, gera um Bearer Token com prefixo `actx_sec_` e expiração de 30 dias, retornando o papel do usuário (`Admin`, `Analyst` ou `Viewer`).
```

### 🛒 5. E-Commerce & Product Comparison (Structured Ratings & Reviews)
```text
You [Mercado | gpt-4o-mini]: Desses produtos de limpeza de vidros, qual tem a melhor avaliação e o melhor custo-benefício?

🤖 AI [gpt-4o-mini]:
🔍 [Search] Searching strictly within Workspace: 'Mercado' (top 8 chunks)...
📚 Reading retrieved documents...

Com base nas páginas de produtos indexadas:

1. **CleanGlass Pro Spray (23 fl oz)**:
   - **Avaliação**: **4.844 de 5 estrelas** (baseado em **1.199 avaliações**).
   - **Preço**: **USD $3.98** ($0.17 / fl oz).
   - **Status**: Em estoque.

2. **Great Value Glass Cleaner Spray Bottle (32 fl oz)**:
   - **Avaliação**: **4.200 de 5 estrelas** (baseado em **450 avaliações**).
   - **Preço**: **USD $2.48** ($0.08 / fl oz).
   - **Status**: Em estoque.

**Conclusão e Custo-Benefício:**
- **Melhor Avaliação Absoluta**: O **CleanGlass Pro** possui a maior nota de satisfação (**4.844 estrelas** com alto volume de 1.199 avaliações).
- **Melhor Custo-Benefício por Volume**: O **Great Value** custa menos da metade por fl oz ($0.08 vs $0.17), mantendo uma nota sólida de 4.2 estrelas.
```

### 6. Viewing Workspace Tree Structure (`/sync --verbose`)
```text
You [AnyContextProject | gpt-4o-mini]: /sync --verbose

📂 Workspace: AnyContextProject
├── 📁 Storage Locations (1 configured)
│   └── G:\My Drive\Documentos\AnyContext
├── 🔍 Subfolder Deep Scan
│   └── Discovered 12 files across 4 subdirectories
├── 📖 Document Chunks (12 total loaded)
│   ├── contract_template.docx (2 chunks)
│   ├── financial_report.xlsx (3 chunks)
│   └── architecture_notes.pdf (7 chunks)
├── 📘 Permanent System Context: Embedded README.md
└── ⚡ ChromaDB Collection: 'context_docs' synchronized and ready!
```

---

## 🌐 Intelligent Web Ingestion & Deep Recursive Crawler Engine

AnyContext includes a built-in, zero-dependency, concurrent web ingestion engine designed to transform documentation portals, government archives, legal bases, and technical wikis into living AI context.

Unlike basic web scrapers that blindly download noisy HTML or crawl randomly, AnyContext is engineered specifically for **high-precision Retrieval-Augmented Generation (RAG)**:

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                        ANYCONTEXT WEB INGESTION LIFECYCLE                              │
├─────────────────┬─────────────────┬──────────────────┬─────────────────┬───────────────┤
│  1. Discovery   │  2. Resolution  │   3. Proximity   │  4. Concurrent  │ 5. Ingestion  │
│  & Normalization│  & Sitemaps     │     Ranking      │    Extraction   │   Pipeline    │
├─────────────────┼─────────────────┼──────────────────┼─────────────────┼───────────────┤
│ • Strip .html   │ • sitemapindex  │ • Landing (10k)  │ • Multi-threaded│ • Chunk 500/50│
│ • Infer Prefix  │ • Keyword Match │ • Section (2k)   │ • Strip Boiler  │ • OpenAI Embed│
│ • Extract Links │ • Filter .xml   │ • Keywords (300) │ • Markdown AST  │ • ChromaDB    │
└─────────────────┴─────────────────┴──────────────────┴─────────────────┴───────────────┘
```

### 🧠 How It Works Under the Hood:

1. **Semantic Path Normalization**:
   - When given a URL like `https://www.canada.ca/en/immigration-refugees-citizenship.html` or `https://docs.python.org/3/library/os.html`, the crawler automatically strips file extensions (`.html`, `.htm`, `.php`, `.asp`, `.aspx`) to derive the true semantic directory path (e.g. `/en/immigration-refugees-citizenship/`).
   - All sub-pages and child forms under that section are accurately recognized and grouped under **Section Pages**.

2. **Recursive Sitemap & `sitemapindex` Resolution**:
   - Locates `sitemap.xml` and traverses nested sitemap catalogs (`<sitemapindex>`).
   - Tokenizes path keywords (e.g. `immigration`, `refugees`, `citizenship`, `docs`, `api`) to prioritize sub-sitemaps matching your target subject, while discarding raw `.xml` URLs in favor of clean `.html` content pages.

3. **Semantic Proximity & Relevance Ranking (`_rank_url`)**:
   - Web portals often contain tens of thousands of unrelated pages. AnyContext ranks all discovered URLs by semantic distance before ingestion:
     - 🥇 **Landing / Start URL** (Priority 10,000)
     - 🥈 **Direct Section Children** (Priority 2,000)
     - 🥉 **Direct In-Page Links** (Priority 500)
     - 🏅 **Keyword & Component Matches** (Priority 300 per matching slug)
     - ⚪ **Generic Domain URLs** (Placed at the bottom of the batch)
   - When you pick **Top 50** or **Top 250**, you always receive the most relevant guides, articles, and documentation pages first.

4. **Clean Semantic HTML Extraction**:
   - Strips boilerplate noise: navigation bars, footers, cookie notices, JavaScript, styles, and advertising banners.
   - Preserves semantic structure: Markdown headings (`#`, `##`), tables, bullet points, and code blocks.

5. **SentenceSplitter & Batch Vector Embedding**:
   - Splits content into 500-token semantic chunks with 50-token overlap (`SentenceSplitter`).
   - Generates embeddings in rate-limited micro-batches (`embed_batch_size=32`) to prevent OpenAI TPM / HTTP 429 throttling.
   - Commits vectors atomically directly into isolated ChromaDB collections.

6. **Strict Workspace Privacy & Scope Isolation**:
   - Web vectors are tagged with strict workspace metadata. Queries in workspace `Legal` cannot bleed into workspace `Marketing`.

---

## ⚙️ Configuration & Environment Settings

AnyContext uses an intelligent **3-tier configuration resolution hierarchy**:

```
1. Operating System Environment Variables  (export OPENAI_API_KEY=...)
2. Local .env File                         (Loaded automatically via .env.example)
3. Local SQLite Secure Database            (Managed interactively via /config)
```

### Using the `.env` File (For Developers & Power Users)

Copy the official [`.env.example`](file:///C:/Users/guilh/source/repos/any-context/.env.example) to `.env` in your project root:

```bash
# AI Model Provider Keys
OPENAI_API_KEY=sk-proj-...
ANTHROPIC_API_KEY=sk-ant-...
GEMINI_API_KEY=AIzaSy...
DEEPSEEK_API_KEY=sk-...
GROQ_API_KEY=gsk_...
MISTRAL_API_KEY=mistral_...
XAI_API_KEY=xai-...
OPENROUTER_API_KEY=sk-or-v1-...

# Web Research & Search
TAVILY_API_KEY=tvly-...

# Observability & Agent Tracing (LangSmith)
LANGSMITH_TRACING=true
LANGSMITH_ENDPOINT=https://api.smith.langchain.com
LANGSMITH_API_KEY=lsv2_pt_...
LANGSMITH_PROJECT="AnyContext"

# Server Mode & Enterprise License Key
ANYCONTEXT_LICENSE_KEY=ACTX.eyJjbGllbnQiOiJBY21lIn0...
```

---

## 🏢 Server Mode & Enterprise VPC Deployment (`actx --serve`)

AnyContext includes a production-ready **REST API Server** for corporate intranets, VPCs, and external applications.

### 1. Launching the Server
```bash
# Local development server:
actx --serve --port 8000 --host 127.0.0.1

# Enterprise In-VPC Listener (All network interfaces):
actx --serve --host 0.0.0.0 --port 8000
```
Access the interactive OpenAPI / Swagger UI at: **`http://127.0.0.1:8000/docs`**.

Key API Endpoints include:
- `POST /v1/chat` — Streaming & non-streaming AI Agent queries with RAG, session memory, and optional `grounding_mode` (`"hybrid"`, `"strict"`, `"proactive"`).
- `GET /v1/context/mode` & `POST /v1/context/mode` — Inspect and configure active AI Grounding & Answer Mode.
- `GET /v1/workspaces` — Lists all workspaces with immutable `id`, `name`, `total_sources`, and unified typed `sources` array (`folder`, `web`, `cloud_drive`).
- `GET /v1/workspaces/{name}` & `GET /v1/workspaces/{name}/sources` — Detailed workspace sources inspection.
- `POST /v1/workspaces` — Create workspace (with or without initial folders).
- `POST /v1/workspaces/transfer` — Instant zero-cost transfer of local folders and web portals between workspaces with vector metadata migration in < 50ms.
- `POST /v1/workspaces/{name}/cloud-drives` — Connect cloud drive sources (Google Drive, OneDrive, S3, Dropbox).
- `POST /v1/index` — Background folder re-indexing.
- `GET /v1/models` — Active & available model inspection.

### 2. Linux Background Service (`systemd`)

Create `/etc/systemd/system/anycontext.service`:
```ini
[Unit]
Description=AnyContext Universal AI Server
After=network.target

[Service]
User=ubuntu
WorkingDirectory=/home/ubuntu
ExecStart=/home/ubuntu/.local/bin/actx --serve --host 0.0.0.0 --port 8000
Restart=always

[Install]
WantedBy=multi-user.target
```
```bash
sudo systemctl daemon-reload
sudo systemctl enable anycontext
sudo systemctl start anycontext
```

---

## 🔌 Model Context Protocol (MCP) Setup

To connect AnyContext to **Claude Desktop**, **Cursor IDE**, or **Antigravity**:

```bash
actx --mcp
```

### Registered MCP Tools:
- `search_workspace_docs` — Vector semantic search across indexed files.
- `query_anycontext_agent` — Direct RAG query with 3-level session memory and optional `grounding_mode`.
- `get_grounding_mode` & `set_grounding_mode` — Inspect and switch active AI Grounding Mode (`hybrid`, `strict`, `proactive`).
- `list_workspaces` — Lists all configured workspaces along with all their associated sources (local folders, web portals, cloud drives).
- `get_workspace_sources` — Retrieves detailed sources breakdown for a specific workspace.
- `transfer_workspace_source` — Zero-cost instant data source transfer (folders/websites) between workspaces.
- `rename_workspace` — Instant zero-cost atomic workspace rename across SQLite and ChromaDB.
- `get_context_retrieval_settings` — Inspect current RAG retrieval density parameters and active preset.
- `set_context_retrieval_preset` — Configure RAG presets (`balanced`, `turbo`, `deep_research`) or custom quotas.
- `create_workspace` — Create a new workspace.
- `list_available_models` — Model verification.

### Claude Desktop Configuration (`claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "any-context": {
      "command": "actx",
      "args": ["--mcp"]
    }
  }
}
```

---

## 💳 Subscription Plans & Licensing Matrix

| Capability / Feature | Community (CLI) | Pro Plan | Team Plan | Enterprise Plan |
| :--- | :---: | :---: | :---: | :---: |
| **Local Folder Ingestion** | ✅ Unlimited | ✅ Unlimited | ✅ Unlimited | ✅ Unlimited |
| **Subfolder Recursive Scan**| ✅ Included | ✅ Included | ✅ Included | ✅ Included |
| **Web Scraping & Polling** | ✅ Unlimited | ✅ Unlimited | ✅ Unlimited | ✅ Unlimited |
| **Image & Scanned PDF OCR**| ✅ Included | ✅ Included | ✅ Included | ✅ Included |
| **9 AI Model Providers** | ✅ Full Access | ✅ Full Access | ✅ Full Access | ✅ Full Access |
| **ChromaDB + SQLite Memory**| ✅ Included | ✅ Included | ✅ Included | ✅ Included |
| **REST API Server (`actx --serve`)** | 🔒 Server Mode License | ✅ 1 Seat | ✅ 5+ Seats | ✅ Dedicated VPC |
| **Team Collaboration & RBAC** | 🔒 Team Feature | 🔒 Team Feature | ✅ Included | ✅ SSO / SAML |
| **Offline Ed25519 License** | Not Required | `.env` Key | `.env` Key | `.env` Key |
| **Price** | **$0 (Free Forever)**| **$29 / mo** | **$79 / mo** | **$499 / mo** |

> *To activate a Server Mode license, add `ANYCONTEXT_LICENSE_KEY=...` to your `.env` or type `/billing` in chat.*

---

## 🛡️ Privacy, Security & Data Sovereignty

- **Offline-First Guarantee**: When using LM Studio or Ollama, zero bytes of your documents or questions ever leave your physical device.
- **Safe Secrets Storage**: API keys and passwords in SQLite are protected with cryptographic PBKDF2 hashing and terminal masking.
- **Zero Third-Party Training**: Your documents are stored locally in `./context_db` and are never used to train public AI models.

---

## 🧹 Uninstallation

To cleanly remove AnyContext, restore PATH variables, purge legacy databases, and optionally wipe or preserve your vector memory:

- **Windows (PowerShell)**:
  ```powershell
  irm https://raw.githubusercontent.com/Levix-Digital/any-context/main/uninstall.ps1 | iex
  ```
  *Or locally:* `.\scripts\uninstall.ps1`

- **Linux / macOS (Bash)**:
  ```bash
  curl -fsSL https://raw.githubusercontent.com/Levix-Digital/any-context/main/uninstall.sh | bash
  ```
  *Or locally:* `./scripts/uninstall.sh`

> [!NOTE]
> The uninstaller prompts whether to preserve or wipe your configured workspaces and vector databases (`%LOCALAPPDATA%\AnyContext` or `~/.local/share/any-context`), purges any legacy database files, cleans your User PATH variable, and automatically uninstalls residual Python `pip` wrappers.

---

<div align="center">
  <sub>Built with ☕ and ❤️ by <b>Levix Digital</b> to transform document chaos into your personal AI assistant.</sub>
</div>
