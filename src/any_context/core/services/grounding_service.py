"""
GroundingService - Core Application Service for AI Grounding Strategies and live Web Search toggling.
Pure domain logic: decoupled from terminal UI, CLI formatters, HTTP, and RPC transports.
"""

from typing import Dict, Any, Optional
from any_context.config.db_store import ConfigDBStore


class GroundingService:
    """Service managing Grounding Strategy modes (strict, hybrid, proactive) and Web Search status."""

    VALID_MODES = ["strict", "hybrid", "proactive"]

    def __init__(self, store: Optional[ConfigDBStore] = None):
        self.store = store or ConfigDBStore()

    def get_grounding_mode(self, workspace: str = "Default") -> str:
        """Returns the active grounding mode for the workspace."""
        ws_name = (workspace or "Default").strip()
        mode = self.store.get_grounding_mode(workspace_name=ws_name)
        return (mode or "strict").lower()

    def set_grounding_mode(self, workspace: str, mode: str) -> Dict[str, Any]:
        """Sets the grounding mode for a workspace (or globally if Default)."""
        ws_name = (workspace or "Default").strip()
        clean_mode = mode.strip().lower()

        if clean_mode not in self.VALID_MODES:
            raise ValueError(f"Invalid grounding mode '{mode}'. Choose from: {', '.join(self.VALID_MODES)}.")

        apply_global = (ws_name.lower() == "default")
        self.store.set_grounding_mode(mode=clean_mode, workspace_name=ws_name, apply_global=apply_global)

        return {
            "workspace": ws_name,
            "mode": clean_mode,
            "message": f"Grounding mode for '{ws_name}' set to '{clean_mode.upper()}'."
        }

    def get_web_search_status(self, workspace: str = "Default") -> bool:
        """Returns whether real-time Web Search is enabled for the workspace."""
        ws_name = (workspace or "Default").strip()
        return bool(self.store.get_web_search_status(workspace_name=ws_name))

    def set_web_search_status(self, workspace: str, enabled: bool) -> Dict[str, Any]:
        """Enables or disables real-time Web Search for the workspace."""
        ws_name = (workspace or "Default").strip()
        val = bool(enabled)
        self.store.set_web_search_status(workspace_name=ws_name, enabled=val)

        return {
            "workspace": ws_name,
            "web_search_enabled": val,
            "message": f"Web Search for workspace '{ws_name}' is now {'ON' if val else 'OFF'}."
        }
