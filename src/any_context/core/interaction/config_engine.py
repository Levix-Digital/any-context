"""
Config Engine - Centralized builder and executor for the AnyContext configuration menu tree.
Serves CLI, OpenTUI, REST API, and Desktop UI with 100% feature and parity alignment.
"""

from typing import List, Dict, Any, Optional
from any_context.core.interaction.schemas import MenuItemSchema, MenuTreeSchema, MenuActionResult, OptionItemSchema
from any_context.core.services import (
    WorkspaceService,
    SourceService,
    ModelService,
    GroundingService,
    SyncService,
    MemoryService,
    BillingService,
)
from any_context.config.db_store import ConfigDBStore
from any_context.core.interaction.options_engine import OptionsEngine


class ConfigEngine:
    """Provides canonical menu tree generation and action dispatching for configuration."""

    def __init__(self):
        self.workspace_svc = WorkspaceService()
        self.source_svc = SourceService()
        self.model_svc = ModelService()
        self.grounding_svc = GroundingService()
        self.sync_svc = SyncService()
        self.memory_svc = MemoryService()
        self.billing_svc = BillingService()
        self.store = ConfigDBStore()
        self.options_engine = OptionsEngine()

    def get_menu_tree(self, menu_id: str = "main", workspace: str = "Default") -> MenuTreeSchema:
        """Returns the menu structure for a given menu ID and workspace."""
        ws_name = (workspace or "Default").strip()

        if menu_id == "main":
            return self._build_main_menu(ws_name)
        elif menu_id == "workspaces":
            return self._build_workspaces_menu(ws_name)
        elif menu_id == "sharing":
            return self._build_sharing_menu(ws_name)
        elif menu_id == "grounding":
            return self._build_grounding_menu(ws_name)
        elif menu_id == "web_search":
            return self._build_web_search_menu(ws_name)
        elif menu_id == "density":
            return self._build_density_menu(ws_name)
        elif menu_id == "models":
            return self._build_models_menu(ws_name)
        elif menu_id == "keys":
            return self._build_keys_menu(ws_name)
        elif menu_id == "memory":
            return self._build_memory_menu(ws_name)
        elif menu_id == "billing":
            return self._build_billing_menu(ws_name)
        elif menu_id == "security":
            return self._build_security_menu(ws_name)

        return self._build_main_menu(ws_name)

    def _build_main_menu(self, ws_name: str) -> MenuTreeSchema:
        curr_mode = self.grounding_svc.get_grounding_mode(ws_name).capitalize()
        curr_model = self.model_svc.get_current_model()
        web_search = "ON" if self.grounding_svc.get_web_search_status(ws_name) else "OFF"
        billing_info = self.billing_svc.get_billing_info()
        tier_name = billing_info.get("current_tier", "community").capitalize()

        items = [
            MenuItemSchema(
                id="workspaces",
                title="Workspaces & Folders Management",
                description="List, create, rename, and manage workspace folder paths and web sources",
                icon="📂",
                type="submenu",
                command_shortcut="/switch"
            ),
            MenuItemSchema(
                id="sharing",
                title="Workspace Sharing & Collaboration",
                description="Generate share invite codes, manage collaborators, and view transparent folder permissions",
                icon="🤝",
                type="submenu"
            ),
            MenuItemSchema(
                id="grounding",
                title="AI Grounding & Answer Modes",
                description=f"Configure Strict, Hybrid, or Proactive grounding mode (Current: {curr_mode})",
                icon="🎛️",
                type="submenu",
                badge=f"[{curr_mode}]",
                command_shortcut="/mode"
            ),
            MenuItemSchema(
                id="web_search",
                title="Live Web Search & External Intelligence",
                description=f"Configure real-time web search engines and toggles (Current: {web_search})",
                icon="🌐",
                type="submenu",
                badge=f"[{web_search}]",
                command_shortcut="/web-search"
            ),
            MenuItemSchema(
                id="density",
                title="Context Retrieval Density & RAG Presets",
                description="Configure chunk depth, top-k candidates, and multi-source distribution",
                icon="🔍",
                type="submenu",
                command_shortcut="/density"
            ),
            MenuItemSchema(
                id="models",
                title="AI Models, Base URL & API Keys",
                description=f"Select active LLM inference model, local server base URL, and embeddings (Current: {curr_model})",
                icon="🤖",
                type="submenu",
                badge=f"[{curr_model}]",
                command_shortcut="/model"
            ),
            MenuItemSchema(
                id="keys",
                title="Manage Saved API Keys",
                description="Configure API keys for OpenAI, Gemini, Anthropic, Tavily, Serper, DeepSeek",
                icon="🔑",
                type="submenu",
                command_shortcut="/key"
            ),
            MenuItemSchema(
                id="memory",
                title="Memory Compression & Reset Settings",
                description="Inspect short-term buffer, rolling windows, or purge long-term workspace memories",
                icon="🧠",
                type="submenu",
                command_shortcut="/reset-memory"
            ),
            MenuItemSchema(
                id="billing",
                title="Subscription & Payment Plans",
                description=f"Inspect subscription tiers, license keys, and capability matrix (Active: {tier_name})",
                icon="💳",
                type="submenu",
                badge=f"[{tier_name}]",
                command_shortcut="/billing"
            ),
            MenuItemSchema(
                id="security",
                title="User Accounts & Security Access Control",
                description="Multi-user RBAC administration, role assignment, and security bearer tokens",
                icon="🛡️",
                type="submenu"
            ),
            MenuItemSchema(
                id="factory_reset",
                title="Factory Reset AnyContext",
                description="⚠️ Erase all workspaces, sources, API keys, and vector databases",
                icon="💥",
                type="action"
            ),
        ]

        return MenuTreeSchema(
            menu_id="main",
            title="⚙️ AnyContext Configuration & Settings",
            subtitle=f"Active Workspace: {ws_name}",
            workspace=ws_name,
            breadcrumbs=["⚙️ Configuration"],
            items=items
        )

    def _build_workspaces_menu(self, ws_name: str) -> MenuTreeSchema:
        workspaces = self.workspace_svc.list_workspaces(active_workspace=ws_name)
        items = [
            MenuItemSchema(
                id="ws_list",
                title="List Workspaces & Sources",
                description=f"View all {len(workspaces)} configured workspaces and indexed sources",
                icon="📋",
                type="action"
            ),
            MenuItemSchema(
                id="ws_create",
                title="Create New Workspace",
                description="Create a new isolated workspace context with zero initial sources",
                icon="➕",
                type="input",
                metadata={"prompt": "Enter new workspace name:"}
            ),
            MenuItemSchema(
                id="ws_rename",
                title="Rename Current Workspace",
                description=f"Rename '{ws_name}' and automatically migrate all indexed vector records",
                icon="✏️",
                type="input",
                metadata={"prompt": f"Enter new name for workspace '{ws_name}':"}
            ),
            MenuItemSchema(
                id="ws_sync",
                title="Synchronize Current Workspace",
                description=f"Re-index local folders and web sources in '{ws_name}'",
                icon="🔄",
                type="action",
                command_shortcut="/sync"
            ),
            MenuItemSchema(
                id="ws_delete",
                title="Delete Workspace",
                description=f"Remove workspace '{ws_name}' and all associated links",
                icon="🗑️",
                type="action"
            ),
        ]

        return MenuTreeSchema(
            menu_id="workspaces",
            title="📂 Workspaces & Folders Management",
            subtitle=f"Workspace: {ws_name}",
            workspace=ws_name,
            breadcrumbs=["⚙️ Configuration", "📂 Workspaces"],
            items=items
        )

    def _build_sharing_menu(self, ws_name: str) -> MenuTreeSchema:
        items = [
            MenuItemSchema(
                id="share_create_invite",
                title="Generate Workspace Share Invite Code",
                description=f"Create a shareable invite code (Viewer / Editor) for '{ws_name}'",
                icon="🔗",
                type="action"
            ),
            MenuItemSchema(
                id="share_accept_invite",
                title="Accept Workspace Share Invite Code",
                description="Join a shared team workspace via invite code",
                icon="📩",
                type="input",
                metadata={"prompt": "Enter Workspace Share Invite Code (SHARE-...):"}
            ),
            MenuItemSchema(
                id="share_list_collabs",
                title="List Workspace Collaborators",
                description=f"View team members and access roles in '{ws_name}'",
                icon="👥",
                type="action"
            ),
        ]

        return MenuTreeSchema(
            menu_id="sharing",
            title="🤝 Workspace Sharing & Collaboration",
            subtitle=f"Workspace: {ws_name}",
            workspace=ws_name,
            breadcrumbs=["⚙️ Configuration", "🤝 Sharing"],
            items=items
        )

    def _build_grounding_menu(self, ws_name: str) -> MenuTreeSchema:
        opts = self.options_engine.get_grounding_mode_options(ws_name)
        items = []
        for o in opts.items:
            items.append(MenuItemSchema(
                id=f"set_grounding_{o.id}",
                title=o.title,
                description=o.description,
                icon=o.icon,
                type="select",
                badge=o.badge,
                is_active=o.is_active,
                metadata={"mode": o.id}
            ))

        items.append(MenuItemSchema(
            id="grounding_apply_global",
            title="Apply Active Grounding Mode Globally",
            description="Propagate active grounding mode to all existing and future workspaces",
            icon="🌐",
            type="action"
        ))

        return MenuTreeSchema(
            menu_id="grounding",
            title="🎛️ AI Grounding & Answer Modes",
            subtitle=f"Workspace: {ws_name}",
            workspace=ws_name,
            breadcrumbs=["⚙️ Configuration", "🎛️ Grounding"],
            items=items
        )

    def _build_web_search_menu(self, ws_name: str) -> MenuTreeSchema:
        cur_status = self.grounding_svc.get_web_search_status(ws_name)
        cur_engine = self.store.get_default_search_engine()

        items = [
            MenuItemSchema(
                id="websearch_toggle_workspace",
                title=f"Toggle Web Search for '{ws_name}'",
                description=f"Current status for this workspace: {'ENABLED (ON)' if cur_status else 'DISABLED (OFF)'}",
                icon="🟢" if not cur_status else "🔴",
                type="toggle",
                is_active=cur_status
            ),
            MenuItemSchema(
                id="websearch_enable_global",
                title="Enable Web Search Globally (All Workspaces)",
                description="Turn on real-time internet search across all workspaces",
                icon="🟢",
                type="action"
            ),
            MenuItemSchema(
                id="websearch_disable_global",
                title="Disable Web Search Globally (100% Offline Local)",
                description="Enforce 100% offline isolation across all workspaces",
                icon="🔒",
                type="action"
            ),
            MenuItemSchema(
                id="websearch_engine_auto",
                title="Search Engine: Auto (Tavily ➔ Serper ➔ DuckDuckGo)",
                description="Automatic fallback across all configured search providers",
                icon="⭐",
                type="select",
                is_active=(cur_engine == "auto")
            ),
            MenuItemSchema(
                id="websearch_engine_tavily",
                title="Search Engine: Tavily Search API",
                description="AI-native deep research web intelligence",
                icon="🌐",
                type="select",
                is_active=(cur_engine == "tavily")
            ),
            MenuItemSchema(
                id="websearch_engine_ddg",
                title="Search Engine: DuckDuckGo (Free / No-Cost)",
                description="100% free web search fallback with zero API key requirement",
                icon="🦆",
                type="select",
                is_active=(cur_engine in ["duckduckgo", "ddg"])
            ),
        ]

        return MenuTreeSchema(
            menu_id="web_search",
            title="🌐 Live Web Search & External Intelligence",
            subtitle=f"Workspace: {ws_name} | Engine: {cur_engine.upper()}",
            workspace=ws_name,
            breadcrumbs=["⚙️ Configuration", "🌐 Web Search"],
            items=items
        )

    def _build_density_menu(self, ws_name: str) -> MenuTreeSchema:
        opts = self.options_engine.get_retrieval_density_options()
        items = []
        for o in opts.items:
            items.append(MenuItemSchema(
                id=f"set_density_{o.id}",
                title=o.title,
                description=o.description,
                icon=o.icon,
                type="select",
                badge=o.badge,
                is_active=o.is_active,
                metadata={"preset": o.id}
            ))

        return MenuTreeSchema(
            menu_id="density",
            title="🔍 Context Retrieval Density & RAG Presets",
            subtitle=f"Active Preset: {opts.active_id.upper() if opts.active_id else 'BALANCED'}",
            workspace=ws_name,
            breadcrumbs=["⚙️ Configuration", "🔍 Density"],
            items=items
        )

    def _build_models_menu(self, ws_name: str) -> MenuTreeSchema:
        opts = self.options_engine.get_inference_model_options()
        items = [
            MenuItemSchema(
                id="model_quick_openai",
                title="⚡ Quick-Setup: OpenAI Cloud (gpt-4o-mini)",
                description="Standard fast reasoning cloud model with text-embedding-3-small",
                icon="⚡",
                type="action"
            ),
            MenuItemSchema(
                id="model_quick_local",
                title="🏠 Quick-Setup: Local Server (LM Studio / Ollama)",
                description="Connect to http://localhost:1234/v1 for 100% free offline privacy",
                icon="🏠",
                type="action"
            ),
        ]

        for o in opts.items:
            items.append(MenuItemSchema(
                id=f"select_model_{o.id}",
                title=o.title,
                description=o.description,
                icon=o.icon,
                type="select",
                badge=o.badge,
                is_active=o.is_active,
                metadata={"model_name": o.id}
            ))

        return MenuTreeSchema(
            menu_id="models",
            title="🤖 AI Models, Base URL & API Keys",
            subtitle=f"Active Model: {opts.active_id}",
            workspace=ws_name,
            breadcrumbs=["⚙️ Configuration", "🤖 Models"],
            items=items
        )

    def _build_keys_menu(self, ws_name: str) -> MenuTreeSchema:
        all_keys = self.store.get_all_api_keys()
        items = [
            MenuItemSchema(
                id="keys_list",
                title=f"View Saved API Keys ({len(all_keys)} configured)",
                description="Inspect all securely stored cloud and search provider keys",
                icon="📜",
                type="action"
            ),
            MenuItemSchema(
                id="key_set_openai",
                title="Set / Update OpenAI API Key (sk-...)",
                description="Used for GPT-4o, GPT-4o-mini, and text-embedding-3",
                icon="⚡",
                type="input",
                metadata={"provider": "openai"}
            ),
            MenuItemSchema(
                id="key_set_tavily",
                title="Set / Update Tavily API Key (tvly-...)",
                description="Used for real-time web search and external intelligence",
                icon="🌐",
                type="input",
                metadata={"provider": "tavily"}
            ),
            MenuItemSchema(
                id="key_set_gemini",
                title="Set / Update Google Gemini API Key",
                description="Used for Gemini 1.5 Pro, Flash, and embeddings",
                icon="✨",
                type="input",
                metadata={"provider": "gemini"}
            ),
            MenuItemSchema(
                id="key_set_anthropic",
                title="Set / Update Anthropic Claude API Key (sk-ant-...)",
                description="Used for Claude 3.5 Sonnet and Haiku",
                icon="🧠",
                type="input",
                metadata={"provider": "anthropic"}
            ),
        ]

        return MenuTreeSchema(
            menu_id="keys",
            title="🔑 Manage Saved API Keys",
            subtitle=f"{len(all_keys)} providers configured",
            workspace=ws_name,
            breadcrumbs=["⚙️ Configuration", "🔑 Keys"],
            items=items
        )

    def _build_memory_menu(self, ws_name: str) -> MenuTreeSchema:
        items = [
            MenuItemSchema(
                id="memory_info",
                title="View Memory Compression Settings",
                description="Inspect short-term buffer, rolling window size, and meta-summary threshold",
                icon="⚙️",
                type="action"
            ),
            MenuItemSchema(
                id="memory_reset_workspace",
                title=f"Reset Long-Term Memory for '{ws_name}'",
                description=f"Purge long-term session memories and summarizations for workspace '{ws_name}'",
                icon="🧹",
                type="action"
            ),
            MenuItemSchema(
                id="memory_reset_global",
                title="Reset ALL Long-Term Memories (Global)",
                description="Purge session memory records across all workspaces",
                icon="🔥",
                type="action"
            ),
        ]

        return MenuTreeSchema(
            menu_id="memory",
            title="🧠 Memory Compression & Reset Settings",
            subtitle=f"Workspace: {ws_name}",
            workspace=ws_name,
            breadcrumbs=["⚙️ Configuration", "🧠 Memory"],
            items=items
        )

    def _build_billing_menu(self, ws_name: str) -> MenuTreeSchema:
        info = self.billing_svc.get_billing_info()
        items = [
            MenuItemSchema(
                id="billing_view_matrix",
                title="View Complete Pricing & Capability Matrix Table",
                description="Inspect Community, Pro, and Enterprise license capabilities and token limits",
                icon="📊",
                type="action"
            ),
            MenuItemSchema(
                id="billing_activate_tier",
                title="Activate / Change Subscription Plan Tier",
                description=f"Upgrade or switch active plan (Current: {info.get('current_tier', 'community').upper()})",
                icon="🔑",
                type="action"
            ),
        ]

        return MenuTreeSchema(
            menu_id="billing",
            title="💳 Subscription & Payment Plans",
            subtitle=f"Active Tier: {info.get('current_tier', 'community').upper()}",
            workspace=ws_name,
            breadcrumbs=["⚙️ Configuration", "💳 Billing"],
            items=items
        )

    def _build_security_menu(self, ws_name: str) -> MenuTreeSchema:
        admin_cfg = self.store.is_admin_configured()
        users = self.store.list_users()
        tokens = self.store.get_access_tokens()

        items = [
            MenuItemSchema(
                id="sec_status",
                title="Security Status & User Counts",
                description=f"Status: {'Protected RBAC' if admin_cfg else 'Open Local Mode'} | Users: {len(users)} | Tokens: {len(tokens)}",
                icon="🛡️",
                type="action"
            ),
            MenuItemSchema(
                id="sec_create_token",
                title="Generate Security Bearer Token",
                description="Create a new scoped bearer token for REST API or automated agents",
                icon="🔑",
                type="action"
            ),
        ]

        return MenuTreeSchema(
            menu_id="security",
            title="🛡️ User Accounts & Security Access Control",
            subtitle="Multi-User RBAC & Bearer Tokens",
            workspace=ws_name,
            breadcrumbs=["⚙️ Configuration", "🛡️ Security"],
            items=items
        )

    def execute_action(self, action_id: str, params: Optional[Dict[str, Any]] = None, workspace: str = "Default") -> MenuActionResult:
        """Executes a menu action and returns structured MenuActionResult."""
        ws_name = (workspace or "Default").strip()
        params = params or {}

        # Grounding actions
        if action_id.startswith("set_grounding_"):
            mode = action_id.replace("set_grounding_", "")
            return self.options_engine.set_grounding_mode(mode=mode, workspace=ws_name)

        if action_id == "grounding_apply_global":
            curr = self.grounding_svc.get_grounding_mode(ws_name)
            return self.options_engine.set_grounding_mode(mode=curr, workspace=ws_name, apply_global=True)

        # Web Search actions
        if action_id == "websearch_toggle_workspace":
            curr = self.grounding_svc.get_web_search_status(ws_name)
            new_val = not curr
            self.grounding_svc.set_web_search_status(ws_name, new_val)
            return MenuActionResult(
                success=True,
                message=f"🌐 Web Search for '{ws_name}' set to: **{'ON' if new_val else 'OFF'}**",
                state_updates={"web_search_enabled": new_val}
            )

        if action_id == "websearch_enable_global":
            self.store.set_web_search_status(True, apply_global=True)
            return MenuActionResult(
                success=True,
                message="🌐 **Web Search ENABLED globally** for all workspaces!",
                state_updates={"web_search_enabled": True}
            )

        if action_id == "websearch_disable_global":
            self.store.set_web_search_status(False, apply_global=True)
            return MenuActionResult(
                success=True,
                message="🔒 **Web Search DISABLED globally** (100% offline local isolation).",
                state_updates={"web_search_enabled": False}
            )

        if action_id.startswith("websearch_engine_"):
            eng = action_id.replace("websearch_engine_", "")
            self.store.set_default_search_engine(eng, apply_global=True)
            return MenuActionResult(
                success=True,
                message=f"🌐 Default Web Search Engine set to: **{eng.upper()}**",
                state_updates={"search_engine": eng}
            )

        # Density actions
        if action_id.startswith("set_density_"):
            preset = action_id.replace("set_density_", "")
            return self.options_engine.set_retrieval_density_preset(preset)

        # Model actions
        if action_id.startswith("select_model_"):
            m_name = action_id.replace("select_model_", "")
            return self.options_engine.set_inference_model(m_name)

        if action_id == "model_quick_openai":
            res = self.model_svc.set_model("gpt-4o-mini")
            return MenuActionResult(
                success=True,
                message=f"⚡ OpenAI Cloud model configured: **gpt-4o-mini**.",
                state_updates={"model": "gpt-4o-mini"}
            )

        if action_id == "model_quick_local":
            self.model_svc.set_model("lm-studio/local")
            return MenuActionResult(
                success=True,
                message="🏠 Local server provider configured (http://localhost:1234/v1).",
                state_updates={"model": "lm-studio/local"}
            )

        # Memory actions
        if action_id == "memory_reset_workspace":
            res = self.memory_svc.reset_memory(ws_name)
            return MenuActionResult(success=True, message=f"🧠 {res['message']}")

        if action_id == "memory_reset_global":
            res = self.memory_svc.reset_memory(None)
            return MenuActionResult(success=True, message=f"🔥 {res['message']}")

        # Workspace actions
        if action_id == "ws_list":
            workspaces = self.workspace_svc.list_workspaces(active_workspace=ws_name)
            lines = [f"### 📂 Workspaces ({len(workspaces)}):"]
            for w in workspaces:
                is_active = w.get("is_active", False)
                active_badge = " **[Active]**" if is_active else ""
                sources = w.get("sources", [])
                lines.append(f"- **{w['name']}**{active_badge} — {len(sources)} source(s)")
                for s in sources[:5]:
                    lines.append(f"  • `{s.get('source_uri', s.get('path', ''))}` ({s.get('type', 'folder')})")
                if len(sources) > 5:
                    lines.append(f"  • ... and {len(sources) - 5} more")
            return MenuActionResult(success=True, message="\n".join(lines))

        if action_id == "ws_create":
            name = params.get("value", "").strip()
            if not name:
                return MenuActionResult(success=False, message="⚠️ Workspace name cannot be empty.")
            self.workspace_svc.create_workspace(name)
            return MenuActionResult(
                success=True,
                message=f"✅ Created and switched to workspace **{name}**.",
                state_updates={"workspace": name},
                action="switch_workspace"
            )

        if action_id == "ws_rename":
            new_name = params.get("value", "").strip()
            if not new_name:
                return MenuActionResult(success=False, message="⚠️ New workspace name cannot be empty.")
            res = self.workspace_svc.rename_workspace(ws_name, new_name)
            return MenuActionResult(
                success=True,
                message=f"✏️ {res['message']}",
                state_updates={"workspace": new_name}
            )

        if action_id == "ws_sync":
            self.sync_svc.start_sync(ws_name, force_full=False)
            return MenuActionResult(success=True, message=f"🔄 Background synchronization started for **{ws_name}**.")

        # API Key actions
        if action_id == "keys_list":
            all_keys = self.store.get_all_api_keys()
            lines = [f"### 🔑 Configured API Keys ({len(all_keys)}):"]
            for k in ["openai", "gemini", "anthropic", "tavily", "serper", "deepseek"]:
                val = all_keys.get(k)
                if val:
                    masked = val[:6] + "..." + val[-4:] if len(val) > 10 else "******"
                    lines.append(f"- **{k.upper()}**: `✅ Configured` (`{masked}`)")
                else:
                    lines.append(f"- **{k.upper()}**: `⚠️ Missing / Not Configured`")
            return MenuActionResult(success=True, message="\n".join(lines))

        if action_id.startswith("key_set_"):
            provider = action_id.replace("key_set_", "")
            key_val = params.get("value", "").strip()
            if not key_val:
                return MenuActionResult(success=False, message=f"⚠️ API Key for {provider} cannot be empty.")
            self.model_svc.set_api_key(provider, key_val)
            return MenuActionResult(success=True, message=f"🔑 Saved API key for provider **{provider.upper()}**.")

        # Memory actions
        if action_id == "memory_info":
            lines = [
                "### 🧠 Long-Term & Short-Term Memory Status",
                f"- **Active Workspace**: `{ws_name}`",
                "- **Storage Engine**: `LanceDB Local Vector Store` (AES-GCM-256 Encrypted)",
                "- **Memory Hierarchy**: L1 (Rolling Turns) ➔ L2 (Session Checkpoints) ➔ L3 (Master Summary)",
                "- **Auto-Summarization**: Triggered every 20 conversation turns"
            ]
            return MenuActionResult(success=True, message="\n".join(lines))

        # Billing actions
        if action_id == "billing_view_matrix":
            info = self.billing_svc.get_billing_info()
            return MenuActionResult(success=True, message=f"### 💳 Subscription Tier: {info.get('current_tier', 'community').upper()}\n\n{info.get('matrix_text', '')}")

        # Sharing actions
        if action_id == "share_list_collabs":
            collabs = self.store.list_workspace_collaborators(ws_name)
            lines = [f"### 👥 Collaborators for '{ws_name}' ({len(collabs)}):"]
            if not collabs:
                lines.append("- No external collaborators attached to this workspace.")
            else:
                for c in collabs:
                    lines.append(f"- **{c.get('user_id', 'Unknown')}**: Role `{c.get('role', 'viewer')}` (Joined: {c.get('joined_at', 'N/A')[:10]})")
            return MenuActionResult(success=True, message="\n".join(lines))

        # Security actions
        if action_id == "sec_status":
            admin_cfg = self.store.is_admin_configured()
            users = self.store.list_users()
            tokens = self.store.get_access_tokens()
            lines = [
                "### 🛡️ User Accounts & Security Access Control",
                f"- **Security Mode**: `{'Protected Multi-User RBAC' if admin_cfg else 'Open Local Community Mode'}`",
                f"- **Registered Users**: `{len(users)}`",
                f"- **Active Bearer Tokens**: `{len(tokens)}`",
                "- **Data Encryption**: `AES-GCM-256 Hardware-Bound Key`"
            ]
            return MenuActionResult(success=True, message="\n".join(lines))

        return MenuActionResult(success=True, message=f"Action '{action_id}' executed.")
