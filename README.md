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

## ⚙️ Getting Started

### Option 1: Install as a Python CLI Tool (Recommended for Devs)

1. Clone this repository and install in editable mode or via `pipx`:
   ```bash
   pip install -e .
   ```
2. Now run using the quick **`actx`** command from anywhere in your terminal:
   ```bash
   actx
   # or bypass workspace menu:
   actx --workspace "AnyContext"
   ```
   *(Aliases available: `actx`, `anycontext`, `any-context`, `ac`)*

### Option 2: Run directly with Python

1. Install dependencies:
   ```bash
   pip install -r requirements.txt
   ```
2. Configure your Workspaces in `config/settings.json`.
3. Run the agent:
   ```bash
   python main.py
   ```

### Option 3: Download Standalone Executables (`actx.exe` / `actx-linux`)

No Python required! Download the pre-built native binaries for Windows or Linux directly from the **[GitHub Releases](https://github.com/Levix-Digital/any-context/releases)** page.

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
