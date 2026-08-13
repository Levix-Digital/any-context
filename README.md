# 🧠 AnyContext (`actx`)

> **Transform any file, drive, or folder into a living, real-time AI context.**

**AnyContext** is the ultimate bridge between your local data and Artificial Intelligence. Developed with an absolute focus on **privacy, modularity, and efficiency**, AnyContext is a smart, autonomous Local AI Engine equipped with **3-Level Hierarchical Long-Term Memory**, a **High-Performance REST API Server**, and a **Model Context Protocol (MCP) Server**.

Whether you are a developer seeking deep codebase insights, a business analyzing confidential reports, or an enterprise deploying a private RAG context layer in your VPC, AnyContext operates **100% on your infrastructure**, ensuring your data never feeds third-party models without explicit permission.

---

## 🚀 Key Features

- **🔒 Absolute Privacy (Offline-First):** Natively integrated with [LM Studio](https://lmstudio.ai/) and local LLMs (Gemma, Llama, Qwen, etc.) or OpenAI-compatible endpoints. Your files, business strategies, and code stay exclusively on your hardware.
- **🌐 REST API Server (`actx --serve`):** Exposes high-performance HTTP endpoints for external web dashboards, VS Code extensions, mobile backends, and automation workflows. Features interactive Swagger UI at `http://127.0.0.1:8000/docs`.
- **🔌 Model Context Protocol (MCP) Server (`actx --mcp`):** Native JSON-RPC stdio implementation of Anthropic's MCP specification, allowing **Claude Desktop**, **Cursor IDE**, and **Antigravity** to query your local knowledge base seamlessly.
- **🏢 Enterprise VPC Ready (`--host 0.0.0.0`):** Simple 1-command deployment for private cloud (AWS, GCP, Azure, On-Premise) serving entire corporate networks via internal VPN/VPC.
- **📂 Multi-Workspace & Granular Folder Management:** Group multiple directories into isolated "Workspaces". Add, view, or remove individual folder paths per workspace dynamically.
- **⚡ Ultra-Fast Incremental Synchronization:** Automatically tracks document SHA-256 hashes and modification timestamps: only indexes new or altered files, and purges deleted disk files from ChromaDB.
- **🧠 3-Level Hierarchical Memory Compression:**
  - **Level 1 (Session Block Summary):** Asynchronously summarizes chat interaction blocks (every 10 interactions / 20 messages) and persists them to long-term vector storage.
  - **Level 2 (Active Rolling Window):** Retains recent active messages in SQLite graph state for fast, lightweight LLM context windows.
  - **Level 3 (Consolidated Meta-Summarization):** Automatically merges older session summaries into high-level Meta-Summaries when ChromaDB reaches user thresholds, keeping vector indices lean and sharp.
- **📘 Permanent System Self-Help Context:** Automatically embeds AnyContext's own complete documentation (`README.md`) into the vector database for all workspaces. Ask the AI agent how to deploy, configure, update, or useAnyContext directly in chat!
- **⚙️ SQLite Configuration Store (`settings.db`):** Thread-safe, ACID-compliant SQLite configuration store (`ConfigDBStore`) featuring automatic background migration from legacy `settings.json` and secure API Key storage with password masking (`sk-...****`).

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
├── ingestion/                # Incremental RAG Ingestion Pipeline
│   └── local_folder_ingestor.py # Recursive folder scanner & ChromaDB updater
├── memory/                   # Standalone 3-Level Hierarchical Memory Engine
│   ├── models.py             # Memory schemas (SHORT_TERM, SESSION_SUMMARY, META_SUMMARY)
│   ├── store.py              # ChromaDB memory vector store wrapper
│   ├── compressor.py         # LLM-powered Level-1 & Level-3 summarization engine
│   └── manager.py            # Asynchronous memory background thread orchestrator
├── server/                   # External Integration Layer (REST & MCP)
│   ├── api.py                # FastAPI REST API Server & Swagger endpoints
│   └── mcp.py                # Model Context Protocol (MCP) stdio JSON-RPC server
└── tools/                    # Agent Dynamic Tools
    └── search_tools.py       # ChromaDB vector retriever tool (search_db)
```

---

## ⚡ Quick Start & Installation

### Option 1: Automatic Terminal Installer Script (No Python Needed!)

1. Download the installer script from the **[Latest Release](https://github.com/Levix-Digital/any-context/releases/latest)**:
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

## 🏢 Enterprise & VPC Deployment Guide (Private Cloud / On-Premise)

AnyContext is designed for effortless deployment inside an enterprise **Virtual Private Cloud (VPC)**, **AWS EC2**, **Google Cloud Compute Engine**, **Azure VM**, or **On-Premise Private Server**.

### 1. Launching in In-VPC Listener Mode
To allow internal company applications (ERPs, CRMs, Intranets, Slack/Teams Bots, VS Code Extensions) to query AnyContext across your private network/VPN:
```bash
actx --serve --host 0.0.0.0 --port 8000
```
> **Note:** Binding to `--host 0.0.0.0` enables listening on all internal network interfaces within your VPC.

### 2. Linux Background Service (`systemd`)
To ensure AnyContext runs continuously as a background service on your VPC Linux instance and restarts automatically on server reboots:

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

Enable and start the service:
```bash
sudo systemctl daemon-reload
sudo systemctl enable anycontext
sudo systemctl start anycontext
```

### 3. Enterprise Department Routing (Multi-Workspace)
External enterprise applications specify the target department using the `"workspace"` JSON field in REST requests:
```json
{
  "message": "What is the policy for business travel expense reimbursement?",
  "workspace": "HumanResources"
}
```
Supported department workspaces example: `"HumanResources"`, `"Finance"`, `"Legal"`, `"Engineering"`.

### 4. 100% Private In-VPC Data & LLM Pipeline
For strict SOC2 / LGPD compliance where zero data may leave the enterprise network:
- Pair AnyContext with an in-VPC local LLM server (e.g. **Ollama**, **vLLM**, or **Azure OpenAI Private Endpoint**).
- Configure the Base URL via `actx --config` to point to `http://internal-llm-server:11434/v1`.
- Documents, vector embeddings (ChromaDB), and memory persist strictly on your VPC storage.

---

## 🌐 REST API Endpoints Specification

| Method | Endpoint | Description |
| :--- | :--- | :--- |
| `GET` | `/v1/health` | Health check, version, and server info. |
| `GET` | `/v1/workspaces` | List all configured workspaces and their associated folder paths. |
| `GET` | `/v1/docs/readme` | Retrieve raw application documentation (README.md) as JSON markdown. |
| `POST` | `/v1/chat` | Send a message to the AI agent with RAG search & session memory. |

| `POST` | `/v1/search` | Perform raw vector search across workspace knowledge bases. |
| `POST` | `/v1/index` | Trigger background re-indexing for a specific or all workspaces. |
| `POST` | `/v1/reset-memory` | Purge long-term vector memory for a workspace or globally. |
| `POST` | `/v1/factory-reset` | Wipe all settings, API keys, workspaces, and vector databases (Factory Reset). |

### API Usage Examples (`curl`)

#### 1. Chat with Agent (`POST /v1/chat`)
```bash
curl -X POST "http://127.0.0.1:8000/v1/chat" \
     -H "Content-Type: application/json" \
     -d '{
           "message": "What were the security requirements discussed in the project specs?",
           "workspace": "MyProject"
         }'
```

#### 2. Search Knowledge Base (`POST /v1/search`)
```bash
curl -X POST "http://127.0.0.1:8000/v1/search" \
     -H "Content-Type: application/json" \
     -d '{
           "query": "authentication bearer tokens",
           "workspace": "MyProject"
         }'
```

#### 3. Trigger Workspace Re-indexing (`POST /v1/index`)
```bash
curl -X POST "http://127.0.0.1:8000/v1/index" \
     -H "Content-Type: application/json" \
     -d '{ "workspace": "MyProject" }'
```

---

## 💬 In-App Slash Commands (During Chat)

- **`/switch`**: Interactively switch active workspace with instant vector DB resync.
- **`/version`** (or **`/v`**): Display AnyContext version information.
- **`/update`**: Check for and install the latest release automatically.
- **`/check-update`**: Check if a newer version is available.
- **`/reset-memory`** (or **`/reset`**): Purge long-term vector memories for the active workspace.
- **`/factory-reset`**: Wipe all workspaces, API keys, settings, and vector databases (Factory Reset).
- **`/config`**: Open the interactive configuration menu (Workspaces, AI Models, API Keys).
- **`/help`**: Display detailed in-app command instructions and tips.
- **`Ctrl+C`**: Gracefully exit while triggering a background memory summary.


---

## ⚙️ Configuration & API Key Management

AnyContext stores configurations and API keys securely in `config/settings.db` (SQLite). Manage settings interactively using `actx --config` or `/config` during chat:

- **🔑 Secure API Key Storage**: Input keys with password masking (`sk-...****`). Supported providers: OpenAI, OpenRouter, Anthropic, Gemini, DeepSeek, Groq.
- **📂 Workspace & Folder Management**: Add, view, or remove individual document folders within any existing workspace.
- **⚡ 1-Click Provider Quick-Setup**:
  - *OpenAI Cloud Preset*: Enter key once; sets `gpt-4o-mini` + `text-embedding-3-small`.
  - *Local Offline Preset*: Auto-configures LM Studio or Ollama (`http://localhost:1234/v1`).
- **🧹 Automatic Embedding Vector Purge**: Changing embedding models automatically clears stale ChromaDB collections to prevent dimension mismatch errors.

---

## 🧹 Uninstallation

To completely uninstall AnyContext (`actx`) and clean PATH variables:

1. Download `uninstall.ps1` (Windows) or `uninstall.sh` (Linux / Git Bash) from **[Latest Release Assets](https://github.com/Levix-Digital/any-context/releases/latest)**.
2. Run in terminal:
   - **Windows (PowerShell)**:
     ```powershell
     .\uninstall.ps1
     ```
   - **Linux / Git Bash**:
     ```bash
     chmod +x uninstall.sh
     ./uninstall.sh
     ```

---

## 🔮 Roadmap

1. **Cloud Drive Ingestors (Google Drive, OneDrive, Dropbox)**
2. **Multi-Agent Orchestration (Sub-Agent Execution & Routing)**
3. **Web Dashboard & GUI Desktop Interface**
4. **Source Code AST & Deep Codebase Analysis Pipeline**

---

> **Built with ☕ and ❤️ by Levix Digital to transform document chaos into your personal AI assistant.**
