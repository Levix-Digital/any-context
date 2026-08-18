from typing import Dict, List, Optional
from any_context.help.models import HelpPage

HELP_REGISTRY: Dict[str, HelpPage] = {
    "switch": HelpPage(
        command="/switch",
        aliases=["switch", "-w", "--workspace"],
        title="📂 Workspace Switching & Scope Isolation",
        description=(
            "The /switch command allows you to change your active workspace in real-time without restarting AnyContext. "
            "Each workspace acts as an isolated knowledge base with its own distinct document folders, web sources, "
            "and ChromaDB vector embeddings. Switching workspaces changes the AI agent's active memory scope instantly."
        ),
        syntax=(
            "CLI Launch : actx -w <workspace_name>\n"
            "  In Chat    : type '/switch' during active chat\n"
            "  View Help  : actx --switch --help   OR   /switch --help   OR   /switch -h"
        ),
        parameters=[
            "-w, --workspace <name> : Directly specify target workspace on CLI launch.",
            "/switch               : Opens interactive menu to choose active workspace.",
            "--help, -h            : Display this detailed help page for /switch."
        ],
        examples=[
            "actx -w AnyContextProject",
            "actx -w LegalConsulting",
            "In Chat: /switch",
            "In Chat: /switch -h"
        ],
        tips=[
            "Workspaces keep your files and projects completely separate, preventing information from one client or project from mixing with another.",
            "You can manage folders inside a workspace anytime using the '/config' menu."
        ]
    ),

    "sync": HelpPage(
        command="/sync",
        aliases=["sync", "index", "/index", "--sync", "--index", "-s"],
        title="⚡ Workspace Document Synchronization & Deep Scanning",
        description=(
            "The /sync (or /index) command performs an incremental scan of all configured folders in the active workspace. "
            "It automatically discovers files across all nested subdirectories, calculates SHA-256 hashes to only index new or modified files, "
            "and purges deleted disk files from the ChromaDB vector database."
        ),
        syntax=(
            "In Chat (Silent)  : /sync   OR   /index\n"
            "  In Chat (Verbose) : /sync --verbose   OR   /index -v\n"
            "  CLI Re-indexing   : actx --index   OR   POST /v1/index (REST API)\n"
            "  View Help         : /sync --help   OR   /sync -h"
        ),
        parameters=[
            "/sync, /index         : Runs fast, clean single-line background synchronization.",
            "--verbose, -v         : Displays a detailed, modern tree view showing discovered subfolders, file counts, and vector chunks.",
            "--help, -h            : Display this detailed help page for /sync."
        ],
        examples=[
            "In Chat: /sync",
            "In Chat: /sync --verbose",
            "In Chat: /index -v",
            "curl -X POST http://127.0.0.1:8000/v1/index -H 'Content-Type: application/json' -d '{\"workspace\":\"Legal\"}'"
        ],
        tips=[
            "Whenever you add, edit, or delete files on your computer, type '/sync' to update the AI's knowledge base immediately.",
            "Use '/sync --verbose' whenever you want to inspect exactly which files and subdirectories are currently indexed."
        ]
    ),

    "model": HelpPage(
        command="/model",
        aliases=["model", "/m", "-m", "models", "switch-model", "llm"],
        title="🤖 Dynamic AI Model Switching (9 Supported Providers)",
        description=(
            "AnyContext allows you to switch between 9 leading AI providers on-the-fly without restarting or re-indexing files. "
            "The model selector dynamically inspects your configured API keys and only presents models ready for inference. "
            "Supported providers: OpenAI, Anthropic Claude, Google Gemini, DeepSeek, Groq, xAI Grok, OpenRouter, Mistral AI, and Local LM Studio/Ollama."
        ),
        syntax=(
            "In Chat (Interactive Menu) : /model   OR   /m\n"
            "  In Chat (Direct Switch)     : /model <model_name>\n"
            "  In Chat (One-Shot Prompt)   : @<model_name> <your message>\n"
            "  REST API                    : POST /v1/chat  with {'model': 'gpt-4o', 'message': '...'}\n"
            "  View Help                   : /model --help   OR   /model -h"
        ),
        parameters=[
            "/model, /m            : Opens interactive selection menu showing available models with active API keys.",
            "/model <name>         : Instantly changes active inference model for the current session.",
            "@<model> <prompt>     : Asks a single question to a specific model without changing session defaults.",
            "--help, -h            : Display this detailed help page for /model."
        ],
        examples=[
            "In Chat: /model",
            "In Chat: /model gpt-4o-mini",
            "In Chat: /model claude-haiku-4-5-20251001",
            "In Chat: /model gemini-flash-latest",
            "In Chat: /model deepseek-chat",
            "In Chat: /model llama-3.3-70b-versatile",
            "In Chat: @gpt-4o Resuma os principais riscos desta minuta contratual",
            "In Chat: @mistral-small-latest Traduza esta ata para o francês"
        ],
        tips=[
            "Switching inference models NEVER re-indexes your documents or clears your vector cache.",
            "The active model is always visible in the prompt prefix: 'You [Workspace | Model]'.",
            "To unlock models for other providers, add their API key via '/config' -> '🔑 Manage Saved API Keys'."
        ]
    ),

    "api-keys": HelpPage(
        command="/api-keys",
        aliases=["api-keys", "api_keys", "keys", "apikey", "apikeys", "providers", "--api-keys", "--keys"],
        title="🔑 Comprehensive AI Providers & API Key Setup Guide",
        description=(
            "AnyContext supports 9+ leading cloud and 100% offline local AI engines. "
            "API keys can be saved securely in the local SQLite database via '/config' or configured in your '.env' file.\n\n"
            "  1. ⚡ OpenAI (GPT-4o-mini, GPT-4o, GPT-4-turbo):\n"
            "     • Portal: https://platform.openai.com/api-keys\n"
            "     • Key Format: sk-proj-... or sk-...\n\n"
            "  2. 🧠 Anthropic (Claude 3.5 Sonnet, Haiku 4.5, Opus 4.5):\n"
            "     • Portal: https://console.anthropic.com/settings/keys\n"
            "     • Key Format: sk-ant-...\n\n"
            "  3. ♊ Google Gemini (Gemini Flash, Gemini 3.5 Flash, Gemini Pro):\n"
            "     • Portal: https://aistudio.google.com/app/apikey\n"
            "     • Key Format: AIzaSy...\n\n"
            "  4. 🐉 DeepSeek (DeepSeek V3 Chat & R1 Reasoning - Ultra Low Cost $0.14/M):\n"
            "     • Portal: https://platform.deepseek.com/api_keys\n"
            "     • Key Format: sk-...\n\n"
            "  5. ⚡ Groq Cloud (Ultra-Fast Inference for Llama 3.3 70B, Mixtral & Gemma 2):\n"
            "     • Portal: https://console.groq.com/keys\n"
            "     • Key Format: gsk_...\n\n"
            "  6. 🌪️ Mistral AI (Mistral Small, Mistral Nemo, Mistral Large):\n"
            "     • Portal: https://console.mistral.ai/api-keys/\n"
            "     • Key Format: mistral_... or generic key string\n\n"
            "  7. 🚀 xAI Grok (Grok-2, Grok-beta):\n"
            "     • Portal: https://console.x.ai\n"
            "     • Key Format: xai-...\n\n"
            "  8. 🌐 OpenRouter (Unified Gateway for 200+ Open & Commercial Models):\n"
            "     • Portal: https://openrouter.ai/keys\n"
            "     • Key Format: sk-or-v1-...\n\n"
            "  9. 🏠 LM Studio & Ollama (100% Free & Local Private Offline LLMs):\n"
            "     • LM Studio: https://lmstudio.ai (Base URL: http://localhost:1234/v1)\n"
            "     • Ollama: https://ollama.com (Base URL: http://localhost:11434/v1)\n"
            "     • Zero API keys required for offline local models!"
        ),
        syntax=(
            "In Chat    : /api-keys   OR   /keys\n"
            "  CLI Launch : actx --api-keys   OR   actx --keys\n"
            "  Config Menu: /config -> '🔑 Manage Saved API Keys'\n"
            "  View Help  : /api-keys --help   OR   /api-keys -h"
        ),
        parameters=[
            "/api-keys, /keys      : Display full guide and direct portal links to obtain API keys.",
            "--api-keys, --keys    : Launch the API keys guide directly from the terminal.",
            "/config               : Open interactive configuration menu to save or update your API keys.",
            "--help, -h            : Display this detailed help page."
        ],
        examples=[
            "In Chat: /api-keys",
            "In Chat: /keys",
            "actx --api-keys",
            "In Chat: /config"
        ],
        tips=[
            "API keys entered via '/config' are stored locally in your SQLite database with PBKDF2 masking (sk-...****).",
            "Developers can alternatively configure keys in their '.env' file using '.env.example' as a template."
        ]
    ),

    "web": HelpPage(
        command="/web",
        aliases=["web", "--web", "scrape", "sitemap", "website", "websites", "urls", "crawler"],
        title="🌐 Interactive Web Discovery & Deep Recursive Crawler",
        description=(
            "The Web Crawler engine allows AnyContext to ingest entire documentation portals, websites, "
            "and government/legal databases into your active workspace knowledge base. "
            "It runs in 2 phases: (1) Fast Discovery (maps internal links and XML sitemaps) and (2) Interactive Scope Selection "
            "(choose between Section Only, Top 50/250/500 pages, or Entire Domain). "
            "Uses multi-threaded concurrent downloading and batch ChromaDB vector indexing."
        ),
        syntax=(
            "In Chat (Interactive Crawler) : /web add <url>   OR   /web add\n"
            "  In Chat (Management Menu)      : /web\n"
            "  In Chat (List Sources)         : /web list\n"
            "  In Chat (Force Sync All)       : /web sync\n"
            "  Conversational Ingestion       : Tell the AI: 'adicione o site https://... ao workspace'\n"
            "  REST API                       : POST /v1/workspaces/{name}/web-urls?url=...\n"
            "  View Help                      : /web --help   OR   /web -h"
        ),
        parameters=[
            "/web                   : Open interactive web sources management menu.",
            "/web add <url>         : Launch interactive site discovery, scope menu, and concurrent deep crawler.",
            "/web list              : List all configured web URLs and last scrape timestamps.",
            "/web sync              : Force re-scrape and synchronize all web URLs in active workspace.",
            "url                    : Target website URL or documentation portal (e.g. 'https://docs.python.org/3/').",
            "--help, -h            : Display this detailed help page for web crawler."
        ],
        examples=[
            "In Chat: /web add https://canada.ca/en/immigration-refugees-citizenship.html",
            "In Chat: /web add https://docs.python.org/3/",
            "In Chat: /web",
            "In Chat: /web list",
            "In Chat: /web sync",
            "Conversational: 'Adicione a documentação https://platform.openai.com/docs ao workspace'",
            "curl -X POST 'http://127.0.0.1:8000/v1/workspaces/MyProject/web-urls?url=https://docs.python.org/3/'"
        ],
        tips=[
            "100% unlocked for Community CLI users! You can crawl and index entire documentation sites.",
            "Fast discovery automatically checks XML sitemaps and presents estimated page counts and times before crawling.",
            "Multi-threaded concurrent crawler downloads 20 to 50 pages per second with smooth progress bar feedback."
        ]
    ),

    "ocr": HelpPage(
        command="/ocr",
        aliases=["ocr", "--ocr", "image", "images", "scan"],
        title="📷 Image & Scanned PDF OCR Text Extraction",
        description=(
            "The Image OCR engine extracts clean text from image files (.png, .jpg, .jpeg, .webp, .tiff, .bmp) "
            "and scanned PDF documents, embedding extracted content into ChromaDB vector storage for instant AI search."
        ),
        syntax=(
            "REST API   : POST /v1/ingest/ocr?workspace_name={name}&image_path={path}\n"
            "  CLI Scan   : Scanned PDFs & images in workspace folders are automatically parsed during /sync\n"
            "  View Help  : actx --ocr --help   OR   /ocr --help   OR   /ocr -h"
        ),
        parameters=[
            "/ocr, --ocr            : Display Image OCR engine status and parameters.",
            "image_path             : Absolute disk path to image or scanned document file.",
            "--help, -h            : Display this detailed help page for OCR."
        ],
        examples=[
            "In Chat: /ocr -h",
            "curl -X POST 'http://127.0.0.1:8000/v1/ingest/ocr?workspace_name=Legal&image_path=C:/Scans/contrato.pdf'"
        ],
        tips=[
            "100% unlocked in Community CLI! Simply drop image scans into your workspace folder and run '/sync'.",
            "Uses optical character recognition with SHA-256 hash deduplication to avoid redundant processing."
        ]
    ),

    "billing": HelpPage(
        command="/billing",
        aliases=["billing", "--billing", "plans", "--plans", "tiers", "pricing", "license"],
        title="💳 Subscription Plans, Licensing & Capability Matrix",
        description=(
            "AnyContext provides a completely free, 100% unlocked Community Edition for local CLI users, "
            "alongside commercial tiers (Pro, Team, Enterprise) for teams running REST API servers (actx --serve), "
            "multi-user collaboration, and enterprise VPC infrastructure."
        ),
        syntax=(
            "In Chat    : /billing   OR   /plans\n"
            "  CLI Launch : actx --billing   OR   actx --plans\n"
            "  REST API   : GET /v1/billing/status   OR   GET /v1/billing/plans\n"
            "  View Help  : /billing --help   OR   /billing -h"
        ),
        parameters=[
            "/billing, /plans       : Display active subscription tier, pricing matrix, and feature permissions.",
            "--billing, --plans     : Launch subscription plan inspector from CLI.",
            "--help, -h            : Display this detailed help page."
        ],
        examples=[
            "In Chat: /billing",
            "In Chat: /plans",
            "actx --billing",
            "In Chat: /billing -h"
        ],
        tips=[
            "Community Edition ($0): 100% unlocked on local terminal (unlimited folders, web scraping, OCR, all 9 LLMs, ChromaDB).",
            "Server Mode (actx --serve) & Multi-Tenant VPC require a Pro, Team, or Enterprise license key configured in '.env' (ANYCONTEXT_LICENSE_KEY=...)."
        ]
    ),

    "update": HelpPage(
        command="/update",
        aliases=["update", "--update", "/check-update", "--check-update"],
        title="🔄 Interactive 1-Click Self-Updater Engine",
        description=(
            "Checks GitHub Releases for newer AnyContext versions and provides an interactive 1-click upgrade prompt. "
            "Automatically downloads and applies the latest release executable with atomic replacement, even on locked Windows binaries."
        ),
        syntax=(
            "In Chat (Interactive Update) : /update\n"
            "  In Chat (Check Only)          : /check-update\n"
            "  CLI Launch (Interactive)      : actx --update\n"
            "  CLI Launch (Check Only)       : actx --check-update\n"
            "  View Help                     : /update --help   OR   /update -h"
        ),
        parameters=[
            "/update, --update       : Check for updates and prompt to install immediately [Y/n].",
            "/check-update, --check  : Check if a new version is available and offer 1-click update.",
            "--help, -h            : Display this detailed help page for the updater."
        ],
        examples=[
            "actx --check-update",
            "actx --update",
            "In Chat: /check-update",
            "In Chat: /update"
        ],
        tips=[
            "When a new version is detected, AnyContext prompts: 'Would you like to download and install now? [Y/n]'. Simply press Enter to update!",
            "Non-blocking startup checks ensure your CLI opens in under 3 milliseconds without waiting for network requests."
        ]
    ),

    "config": HelpPage(
        command="/config",
        aliases=["config", "--config", "-c"],
        title="⚙️ Interactive Configuration & Settings Menu",
        description=(
            "The /config command launches the full interactive configuration menu. "
            "From this menu, you can add or remove document folders per workspace, configure AI models "
            "(OpenAI, Anthropic Claude, Gemini, DeepSeek, Groq, Mistral, xAI Grok, OpenRouter, LM Studio, Ollama), "
            "manage saved API Keys, adjust memory compression limits, and manage workspaces."
        ),
        syntax=(
            "CLI Launch : actx --config\n"
            "  In Chat    : type '/config' during active chat\n"
            "  View Help  : actx --config --help   OR   /config --help   OR   /config -h"
        ),
        parameters=[
            "📂 Workspaces & Folders Management : Add, view, or remove document folder paths.",
            "🌐 Web Sources & Polling URLs     : Manage website URLs ingested into the active workspace.",
            "🤖 AI Models & Base URL            : Select LLM inference and embedding models.",
            "🔑 Manage Saved API Keys           : Store API keys securely in SQLite.",
            "🧠 Memory Compression Settings     : Adjust short-term and meta-summary limits.",
            "🛡️ User Accounts & Security RBAC   : Manage Admin, Team Users, and Bearer Tokens.",
            "💥 Factory Reset                   : Wipe all settings and reset to defaults.",
            "--help, -h                        : Display this detailed help page for /config."
        ],
        examples=[
            "actx --config",
            "actx --config --help",
            "In Chat: /config"
        ],
        tips=[
            "Supports 3 AI Setup Modes: ⚡ Cloud Presets, 🏠 Local Offline Server (LM Studio / Ollama), and 🛠️ Custom Provider Setup.",
            "Changing embedding models automatically clears stale ChromaDB collections to prevent vector dimension mismatch errors."
        ]
    ),

    "auth": HelpPage(
        command="/auth",
        aliases=["auth", "login", "users", "tokens", "security", "rbac"],
        title="🔐 User Accounts, Access Control & RBAC Authentication",
        description=(
            "AnyContext supports two distinct operational security modes:\n"
            "1. Open Local Mode (Default): Zero friction for single-user personal use on your local machine.\n"
            "2. Enterprise / Multi-User Mode: Protected mode for team servers (VPC / On-Premise). "
            "Enforces user login (Admin, Analyst, Viewer) and Bearer Access Tokens (actx_sec_...)."
        ),
        syntax=(
            "REST API   : POST /v1/auth/login   OR   POST /v1/auth/setup-admin\n"
            "  In Chat    : /config -> '🛡️ User Accounts & Security Access Control'\n"
            "  View Help  : actx --auth --help   OR   /auth --help   OR   /auth -h"
        ),
        parameters=[
            "👑 Admin Role   : Full system control (creates users, manages workspaces, API keys, factory reset).",
            "🔬 Analyst Role : Can query AI chat, search vector DB, and trigger folder re-indexing.",
            "👁️ Viewer Role  : Read-only access to query AI chat and search vector DB.",
            "🔑 Bearer Token : Token string format 'actx_sec_...' sent in HTTP Authorization headers.",
            "--help, -h     : Display this detailed help page for security and RBAC."
        ],
        examples=[
            "curl -H 'Authorization: Bearer actx_sec_...' http://localhost:8000/v1/chat",
            "actx --auth --help",
            "In Chat: /auth -h"
        ],
        tips=[
            "When running in REST API Server mode (actx --serve), if no Admin is configured yet, access to data endpoints is blocked until an Administrator is initialized!",
            "Each user or token can be restricted to specific workspace scopes (e.g. ['Finance', 'Legal'])."
        ]
    ),

    "share": HelpPage(
        command="🤝 Workspace Sharing",
        aliases=["share", "sharing", "invite", "join-workspace", "collaboration"],
        title="🤝 Google Drive-Style Workspace Collaboration & Sharing",
        description=(
            "Workspace Sharing allows workspace owners to share an existing workspace with team collaborators. "
            "Collaborators gain access to the agent's AI RAG context and knowledge base for that project. "
            "Folder visibility is transparent (all indexed folder paths are visible with ownership tags), but edit/delete "
            "permissions remain strictly locked to the user who physically added each folder!"
        ),
        syntax=(
            "REST API   : POST /v1/workspaces/share/invite   OR   POST /v1/workspaces/share/accept\n"
            "  In Chat    : /config -> '🤝 Workspace Sharing & Collaboration'\n"
            "  View Help  : actx --share --help   OR   /share --help   OR   /share -h"
        ),
        parameters=[
            "👁️ Viewer Role : Can query AI chat & search vector DB. Cannot add or delete folders.",
            "✏️ Editor Role : Can query AI chat & search vector DB + add their own local folders to the workspace.",
            "👑 Owner Role  : Full control over workspace folders and collaborator permissions.",
            "--help, -h     : Display this detailed help page for workspace sharing."
        ],
        examples=[
            "In Chat: /config  ->  Select '🤝 Workspace Sharing & Collaboration'",
            "POST /v1/workspaces/share/invite  ->  {'workspace_name': 'Migration', 'access_level': 'editor'}",
            "POST /v1/workspaces/share/accept  ->  {'invite_code': 'SHARE-MIGR-1234', 'user_email': 'amanda@advocacia.com'}",
            "In Chat: /share -h"
        ],
        tips=[
            "All collaborators can see the full list of workspace folders, tagged as '[👑 Your Folder]' or '[🔒 Read-Only (Added by Amanda)]'.",
            "No user can modify or open local disk files belonging to another user!"
        ]
    ),

    "serve": HelpPage(
        command="--serve",
        aliases=["serve", "server", "-s"],
        title="🌐 High-Performance REST API Server & Enterprise VPC Listener",
        description=(
            "Starts the FastAPI ASGI Web Server exposing RAG vector search, isolated workspaces, "
            "3-level session memory, and RBAC authentication endpoints for external web apps, VS Code extensions, "
            "and enterprise intranet services. Features interactive Swagger UI documentation at http://127.0.0.1:8000/docs."
        ),
        syntax=(
            "CLI Launch : actx --serve [--port PORT] [--host HOST]\n"
            "  View Help  : actx --serve --help   OR   actx serve -h   OR   /serve --help   OR   /serve /h"
        ),
        parameters=[
            "--port <int>  : Specify port number (default: 8000).",
            "--host <str>  : Specify host interface. Use '--host 0.0.0.0' for Enterprise VPC mode listening on all internal network interfaces.",
            "--help, -h    : Display this detailed help page for REST server deployment."
        ],
        examples=[
            "actx --serve",
            "actx --serve --port 8000 --host 127.0.0.1",
            "actx --serve --host 0.0.0.0 --port 8000   (Enterprise VPC Mode)",
            "actx --serve --help",
            "In Chat: /serve -h"
        ],
        tips=[
            "Binding to '--host 0.0.0.0' allows any authorized service on your company VPN/VPC to query AnyContext.",
            "Server Mode requires a Pro, Team, or Enterprise license key configured in '.env' (ANYCONTEXT_LICENSE_KEY=...)."
        ]
    ),

    "mcp": HelpPage(
        command="--mcp",
        aliases=["mcp", "model-context-protocol"],
        title="🔌 Model Context Protocol (MCP) Stdio JSON-RPC Server",
        description=(
            "Launches AnyContext as a standard Model Context Protocol (MCP) server communicating over stdio JSON-RPC 2.0. "
            "Allows Anthropic Claude Desktop, Cursor IDE, and Antigravity AI agents to query your local knowledge base seamlessly."
        ),
        syntax=(
            "CLI Launch : actx --mcp\n"
            "  View Help  : actx --mcp --help   OR   actx mcp -h   OR   /mcp --help   OR   /mcp /h"
        ),
        parameters=[
            "--mcp                 : Runs stdio JSON-RPC 2.0 listener for Claude Desktop and Cursor IDE.",
            "--help, -h            : Display this detailed help page for MCP server configuration."
        ],
        examples=[
            "actx --mcp",
            "actx --mcp --help",
            "In Chat: /mcp -h",
            "Claude Desktop Config (claude_desktop_config.json):\n{\n  'mcpServers': {\n    'any-context': {\n      'command': 'actx',\n      'args': ['--mcp']\n    }\n  }\n}"
        ],
        tips=[
            "MCP tools include 'search_workspace_docs', 'query_anycontext_agent', 'list_workspaces', 'create_access_token', and 'get_anycontext_system_documentation'."
        ]
    ),

    "reset-memory": HelpPage(
        command="/reset-memory",
        aliases=["reset-memory", "/reset", "reset"],
        title="🧹 3-Level Hierarchical Long-Term Memory Reset",
        description=(
            "Purges long-term session memory summaries from ChromaDB for the active workspace or globally across all workspaces. "
            "Resets Level-1 session block summaries and Level-3 consolidated meta-summaries."
        ),
        syntax=(
            "REST API   : POST /v1/reset-memory\n"
            "  In Chat    : type '/reset-memory' or '/reset' during chat\n"
            "  View Help  : actx --reset-memory --help   OR   /reset-memory --help   OR   /reset-memory -h"
        ),
        parameters=[
            "/reset-memory         : Reset memory for active workspace (interactive confirmation).",
            "/config                : Open memory settings menu to perform global or specific memory reset.",
            "--help, -h            : Display this detailed help page for memory resets."
        ],
        examples=[
            "In Chat: /reset-memory",
            "In Chat: /reset -h",
            "actx --reset-memory --help"
        ],
        tips=[
            "Resetting memory only clears session conversation summaries; your indexed document files and vectors remain intact!"
        ]
    ),

    "clear": HelpPage(
        command="/clear",
        aliases=["clear", "/cls", "cls"],
        title="🧹 Terminal Screen Clear",
        description=(
            "Clears the terminal scrollback/screen and repaints the clean signature AnyContext banner and status header. "
            "AnyContext also automatically clears the terminal upon launching the interactive chat session for a focused, distraction-free environment."
        ),
        syntax=(
            "In Chat    : type '/clear' or '/cls' during chat\n"
            "  View Help  : /clear --help   OR   /clear -h"
        ),
        parameters=[
            "/clear, /cls          : Clear terminal screen and redraw the clean signature banner.",
            "--help, -h            : Display this help page."
        ],
        examples=[
            "In Chat: /clear",
            "In Chat: /cls",
            "In Chat: /clear -h"
        ],
        tips=[
            "Use /clear anytime during long multi-turn sessions to keep your workspace view pristine without losing session context."
        ]
    ),

    "factory-reset": HelpPage(
        command="/factory-reset",
        aliases=["factory-reset", "--factory-reset", "/reset-factory"],
        title="💥 Factory Reset (Complete System Reset)",
        description=(
            "Wipes all SQLite settings.db tables (workspaces, models, API keys, users, access tokens) "
            "and deletes all local vector database directories ('./context_db' and './memory'). "
            "Resets AnyContext completely back to clean factory defaults."
        ),
        syntax=(
            "CLI Launch : actx --factory-reset\n"
            "  REST API   : POST /v1/factory-reset\n"
            "  In Chat    : type '/factory-reset' during chat\n"
            "  View Help  : actx --factory-reset --help   OR   /factory-reset --help   OR   /factory-reset -h"
        ),
        parameters=[
            "--factory-reset       : Run factory reset from CLI launch.",
            "/factory-reset        : Run factory reset from inside chat loop.",
            "--help, -h            : Display this detailed help page for factory reset."
        ],
        examples=[
            "actx --factory-reset",
            "actx --factory-reset --help",
            "In Chat: /factory-reset -h"
        ],
        tips=[
            "Use Factory Reset if you want to start fresh or transfer your AnyContext installation to a new environment."
        ]
    )
}


def get_help_page(command_or_alias: str) -> Optional[HelpPage]:
    """Resolves a command or alias string to its registered HelpPage instance."""
    query = command_or_alias.strip().lower()
    if query.startswith("/") or query.startswith("-"):
        query = query.lstrip("/-")

    for key, page in HELP_REGISTRY.items():
        if query == key or query in [a.lstrip("/-").lower() for a in page.aliases]:
            return page

    return None
