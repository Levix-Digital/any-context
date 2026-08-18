# 🧠 AnyContext (`actx`)

> **Transform any file, folder, website, or drive into a living, real-time AI context.**

**AnyContext** is the ultimate bridge between your local data and Artificial Intelligence. Developed with an absolute focus on **privacy, modularity, and efficiency**, AnyContext is a smart, autonomous Local AI Engine equipped with **3-Level Hierarchical Long-Term Memory**, a **High-Performance REST API Server**, and a **Model Context Protocol (MCP) Server**.

Whether you are a developer seeking deep codebase insights, a business analyzing confidential reports, or an enterprise deploying a private RAG context layer in your VPC, AnyContext operates **100% on your infrastructure**, ensuring your data never feeds third-party models without explicit permission.

---

## 🚀 Key Features

- **🔒 Absolute Privacy (Offline-First):** Natively integrated with [LM Studio](https://lmstudio.ai/) and local LLMs (Gemma, Llama, Qwen, etc.) or OpenAI-compatible endpoints. Your files, business strategies, and code stay exclusively on your hardware.
- **🌐 REST API Server (`actx --serve`):** Exposes high-performance HTTP endpoints for external web dashboards, VS Code extensions, mobile backends, and automation workflows. Features interactive Swagger UI at `http://127.0.0.1:8000/docs`.
- **🔌 Model Context Protocol (MCP) Server (`actx --mcp`):** Native JSON-RPC stdio implementation of Anthropic's MCP specification, allowing **Claude Desktop**, **Cursor IDE**, and **Antigravity** to query your local knowledge base seamlessly.
- **🏢 Enterprise VPC Ready (`--host 0.0.0.0`):** Simple 1-command deployment for private cloud (AWS, GCP, Azure, On-Premise) serving entire corporate networks via internal VPN/VPC.
- **📂 Multi-Workspace & Granular Folder Management:** Group multiple directories into isolated "Workspaces". Add, view, or remove individual folder paths per workspace dynamically.
- **🌐 Intelligent Web Ingestion & Deep Recursive Crawler (`/web add`):** Semantic path prefix normalization, smart recursive sitemapindex traversal, proximity & relevance ranking (`_rank_url`), clean Markdown extraction, SentenceSplitter chunking (500/50), and atomic ChromaDB batch vectorization.
- **⚡ Ultra-Fast Incremental Synchronization:** Automatically tracks document SHA-256 hashes and modification timestamps: only indexes new or altered files, and purges deleted disk files from ChromaDB.
- **🧠 3-Level Hierarchical Memory Compression:**
  - **Level 1 (Session Block Summary):** Asynchronously summarizes chat interaction blocks (every 10 interactions / 20 messages) and persists them to long-term vector storage.
  - **Level 2 (Active Rolling Window):** Retains recent active messages in SQLite graph state for fast, lightweight LLM context windows.
  - **Level 3 (Consolidated Meta-Summarization):** Automatically merges older session summaries into high-level Meta-Summaries when ChromaDB reaches user thresholds, keeping vector indices lean and sharp.
- **📘 Permanent System Self-Help Context:** Automatically embeds AnyContext's own complete documentation (`README.md`) into the vector database for all workspaces. Ask the AI agent how to deploy, configure, update, or use AnyContext directly in chat!
- **🔐 User Access Control & RBAC Authentication:** Zero-friction open mode for personal use. Dual-mode support for Enterprise/Teams with User Accounts, Roles (`Admin`, `Analyst`, `Viewer`), Bearer Tokens (`actx_sec_...`), and Workspace-level Access Scopes.
- **🤝 Google Drive-Style Workspace Collaboration:** Share existing workspaces with team members (`Viewer` or `Editor` roles). Transparent folder visibility across all collaborators with strict folder ownership locking (`[👑 Your Folder]` vs `[🔒 Read-Only]`).
- **⚙️ SQLite Configuration Store (`settings.db`):** Thread-safe, ACID-compliant SQLite configuration store (`ConfigDBStore`) serving as the single source of truth for all settings, workspaces, RBAC users, tokens, and encrypted API Key storage with password masking (`sk-...****`).
- **🔄 Auto-Updater (`actx --update` / `/update`):** Non-blocking startup release notification, manual check (`actx --check-update`), and 1-click self-updater supporting locked Windows executables and private GitHub repositories.

---

## 🏗️ Project Architecture

```text
src/any_context/
├── cli/                      # Terminal User Interface & Command Handling
│   ├── banner.py             # Signature ASCII Art splash screen & branding
│   ├── chat_loop.py          # Interactive chat loop & slash command intercepter
│   ├── config_menu.py        # Interactive configuration menu & onboarding wizard
│   ├── updater.py            # Self-update manager & release checker
│   └── workspace_selector.py # Workspace selection & CLI argument parser
├── config/                   # Persistent SQLite Configuration System
│   ├── app_settings.py       # Pydantic schemas & settings loader
│   └── db_store.py           # SQLite ConfigDBStore manager
├── core/                     # LangGraph Orchestration Engine
│   ├── agent.py              # Agent graph definition & tool binding
│   └── utils.py              # API key resolvers & prompt finders
├── help/                     # Architectural Help & Documentation Module
│   ├── manager.py            # Flags (--help, -h), /help, & interactive manual
│   ├── models.py             # HelpPage schema
│   └── registry.py           # Comprehensive command manuals
├── ingestion/                # Incremental RAG Ingestion Pipeline
│   ├── local_folder_ingestor.py # Recursive folder scanner & ChromaDB updater
│   ├── web_crawler.py        # Interactive site discovery, proximity ranker & crawler
│   ├── web_ingestor.py       # HTML text extraction & Markdown AST parsing
│   └── web_scheduler.py      # Background recurring scheduler & SQLite web sources store
├── memory/                   # Standalone 3-Level Hierarchical Memory Engine
│   ├── models.py             # Memory schemas (SHORT_TERM, SESSION_SUMMARY, META_SUMMARY)
│   ├── store.py              # ChromaDB memory vector store wrapper
│   ├── compressor.py         # LLM-powered Level-1 & Level-3 summarization engine
│   └── manager.py            # Asynchronous memory background thread orchestrator
├── server/                   # External Integration Layer (REST & MCP)
│   ├── api.py                # FastAPI REST API Server & Swagger endpoints
│   └── mcp.py                # Model Context Protocol (MCP) stdio JSON-RPC server
├── workspace_sharing/        # Workspace Collaboration & Sharing Module
│   ├── manager.py            # Workspace permissions & transparent folder view
│   ├── models.py             # WorkspaceFolderEntry, WorkspacePermission, WorkspaceShareInvite
│   └── store.py              # SQLite tables (workspace_folders, workspace_user_permissions, workspace_share_invites)
└── tools/                    # Agent Dynamic Tools
    └── search_tools.py       # ChromaDB vector retriever tool (search_db)
```

---

## ⚡ Quick Start & Installation

### Option 1: Automatic Terminal Installer Script (No Python Needed!)

1. Download the installer script from the **[Latest Release](https://github.com/Levix-Digital/any-context-releases/releases/latest)**:
   - **Windows**: `install.ps1`
   - **Linux / Git Bash**: `install.sh`
2. Run the script in your terminal:
   - **Windows (PowerShell)**:
     ```powershell
     .\install.ps1
     ```
   - **Linux / Git Bash (Terminal)**:
     ```bash
     chmod +x install.sh
     ./install.sh
     ```
*The installer configures your User PATH environment variable, enabling `actx` globally.*

---

### Option 2: Install as a Python Package

```bash
git clone https://github.com/Levix-Digital/any-context.git
cd any-context
pip install -e .
```
*(Available command aliases: `actx`, `anycontext`, `any-context`, `ac`)*

---

## 💻 Operating Modes

AnyContext supports three distinct operating modes:

### Mode 1: Interactive Terminal Chat (`actx`)
Launch the interactive agent directly in your console:
```bash
actx
# Specify a workspace directly:
actx -w "MyProject"
# View version:
actx -v
```

### Mode 2: REST API Server (`actx --serve`)
Start the FastAPI REST Server to allow external web apps, VS Code extensions, or backend services to connect:
```bash
actx --serve --port 8000 --host 127.0.0.1
# or simply:
actx server
```
Access interactive OpenAPI / Swagger documentation at **`http://127.0.0.1:8000/docs`**.

### Mode 3: Model Context Protocol (MCP) Server (`actx --mcp`)
Start AnyContext as a standard MCP server communicating over stdio JSON-RPC 2.0:
```bash
actx --mcp
```

#### Configuring Claude Desktop (`claude_desktop_config.json`):
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

## 🤝 Workspace Collaboration & Sharing (Google Drive Style)

AnyContext allows workspace owners to share an existing workspace with team members:

- **👁️ Viewer Role**: Can query AI chat & search vector DB. Cannot add or delete folders.
- **✏️ Editor Role**: Can query AI chat & search vector DB + add their own local folders to the workspace.
- **📁 Transparent Folder Visibility & Ownership**: All collaborators see the complete list of folders feeding the AI context (`[👑 Your Folder]` vs `[🔒 Read-Only (Added by Amanda)]`). Edit and delete actions remain strictly locked to the folder's physical owner!

---

## 🌐 REST API Endpoints Specification

| Method | Endpoint | Description |
| :--- | :--- | :--- |
| `GET` | `/v1/health` | Health check, version, and server security status. |
| `GET` | `/v1/auth/status` | Check if Admin account is configured & security mode status. |
| `POST` | `/v1/auth/setup-admin` | Initial Administrator setup wizard (first-time deployment). |
| `POST` | `/v1/auth/login` | Authenticate user credentials and retrieve Bearer Access Token. |
| `GET` | `/v1/users` | List all team user accounts (Admin only). |
| `POST` | `/v1/users` | Create new team user with role and workspace scopes (Admin only). |
| `DELETE` | `/v1/users/{user_id}` | Revoke/delete a team user account (Admin only). |
| `GET` | `/v1/tokens` | List active Bearer security access tokens (Admin only). |
| `POST` | `/v1/tokens` | Generate new Bearer security access token (Admin only). |
| `DELETE` | `/v1/tokens/{token_id}` | Revoke a Bearer security access token (Admin only). |
| `POST` | `/v1/workspaces/share/invite` | Generate a workspace share invite code (`SHARE-WKS-XXXX`). |
| `POST` | `/v1/workspaces/share/accept` | Accept a workspace share invite code to join a workspace. |
| `GET` | `/v1/workspaces/{name}/collaborators` | List collaborators of a workspace. |
| `GET` | `/v1/workspaces/{name}/folders` | List transparent workspace folders with ownership tags. |
| `POST` | `/v1/workspaces/{name}/folders` | Add a new folder to a workspace (Editor permission required). |
| `DELETE` | `/v1/workspaces/{name}/folders/{id}` | Delete a workspace folder (Folder Owner permission required). |
| `GET` | `/v1/workspaces` | List all configured workspaces and associated folder paths. |
| `GET` | `/v1/docs/readme` | Retrieve raw application documentation (`README.md`) as JSON. |
| `POST` | `/v1/chat` | Send a message to the AI agent with RAG search & session memory. |
| `POST` | `/v1/search` | Perform raw vector search across workspace knowledge bases. |
| `POST` | `/v1/index` | Trigger background re-indexing for a specific or all workspaces. |
| `POST` | `/v1/reset-memory` | Purge long-term vector memory for a workspace or globally. |
| `POST` | `/v1/factory-reset` | Wipe all settings, API keys, users, workspaces, and databases (Factory Reset). |

---

## 🔮 Roadmap

1. **Cloud Drive Ingestors (Google Drive, OneDrive, Dropbox)**
2. **Multi-Agent Orchestration (Sub-Agent Execution & Routing)**
3. **Web Dashboard & GUI Desktop Interface**
4. **Source Code AST & Deep Codebase Analysis Pipeline**

---

> **Built with ☕ and ❤️ by Levix Digital to transform document chaos into your personal AI assistant.**
