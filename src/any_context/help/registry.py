from typing import Dict, List, Optional
from any_context.help.models import HelpPage

HELP_REGISTRY: Dict[str, HelpPage] = {
    "switch": HelpPage(
        command="/switch",
        aliases=["switch", "-w", "--workspace"],
        title="📂 Workspace Switching & Vector DB Synchronization",
        description=(
            "The /switch command allows you to change your active workspace in real-time. "
            "When a workspace is selected, AnyContext instantly performs an incremental scan of all "
            "configured folders for that workspace, updates document embeddings in ChromaDB, and switches "
            "the active context scope for the AI agent."
        ),
        syntax=(
            "CLI Launch : actx -w <workspace_name>\n"
            "  In Chat    : type '/switch' during active chat\n"
            "  View Help  : actx --switch --help   OR   /switch --help   OR   /switch -h   OR   /switch /help   OR   /switch /h"
        ),
        parameters=[
            "-w, --workspace <name> : Directly specify target workspace on CLI launch.",
            "/switch               : Opens interactive menu to choose active workspace.",
            "--help, -h, /help, /h : Display this detailed help page for /switch."
        ],
        examples=[
            "actx -w MyProject",
            "actx -w HumanResources",
            "actx --switch --help",
            "In Chat: /switch -h   (opens this help manual page)"
        ],
        tips=[
            "If you add or edit files in your workspace folder, type '/switch' and re-select your current workspace to trigger a fast incremental resync!",
            "Workspaces isolate ChromaDB vector collections, ensuring zero document cross-contamination between projects."
        ]
    ),

    "config": HelpPage(
        command="/config",
        aliases=["config", "--config", "-c"],
        title="⚙️ Interactive Configuration & Security Management Menu",
        description=(
            "The /config command launches the full interactive configuration menu. "
            "From this menu, you can add or remove document folders per workspace, configure AI models "
            "(OpenAI, LM Studio, Ollama, Claude, Gemini, DeepSeek, Groq), manage API Keys, tweak memory settings, "
            "manage Workspace Sharing, and configure RBAC User Accounts & Security Tokens."
        ),
        syntax=(
            "CLI Launch : actx --config\n"
            "  In Chat    : type '/config' during active chat\n"
            "  View Help  : actx --config --help   OR   /config --help   OR   /config -h   OR   /config /help   OR   /config /h"
        ),
        parameters=[
            "📂 Workspaces & Folders Management : Add/remove document folder paths.",
            "🤝 Workspace Sharing & Collab      : Share workspaces & generate invite codes (Google Drive style).",
            "🤖 AI Models & Base URL            : Select LLM inference and embedding models.",
            "🔑 Manage Saved API Keys           : Store API keys securely in SQLite.",
            "🧠 Memory Compression Settings     : Adjust short-term and meta-summary limits.",
            "🛡️ User Accounts & Security RBAC   : Manage Admin, Team Users, and Bearer Tokens.",
            "💥 Factory Reset                   : Wipe all settings and reset to defaults.",
            "--help, -h, /help, /h             : Display this detailed help page for /config."
        ],
        examples=[
            "actx --config",
            "actx --config --help",
            "In Chat: /config /help   (opens this help manual page)"
        ],
        tips=[
            "Supports 3 AI Setup Modes: ⚡ OpenAI Cloud (Default), 🏠 Local Offline Server (LM Studio / Ollama), and 🛠️ Custom Provider Setup.",
            "Changing embedding models automatically clears stale ChromaDB collections to prevent vector dimension mismatch errors.",
            "Passwords and API keys are stored securely with SHA-256 PBKDF2 hashing and masking."
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
            "  View Help  : actx --auth --help   OR   /auth --help   OR   /auth -h   OR   /login /help   OR   /login /h"
        ),
        parameters=[
            "👑 Admin Role   : Full system control (creates users, manages workspaces, API keys, factory reset).",
            "🔬 Analyst Role : Can query AI chat, search vector DB, and trigger folder re-indexing.",
            "👁️ Viewer Role  : Read-only access to query AI chat and search vector DB.",
            "🔑 Bearer Token : Token string format 'actx_sec_...' sent in HTTP Authorization headers.",
            "--help, -h, /help, /h : Display this detailed help page for security and RBAC."
        ],
        examples=[
            "actx login --server http://192.168.1.50:8000",
            "curl -H 'Authorization: Bearer actx_sec_...' http://localhost:8000/v1/chat",
            "actx --auth --help",
            "In Chat: /login -h   (opens this help manual page)"
        ],
        tips=[
            "When running in REST API Server mode (actx --serve), if no Admin is configured yet, access to data endpoints is blocked until an Administrator is initialized!",
            "Each user or token can be restricted to specific workspace scopes (e.g. ['Finance', 'HR'])."
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
            "  View Help  : actx --share --help   OR   /share --help   OR   /share -h   OR   /share /help   OR   /share /h"
        ),
        parameters=[
            "👁️ Viewer Role : Can query AI chat & search vector DB. Cannot add or delete folders.",
            "✏️ Editor Role : Can query AI chat & search vector DB + add their own local folders to the workspace.",
            "👑 Owner Role  : Full control over workspace folders and collaborator permissions.",
            "--help, -h, /help, /h : Display this detailed help page for workspace sharing."
        ],
        examples=[
            "In Chat: /config  ->  Select '🤝 Workspace Sharing & Collaboration'",
            "POST /v1/workspaces/share/invite  ->  {'workspace_name': 'Migration', 'access_level': 'editor'}",
            "POST /v1/workspaces/share/accept  ->  {'invite_code': 'SHARE-MIGR-1234', 'user_email': 'amanda@advocacia.com'}",
            "actx --share --help",
            "In Chat: /share -h   (opens this help manual page)"
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
            "--help, -h, /help, /h : Display this detailed help page for REST server deployment."
        ],
        examples=[
            "actx --serve",
            "actx --serve --port 8000 --host 127.0.0.1",
            "actx --serve --host 0.0.0.0 --port 8000   (Enterprise VPC Mode)",
            "actx --serve --help",
            "In Chat: /serve -h   (opens this help manual page)"
        ],
        tips=[
            "Binding to '--host 0.0.0.0' allows any authorized service on your company VPN/VPC to query AnyContext.",
            "Use Linux systemd service configuration to run AnyContext continuously as a background service."
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
            "--help, -h, /help, /h : Display this detailed help page for MCP server configuration."
        ],
        examples=[
            "actx --mcp",
            "actx --mcp --help",
            "In Chat: /mcp -h   (opens this help manual page)",
            "Claude Desktop Config (claude_desktop_config.json):\n{\n  'mcpServers': {\n    'any-context': {\n      'command': 'actx',\n      'args': ['--mcp']\n    }\n  }\n}"
        ],
        tips=[
            "MCP tools include 'search_workspace_docs', 'query_anycontext_agent', 'list_workspaces', 'create_access_token', and 'get_anycontext_system_documentation'."
        ]
    ),

    "update": HelpPage(
        command="/update",
        aliases=["update", "--update", "/check-update", "--check-update"],
        title="🔄 Non-Blocking 1-Click Self-Updater Engine",
        description=(
            "Checks GitHub Releases for newer AnyContext versions and automatically downloads and installs "
            "the latest release executable. Handles locked Windows executable files seamlessly."
        ),
        syntax=(
            "CLI Launch : actx --update   OR   actx --check-update\n"
            "  In Chat    : type '/update' or '/check-update' during chat\n"
            "  View Help  : actx --update --help   OR   /update --help   OR   /update -h   OR   /update /h"
        ),
        parameters=[
            "/update, --update       : Check and install latest release immediately.",
            "/check-update, --check  : Check if a new version is available without installing.",
            "--help, -h, /help, /h   : Display this detailed help page for the updater."
        ],
        examples=[
            "actx --update",
            "actx --check-update",
            "actx --update --help",
            "In Chat: /update -h   (opens this help manual page)"
        ],
        tips=[
            "Startup update checks are completely non-blocking and will never slow down your CLI launch time."
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
            "  View Help  : actx --reset-memory --help   OR   /reset-memory --help   OR   /reset-memory -h   OR   /reset /h"
        ),
        parameters=[
            "/reset-memory         : Reset memory for active workspace (interactive confirmation).",
            "/config                : Open memory settings menu to perform global or specific memory reset.",
            "--help, -h, /help, /h : Display this detailed help page for memory resets."
        ],
        examples=[
            "In Chat: /reset-memory",
            "In Chat: /reset -h   (opens this help manual page)",
            "actx --reset-memory --help"
        ],
        tips=[
            "Resetting memory only clears session conversation summaries; your indexed document files remain intact!"
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
            "  View Help  : actx --factory-reset --help   OR   /factory-reset --help   OR   /factory-reset -h   OR   /factory-reset /h"
        ),
        parameters=[
            "--factory-reset       : Run factory reset from CLI launch.",
            "/factory-reset        : Run factory reset from inside chat loop.",
            "--help, -h, /help, /h : Display this detailed help page for factory reset."
        ],
        examples=[
            "actx --factory-reset",
            "actx --factory-reset --help",
            "In Chat: /factory-reset -h   (opens this help manual page)"
        ],
        tips=[
            "Use Factory Reset if you want to start fresh or transfer your AnyContext installation to a new environment."
        ]
    ),

    "billing": HelpPage(
        command="/billing",
        aliases=["billing", "--billing", "plans", "--plans", "tiers", "pricing"],
        title="💳 Subscription Plans, Tiers & Capability Matrix",
        description=(
            "AnyContext offers structured subscription tiers tailored from individual local researchers "
            "to multi-user law firms, engineering teams, and enterprise VPC deployments. "
            "Manage and inspect your active plan tier, pricing, capabilities, and license key."
        ),
        syntax=(
            "CLI Launch : actx --billing   OR   actx --plans\n"
            "  REST API   : GET /v1/billing/plans   OR   GET /v1/billing/status\n"
            "  In Chat    : type '/billing' or '/plans' during active chat\n"
            "  View Help  : actx --billing --help   OR   /billing --help   OR   /billing -h   OR   /billing /h"
        ),
        parameters=[
            "/billing, /plans       : Display active subscription tier, pricing table, and feature capabilities.",
            "--billing, --plans     : Launch subscription plan inspector from CLI.",
            "--help, -h, /help, /h  : Display this detailed help page for subscription plans and pricing."
        ],
        examples=[
            "actx --billing",
            "actx --plans --help",
            "In Chat: /billing",
            "In Chat: /plans -h   (opens this help manual page)"
        ],
        tips=[
            "Plan Tiers include: Community ($0), Starter ($12/mo), Pro Multi-Context ($29/mo), Team ($79/mo base + $15/extra seat), and Enterprise ($499/mo).",
            "Community / Open mode is enabled by default for personal local folder use."
        ]
    ),

    "web": HelpPage(
        command="/web",
        aliases=["web", "--web", "scrape", "sitemap", "website", "websites", "urls"],
        title="🌐 Web Scraping & Recurring Polling Engine",
        description=(
            "The Web Scraping engine allows AnyContext to ingest web pages, documentation sites, "
            "and sitemaps into your workspace vector knowledge base. Includes background SHA-256 hash tracking "
            "and recurring polling to automatically re-index updated web pages. Web sources are seamlessly available "
            "to the AI Agent, REST API, CLI, and MCP clients (Cursor/Claude Desktop)."
        ),
        syntax=(
            "In Chat    : /web   OR   /web add <url>   OR   /web list   OR   /web sync\n"
            "  AI Agent   : Tell the AI: 'adicione o site https://... ao workspace'\n"
            "  Config Menu: /config -> 📂 Workspaces -> 🌐 Manage Web URLs\n"
            "  REST API   : POST /v1/workspaces/{name}/web-urls?url=...\n"
            "  MCP Tool   : add_workspace_web_url(workspace=..., url=...)\n"
            "  View Help  : actx --web --help   OR   /web --help   OR   /web -h"
        ),
        parameters=[
            "/web                   : Open interactive web sources management menu for active workspace.",
            "/web add <url>         : Scrape and index a website immediately into the active workspace.",
            "/web list              : List all configured web URLs and polling status for active workspace.",
            "/web sync              : Re-scrape and synchronize all web URLs in active workspace.",
            "url                    : Target web page URL to scrape (e.g. 'https://docs.python.org/3/').",
            "--help, -h, /help, /h  : Display this detailed help page for web scraping."
        ],
        examples=[
            "In Chat: /web",
            "In Chat: /web add https://docs.python.org/3/",
            "In Chat: /web list",
            "In Chat: /web sync",
            "Natural prompt: 'Adicione a documentação da OpenAI https://platform.openai.com/docs ao meu workspace'",
            "curl -X POST http://127.0.0.1:8000/v1/workspaces/MyProject/web-urls?url=https://docs.python.org/3/",
            "actx --web --help"
        ],
        tips=[
            "Web Scraping is supported on 'Pro', 'Team', and 'Enterprise' plan tiers.",
            "Only updated web pages with modified SHA-256 content hashes trigger vector re-indexing, saving API tokens and compute.",
            "Once indexed, scraped web content is instantly searchable alongside local files when using search_db or asking the AI."
        ]
    ),

    "ocr": HelpPage(
        command="/ocr",
        aliases=["ocr", "--ocr", "image", "images", "scan"],
        title="📷 Image & Scanned PDF OCR Text Extraction Daemon",
        description=(
            "The Image OCR engine extracts clean text content from image files (.png, .jpg, .jpeg, .webp, .tiff, .bmp) "
            "and scanned PDF documents, indexing extracted text into ChromaDB vector storage for instant AI search."
        ),
        syntax=(
            "REST API   : POST /v1/ingest/ocr?workspace_name={name}&image_path={path}\n"
            "  View Help  : actx --ocr --help   OR   /ocr --help   OR   /ocr -h   OR   /ocr /help   OR   /ocr /h"
        ),
        parameters=[
            "/ocr, --ocr            : Display Image OCR engine status and parameters.",
            "image_path             : Absolute disk path to image file.",
            "--help, -h, /help, /h  : Display this detailed help page for OCR ingestion."
        ],
        examples=[
            "actx --ocr --help",
            "In Chat: /ocr -h   (opens this help manual page)",
            "curl -X POST 'http://127.0.0.1:8000/v1/ingest/ocr?workspace_name=MyProject&image_path=/path/to/scan.png'"
        ],
        tips=[
            "Image & Scanned PDF OCR is supported on 'Starter', 'Pro', 'Team', and 'Enterprise' plan tiers.",
            "Uses pytesseract / PIL OCR engine with automatic SHA-256 hash deduplication."
        ]
    ),

    "api-keys": HelpPage(
        command="/api-keys",
        aliases=["api-keys", "api_keys", "keys", "apikey", "apikeys", "providers", "--api-keys", "--keys"],
        title="🔑 How to Obtain API Keys & AI Provider Setup Guide",
        description=(
            "AnyContext supports 9+ cloud and local offline AI providers. Below is the quick guide with links and key formats:\n\n"
            "  1. ⚡ OpenAI (ChatGPT, GPT-4o, o1 & Embeddings):\n"
            "     • Dashboard: https://platform.openai.com/api-keys\n"
            "     • Key Format: sk-proj-... or sk-...\n"
            "     • Base URL: https://api.openai.com/v1\n\n"
            "  2. 🧠 Anthropic (Claude 3.5 Sonnet, Opus & Haiku):\n"
            "     • Dashboard: https://console.anthropic.com/settings/keys\n"
            "     • Key Format: sk-ant-...\n"
            "     • Base URL: https://api.anthropic.com/v1\n\n"
            "  3. ♊ Google Gemini (Gemini 1.5 Pro, Flash & Text Embedding 004):\n"
            "     • Dashboard: https://aistudio.google.com/app/apikey\n"
            "     • Key Format: AIzaSy...\n"
            "     • Base URL: https://generativelanguage.googleapis.com/v1beta/openai/\n\n"
            "  4. 🪟 Microsoft Azure OpenAI Service (Enterprise Cloud):\n"
            "     • Portal: https://portal.azure.com (Azure OpenAI Resource -> Keys and Endpoint)\n"
            "     • Base URL: https://<your-resource>.openai.azure.com/openai/deployments/<deployment>\n\n"
            "  5. 🚀 xAI Grok (Grok-2 & Grok-beta):\n"
            "     • Dashboard: https://console.x.ai\n"
            "     • Key Format: xai-...\n"
            "     • Base URL: https://api.x.ai/v1\n\n"
            "  6. 🐉 DeepSeek (DeepSeek V3 & R1 Reasoning):\n"
            "     • Dashboard: https://platform.deepseek.com/api_keys\n"
            "     • Key Format: sk-...\n"
            "     • Base URL: https://api.deepseek.com/v1\n\n"
            "  7. ⚡ Groq Cloud (Ultra-Fast Inference for Llama 3.3, Mixtral & Gemma):\n"
            "     • Dashboard: https://console.groq.com/keys\n"
            "     • Key Format: gsk_...\n"
            "     • Base URL: https://api.groq.com/openai/v1\n\n"
            "  8. 🌐 OpenRouter (Unified Aggregator for 200+ AI Models):\n"
            "     • Dashboard: https://openrouter.ai/keys\n"
            "     • Key Format: sk-or-v1-...\n"
            "     • Base URL: https://openrouter.ai/api/v1\n\n"
            "  9. 🏠 LM Studio & Ollama (100% Free & Local Private Offline LLMs):\n"
            "     • LM Studio: Download from https://lmstudio.ai -> Base URL: http://localhost:1234/v1\n"
            "     • Ollama: Download from https://ollama.com -> Base URL: http://localhost:11434/v1\n"
            "     • No API Key required for local offline servers!"
        ),
        syntax=(
            "In Chat    : /api-keys   OR   /keys   OR   /help -> '🔑 /api-keys'\n"
            "  CLI Launch : actx --api-keys   OR   actx --keys\n"
            "  Config Menu: /config -> '🔑 Manage Saved API Keys' -> '📖 How to Get API Keys'"
        ),
        parameters=[
            "/api-keys, /keys      : View step-by-step guide and portal links for obtaining AI keys.",
            "--api-keys, --keys    : Launch the API keys guide directly from CLI.",
            "/config               : Open configuration menu to save or update your API keys securely in SQLite.",
            "--help, -h, /help, /h : Display this help manual page."
        ],
        examples=[
            "In Chat: /api-keys",
            "In Chat: /keys",
            "In Chat: /help",
            "actx --api-keys",
            "actx --keys"
        ],
        tips=[
            "API keys are stored securely in local SQLite with PBKDF2 masking and never leave your machine.",
            "For 100% private and free usage without API keys or credit cards, run LM Studio or Ollama locally."
        ]
    ),

    "model": HelpPage(
        command="/model",
        aliases=["model", "/m", "-m", "models", "switch-model", "llm"],
        title="🤖 On-The-Fly Inference Model Switching",
        description=(
            "AnyContext allows you to switch your AI inference model dynamically without re-indexing your document vectors. "
            "The model switcher strictly inspects available API keys and only presents models that have valid credentials "
            "configured (OpenAI, Anthropic Claude, Google Gemini, DeepSeek, Groq, xAI Grok, OpenRouter, or Local LM Studio/Ollama)."
        ),
        syntax=(
            "In Chat (Menu)   : /model   OR   /m\n"
            "  In Chat (Direct) : /model <model_name>   (e.g. /model gpt-4o)\n"
            "  In Chat (One-Shot): @<model_name> <message>   (e.g. @claude-3-5-sonnet-20241022 summarize this)\n"
            "  REST API         : POST /v1/chat  with payload {'model': 'gpt-4o', 'message': '...'}\n"
            "  MCP Tool         : query_anycontext_agent(message=..., model='gpt-4o')\n"
            "  View Help        : actx --model --help   OR   /model --help   OR   /model -h"
        ),
        parameters=[
            "/model, /m            : Opens interactive model selector showing only models with active API keys.",
            "/model <name>         : Instantly switches the active inference model for the current chat session.",
            "@<model> <prompt>     : Runs a single prompt with the specified model, then reverts to session default.",
            "--help, -h, /help, /h : Display this help manual page."
        ],
        examples=[
            "In Chat: /model",
            "In Chat: /model gpt-4o",
            "In Chat: /model claude-3-5-sonnet-20241022",
            "In Chat: /model deepseek-chat",
            "In Chat: @gpt-4o Explique os principais pontos dos contratos",
            "curl -X POST http://127.0.0.1:8000/v1/chat -d '{\"message\":\"hello\",\"model\":\"gpt-4o\"}'"
        ],
        tips=[
            "Switching inference models does NOT affect document embeddings or trigger vector database re-indexing.",
            "The active inference model is always clearly visible in the prompt: 'You [Workspace | Model]' and 'AI [Model]'.",
            "To unlock additional models, add their API keys via /config -> '🔑 Manage Saved API Keys'."
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
