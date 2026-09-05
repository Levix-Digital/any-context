"""
Command Registry - Canonical catalog of all 23 AnyContext slash commands.
Shared between CLI, TUI, RPC Bridge, and consumer interfaces.
"""

from dataclasses import dataclass, field
from typing import List, Optional


@dataclass
class CommandMeta:
    name: str
    args: str
    description: str
    category: str
    direct_execution: bool = True
    aliases: List[str] = field(default_factory=list)


COMMANDS_REGISTRY: List[CommandMeta] = [
    CommandMeta(
        name="/switch",
        args="[workspace]",
        description="Switch, list, or select active workspace",
        category="Workspace",
        direct_execution=True,
        aliases=["/workspace"]
    ),
    CommandMeta(
        name="/model",
        args="<name>",
        description="Change active AI inference model",
        category="Model",
        direct_execution=False,
        aliases=["/m"]
    ),
    CommandMeta(
        name="/mode",
        args="[strategy]",
        description="Select AI Grounding Strategy mode",
        category="AI Grounding",
        direct_execution=True,
        aliases=["/grounding"]
    ),
    CommandMeta(
        name="/web-search",
        args="[on|off]",
        description="Toggle real-time workspace Web Search",
        category="Web Search",
        direct_execution=True,
        aliases=["/search"]
    ),
    CommandMeta(
        name="/sync",
        args="[--force|--status]",
        description="Synchronize local folders and web sources",
        category="Sources",
        direct_execution=True,
        aliases=["/reindex"]
    ),
    CommandMeta(
        name="/sources",
        args="[--all|--delete]",
        description="List or delete indexed documents and web portals in workspace",
        category="Sources",
        direct_execution=True,
        aliases=["/list-sources"]
    ),
    CommandMeta(
        name="/folder",
        args="--add <path>",
        description="Add, list, or remove local folder from workspace",
        category="Sources",
        direct_execution=False,
        aliases=["/dir"]
    ),
    CommandMeta(
        name="/web",
        args="--add <url>",
        description="Add, list, or crawl documentation portal or web URL",
        category="Sources",
        direct_execution=False,
        aliases=["/url"]
    ),
    CommandMeta(
        name="/transfer",
        args="<from_ws> <to_ws> <item>",
        description="Transfer source to another workspace in <50ms ($0.00)",
        category="Sources",
        direct_execution=False,
        aliases=["/move-source"]
    ),
    CommandMeta(
        name="/link",
        args="<source> [ws]",
        description="Link shared source across workspaces",
        category="Sources",
        direct_execution=False,
        aliases=[]
    ),
    CommandMeta(
        name="/unlink",
        args="<source>",
        description="Unlink shared source from workspace",
        category="Sources",
        direct_execution=False,
        aliases=[]
    ),
    CommandMeta(
        name="/shared",
        args="",
        description="List reusable indexed shared sources",
        category="Sources",
        direct_execution=True,
        aliases=[]
    ),
    CommandMeta(
        name="/rename",
        args="<old> <new>",
        description="Rename custom workspace and migrate vector records",
        category="Workspace",
        direct_execution=False,
        aliases=[]
    ),
    CommandMeta(
        name="/config",
        args="",
        description="Open interactive configuration and settings wizard",
        category="System",
        direct_execution=True,
        aliases=["/settings"]
    ),
    CommandMeta(
        name="/key",
        args="<provider> <api-key>",
        description="Configure AI provider API keys",
        category="System",
        direct_execution=False,
        aliases=["/api-key"]
    ),
    CommandMeta(
        name="/models",
        args="--list",
        description="Display catalog of available AI models",
        category="Model",
        direct_execution=True,
        aliases=[]
    ),
    CommandMeta(
        name="/billing",
        args="",
        description="Manage subscription tier and plans",
        category="System",
        direct_execution=True,
        aliases=["/plan", "/pricing"]
    ),
    CommandMeta(
        name="/reset-memory",
        args="",
        description="Reset long-term session memory database",
        category="Memory",
        direct_execution=True,
        aliases=["/forget"]
    ),
    CommandMeta(
        name="/clear",
        args="",
        description="Clear visual chat history view",
        category="Chat",
        direct_execution=True,
        aliases=["/cls"]
    ),
    CommandMeta(
        name="/paste",
        args="",
        description="Enter dedicated multi-line capture mode",
        category="Chat",
        direct_execution=True,
        aliases=["/multiline", "/mline"]
    ),
    CommandMeta(
        name="/help",
        args="[command]",
        description="Display interactive help documentation",
        category="Help",
        direct_execution=True,
        aliases=["/menu", "/commands", "/slash"]
    ),
    CommandMeta(
        name="/version",
        args="",
        description="Display AnyContext version and build info",
        category="System",
        direct_execution=True,
        aliases=["/v"]
    ),
    CommandMeta(
        name="/check-update",
        args="",
        description="Check if a newer AnyContext release is available on GitHub",
        category="System",
        direct_execution=True,
        aliases=["/check"]
    ),
    CommandMeta(
        name="/update",
        args="[version]",
        description="Download and install latest AnyContext release",
        category="System",
        direct_execution=True,
        aliases=["/self-update", "/upgrade"]
    ),
    CommandMeta(
        name="/inspect",
        args="[limit]",
        description="Inspect vector database chunks and session memory",
        category="Sources",
        direct_execution=True,
        aliases=["/chunks", "/lance"]
    ),
    CommandMeta(
        name="/density",
        args="[level]",
        description="Configure UI compact/comfortable visual density",
        category="System",
        direct_execution=True,
        aliases=[]
    ),
    CommandMeta(
        name="/history",
        args="[clear]",
        description="View or clear recent conversation history",
        category="Chat",
        direct_execution=True,
        aliases=["/clear-history"]
    ),
    CommandMeta(
        name="/menu",
        args="",
        description="Open interactive Slash Command Palette",
        category="Help",
        direct_execution=True,
        aliases=["/palette"]
    ),
    CommandMeta(
        name="/logs",
        args="[limit]",
        description="Display recent system observability and execution logs",
        category="System",
        direct_execution=True,
        aliases=["/log"]
    ),
    CommandMeta(
        name="/diagnostics",
        args="",
        description="Inspect system health, Bun runtime, database, and latency metrics",
        category="System",
        direct_execution=True,
        aliases=["/diag", "/perf", "/health"]
    ),
    CommandMeta(
        name="/onboarding",
        args="",
        description="Launch first-time AI onboarding and API key setup wizard",
        category="System",
        direct_execution=True,
        aliases=["/setup"]
    ),
    CommandMeta(
        name="/exit",
        args="",
        description="Save session memory and exit",
        category="System",
        direct_execution=True,
        aliases=["/quit", "/q"]
    )
]



def find_command_meta(token: str) -> Optional[CommandMeta]:
    """Finds command metadata by name or alias."""
    clean = token.strip().lower()
    base = clean.split("@")[0] if "@" in clean else clean
    for meta in COMMANDS_REGISTRY:
        if meta.name.lower() == clean or clean in [a.lower() for a in meta.aliases]:
            return meta
        if "@" in clean and (meta.name.lower() == base or base in [a.lower() for a in meta.aliases]):
            return meta
    return None
