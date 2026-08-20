from typing import Dict, List, Optional
from any_context.help.models import HelpPage

HELP_REGISTRY: Dict[str, HelpPage] = {
    "switch": HelpPage(
        command="/switch",
        aliases=["switch", "workspace", "/workspace", "-w", "--workspace"],
        title="📂 Workspace Management, Switching & Scope Isolation",
        description=(
            "The /switch (or /workspace) command allows you to switch between workspaces or create new workspaces on the fly. "
            "Each workspace acts as an isolated context scope for your documents, web sources, and long-term memory.\n\n"
            "✨ DECOUPLED WORKSPACE ARCHITECTURE:\n"
            "Creating a workspace and attaching data sources are completely separate actions. You can create an empty workspace "
            "(e.g. for web scraping only, market research, or agent tasks) without being forced to attach a local folder. "
            "You can later attach local folders via '/config' or web documentation via '/web add' at any time."
        ),
        syntax=(
            "CLI Launch (Direct Switch) : actx -w <workspace_name>\n"
            "  In Chat (Interactive Menu) : /switch   OR   /workspace\n"
            "  In Chat (Create / Switch)  : /switch <name>   OR   /workspace add <name>\n"
            "  REST API                   : POST /v1/workspaces?name=<name>\n"
            "  View Help                  : actx --switch --help   OR   /switch --help   OR   /switch -h"
        ),
        parameters=[
            "-w, --workspace <name> : Directly specify target workspace on CLI launch.",
            "/switch, /workspace   : Opens interactive menu with workspace list and '➕ Create New Workspace' option.",
            "/switch <name>        : Switch directly to <name> (creates empty workspace if it doesn't exist).",
            "/workspace add <name> : Create and switch directly to a new workspace.",
            "--help, -h            : Display this detailed help page for /switch."
        ],
        examples=[
            "In Chat: /switch",
            "In Chat: /switch Mercado",
            "In Chat: /workspace create TechDocs",
            "actx -w LegalConsulting",
            "In Chat: /switch -h",
            "curl -X POST 'http://127.0.0.1:8000/v1/workspaces?name=Mercado'"
        ],
        tips=[
            "Workspaces can be 100% web-based: create an empty workspace and use '/web add <url>' to index entire documentation portals.",
            "Workspaces keep your files and projects completely separate, preventing information from one client or project from mixing with another.",
            "You can manage folders inside a workspace anytime using the '/config' -> '📂 Workspaces & Folders' menu."
        ]
    ),

    "sync": HelpPage(
        command="/sync",
        aliases=["sync", "index", "/index", "--sync", "--index", "-s"],
        title="⚡ Workspace Document Synchronization & Temporal Ingestion",
        description=(
            "The /sync (or /index) command performs an incremental scan of all configured folders in the active workspace. "
            "It automatically discovers files across all nested subdirectories, calculates SHA-256 hashes to only index new or modified files, "
            "purges deleted disk files from the ChromaDB vector database, and captures filesystem timestamps (last modified date and creation date) "
            "with 'Local Document' classification for Temporal RAG recency ranking."
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
            "Use '/sync --verbose' whenever you want to inspect exactly which files, timestamps, and subdirectories are currently indexed.",
            "Temporal RAG automatically stamps every local chunk with its filesystem modified date so the AI prioritizes the latest versions."
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
        title="🌐 Interactive Web Discovery, Temporal Metadata & Deep Semantic Crawler",
        description=(
            "The AnyContext Web Crawler is an intelligent, high-performance RAG ingestion engine designed to transform "
            "entire documentation portals, government archives, legal bases, and websites into living AI context.\n\n"
            "🧠 HOW THE CRAWLER & TEMPORAL RAG WORK UNDER THE HOOD:\n"
            "  1. Semantic Path Normalization : Automatically strips file extensions (.html, .htm, .php, .aspx) from URLs "
            "to identify the true semantic directory of the section and capture all child pages.\n"
            "  2. Smart Sitemap & Index Parser : Locates sitemap.xml files and recursively resolves nested 'sitemapindex' catalogs, "
            "tokenizing path keywords to prioritize relevant sub-sitemaps (e.g. 'immigration', 'docs', 'api') and filtering out raw XML.\n"
            "  3. Semantic Proximity & Relevance Ranking : Rather than crawling in random or alphabetical order, URLs are ranked "
            "by relevance to your target topic (Landing Page > Direct Section Children > In-Page Links > Keyword Matches > Generic Domain).\n"
            "  4. 5-Tier Temporal Metadata Extraction : Automatically extracts publication/update dates and classifies content type "
            "(Canonical Service vs Historical News) via Schema.org metadata, visible footer dates, URL regex, HTTP headers, and crawl timestamps.\n"
            "  5. Recency Primacy & Conflict Resolution : The AI agent compares timestamps across chunks, giving absolute ground truth priority "
            "to recent canonical rules and explicit status alerts over older historical press releases.\n"
            "  6. Clean Semantic HTML Extraction : Strips boilerplate (navbars, footers, cookie banners, scripts, ads) while "
            "preserving headings (#, ##), tables, lists, and core article text.\n"
            "  7. IngestionPipeline & Chunking : Chunks content using SentenceSplitter (chunk_size=1024, chunk_overlap=200), calculates embeddings "
            "in micro-batches with OpenAI / Local embeddings, and commits vectors directly into isolated ChromaDB collections.\n"
            "  8. Workspace Isolation : Web vectors are strictly scoped by workspace metadata, ensuring complete privacy."
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
            "/web                   : Open interactive web sources management menu (list, re-sync, or delete sources and purge vectors).",
            "/web add <url>         : Launch interactive site discovery, view Discovery Report, pick scope (Section, Top 50/250/500, Domain), and crawl.",
            "/web list              : List all configured root web URLs, indexed page counts, and last scrape timestamps.",
            "/web sync              : Force re-scrape and synchronize all web URLs in active workspace.",
            "url                    : Target website URL or documentation portal (e.g. 'https://docs.python.org/3/').",
            "--help, -h            : Display this detailed help page for web crawler."
        ],
        examples=[
            "In Chat: /web add https://www.canada.ca/en/immigration-refugees-citizenship.html",
            "In Chat: /web add https://docs.python.org/3/",
            "In Chat: /web add https://platform.openai.com/docs/",
            "In Chat: /web",
            "In Chat: /web list",
            "In Chat: /web sync",
            "Conversational: 'Adicione a documentação https://docs.anthropic.com ao workspace'",
            "curl -X POST 'http://127.0.0.1:8000/v1/workspaces/MyProject/web-urls?url=https://docs.python.org/3/'"
        ],
        tips=[
            "100% unlocked for Community CLI users! You can crawl and index entire documentation portals at zero cost.",
            "Fast discovery automatically checks XML sitemaps and presents estimated page counts and times before crawling.",
            "Proximity ranking ensures that Top 50 / Top 250 options always capture the most relevant guides and forms first.",
            "Multi-threaded concurrent crawler downloads 20 to 50 pages per second with smooth, cursor-safe live progress bar feedback.",
            "Temporal RAG tags chunks with 'Last Modified' dates so the agent never confuses past 2023 press releases with current 2026 rules."
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
            "🤝 Workspace Sharing & Invites     : Manage team collaboration and folder access levels.",
            "🔍 Context Retrieval Density       : Configure RAG presets (Balanced, Turbo, Deep Research).",
            "🤖 AI Models & Base URL            : Select LLM inference and embedding models.",
            "🔑 Manage Saved API Keys           : Store API keys securely in SQLite.",
            "🧠 Memory Compression Settings     : Adjust short-term and meta-summary limits.",
            "💳 Subscription & Payment Plans    : Select pricing tiers and manage license keys.",
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
            "MCP tools include 'search_workspace_docs', 'query_anycontext_agent', 'list_workspaces', 'get_workspace_sources', 'transfer_workspace_source', 'rename_workspace', 'create_access_token', and 'get_anycontext_system_documentation'."
        ]
    ),

    "reset-memory": HelpPage(
        command="/reset-memory",
        aliases=["reset-memory", "/reset", "reset", "memory", "/memory"],
        title="🧹 3-Level Structured Long-Term Memory & Reset Engine",
        description=(
            "AnyContext features a state-of-the-art 3-Level Long-Term Memory Architecture with Structured 5-Dimension High-Fidelity Extraction.\n\n"
            "🧠 THE 5 MEMORY DIMENSIONS SAVED UPON EXIT (/exit or /q):\n"
            "  1. 👤 User Directives & Preferences : Rules, constraints, formatting habits, and explicit decisions.\n"
            "  2. 🏗️ Technical Architecture & Key Decisions : Parameters, constants, algorithms, schemas, and configurations.\n"
            "  3. 📁 Files, Code Symbols & Databases : Files touched, function names, routes, tables, and release tags.\n"
            "  4. 📌 Critical Context & Problem Resolution : Root-cause diagnoses, bug fixes, and verified solutions.\n"
            "  5. 🚀 Pending Tasks & Next Steps : Roadmap milestones, open tasks, and verification protocols.\n\n"
            "The /reset-memory command purges conversation session memory from ChromaDB for the active workspace or globally, "
            "allowing you to start fresh conversations without losing indexed document files or web sources."
        ),
        syntax=(
            "REST API   : POST /v1/reset-memory\n"
            "  In Chat    : type '/reset-memory' or '/reset' during chat\n"
            "  View Help  : actx --reset-memory --help   OR   /reset-memory --help   OR   /reset-memory -h"
        ),
        parameters=[
            "/reset-memory, /reset : Reset conversation memory for the active workspace (interactive confirmation).",
            "/config               : Open memory settings menu to perform global or workspace-specific memory reset.",
            "--help, -h           : Display this detailed help page for memory management."
        ],
        examples=[
            "In Chat: /reset-memory",
            "In Chat: /reset",
            "In Chat: /reset -h",
            "actx --reset-memory --help"
        ],
        tips=[
            "Memory summaries are stored in 1024-token chunks with 200-token overlap, preserving deep multi-step project context across sessions.",
            "Resetting memory only clears session conversation summaries; your indexed document files, web pages, and vectors remain 100% intact!"
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
    ),

    "history": HelpPage(
        command="/history",
        aliases=["history", "/hist", "hist", "/clear-history", "clear-history", "--history"],
        title="📜 Workspace Input History & Arrow Key Navigation",
        description=(
            "AnyContext maintains an isolated, persistent input history for each workspace stored at '~/.any_context/history/{workspace}.history'. "
            "You can seamlessly navigate through previous prompts by pressing [↑] Up Arrow or [↓] Down Arrow during the interactive chat loop. "
            "Switching workspaces immediately switches to that workspace's dedicated history scope."
        ),
        syntax=(
            "Navigate in Chat : Press [↑] Up Arrow / [↓] Down Arrow\n"
            "  View Recent List : /history   OR   /hist\n"
            "  Clear History    : /clear-history   OR   /reset-history\n"
            "  View Help        : /history --help   OR   /history -h"
        ),
        parameters=[
            "/history, /hist       : Display the list of recent prompt inputs for the active workspace.",
            "/clear-history        : Permanently purge the input history file for the active workspace.",
            "[↑] Up / [↓] Down     : Interactive keyboard shortcuts to cycle through past prompts in terminal.",
            "--help, -h            : Display this detailed help page for input history."
        ],
        examples=[
            "In Chat: [↑] (Up Arrow)",
            "In Chat: /history",
            "In Chat: /hist",
            "In Chat: /clear-history",
            "In Chat: /history -h"
        ],
        tips=[
            "History is completely isolated per workspace: prompts in 'Mercado' will never mix with 'Legal'.",
            "History is persistent across sessions: previous prompts remain available when you reopen AnyContext tomorrow.",
            "Passwords and sensitive inputs are masked and never written to history files."
        ]
    ),

    "paste": HelpPage(
        command="/paste",
        aliases=["paste", "/multiline", "multiline", "/mline"],
        title="📋 Multi-line Text Input & Paste Mode",
        description=(
            "AnyContext provides 4 versatile ways to input long prompts, formatted documents, code snippets, or texts with line breaks:\n\n"
            "1. 🌟 DIRECT BRACKETED PASTE: Directly paste (Ctrl+V) multi-line text into the prompt. All lines are held in buffer without premature submission until you press [Enter].\n"
            "2. ⌨️ MANUAL NEWLINES (Ctrl+J or Esc then Enter): Press [Ctrl + J] or [Esc] then [Enter] anytime to insert clean newlines with visual continuation (... ). Note: Alt+Enter is reserved by Windows Terminal for fullscreen toggle.\n"
            "3. 🔤 SHELL-STYLE CONTINUATION (\\): End any line with a backslash '\\' and press [Enter] to continue writing your prompt on the next line without sending.\n"
            "4. 📦 TRIPLE QUOTES BLOCK: Start your prompt with '\"\"\"' (or \"'''\"), type or paste freely across multiple lines, and close with '\"\"\"' to send.\n"
            "5. 📋 DEDICATED /PASTE MODE: Type '/paste' to open an explicit multiline capture buffer. Type '/send' or '\"\"\"' on a new line to finish, or '/cancel' to abort."
        ),
        syntax=(
            "Direct in Chat  : Paste with [Ctrl + V]  OR  press [Ctrl + J]  OR  [Esc] then [Enter]\n"
            "  Line Continuation: End line with '\\' and press [Enter]\n"
            "  Triple Quotes   : \"\"\" <your multi-line text> \"\"\"\n"
            "  Dedicated Mode  : /paste   OR   /multiline\n"
            "  View Help       : /paste --help   OR   /paste -h"
        ),
        parameters=[
            "/paste, /multiline    : Opens explicit multi-line paste capture mode in chat.",
            "\"\"\" ... \"\"\"           : Wraps a multi-line prompt block with triple quote delimiters.",
            "\\ + [Enter]           : Trailing backslash shell-style line continuation.",
            "[Ctrl + J]            : Universal terminal newline shortcut (Linefeed).",
            "[Esc] then [Enter]    : Alternative prompt-toolkit shortcut to insert a newline without submitting.",
            "--help, -h            : Display this detailed help page for multi-line and paste mode."
        ],
        examples=[
            "In Chat: (Press Ctrl+V to paste 50 lines of contract text, then press Enter)",
            "In Chat: Quero saber sobre:\\ [Enter] Startup Visa Canada [Enter]",
            "In Chat: [Ctrl + J] between paragraphs",
            "In Chat: \"\"\"Here is my meeting transcript:\n- Item 1\n- Item 2\"\"\"",
            "In Chat: /paste",
            "In Chat: /paste -h"
        ],
        tips=[
            "Direct Paste (Ctrl+V) preserves all indentation, newlines, and bullet points perfectly.",
            "On Windows Terminal, Alt+Enter is used by the OS for fullscreen, so use [Ctrl + J] or trailing '\\' for line breaks.",
            "You can cancel a multi-line block anytime by typing '/cancel' or pressing Ctrl+C.",
            "Multi-line prompts are preserved in your workspace history (~/.any_context/history/{workspace}.history) and can be recalled with [↑] Up Arrow."
        ]
    ),

    "transfer": HelpPage(
        command="/transfer",
        aliases=["transfer", "/move-source", "move-source", "/transfer-source", "transfer-source"],
        title="🔄 Instant Zero-Cost Source Transfer (Folders & Web Portals)",
        description=(
            "AnyContext allows you to instantly move any local folder or crawled web portal between workspaces in < 50ms without recalculating vector embeddings ($0.00 API cost).\n\n"
            "• Zero API Cost: Reuses already calculated semantic vector embeddings by dynamically updating metadata tags in ChromaDB.\n"
            "• Full Scope Isolation: Immediately decouples the source from the original workspace and links it exclusively to the target workspace.\n"
            "• Tripartite Availability: Fully supported across CLI (`/transfer`), REST API (`POST /v1/workspaces/transfer`), and MCP Server (`transfer_workspace_source` tool for Claude/Cursor/Antigravity).\n"
            "• Collaborative Security: In Team and Enterprise plans, only users with 'Owner' or 'Admin' permissions in both workspaces can execute transfers."
        ),
        syntax=(
            "CLI Interactive   : /transfer   OR   /config (Workspaces -> Transfer)\n"
            "  CLI Direct        : /transfer <source_workspace> <target_workspace> <folder_path_or_url>\n"
            "  REST API Server   : POST /v1/workspaces/transfer\n"
            "  MCP Tool          : transfer_workspace_source\n"
            "  View Help         : /transfer --help   OR   /transfer -h"
        ),
        parameters=[
            "/transfer            : Opens the interactive guided source transfer wizard in CLI.",
            "<source_workspace>   : The origin workspace containing the folder or web portal.",
            "<target_workspace>   : The destination workspace receiving the transferred source.",
            "<folder_path_or_url> : The absolute folder path (e.g. C:\\Docs\\Legal) or website URL (e.g. https://canada.ca).",
            "--help, -h           : Display this detailed help page for source transfers."
        ],
        examples=[
            "In Chat: /transfer",
            "In Chat: /transfer Default Legal C:\\Docs\\Contracts",
            "In Chat: /transfer Default CanadaPortal https://canada.ca",
            "In REST API: POST /v1/workspaces/transfer {\"source_workspace\":\"Default\",\"target_workspace\":\"Legal\",\"source_path_or_url\":\"C:\\Docs\"}",
            "In MCP Server: transfer_workspace_source(source_workspace='Default', target_workspace='Legal', source_type='folder', source_path_or_url='C:\\Docs')",
            "In Chat: /transfer -h"
        ],
        tips=[
            "Transfers execute in sub-50 milliseconds regardless of how many thousands of pages or files are in the source.",
            "Zero tokens are spent: vector embeddings are preserved and moved without calling OpenAI/LLMs.",
            "External AI Agents connected via MCP (Claude Desktop, Cursor IDE) can transfer sources via natural language prompts."
        ]
    ),

    "density": HelpPage(
        command="/density",
        aliases=["density", "preset", "presets", "rag-preset", "top-k", "candidate-pool"],
        title="🔍 Multi-Source RAG Retrieval Density & Presets",
        description=(
            "AnyContext features an enterprise High-Density Multi-Source Retrieval Engine. "
            "When workspaces contain 20+ websites, legal codes, or medical guidelines, the engine retrieves a wide "
            "candidate pool (up to 150 chunks) and applies Source-Fair Round-Robin allocation to guarantee all sources "
            "are represented in the AI context window without single-document monopoly."
        ),
        syntax=(
            "In CLI Config Menu : actx --config -> 🔍 Context Retrieval Density & RAG Presets\n"
            "  REST API Endpoint  : GET /v1/context/settings   AND   POST /v1/context/settings\n"
            "  MCP Server Tool    : get_context_retrieval_settings  AND  set_context_retrieval_preset\n"
            "  View Help          : /density --help   OR   /help density"
        ),
        parameters=[
            "⚡ Balanced (Default)   : Top-40 diversified chunks, Candidate Pool 100, Max 3 chunks per source (TTFT < 300ms, ideal for Cloud models & 20+ portals).",
            "🚀 Turbo               : Top-20 diversified chunks, Candidate Pool 50, Max 2 chunks per source (Ultra-fast TTFT, ideal for LM Studio/Ollama offline).",
            "🔬 Deep Research       : Top-60 diversified chunks, Candidate Pool 150, Max 4 chunks per source (Maximum coverage for massive legal dossiers & 50+ websites).",
            "🛠️ Custom              : Fine-tune exact top_k, candidate_pool_size, and max_chunks_per_source values.",
            "--help, -h             : Display this detailed help page for RAG retrieval presets."
        ],
        examples=[
            "In Config: actx --config -> 🔍 Context Retrieval Density",
            "In REST API: POST /v1/context/settings {\"preset\":\"deep_research\"}",
            "In REST API: POST /v1/context/settings {\"preset\":\"turbo\"}",
            "In MCP: set_context_retrieval_preset(preset='balanced')",
            "In Chat: /help density"
        ],
        tips=[
            "Source-Fair Round-Robin ensures that asking broad questions (e.g. 'Quais programas existem?') captures all 15+ provinces in a single prompt.",
            "Top-40 chunks represent ~5.000 tokens, consuming less than $0.0009 on gpt-4o-mini with sub-300ms first-token latency."
        ]
    ),

    "rename": HelpPage(
        command="/rename",
        aliases=["rename", "workspace-rename", "renamews", "mv-ws"],
        title="✏️ Atomic Workspace Renaming & Vector Migration",
        description=(
            "Renames an existing workspace across all SQLite relational tables (folders, web URLs, crawled pages, permissions, session memory) "
            "and updates ChromaDB vector metadata in sub-50 milliseconds with zero token expenditure ($0.00 API cost)."
        ),
        syntax=(
            "In Chat (Interactive) : /rename\n"
            "  In Chat (Direct Args)  : /rename <old_workspace> <new_workspace>\n"
            "  In Chat (Full Syntax)  : /workspace rename <old_workspace> <new_workspace>\n"
            "  In CLI Config Menu     : actx --config -> 📂 Workspaces & Folders -> ✏️ Rename Workspace\n"
            "  REST API Endpoint      : POST /v1/workspaces/rename {\"old_name\": \"...\", \"new_name\": \"...\"}\n"
            "  MCP Server Tool        : rename_workspace(old_name='...', new_name='...')\n"
            "  View Help              : /rename --help   OR   /rename -h"
        ),
        parameters=[
            "/rename                  : Opens the interactive guided workspace rename wizard.",
            "<old_workspace>         : The current name of the workspace to rename.",
            "<new_workspace>         : The new desired name for the workspace.",
            "--help, -h               : Display this detailed help page for workspace renaming."
        ],
        examples=[
            "In Chat: /rename",
            "In Chat: /rename TesteDestino CanadaLegal",
            "In Chat: /workspace rename Default MainHQ",
            "In REST API: POST /v1/workspaces/rename {\"old_name\":\"TesteDestino\",\"new_name\":\"CanadaLegal\"}",
            "In MCP Server: rename_workspace(old_name='TesteDestino', new_name='CanadaLegal')",
            "In Chat: /rename -h"
        ],
        tips=[
            "Renaming executes atomically in sub-50ms regardless of how many thousands of document chunks exist.",
            "Zero tokens are spent: vector embeddings are preserved without calling OpenAI/LLMs.",
            "If you rename your active workspace, your current session prompt automatically updates to the new name."
        ]
    ),
    "sources": HelpPage(
        command="/sources",
        aliases=["sources", "/sources", "workspace sources", "/workspace sources", "workspace-sources", "listsources", "sources-list", "workspace-list"],
        title="📁 Workspace Multi-Source Listing & Tripartite Parity",
        description=(
            "The /sources command lists all data sources attached to your workspaces in a unified, UI-agnostic format. "
            "AnyContext supports 3 core source types: Local Folders, Web Portals/URLs, and Cloud Drives (Google Drive, OneDrive, S3, Dropbox).\n\n"
            "✨ TRIPARTITE PARITY & UI-AGNOSTIC ARCHITECTURE:\n"
            "Workspace and source listing is fully synchronized across all 3 interfaces: CLI terminal (/sources, /config), "
            "REST API (GET /v1/workspaces, GET /v1/workspaces/{name}/sources), and MCP Protocol (list_workspaces, get_workspace_sources)."
        ),
        syntax=(
            "In Chat (Active Workspace) : /sources   OR   /workspace sources\n"
            "  In Chat (All Workspaces)    : /sources all   OR   /workspace list\n"
            "  In CLI Config Menu          : actx --config -> 📂 Workspaces & Folders -> 📋 List Workspaces & Folders\n"
            "  REST API Endpoints          : GET /v1/workspaces\n"
            "                                GET /v1/workspaces/{name}\n"
            "                                GET /v1/workspaces/{name}/sources\n"
            "  MCP Server Tools            : list_workspaces()\n"
            "                                get_workspace_sources(workspace='...')\n"
            "  View Help                   : /sources --help   OR   /sources -h"
        ),
        parameters=[
            "/sources                 : Lists all local folders, web portals, and cloud drives in the active workspace.",
            "/sources all             : Lists all sources across every configured workspace.",
            "--help, -h               : Display this detailed help page for /sources."
        ],
        examples=[
            "In Chat: /sources",
            "In Chat: /sources all",
            "In Chat: /workspace sources",
            "curl -H 'Authorization: Bearer actx_sec_...' http://127.0.0.1:8000/v1/workspaces",
            "curl -H 'Authorization: Bearer actx_sec_...' http://127.0.0.1:8000/v1/workspaces/Default/sources",
            "In MCP Server: list_workspaces()",
            "In MCP Server: get_workspace_sources(workspace='Legal')"
        ],
        tips=[
            "Web portals display page count, crawl scope, and last scraped timestamp.",
            "Cloud drives (Google Drive, OneDrive, S3, Dropbox) display connection status and sync metadata.",
            "REST API responses return both legacy 'paths' and rich 'folders', 'web_sources', 'cloud_drives', and 'sources' arrays."
        ]
    ),
    "mode": HelpPage(
        command="/mode",
        aliases=["mode", "answer-mode", "answermode", "grounding", "grounding-mode", "am", "/answer-mode", "/grounding", "/am"],
        title="🎛️ AI Grounding & Answer Mode Manager (Hybrid, Strict, Proactive)",
        description="Dynamically configures how the AI Agent reasons, cites sources, and handles external knowledge. Switch seamlessly between Strict (100% verified facts, zero speculation for audits/legal), Hybrid (balanced default with dual-layer labeled suggestions), and Proactive (broad synthesis & research recommendations).",
        syntax="/mode [hybrid | strict | proactive]",
        parameters=[
            "/mode                    : Opens the interactive Questionary selector to change grounding mode.",
            "/mode hybrid             : Switches to Hybrid mode (Layer 1 workspace facts + Layer 2 labeled external suggestions).",
            "/mode strict             : Switches to Strict mode (100% verified facts from indexed docs, zero speculation).",
            "/mode proactive          : Switches to Proactive mode (Broad synthesis, insights, and /web add recommendations).",
            "--help, -h               : Display this detailed help page for /mode."
        ],
        examples=[
            "In Chat: /mode",
            "In Chat: /mode strict",
            "In Chat: /mode hybrid",
            "In Chat: /mode proactive",
            "In Chat: /answer-mode strict",
            "REST API: GET /v1/context/mode",
            "REST API: POST /v1/context/mode -d '{\"mode\": \"strict\"}'",
            "REST API: POST /v1/chat -d '{\"message\": \"...\", \"grounding_mode\": \"strict\"}'",
            "In MCP Server: set_grounding_mode(mode='strict')",
            "In MCP Server: get_grounding_mode()",
            "In MCP Server: query_anycontext_agent(message='...', grounding_mode='strict')"
        ],
        tips=[
            "The active grounding mode is displayed dynamically in your prompt: You [Workspace | Model | Mode]:",
            "Use 'Strict' for contracts, compliance audits, medical/legal queries, and formal reports.",
            "Use 'Hybrid' (default) for everyday consulting, software development, and document analysis.",
            "Use 'Proactive' for market research, brainstorm sessions, and strategy planning.",
            "Changing the mode immediately invalidates the agent cache and reconfigures system directives without restarting the session."
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
