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
        syntax="actx -w <workspace_name>   OR   type '/switch' during chat",
        parameters=[
            "-w, --workspace <name> : Directly specify target workspace on CLI launch.",
            "/switch               : Opens interactive menu to choose active workspace."
        ],
        examples=[
            "actx -w MyProject",
            "actx -w HumanResources",
            "In Chat: /switch  ->  Select 'Integration'"
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
            "and manage RBAC User Accounts & Security Tokens."
        ),
        syntax="actx --config   OR   type '/config' during chat",
        parameters=[
            "📂 Workspaces & Folders Management : Add/remove document folder paths.",
            "🤖 AI Models & Base URL            : Select LLM inference and embedding models.",
            "🔑 Manage Saved API Keys           : Store API keys securely in SQLite.",
            "🧠 Memory Compression Settings     : Adjust short-term and meta-summary limits.",
            "🛡️ User Accounts & Security RBAC   : Manage Admin, Team Users, and Bearer Tokens.",
            "💥 Factory Reset                   : Wipe all settings and reset to defaults."
        ],
        examples=[
            "actx --config",
            "In Chat: /config"
        ],
        tips=[
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
        syntax="actx login --server <url>   OR   POST /v1/auth/login   OR   /config -> '🛡️ User Accounts'",
        parameters=[
            "👑 Admin Role   : Full system control (creates users, manages workspaces, API keys, factory reset).",
            "🔬 Analyst Role : Can query AI chat, search vector DB, and trigger folder re-indexing.",
            "👁️ Viewer Role  : Read-only access to query AI chat and search vector DB.",
            "🔑 Bearer Token : Token string format 'actx_sec_...' sent in HTTP Authorization headers."
        ],
        examples=[
            "actx login --server http://192.168.1.50:8000",
            "curl -H 'Authorization: Bearer actx_sec_...' http://localhost:8000/v1/chat",
            "In Chat: /config  ->  Select '🛡️ User Accounts & Security Access Control'"
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
        syntax="POST /v1/workspaces/share/invite   OR   /config -> '🤝 Workspace Sharing'",
        parameters=[
            "👁️ Viewer Role : Can query AI chat & search vector DB. Cannot add or delete folders.",
            "✏️ Editor Role : Can query AI chat & search vector DB + add their own local folders to the workspace.",
            "👑 Owner Role  : Full control over workspace folders and collaborator permissions."
        ],
        examples=[
            "In Chat: /config  ->  Select '🤝 Workspace Sharing & Collaboration'",
            "POST /v1/workspaces/share/invite  ->  {'workspace_name': 'Migration', 'access_level': 'editor'}",
            "POST /v1/workspaces/share/accept  ->  {'invite_code': 'SHARE-MIGR-1234', 'user_email': 'amanda@advocacia.com'}"
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
        syntax="actx --serve [--port PORT] [--host HOST]",
        parameters=[
            "--port <int>  : Specify port number (default: 8000).",
            "--host <str>  : Specify host interface. Use '--host 0.0.0.0' for Enterprise VPC mode listening on all internal network interfaces."
        ],
        examples=[
            "actx --serve",
            "actx --serve --port 8000 --host 127.0.0.1",
            "actx --serve --host 0.0.0.0 --port 8000   (Enterprise VPC Mode)"
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
        syntax="actx --mcp",
        parameters=[
            "--mcp : Runs stdio JSON-RPC 2.0 listener for Claude Desktop and Cursor IDE."
        ],
        examples=[
            "actx --mcp",
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
        syntax="actx --update   OR   type '/update' during chat",
        parameters=[
            "/update, --update       : Check and install latest release immediately.",
            "/check-update, --check  : Check if a new version is available without installing."
        ],
        examples=[
            "actx --update",
            "actx --check-update",
            "In Chat: /update"
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
        syntax="POST /v1/reset-memory   OR   type '/reset-memory' during chat",
        parameters=[
            "/reset-memory : Reset memory for active workspace (interactive confirmation).",
            "/config        : Open memory settings menu to perform global or specific memory reset."
        ],
        examples=[
            "In Chat: /reset-memory",
            "In Chat: /reset"
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
        syntax="actx --factory-reset   OR   POST /v1/factory-reset   OR   type '/factory-reset' during chat",
        parameters=[
            "--factory-reset : Run factory reset from CLI launch.",
            "/factory-reset  : Run factory reset from inside chat loop."
        ],
        examples=[
            "actx --factory-reset",
            "In Chat: /factory-reset"
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
