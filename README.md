# 🧠 AnyContext

> **Transform any file, drive, or folder into a living, real-time AI context.**

**AnyContext** is the ultimate bridge between your local data and Artificial Intelligence. Developed with an absolute focus on **privacy, modularity, and efficiency**, AnyContext is not just another RAG script; it's a smart Local Agent equipped with long-term memory, capable of instantly scanning your directories, learning from your documents, and providing accurate, well-founded answers.

Whether you are a developer trying to understand a complex repository or a business dealing with thousands of confidential reports, AnyContext runs **100% on your machine**, ensuring your data never feeds third-party models without your permission.

---

## 🚀 Why choose AnyContext?

- **🔒 Absolute Privacy (Offline-First):** Natively integrated with [LM Studio](https://lmstudio.ai/) and local models. Your local files, business strategies, and secrets remain exclusively on your hardware.
- **📂 Multi-Workspace Ingestion:** Tired of AI mixing marketing scripts with Python code? AnyContext groups multiple directories into isolated "Workspaces". The AI filters vectors in real-time, keeping the context strictly focused on the topic you requested.
- **⚡ Ultra-Fast Incremental Synchronization:** No more waiting hours to vectorize large folders. Our ingestor tracks modifications and document hashes: it only indexes new or altered files and automatically purges files that were deleted from your disk out of the database.
- **🧠 Long-Term Memory:** AnyContext remembers you. With a robust SQLite-based system, it creates background summaries of your past conversations, ensuring the agent evolves with you over time.
- **🛠️ LangGraph & Clean Architecture:** Incredibly organized and modular code. Built with **LangGraph**, **LlamaIndex**, and **ChromaDB**, the system already fully supports *LangGraph Studio* for visual debugging and scalability.

---

## 🏗️ Current Architecture

Currently, the project consists of:
- **`src/any_context/ingestion/`**: The incremental synchronization engine. Recursively reads directories, filters valid extensions, generates deterministic UUIDs, and atomically updates ChromaDB.
- **`src/any_context/core/`**: The orchestrating brain (LangGraph). Features dynamic tools to query the database (`search_db`). **UI-Ready**: Completely decoupled from terminal logic.
- **`src/any_context/cli/`**: The presentation layer. Manages interactive workspace selection (via `questionary`), argument parsing, ANSI text formatting, and intercepting slash commands (`/help`, `/switch`).
- **`config/settings.json`**: The central hub. Defines your AI models, database paths, and your unlimited Workspaces.

---

## ⚡ Quick Start & Installation

### Option 1: Automatic Installer Script (No Python Needed!)

1. Go to the **[Latest GitHub Release](https://github.com/Levix-Digital/any-context/releases/latest)** and download the installer script for your OS:
   - **Windows**: `install.ps1`
   - **Linux**: `install.sh`
2. Run the script in your terminal:
   - **Windows (PowerShell)**:
     ```powershell
     .\install.ps1
     ```
   - **Linux (Terminal)**:
     ```bash
     chmod +x install.sh
     ./install.sh
     ```
*This downloads the `actx` binary, configures your user PATH, and enables the `actx` command globally.*

---

### Option 2: Install as a Python Package CLI

1. Clone this repository and install:
   ```bash
   pip install -e .
   ```
2. Run from anywhere in your terminal:
   ```bash
   actx
   # or bypass workspace menu:
   actx --workspace "AnyContext"
   ```
   *(Aliases available: `actx`, `anycontext`, `any-context`, `ac`)*

---

### Option 3: Manual Standalone Executable Download

Download pre-built native binaries (`actx-windows-x86_64.exe` or `actx-linux-x86_64`) directly from the **[GitHub Releases](https://github.com/Levix-Digital/any-context/releases)** page.

---

## 🔮 The Future of AnyContext (Roadmap)

1. **Cloud Drive Ingestors (Google Drive, OneDrive, Dropbox)**
2. **Multi-Agent Orchestration (Orchestrator Agent)**
3. **Source Code Pipeline & Codebase Analysis**
4. **FastAPI REST Service & Web UI Interface**

---

### 💻 During the Chat:
- Type `/help` to see the available commands.
- Type `/switch` to hot-swap your active workspace without restarting the application!
- Press `Ctrl+C` to cleanly exit and generate a background summary of your session.

---

> **Built with ☕ and ❤️ to transform file chaos into your personal AI assistant.**
