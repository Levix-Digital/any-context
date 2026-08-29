"""
Options Engine - Provides structured option lists and state handlers for quick selectors.
Handles /mode, /model, /density, /web-search option schemas and execution.
"""

from typing import List, Dict, Any, Optional
from any_context.core.interaction.schemas import OptionItemSchema, OptionsGroupSchema, MenuActionResult
from any_context.core.services import GroundingService, ModelService, WorkspaceService
from any_context.config.db_store import ConfigDBStore


class OptionsEngine:
    """Provides centralized options management for all presentation adapters."""

    def __init__(self):
        self.grounding_svc = GroundingService()
        self.model_svc = ModelService()
        self.workspace_svc = WorkspaceService()
        self.store = ConfigDBStore()

    def get_grounding_mode_options(self, workspace: str = "Default") -> OptionsGroupSchema:
        """Returns the available Grounding & Answer Mode options with current active mode marked."""
        ws_name = (workspace or "Default").strip()
        current_mode = self.grounding_svc.get_grounding_mode(ws_name).lower()

        modes = [
            {
                "id": "strict",
                "title": "Strict (Audit & Legal)",
                "description": "100% grounded to indexed documents, zero speculation",
                "icon": "🛡️",
            },
            {
                "id": "hybrid",
                "title": "Hybrid (Balanced)",
                "description": "Workspace facts + clearly labeled suggestions (Default)",
                "icon": "⚖️",
            },
            {
                "id": "proactive",
                "title": "Proactive (Research & Ideation)",
                "description": "Broad synthesis, insights & web recommendations",
                "icon": "🚀",
            },
        ]

        items = []
        for m in modes:
            is_act = (m["id"] == current_mode)
            items.append(OptionItemSchema(
                id=m["id"],
                title=m["title"],
                description=m["description"],
                icon=m["icon"],
                badge="[Active]" if is_act else "",
                is_active=is_act,
                metadata={"workspace": ws_name}
            ))

        return OptionsGroupSchema(
            type="grounding_mode",
            title=f"🎛️ AI Grounding & Answer Mode ({ws_name})",
            description="Select how the AI assistant balances indexed workspace facts with broader synthesis:",
            active_id=current_mode,
            items=items
        )

    def get_workspace_options(self, current_workspace: str = "Default") -> OptionsGroupSchema:
        """Returns the available workspaces as an OptionsGroupSchema."""
        curr = (current_workspace or "Default").strip()
        workspaces = self.workspace_svc.list_workspaces(active_workspace=curr)

        items = []
        for ws in workspaces:
            name = ws["name"]
            is_act = (name.lower() == curr.lower())
            
            if name.lower() == "global":
                icon = "🏢"
                type_label = "Global Knowledge Base"
            elif name.lower() == "shared sources":
                icon = "📦"
                type_label = "Shared Sources Library"
            else:
                icon = "📁"
                type_label = "Project Workspace"

            # Get sources count for this workspace
            try:
                sources_data = self.store.get_workspace_sources(workspace_name=name)
                if isinstance(sources_data, dict):
                    sources_count = sources_data.get("total_sources", len(sources_data.get("sources", [])))
                elif isinstance(sources_data, list):
                    sources_count = len(sources_data)
                else:
                    sources_count = 0
            except Exception:
                sources_count = len(ws.get("paths", []))

            sources_badge = f"{sources_count} sources" if sources_count > 0 else "Empty"
            badge = f"[{sources_badge}]" + (" [Active]" if is_act else "")

            desc = f"{type_label} | {sources_count} indexed sources"
            if ws.get("created_at"):
                desc += f" (Created: {str(ws['created_at'])[:10]})"

            items.append(OptionItemSchema(
                id=name,
                title=name,
                description=desc,
                icon=icon,
                badge=badge,
                is_active=is_act,
                metadata={"workspace": name, "sources_count": sources_count}
            ))

        return OptionsGroupSchema(
            type="workspace",
            title="📂 Switch Active Workspace",
            description="Select a workspace to switch active context and isolated scope:",
            active_id=curr,
            items=items
        )

    def set_workspace(self, workspace_name: str) -> MenuActionResult:
        """Switches the active workspace."""
        clean_name = (workspace_name or "Default").strip()
        if not self.store.get_workspace_meta(clean_name):
            try:
                self.store.add_workspace(name=clean_name, paths=[])
            except Exception:
                pass

        return MenuActionResult(
            success=True,
            message=f"📂 Active workspace switched to: **{clean_name}**",
            state_updates={"workspace": clean_name}
        )

    def set_grounding_mode(self, mode: str, workspace: str = "Default", apply_global: bool = False) -> MenuActionResult:
        """Applies the selected grounding mode and returns a MenuActionResult."""
        ws_name = (workspace or "Default").strip()
        clean_mode = mode.lower().strip()
        if clean_mode not in ["strict", "hybrid", "proactive"]:
            clean_mode = "hybrid"

        saved_mode = self.store.set_grounding_mode(clean_mode, workspace_name=None if apply_global else ws_name, apply_global=apply_global)
        target_label = "all workspaces (global)" if apply_global else f"workspace '{ws_name}'"

        return MenuActionResult(
            success=True,
            message=f"🛡️ AI Grounding Mode for {target_label} set to: **{saved_mode.capitalize()}**",
            state_updates={"grounding_mode": saved_mode}
        )

    def get_inference_model_options(self) -> OptionsGroupSchema:
        """Returns the available AI Inference Models with current active model marked."""
        curr = self.model_svc.get_current_model()
        catalog = self.model_svc.list_models()

        items = []
        for m in catalog:
            model_id = m.get("id", m.get("name", ""))
            is_act = (model_id.lower() == curr.lower() or m.get("name", "").lower() == curr.lower())
            status_badge = "✅ Configured" if m.get("is_available") else "⚠️ Key Missing"
            active_badge = " [Active]" if is_act else ""
            items.append(OptionItemSchema(
                id=model_id,
                title=f"{m['name']} ({m.get('provider', 'cloud')})",
                description=f"Provider: {m.get('provider')} | ID: {model_id} | {status_badge}",
                icon="🤖",
                badge=f"{status_badge}{active_badge}",
                is_active=is_act,
                metadata={"model_id": model_id, "provider": m.get("provider"), "is_available": m.get("is_available", False)}
            ))

        return OptionsGroupSchema(
            type="inference_model",
            title="🤖 AI Inference Models",
            description="Select active LLM model for contextual reasoning and chat:",
            active_id=curr,
            items=items
        )

    def set_inference_model(self, model_name: str) -> MenuActionResult:
        """Switches the active inference model."""
        res = self.model_svc.set_model(model_name)
        key_status = "✅ API Key Ready" if res["has_key"] else "⚠️ API Key Missing"
        return MenuActionResult(
            success=True,
            message=f"🤖 Inference Model switched to: **{res['model']}** ({res['provider']} - {key_status})",
            state_updates={"model": res["model"]}
        )

    def get_retrieval_density_options(self) -> OptionsGroupSchema:
        """Returns retrieval density presets."""
        settings = self.store.get_app_settings()
        ctx = settings.context if settings else None
        current_preset = ctx.retrieval_preset.lower() if ctx else "balanced"

        presets = [
            {
                "id": "balanced",
                "title": "Balanced (Top-40 Chunks, Pool 100)",
                "description": "Recommended for Cloud models and 20+ sources (Default)",
                "icon": "⚡",
            },
            {
                "id": "turbo",
                "title": "Turbo (Top-20 Chunks, Pool 50)",
                "description": "Ultra-fast TTFT, ideal for LM Studio / Ollama local offline",
                "icon": "🚀",
            },
            {
                "id": "deep_research",
                "title": "Deep Research (Top-60 Chunks, Pool 150)",
                "description": "Maximum density for massive dossiers & 50+ websites",
                "icon": "🔬",
            },
        ]

        items = []
        for p in presets:
            is_act = (p["id"] == current_preset)
            items.append(OptionItemSchema(
                id=p["id"],
                title=p["title"],
                description=p["description"],
                icon=p["icon"],
                badge="[Active]" if is_act else "",
                is_active=is_act
            ))

        return OptionsGroupSchema(
            type="retrieval_density",
            title="🔍 Context Retrieval Density & Multi-Source RAG Presets",
            description="Select density preset for retrieval chunk depth and candidate pooling:",
            active_id=current_preset,
            items=items
        )

    def set_retrieval_density_preset(self, preset: str) -> MenuActionResult:
        """Applies a retrieval density preset."""
        settings = self.store.get_app_settings()
        ctx = settings.context if settings else None
        if not ctx:
            return MenuActionResult(success=False, message="⚠️ Could not load context settings.", error="missing_settings")

        clean_p = preset.lower().strip()
        if clean_p in ["balanced", "turbo", "deep_research"]:
            ctx.apply_preset(clean_p)
            self.store.update_context_settings(ctx)
            return MenuActionResult(
                success=True,
                message=f"🔍 Applied **{clean_p.capitalize()} Preset**: Top-K {ctx.top_k} chunks, Pool {ctx.candidate_pool_size}, Max {ctx.max_chunks_per_source} per source.",
                state_updates={"retrieval_preset": clean_p}
            )

        return MenuActionResult(success=False, message=f"⚠️ Unknown preset '{preset}'.", error="unknown_preset")

    def get_update_options(self, target_version: Optional[str] = None) -> OptionsGroupSchema:
        """Returns the available update action options with active instance detection."""
        from any_context.core.services.update_service import UpdateService
        from any_context import __version__ as CURRENT_VERSION
        update_svc = UpdateService()

        if target_version:
            target_tag = target_version if target_version.startswith("v") else f"v{target_version}"
        else:
            has_up, latest_tag = update_svc.check_for_updates()
            target_tag = latest_tag or f"v{CURRENT_VERSION}"

        active_instances = update_svc.find_active_instances()
        count = len(active_instances)

        if count > 0:
            sub = f"ℹ️ Detected {count} other active AnyContext session(s). How would you like to update?"
        else:
            sub = f"🚀 Ready to download and install AnyContext {target_tag}."

        items = [
            OptionItemSchema(
                id="background",
                title="⚡ Update in background (Recommended)",
                description="Active background sessions continue working undisturbed.",
                icon="⚡",
                badge="[Recommended]",
                is_active=True,
                metadata={"target_version": target_tag}
            ),
            OptionItemSchema(
                id="close",
                title="⏹️ Close other instances and update now",
                description=f"Terminates {count} background process(es) before updating.",
                icon="⏹️",
                is_active=False,
                metadata={"target_version": target_tag}
            ),
            OptionItemSchema(
                id="cancel",
                title="🔙 Cancel update",
                description="Aborts the update process and returns to chat.",
                icon="🔙",
                is_active=False,
                metadata={"target_version": target_tag}
            )
        ]

        return OptionsGroupSchema(
            type="update",
            title=f"🚀 AnyContext Update Available: v{CURRENT_VERSION} → {target_tag}",
            description=sub,
            active_id="background",
            items=items
        )

    def execute_update_option(self, option_id: str, is_tui: bool = False) -> MenuActionResult:
        """Executes the chosen update action without abrupt auto-restart."""
        from any_context.core.services.update_service import UpdateService
        update_svc = UpdateService()

        clean_id = (option_id or "background").lower().strip()
        if clean_id == "cancel":
            return MenuActionResult(
                success=True,
                message="⚠️ Update cancelled by user.",
                state_updates={"action": "none"}
            )

        auto_close = (clean_id == "close")
        success, msg, updates = update_svc.execute_binary_update(
            auto_close_instances=auto_close,
            force_background=not auto_close,
            auto_restart=False,
            is_tui=is_tui
        )

        return MenuActionResult(
            success=success,
            message=msg,
            error=None if success else "update_failed",
            state_updates=updates
        )

    def get_delete_workspace_options(self, current_workspace: str = "Default") -> OptionsGroupSchema:
        """Returns the list of custom workspaces that can be deleted."""
        curr = (current_workspace or "Default").strip()
        workspaces = self.workspace_svc.list_workspaces(active_workspace=curr)

        items = []
        for ws in workspaces:
            name = ws["name"]
            if name.lower() in ["default", "global", "shared sources"]:
                continue

            is_act = (name.lower() == curr.lower())
            try:
                sources_data = self.store.get_workspace_sources(workspace_name=name)
                if isinstance(sources_data, dict):
                    sources_count = sources_data.get("total_sources", len(sources_data.get("sources", [])))
                elif isinstance(sources_data, list):
                    sources_count = len(sources_data)
                else:
                    sources_count = 0
            except Exception:
                sources_count = len(ws.get("paths", []))

            badge = "[Active]" if is_act else ""
            items.append(OptionItemSchema(
                id=f"delete_ws_{name}",
                title=f"🗑️ Delete '{name}'",
                description=f"Remove workspace and purge {sources_count} source(s) and all indexed vector chunks",
                icon="🗑️",
                badge=badge,
                is_active=False,
                metadata={"workspace_name": name, "is_active": is_act}
            ))

        items.append(OptionItemSchema(
            id="cancel_delete",
            title="🔙 Cancel",
            description="Aborts workspace deletion and returns to chat",
            icon="🔙",
            badge="",
            is_active=False,
            metadata={}
        ))

        return OptionsGroupSchema(
            type="delete_workspace",
            title="🗑️ Delete Workspace",
            description="Select a workspace to delete:",
            active_id=items[0].id if items else "",
            items=items
        )

    def get_confirm_delete_workspace_options(self, workspace_to_delete: str, is_active: bool = False) -> OptionsGroupSchema:
        """Returns confirmation options before permanently deleting a workspace."""
        ws_target = workspace_to_delete.strip()
        items = [
            OptionItemSchema(
                id=f"confirm_delete_yes_{ws_target}",
                title=f"🗑️ Yes, permanently delete '{ws_target}'",
                description=f"All sources and vector chunks for '{ws_target}' will be permanently deleted.",
                icon="🗑️",
                badge="[Permanent Action]",
                is_active=False,
                metadata={"workspace_name": ws_target, "confirmed": True, "is_active": is_active}
            ),
            OptionItemSchema(
                id="confirm_delete_cancel",
                title=f"🔙 Cancel (Keep '{ws_target}')",
                description="Do not delete workspace, keep all data intact.",
                icon="🔙",
                badge="[Safe]",
                is_active=True,
                metadata={"workspace_name": ws_target, "confirmed": False}
            )
        ]

        return OptionsGroupSchema(
            type="confirm_delete_workspace",
            title=f"⚠️ Are you sure you want to delete workspace '{ws_target}'?",
            description=f"This action cannot be undone. All indexed context in '{ws_target}' will be purged.",
            active_id="confirm_delete_cancel",
            items=items
        )

    def execute_delete_workspace_option(self, option_id: str, current_workspace: str = "Default") -> MenuActionResult:
        """Handles selecting or confirming workspace deletion."""
        curr = (current_workspace or "Default").strip()
        clean_id = (option_id or "").strip()

        if clean_id in ["cancel_delete", "confirm_delete_cancel", "cancel"]:
            return MenuActionResult(
                success=True,
                message="⚠️ Workspace deletion cancelled.",
                state_updates={"action": "none"}
            )

        if clean_id.startswith("delete_ws_"):
            target_ws = clean_id.replace("delete_ws_", "").strip()
            if target_ws.lower() in ["default", "global", "shared sources"]:
                return MenuActionResult(success=False, message=f"⚠️ Cannot delete protected system workspace '{target_ws}'.", error="cannot_delete_system_ws")
            return MenuActionResult(
                success=True,
                message=f"⚠️ Confirm deletion of workspace '{target_ws}'",
                action="open_confirm_delete_workspace_modal",
                state_updates={"target_workspace": target_ws}
            )

        if clean_id.startswith("confirm_delete_yes_"):
            target_ws = clean_id.replace("confirm_delete_yes_", "").strip()
            if target_ws.lower() in ["default", "global", "shared sources"]:
                return MenuActionResult(success=False, message=f"⚠️ Cannot delete protected system workspace '{target_ws}'.", error="cannot_delete_system_ws")

            res = self.workspace_svc.delete_workspace(target_ws)
            was_active = (curr.lower() == target_ws.lower())
            new_ws = "Default" if was_active else curr
            return MenuActionResult(
                success=True,
                message=f"🗑️ Workspace **{target_ws}** deleted successfully." + (f" Switched active workspace back to **Default**." if was_active else ""),
                state_updates={"workspace": new_ws} if was_active else {},
                action="delete_workspace_success"
            )

        return MenuActionResult(success=False, message="⚠️ Unknown delete option.", error="unknown_option")
