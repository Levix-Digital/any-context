"""
Command Dispatcher - Universal parser and executor for AnyContext slash commands.
Translates user command lines into Core Service calls and returns structured CommandResult.
"""

import shlex
from typing import List, Dict, Any, Optional

from any_context import __version__
from any_context.commands.registry import find_command_meta, COMMANDS_REGISTRY
from any_context.commands.result import CommandResult
from any_context.core.services import (
    WorkspaceService,
    SourceService,
    ModelService,
    GroundingService,
    SyncService,
    MemoryService,
    BillingService,
)


def parse_args(command_line: str) -> List[str]:
    """Tokenizes command line arguments safely respecting quotes."""
    clean = command_line.strip()
    if not clean:
        return []
    try:
        parts = shlex.split(clean, posix=False)
        return [p.strip().strip("'\"") for p in parts if p.strip()]
    except Exception:
        return [p.strip().strip("'\"") for p in clean.split() if p.strip()]


class CommandDispatcher:
    """Universal Command Dispatcher mapping slash commands to Core Application Services."""

    def __init__(self):
        self.workspace_svc = WorkspaceService()
        self.source_svc = SourceService()
        self.model_svc = ModelService()
        self.grounding_svc = GroundingService()
        self.sync_svc = SyncService()
        self.memory_svc = MemoryService()
        self.billing_svc = BillingService()

    def dispatch(self, command_line: str, active_workspace: str = "Default") -> CommandResult:
        """Parses and executes a slash command, returning a standardized CommandResult."""
        ws_name = (active_workspace or "Default").strip()
        parts = parse_args(command_line)
        if not parts:
            return CommandResult(success=True, message="")

        cmd_token = parts[0].lower()
        if not cmd_token.startswith("/"):
            cmd_token = "/" + cmd_token

        meta = find_command_meta(cmd_token)
        canonical = meta.name if meta else cmd_token

        try:
            # 1. /exit or /quit
            if canonical == "/exit":
                return CommandResult(
                    success=True,
                    message="👋 Saving session memory and exiting AnyContext. See you soon!",
                    action="exit"
                )

            # 2. /clear or /cls
            if canonical == "/clear":
                return CommandResult(
                    success=True,
                    message="🧹 Screen cleared.",
                    action="clear"
                )

            # 3. /version
            if canonical == "/version":
                return CommandResult(
                    success=True,
                    message=f"🤖 **AnyContext (actx)** `v{__version__}` — *Universal Multi-Context RAG Assistant & Engine* (Levix Digital)"
                )

            # 4. /help or /menu
            if canonical == "/help":
                return self._handle_help(parts)

            # 5. /switch or /workspace
            if canonical == "/switch":
                return self._handle_switch(parts, ws_name)

            # 6. /model
            if canonical == "/model":
                return self._handle_model(parts)

            # 7. /mode or /grounding
            if canonical == "/mode":
                return self._handle_mode(parts, ws_name)

            # 8. /web-search
            if canonical == "/web-search":
                return self._handle_web_search(parts, ws_name)

            # 9. /sync
            if canonical == "/sync":
                return self._handle_sync(parts, ws_name)

            # 10. /sources
            if canonical == "/sources":
                return self._handle_sources(parts, ws_name)

            # 11. /folder
            if canonical == "/folder":
                return self._handle_folder(parts, ws_name)

            # 12. /web
            if canonical == "/web":
                return self._handle_web(parts, ws_name)

            # 13. /transfer
            if canonical == "/transfer":
                return self._handle_transfer(parts)

            # 14. /link
            if canonical == "/link":
                return self._handle_link(parts, ws_name)

            # 15. /unlink
            if canonical == "/unlink":
                return self._handle_unlink(parts, ws_name)

            # 16. /shared
            if canonical == "/shared":
                return self._handle_shared()

            # 17. /rename
            if canonical == "/rename":
                return self._handle_rename(parts)

            # 18. /config
            if canonical == "/config":
                return self._handle_config(ws_name)

            # 19. /key
            if canonical == "/key":
                return self._handle_key(parts)

            # 20. /models
            if canonical == "/models":
                return self._handle_models()

            # 21. /billing
            if canonical == "/billing":
                return self._handle_billing()

            # 22. /reset-memory
            if canonical == "/reset-memory":
                return self._handle_reset_memory(ws_name)

            # 23. /paste
            if canonical == "/paste":
                return CommandResult(
                    success=True,
                    message="📋 Multi-line paste capture mode active.",
                    action="paste_mode"
                )

            # 24. /update
            if canonical == "/update":
                try:
                    from any_context.update.updater import check_for_updates
                    has_up, latest_v, msg = check_for_updates()
                    return CommandResult(
                        success=True,
                        message=msg or f"🚀 AnyContext v{__version__} is up to date."
                    )
                except Exception as e:
                    return CommandResult(
                        success=True,
                        message=f"🚀 AnyContext v{__version__}. (Could not reach update server: {e})"
                    )

            # 25. /inspect
            if canonical == "/inspect":
                try:
                    from any_context.vector_engine.store import LanceDBStore
                    from any_context.config.app_settings import AppSettings
                    settings = AppSettings.load()
                    db_save_path = settings.context.db_path if settings else "./context_db"
                    lance_store = LanceDBStore.get_instance(db_path=os.path.join(db_save_path, "lancedb"))
                    ws_count = lance_store.count_records(workspace_name=ws_name, table_name="workspace_chunks")
                    total_count = lance_store.count_records(table_name="workspace_chunks")
                    return CommandResult(
                        success=True,
                        message=f"🔍 Vector Store Inspection for `{ws_name}`:\n  • Workspace Chunks: **{ws_count}**\n  • Total Database Chunks: **{total_count}**\n  • Storage Engine: **LanceDB (Apache Arrow / Rust)**"
                    )
                except Exception as e:
                    return CommandResult(
                        success=False,
                        message=f"⚠️ Could not inspect vector store: {e}"
                    )

            # 26. /density
            if canonical == "/density":
                level = parts[1] if len(parts) > 1 else "comfortable"
                return CommandResult(
                    success=True,
                    message=f"🎨 UI Density set to: **{level}**"
                )

            # 27. /history
            if canonical == "/history":
                return CommandResult(
                    success=True,
                    message="📜 Conversation history is active."
                )

            # 28. /menu
            if canonical == "/menu":
                return CommandResult(
                    success=True,
                    message="💡 Opening interactive configuration menu.",
                    action="open_config_modal"
                )

            return CommandResult(
                success=False,
                message=f"❌ Unknown command `{cmd_token}`. Type `/help` to view all available commands.",
                error="unknown_command"
            )

        except Exception as e:
            return CommandResult(
                success=False,
                message=f"⚠️ Error executing `{cmd_token}`: {str(e)}",
                error=str(e)
            )

    # -------------------------------------------------------------------------
    # Command Handlers
    # -------------------------------------------------------------------------

    def _handle_help(self, parts: List[str]) -> CommandResult:
        if len(parts) > 1:
            query = parts[1].strip().lower()
            if not query.startswith("/"):
                query = "/" + query
            meta = find_command_meta(query)
            if meta:
                aliases_str = f" (Aliases: {', '.join(meta.aliases)})" if meta.aliases else ""
                msg = (
                    f"### 📖 Command Help: `{meta.name}`\n\n"
                    f"• **Usage**: `{meta.name} {meta.args}`{aliases_str}\n"
                    f"• **Category**: `{meta.category}`\n"
                    f"• **Description**: {meta.description}\n"
                )
                return CommandResult(success=True, message=msg)

        categories: Dict[str, List[str]] = {}
        for c in COMMANDS_REGISTRY:
            cat = c.category
            if cat not in categories:
                categories[cat] = []
            args_part = f" `{c.args}`" if c.args else ""
            categories[cat].append(f"• **`{c.name}`**{args_part} — {c.description}")

        lines = ["### 📚 AnyContext Slash Commands Catalog\n"]
        for cat, items in categories.items():
            lines.append(f"**[{cat}]**")
            lines.extend(items)
            lines.append("")

        lines.append("*Type `/help <command>` for detailed info on any specific command.*")
        return CommandResult(success=True, message="\n".join(lines))

    def _handle_switch(self, parts: List[str], current_ws: str) -> CommandResult:
        if len(parts) == 1:
            workspaces = self.workspace_svc.list_workspaces(active_workspace=current_ws)
            ws_lines = []
            for w in workspaces:
                tag = " `[Active]`" if w["is_active"] else ""
                ws_lines.append(f"• **{w['name']}**{tag}")
            msg = (
                f"### 📂 Workspaces ({len(workspaces)})\n\n"
                + "\n".join(ws_lines)
                + "\n\n*Usage:* `/switch <name>` to switch or create | `/switch --delete <name>` to remove."
            )
            return CommandResult(success=True, message=msg)

        if "--list" in parts or "-l" in parts:
            workspaces = self.workspace_svc.list_workspaces(active_workspace=current_ws)
            ws_lines = [f"• **{w['name']}**{' `[Active]`' if w['is_active'] else ''}" for w in workspaces]
            return CommandResult(success=True, message=f"### 📂 Workspaces\n\n" + "\n".join(ws_lines))

        if "--delete" in parts or "-d" in parts or "--remove" in parts:
            idx = next(i for i, p in enumerate(parts) if p in ["--delete", "-d", "--remove"])
            if len(parts) <= idx + 1:
                return CommandResult(success=False, message="❌ Specify workspace name: `/switch --delete <name>`")
            target = parts[idx + 1]
            res = self.workspace_svc.delete_workspace(target)
            new_ws = "Default" if current_ws.lower() == target.lower() else current_ws
            return CommandResult(
                success=True,
                message=f"🗑️ {res['message']}",
                state_updates={"workspace": new_ws} if new_ws != current_ws else {}
            )

        # Switch or create target workspace
        target = parts[1]
        if target.lower() in ["add", "create"] and len(parts) > 2:
            target = parts[2]

        res = self.workspace_svc.create_workspace(target)
        # Trigger background auto-sync check on switch
        self.sync_svc.start_sync(workspace=target, force_full=False)

        action_word = "Created and switched to" if res.get("created") else "Switched to"
        return CommandResult(
            success=True,
            message=f"🔄 **{action_word} workspace:** `{target}`",
            state_updates={"workspace": target},
            action="switch_workspace"
        )

    def _handle_model(self, parts: List[str]) -> CommandResult:
        if len(parts) == 1:
            curr = self.model_svc.get_current_model()
            return CommandResult(
                success=True,
                message=f"🤖 **Active AI Model:** `{curr}`\n\n*Usage:* `/model <name>` (e.g. `/model gpt-4o-mini`, `/model claude-3-5-sonnet`)"
            )

        new_model = parts[1].strip()
        res = self.model_svc.set_model(new_model)
        key_status = "✅ API Key Ready" if res["has_key"] else "⚠️ API Key Missing"
        return CommandResult(
            success=True,
            message=f"🤖 **Inference Model Switched to:** `{res['model']}`\n• Provider: `{res['provider']}` ({key_status})",
            state_updates={"model": res["model"]}
        )

    def _handle_mode(self, parts: List[str], ws_name: str) -> CommandResult:
        if len(parts) == 1:
            curr = self.grounding_svc.get_grounding_mode(ws_name)
            return CommandResult(
                success=True,
                message=f"🛡️ **Grounding Mode for `{ws_name}`:** `{curr.upper()}`\n\n*Usage:* `/mode <strict|hybrid|proactive>`",
                action="open_mode_modal"
            )

        target_mode = parts[1].strip().lower()
        res = self.grounding_svc.set_grounding_mode(ws_name, target_mode)
        return CommandResult(
            success=True,
            message=f"🛡️ **Grounding Mode for `{ws_name}`:** `{res['mode'].upper()}`",
            state_updates={"grounding_mode": res["mode"]}
        )

    def _handle_web_search(self, parts: List[str], ws_name: str) -> CommandResult:
        if len(parts) == 1:
            curr = self.grounding_svc.get_web_search_status(ws_name)
            new_val = not curr
        else:
            arg = parts[1].strip().lower().lstrip("-")
            new_val = arg in ["on", "true", "1", "enable"]

        res = self.grounding_svc.set_web_search_status(ws_name, new_val)
        status_text = "🟢 **ON**" if res["web_search_enabled"] else "🔴 **OFF**"
        return CommandResult(
            success=True,
            message=f"🌐 **Real-time Web Search for `{ws_name}`:** {status_text}",
            state_updates={"web_search_enabled": res["web_search_enabled"]}
        )

    def _handle_sync(self, parts: List[str], ws_name: str) -> CommandResult:
        force = "--force" in parts or "-f" in parts
        if "--status" in parts or "-s" in parts:
            status = self.sync_svc.get_sync_status(ws_name)
            return CommandResult(
                success=True,
                message=f"⚡ **Sync Status for `{ws_name}`:** `{status['status']}`"
            )

        self.sync_svc.start_sync(ws_name, force_full=force)
        return CommandResult(
            success=True,
            message=f"⚡ **Background synchronization started** for workspace `{ws_name}`{' (Force Full)' if force else ''}."
        )

    def _handle_sources(self, parts: List[str], ws_name: str) -> CommandResult:
        show_all = "--all" in parts or "-a" in parts

        if show_all:
            workspaces = self.workspace_svc.list_workspaces()
            lines = ["### 📂 All Indexed Sources Across Workspaces\n"]
            for ws in workspaces:
                w_name = ws.get("name", "Default")
                s = self.source_svc.list_sources(w_name)
                lines.append(f"**Workspace: `{w_name}`** ({s['total_count']} sources)")
                for f in s.get("folders", []):
                    lines.append(f"  • 📁 `{f}`")
                for w in s.get("web_sources", []):
                    title = w.get("title") or w.get("url", "Web Portal")
                    url = w.get("url") or w.get("root_url", "")
                    pages = w.get("page_count", 1)
                    lines.append(f"  • 🌐 **[{title}]({url})** • {pages} pages")
                for d in s.get("cloud_drives", []):
                    d_title = d.get("title") or d.get("mount_path_or_id")
                    lines.append(f"  • ☁️ `{d_title}`")
                if not s["total_count"]:
                    lines.append("  *(No sources)*")
                lines.append("")
            return CommandResult(success=True, message="\n".join(lines).strip())

        sources = self.source_svc.list_sources(ws_name)
        folders = sources.get("folders", [])
        webs = sources.get("web_sources", [])
        drives = sources.get("cloud_drives", [])

        lines = [f"### 📂 Indexed Sources in `{ws_name}` ({sources['total_count']} sources)\n"]
        if folders:
            lines.append("**📁 Local Folders:**")
            for f in folders:
                lines.append(f"• `{f}`")
            lines.append("")

        if webs:
            lines.append("**🌐 Web Portals & URLs:**")
            for w in webs:
                title = w.get("title") or w.get("url", "Web Source")
                url = w.get("url") or w.get("root_url", "")
                pages = w.get("page_count", 1)
                lines.append(f"• **[{title}]({url})** • {pages} pages (`{url}`)")
            lines.append("")

        if drives:
            lines.append("**☁️ Cloud Drives:**")
            for d in drives:
                d_title = d.get("title") or d.get("mount_path_or_id")
                lines.append(f"• `{d_title}`")
            lines.append("")

        if not sources["total_count"]:
            lines.append("*No sources indexed in this workspace yet. Add with `/folder --add <path>` or `/web --add <url>`.*")

        return CommandResult(success=True, message="\n".join(lines).strip())

    def _handle_folder(self, parts: List[str], ws_name: str) -> CommandResult:
        if len(parts) == 1 or "--list" in parts or "-l" in parts:
            sources = self.source_svc.list_sources(ws_name)
            folders = sources.get("folders", [])
            lines = [f"### 📁 Local Folders in `{ws_name}` ({len(folders)})\n"]
            for f in folders:
                lines.append(f"• `{f}`")
            if not folders:
                lines.append("*No folders added. Use `/folder --add <path>` to add one.*")
            return CommandResult(success=True, message="\n".join(lines))

        if "--add" in parts or "-a" in parts:
            idx = next(i for i, p in enumerate(parts) if p in ["--add", "-a"])
            if len(parts) <= idx + 1:
                return CommandResult(success=False, message="❌ Specify folder path: `/folder --add <path>`")
            target_path = parts[idx + 1]
            res = self.source_svc.add_folder(ws_name, target_path)
            self.sync_svc.start_sync(ws_name, force_full=False)
            return CommandResult(success=True, message=f"✅ {res['message']}\n⚡ Indexing started in background.")

        if "--remove" in parts or "-r" in parts or "--delete" in parts:
            idx = next(i for i, p in enumerate(parts) if p in ["--remove", "-r", "--delete"])
            if len(parts) <= idx + 1:
                return CommandResult(success=False, message="❌ Specify folder path: `/folder --remove <path>`")
            target_path = parts[idx + 1]
            res = self.source_svc.remove_folder(ws_name, target_path)
            return CommandResult(success=True, message=f"🗑️ {res['message']}")

        # Fallback: treat single argument as folder path to add
        target_path = parts[1]
        res = self.source_svc.add_folder(ws_name, target_path)
        self.sync_svc.start_sync(ws_name, force_full=False)
        return CommandResult(success=True, message=f"✅ {res['message']}\n⚡ Indexing started in background.")

    def _handle_web(self, parts: List[str], ws_name: str) -> CommandResult:
        if len(parts) == 1 or "--list" in parts or "-l" in parts:
            sources = self.source_svc.list_sources(ws_name)
            webs = sources.get("web_urls", [])
            lines = [f"### 🌐 Web Portals in `{ws_name}` ({len(webs)})\n"]
            for w in webs:
                lines.append(f"• [{w}]({w})")
            if not webs:
                lines.append("*No web portals added. Use `/web --add <url>` to add one.*")
            return CommandResult(success=True, message="\n".join(lines))

        if "--add" in parts or "-a" in parts:
            idx = next(i for i, p in enumerate(parts) if p in ["--add", "-a"])
            if len(parts) <= idx + 1:
                return CommandResult(success=False, message="❌ Specify URL: `/web --add <url>`")
            target_url = parts[idx + 1]
            res = self.source_svc.add_web(ws_name, target_url)
            self.sync_svc.start_sync(ws_name, force_full=False)
            return CommandResult(success=True, message=f"✅ {res['message']}\n⚡ Crawler started in background.")

        if "--remove" in parts or "-r" in parts:
            idx = next(i for i, p in enumerate(parts) if p in ["--remove", "-r"])
            if len(parts) <= idx + 1:
                return CommandResult(success=False, message="❌ Specify URL: `/web --remove <url>`")
            target_url = parts[idx + 1]
            res = self.source_svc.remove_web(ws_name, target_url)
            return CommandResult(success=True, message=f"🗑️ {res['message']}")

        # Fallback: treat argument as URL to add
        target_url = parts[1]
        res = self.source_svc.add_web(ws_name, target_url)
        self.sync_svc.start_sync(ws_name, force_full=False)
        return CommandResult(success=True, message=f"✅ {res['message']}\n⚡ Crawler started in background.")

    def _handle_transfer(self, parts: List[str]) -> CommandResult:
        if len(parts) < 4:
            return CommandResult(
                success=False,
                message="❌ Usage: `/transfer <from_workspace> <to_workspace> <item_path_or_url>`"
            )
        from_ws = parts[1]
        to_ws = parts[2]
        item = parts[3]
        res = self.source_svc.transfer_source(from_ws, to_ws, item)
        return CommandResult(
            success=res.get("success", True),
            message=f"⚡ **Source Transfer:** {res.get('message', 'Transfer completed.')}"
        )

    def _handle_link(self, parts: List[str], ws_name: str) -> CommandResult:
        if len(parts) < 2:
            return CommandResult(success=False, message="❌ Usage: `/link <source_path_or_url> [target_workspace]`")
        source = parts[1]
        target_ws = parts[2] if len(parts) > 2 else ws_name
        res = self.source_svc.link_source(source, target_ws)
        return CommandResult(success=True, message=f"🔗 {res.get('message', 'Source linked.')}")

    def _handle_unlink(self, parts: List[str], ws_name: str) -> CommandResult:
        if len(parts) < 2:
            return CommandResult(success=False, message="❌ Usage: `/unlink <source_path_or_url> [target_workspace]`")
        source = parts[1]
        target_ws = parts[2] if len(parts) > 2 else ws_name
        res = self.source_svc.unlink_source(source, target_ws)
        return CommandResult(success=True, message=f"🔓 {res.get('message', 'Source unlinked.')}")

    def _handle_shared(self) -> CommandResult:
        shared = self.source_svc.list_shared_sources()
        lines = [f"### 🌐 Shared Reusable Sources ({len(shared)})\n"]
        for s in shared:
            lines.append(f"• `{s.get('identifier', '')}` ({s.get('source_type', '')})")
        if not shared:
            lines.append("*No shared sources registered yet.*")
        return CommandResult(success=True, message="\n".join(lines))

    def _handle_rename(self, parts: List[str]) -> CommandResult:
        if len(parts) < 3:
            return CommandResult(success=False, message="❌ Usage: `/rename <old_name> <new_name>`")
        old_name = parts[1]
        new_name = parts[2]
        res = self.workspace_svc.rename_workspace(old_name, new_name)
        return CommandResult(
            success=True,
            message=f"✏️ {res['message']}",
            state_updates={"workspace": new_name}
        )

    def _handle_config(self, ws_name: str) -> CommandResult:
        curr_model = self.model_svc.get_current_model()
        curr_mode = self.grounding_svc.get_grounding_mode(ws_name)
        curr_search = self.grounding_svc.get_web_search_status(ws_name)
        sources = self.source_svc.list_sources(ws_name)

        msg = (
            f"### ⚙️ AnyContext System Configuration\n\n"
            f"• **Active Workspace**: `{ws_name}`\n"
            f"• **Inference Model**: `{curr_model}`\n"
            f"• **Grounding Strategy**: `{curr_mode.upper()}`\n"
            f"• **Live Web Search**: `{'ON' if curr_search else 'OFF'}`\n"
            f"• **Indexed Sources**: `{sources['total_count']} source(s)`\n"
            f"• **Version**: `v{__version__}`\n"
        )
        return CommandResult(success=True, message=msg, action="open_config_modal")

    def _handle_key(self, parts: List[str]) -> CommandResult:
        if len(parts) < 3:
            return CommandResult(success=False, message="❌ Usage: `/key <provider> <api-key>` (e.g. `/key openai sk-...`)")
        provider = parts[1]
        key = parts[2]
        res = self.model_svc.set_api_key(provider, key)
        return CommandResult(success=True, message=f"🔑 {res['message']}")

    def _handle_models(self) -> CommandResult:
        catalog = self.model_svc.list_models()
        lines = [f"### 🤖 AI Models Catalog ({len(catalog)} available)\n"]
        for m in catalog:
            status = "✅ Configured" if m["is_available"] else "⚠️ Needs Key"
            lines.append(f"• **`{m['name']}`** `[{m['provider']}]` — {status}")
        return CommandResult(success=True, message="\n".join(lines))

    def _handle_billing(self) -> CommandResult:
        info = self.billing_svc.get_billing_info()
        return CommandResult(
            success=True,
            message=f"### 💳 Subscription & Tier\n\n• **Active Tier**: `{info['current_tier'].upper()}`\n• **Status**: `{info['status'].upper()}`\n\n{info['matrix_text']}"
        )

    def _handle_reset_memory(self, ws_name: str) -> CommandResult:
        res = self.memory_svc.reset_memory(ws_name)
        return CommandResult(success=True, message=f"🧠 {res['message']}")


# Global dispatcher singleton
_dispatcher = CommandDispatcher()


def dispatch_command(command_line: str, active_workspace: str = "Default") -> CommandResult:
    """Dispatches a command line string to the universal command engine."""
    return _dispatcher.dispatch(command_line, active_workspace=active_workspace)
