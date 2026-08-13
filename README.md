# 🧠 AnyContext (`actx`)

> **Transform any file, drive, or folder into a living, real-time AI context.**

**AnyContext** is the ultimate bridge between your local data and Artificial Intelligence. Developed with an absolute focus on **privacy, modularity, and efficiency**, AnyContext is not just another RAG script; it is a smart, autonomous Local Agent equipped with **3-Level Hierarchical Long-Term Memory**, capable of instantly scanning your directories, learning from your documents, and providing accurate, well-founded answers.

Whether you are a developer seeking to understand a complex repository or a business dealing with thousands of confidential reports, AnyContext runs **100% on your machine**, ensuring your data never feeds third-party models without your permission.

---

## 🚀 Key Features

- **🔒 Absolute Privacy (Offline-First):** Natively integrated with [LM Studio](https://lmstudio.ai/) and local LLMs (Gemma, Llama, Qwen, etc.) or OpenAI-compatible endpoints. Your files, business strategies, and code stay exclusively on your hardware.
- **📂 Multi-Workspace Ingestion:** Group multiple directories into isolated "Workspaces". The AI filters vectors in real-time, keeping the context strictly focused on the requested topic.
- **⚡ Ultra-Fast Incremental Synchronization:** Automatically tracks document SHA-256 hashes and file modification timestamps: only indexes new or altered files, and purges deleted disk files from ChromaDB.
- **🧠 3-Level Hierarchical Memory Compression:**
  - **Level 1 (Session Block Summary):** Asynchronously summarizes chat interaction blocks (every 10 interactions / 20 messages) and persists them to long-term vector storage.
  - **Level 2 (Active Rolling Window):** Retains recent active messages in SQLite graph state for fast, lightweight LLM context windows.
  - **Level 3 (Consolidated Meta-Summarization):** Automatically merges older session summaries into high-level Meta-Summaries when ChromaDB reaches user thresholds, keeping vector indices lean, sharp, and non-redundant.
- **⚙️ SQLite Configuration Store (`settings.db`):** Replaces flat JSON files with a thread-safe, ACID-compliant SQLite configuration store (`ConfigDBStore`) featuring automatic background migration from legacy `settings.json`.
- **🧙 Interactive Onboarding Wizard & Configuration Menu:** First-run onboarding wizard for new installations and full CLI management via `actx --config` or `/config` during chat.
- **🛠️ LangGraph & Clean Modular Architecture:** Decoupled engine built with **LangGraph**, **LlamaIndex**, and **ChromaDB**, fully compatible with *LangGraph Studio*.

---

## 🏗️ Project Architecture

```text
src/any_context/
├── cli/                 # Presentation layer & user interaction
│   ├── chat_loop.py     # Main interactive chat loop & slash command intercepter
│   ├── config_menu.py   # Interactive configuration menu & onboarding wizard
│   └── workspace_selector.py # Workspace selection & CLI argument parser
├── config/              # Persistent configuration system
│   ├── app_settings.py  # Pydantic schemas & settings loader
│   └── db_store.py      # SQLite ConfigDBStore manager
├── core/                # LangGraph orchestration brain
│   ├── agent.py         # Agent graph definition & tool binding
│   └── utils.py         # API key resolvers & prompt finders
├── ingestion/           # Incremental RAG ingestion engine
│   └── local_folder_ingestor.py # Recursive folder scanner & ChromaDB updater
├── memory/              # Standalone 3-Level Hierarchical Memory Engine
│   ├── models.py        # Memory schemas (SHORT_TERM, SESSION_SUMMARY, META_SUMMARY)
│   ├── store.py         # ChromaDB memory vector store wrapper
│   ├── compressor.py    # LLM-powered Level-1 & Level-3 summarization engine
│   └── manager.py       # Asynchronous memory background thread orchestrator
└── tools/               # Agent dynamic search tools
    └── search_tools.py  # ChromaDB vector retriever tool (search_db)
```

---

## ⚡ Quick Start & Installation

### Option 1: Automatic Terminal Installer Script (No Python Needed!)

1. Go to the **[Latest GitHub Release](https://github.com/Levix-Digital/any-context/releases/latest)** and download the installer script for your OS:
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
*This script automatically downloads the latest `actx` binary, configures your User PATH environment variable, and enables the `actx` command globally.*

---

### Option 2: Install as a Python Package CLI

1. Clone this repository and install:
   ```bash
   pip install -e .
   ```
2. Run from anywhere in your terminal:
   ```bash
   actx
   # or specify a workspace directly:
   actx --workspace "AnyContext"
   # or open the configuration menu:
   actx --config
   ```
   *(Available command aliases: `actx`, `anycontext`, `any-context`, `ac`)*

---

### Option 3: Standalone Executable Download

Download pre-built native binaries (`actx-windows-x86_64.exe` or `actx-linux-x86_64`) directly from the **[GitHub Releases](https://github.com/Levix-Digital/any-context/releases)** page.

---

## 💬 During the Chat (Slash Commands)

- **`/switch`**: Change the active workspace interactively with instant vector database resynchronization.
- **`/update`**: Automatically download and install the latest AnyContext release.
- **`/check-update`**: Check if a newer version of AnyContext is available.
- **`/reset-memory`**: Reset long-term vector memory entries for the active workspace.
- **`/config`**: Open the interactive configuration menu to manage Workspaces, AI models, base URLs, and memory limits.
- **`/help`**: Display detailed in-app command instructions and tips.
- **`Ctrl+C`**: Gracefully exit the application while triggering an asynchronous long-term memory summary in the background.

---

## ⚙️ Configuration & API Key Management

AnyContext stores configurations and API keys securely in `config/settings.db` (SQLite). Manage settings interactively using `actx --config` or `/config` during chat:

- **🔑 Secure API Key Storage**: Input keys with password masking (`sk-...****`). Supported providers: OpenAI, OpenRouter, Anthropic, Gemini, DeepSeek, Groq.
- **⚡ 1-Click Provider Quick-Setup**:
  - *OpenAI Cloud Preset*: Enter key once; automatically sets `gpt-4o-mini` + `text-embedding-3-small`.
  - *Local Offline Preset*: Auto-configures LM Studio or Ollama (`http://localhost:1234/v1`).
- **❓ API Keys Guide**: Integrated in-app guide explaining where and how to obtain API keys for main providers.

```json
{
    "workspaces": [
        {
            "name": "MyProject",
            "paths": [
                "C:\\Users\\User\\Documents\\Project"
            ]
        }
    ],
    "models": {
        "local_embedding_model": "text-embedding-multilingual-e5-small",
        "local_openai_embedding_model": "text-embedding-3-small",
        "inference_model": "gpt-4o-mini",
        "summary_model": "google/gemma-4-e2b",
        "model_provider": "openai",
        "local_base_url": "http://localhost:1234/v1"
    },
    "memory": {
        "short_term_buffer_size": 20,
        "rolling_window_messages": 10,
        "meta_summary_threshold": 30,
        "meta_summary_batch_size": 10
    }
}
```

---

## 🧹 Uninstallation

To completely uninstall AnyContext (`actx`) and automatically remove PATH environment variables:

1. Download `uninstall.ps1` (Windows) or `uninstall.sh` (Linux / Git Bash) from the **[Latest Release Assets](https://github.com/Levix-Digital/any-context/releases/latest)**.
2. Run in your terminal:
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
2. **Multi-Agent Orchestration (Orchestrator & Sub-Agent Execution)**
3. **FastAPI REST Service & Web UI Interface**
4. **Source Code AST & Deep Codebase Analysis Pipeline**

---

> **Built with ☕ and ❤️ to transform file chaos into your personal AI assistant.**
