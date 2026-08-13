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
- **`ingestion/local_folder_ingestor.py`**: The incremental synchronization engine. Recursively reads directories, filters valid extensions (including images via OCR), generates deterministic UUIDs, and atomically updates ChromaDB.
- **`core/agent.py`**: The orchestrating brain (LangGraph). Features dynamic tools to query the database (`search_db`) and real-time session memory.
- **`config/settings.json`**: The central hub. Defines your AI models, database paths, and your unlimited Workspaces.

---

## 🔮 The Future of AnyContext (Roadmap)

The current version (Local Folder Agent) is just the foundation. The project is architected to scale and become the "Central Nervous System" of your data ecosystem.

Here are the next stages of our development:

### 1. Cloud Drive Ingestors
Expansion of the `ingestion/` folder to natively support:
- **Google Drive Ingestor**
- **OneDrive Ingestor**
- **Dropbox Ingestor**
Each service will be processed in isolation, seamlessly bringing the online world into your local context.

### 2. Multi-Agent Orchestration (Orchestrator Agent)
As data sources increase, we will create a multi-agent architecture:
- `folder-agent` (Current)
- `drive-agent`
- `orchestrator-agent` (A central "Router" agent that receives user messages, understands the intent, and delegates the search to the most appropriate specialist agent, allowing granular access control and feature toggling).

### 3. Source Code Ingestion (Source Code Pipeline)
One of the major goals for the final stage of the project. A specialized module to understand file trees, dependencies, and legacy code logic, allowing the AI to act as a Senior Software Engineer with full knowledge of your repository.

---

## ⚙️ Getting Started

1. Clone this repository.
2. Create and activate your virtual environment (e.g., `.venv`).
3. Install the dependencies:
   ```bash
   pip install -r requirements.txt
   ```
4. Copy your credentials to the `.env` file (if using closed APIs like OpenAI).
5. Configure your Workspaces and folder paths by editing `config/settings.json`.
6. Run the agent:
   ```bash
   python main.py
   ```
7. *(Optional)* Develop visually using LangGraph Studio:
   ```bash
   langgraph dev
   ```

---

> **Built with ☕ and ❤️ to transform file chaos into your personal AI assistant.**
