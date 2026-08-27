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
            is_act = (m["name"].lower() == curr.lower())
            status_badge = "✅ Configured" if m.get("is_available") else "⚠️ Key Missing"
            active_badge = " [Active]" if is_act else ""
            items.append(OptionItemSchema(
                id=m["name"],
                title=f"{m['name']} ({m.get('provider', 'cloud')})",
                description=f"Provider: {m.get('provider')} | {status_badge}",
                icon="🤖",
                badge=f"{status_badge}{active_badge}",
                is_active=is_act,
                metadata={"provider": m.get("provider"), "is_available": m.get("is_available", False)}
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
