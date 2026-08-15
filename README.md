# 🧠 AnyContext (`actx`)

> **Transform any file, drive, folder, or website into a living, real-time AI context.**

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

**AnyContext** is the universal intelligence layer for your personal and professional data. Built with an uncompromising focus on **privacy, speed, and versatility**, AnyContext connects your local folders, scanned documents, and web portals directly to the world's most capable Artificial Intelligence models.

Whether you are a **lawyer reviewing hundreds of contract pages**, a **consultant managing immigration dossiers**, a **researcher navigating scientific publications**, or an **enterprise deploying a secure in-VPC context server**, AnyContext operates **100% on your terms and infrastructure**.

---

## 🌟 Why AnyContext? (In Plain English)

Traditional AI tools require you to manually copy and paste files into web chats, exposing your confidential documents to external clouds and forgetting past conversations the moment you close the tab.

**AnyContext changes everything:**

1. **You Point to Your Folders**: Point AnyContext to any folder on your computer (PDFs, Word docs, Excel spreadsheets, images, text files, or source code).
2. **Instant Local Memory**: AnyContext reads and organizes your documents locally into an ultra-fast vector database (ChromaDB).
3. **Ask Anything Naturally**: Open the terminal and ask questions in plain English or Portuguese (*"What is the renewal deadline in contract 42?"*, *"Compare the tax policies across our 2025 financial reports"*).
4. **Cites Real Sources**: The AI answers accurately, quoting the exact file name, page, and chunk where the information was found.
5. **Zero Cloud Lock-In**: Choose from **9 leading AI providers** (OpenAI, Claude, Gemini, DeepSeek, Groq, Mistral, xAI Grok, OpenRouter) or run **100% offline and free** using local models (LM Studio / Ollama).

---

## 🚀 Key Features & Superpowers

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
- **⚡ Sub-3ms Instant Startup & Clean Single-Line Synchronization**:
  - Signature ASCII banner renders in under 3 milliseconds.
  - Clean single-line background sync spinner (`✔ Workspace 'AnyContext' ready`).
  - Modern hierarchical tree view for verbose inspection (`/sync --verbose` or `/index -v`).
- **🎯 Typo-Resilient Slash Command Interception**:
  - Mistyped commands (e.g. `/check-updaete`, `/swich`, `/modeel`, `/sinc`) are caught by the intelligent fuzzy matcher and suggest the correct command without wasting AI tokens or running unnecessary vector searches.
- **🔄 Interactive 1-Click Self-Updater (`/update` & `/check-update`)**:
  - Detects GitHub releases with cache-busting and prompts: `? Would you like to download and install vX.Y.Z now? [Y/n]`.
  - Performs atomic self-replacement, even on locked Windows binaries.
- **🌐 REST API Server Mode (`actx --serve`)**:
  - High-performance FastAPI server for external web dashboards, mobile apps, and VS Code extensions.
  - Interactive Swagger UI documentation at `http://127.0.0.1:8000/docs`.
  - Protected with Ed25519 cryptographic licensing configured via `ANYCONTEXT_LICENSE_KEY` in `.env`.
- **🔌 Model Context Protocol (MCP) Server (`actx --mcp`)**:
  - Native JSON-RPC stdio implementation of Anthropic's MCP specification for **Claude Desktop**, **Cursor IDE**, and **Antigravity**.
- **🧠 3-Level Hierarchical Long-Term Memory**:
  - **Level 1 (Session Block Summary)**: Summarizes conversation blocks and persists them to long-term memory.
  - **Level 2 (Active Rolling Window)**: Keeps recent messages in SQLite graph state for fast response times.
  - **Level 3 (Consolidated Meta-Summarization)**: Consolidates older memory vectors, keeping your context lean and relevant.
- **📊 Observability & Agent Tracing (LangSmith)**:
  - Native, zero-overhead telemetry for power users and enterprise teams to audit token costs, latency, tool calls, and RAG retrieval chunks.
- **📘 Permanent Self-Help System**:
  - The AI agent has full access to this documentation. You can ask in chat: *"Como eu adiciono um site ao meu workspace?"* or *"Quais comandos estão disponíveis?"*.

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

| Command | Aliases | Description |
| :--- | :--- | :--- |
| **`/switch`** | `-w`, `--workspace` | Open interactive menu to switch active workspace scope. |
| **`/sync`** | `/index`, `-s` | Synchronize workspace files incrementally (single-line clean mode). |
| **`/sync -v`** | `/index --verbose` | Synchronize workspace with detailed modern tree structure view. |
| **`/model`** | `/m`, `models` | Open key-aware AI model selector across 9 providers. |
| **`@model <msg>`** | — | One-shot prompt to a specific model without changing session defaults. |
| **`/api-keys`** | `/keys`, `providers`| Step-by-step guide with portal links to obtain API keys. |
| **`/web`** | `scrape`, `urls` | Open interactive web sources management menu. |
| **`/web add <url>`**| — | Ingest a website URL immediately into the active workspace. |
| **`/web list`** | — | List all registered web URLs and polling status. |
| **`/web sync`** | — | Force re-scrape and synchronize all web URLs in workspace. |
| **`/ocr`** | `image`, `scan` | View Image & Scanned PDF OCR parsing status. |
| **`/config`** | `-c`, `--config` | Open interactive settings menu (Workspaces, AI Models, API Keys). |
| **`/billing`** | `/plans`, `pricing` | View subscription tiers, capabilities, and license status. |
| **`/check-update`**| `--check-update`| Check for newer releases with 1-click upgrade confirmation. |
| **`/update`** | `--update` | Download and apply the latest AnyContext release immediately. |
| **`/reset-memory`**| `/reset` | Purge conversation session memory for the active workspace. |
| **`/factory-reset`**| `--factory-reset`| Reset all settings, workspaces, API keys, and databases to defaults. |
| **`/help [cmd]`** | `/h`, `help` | Open the interactive manual index or get help for a specific command. |
| **`Ctrl+C`** | — | Interrupt AI generation immediately or prompt graceful exit. |

---

## 💡 Real-World Usage Examples

### 1. Asking Questions with Document RAG
```text
You [LegalDocs | gpt-4o-mini]: Qual é o prazo de vigência e as cláusulas de rescisão do Contrato de Prestação de Serviços da Acme?

🤖 AI [gpt-4o-mini]:
🔍 [Search] Searching strictly within Workspace: 'LegalDocs' (retrieving top 8 chunks)...
📚 Reading retrieved documents...

De acordo com a Cláusula 8.1 do arquivo 'Contrato_Acme_2025.pdf':
- O prazo de vigência é de 24 (vinte e quatro) meses a partir da data de assinatura (15/01/2025).
- A rescisão imotivada exige aviso prévio formal por escrito de no mínimo 30 dias (Cláusula 8.3).
```

### 2. One-Shot Prompt with Another Model (`@model`)
```text
You [LegalDocs | gpt-4o-mini]: @claude-haiku-4-5-20251001 Faça um resumo executivo deste contrato em 3 tópicos para envio por e-mail.

🤖 AI [claude-haiku-4-5-20251001]:
Aqui está o resumo executivo:
1. Objeto: Prestação de serviços de consultoria técnica especializada.
2. Valor: R$ 45.000,00 divididos em 3 parcelas iguais.
3. Vigência: 24 meses com rescisão mediante aviso prévio de 30 dias.
```

### 3. Interactive Web Discovery & Deep Site Crawling (`/web add`)
```text
You [Immigration | gpt-4o-mini]: /web add https://www.canada.ca/en/immigration-refugees-citizenship.html

⠋ [Discovery] Mapping site structure, internal links & sitemaps for 'canada.ca'...

================================================================================
🌐 Website Discovery Report: Immigration, Refugees and Citizenship Canada
🔗 https://www.canada.ca/en/immigration-refugees-citizenship.html
================================================================================
  • 📄 Section Pages (matching path prefix) : 142 pages
  • 🌐 Total Internal Domain URLs Found    : 1,580 pages
  • 🗺️ XML Sitemap Detected                : Yes (Structured XML)
================================================================================

? Select indexing scope for workspace 'Immigration':
  ❯ 1. 📄 Current Section Only (142 pages) [Recommended]
    2. ⚡ Fast Crawl Limit (Top 50 pages) ~ 5s
    3. 🚀 Deep Crawl Limit (Top 250 pages) ~ 20s
    4. 📦 Extensive Crawl Limit (Top 500 pages) ~ 45s
    5. 🌐 Entire Discovered Domain (1,580 pages)
    6. 📄 Single Start Page Only (1 page) ~ 1s

⠸ [Web Crawler] [████████████████████████] 142/142 (100%) • 142 indexed
✔ Successfully ingested and indexed 142 web pages (845,210 chars) into workspace 'Immigration'!

You [Immigration | gpt-4o-mini]: Quem é elegível para aplicar para um Open Work Permit no Canadá?
🤖 AI [gpt-4o-mini]: De acordo com a documentação do IRCC indexada...
```

### 4. Viewing Workspace Tree Structure (`/sync --verbose`)
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

To cleanly remove AnyContext and remove PATH variables:

- **Windows (PowerShell)**:
  ```powershell
  irm https://raw.githubusercontent.com/Levix-Digital/any-context/main/uninstall.ps1 | iex
  ```
- **Linux / macOS (Bash)**:
  ```bash
  curl -fsSL https://raw.githubusercontent.com/Levix-Digital/any-context/main/uninstall.sh | bash
  ```

---

<div align="center">
  <sub>Built with ☕ and ❤️ by <b>Levix Digital</b> to transform document chaos into your personal AI assistant.</sub>
</div>
