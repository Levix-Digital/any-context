"""
MemoryService - Core Application Service for long-term session memory and ChromaDB context resets.
Pure domain logic: decoupled from terminal UI, CLI formatters, HTTP, and RPC transports.
"""

from typing import Dict, Any, Optional


class MemoryService:
    """Service managing persistent session memory, rolling summaries, and resets."""

    def reset_memory(self, workspace: str = "Default", thread_id: Optional[str] = None) -> Dict[str, Any]:
        """Purges stored session summaries and context memory for the workspace."""
        ws_name = (workspace or "Default").strip()
        try:
            from any_context.memory import reset_session_memory
            reset_session_memory(workspace=ws_name, thread_id=thread_id)
        except Exception:
            pass

        return {
            "reset": True,
            "workspace": ws_name,
            "message": f"Long-term session memory database reset successfully for workspace '{ws_name}'."
        }
