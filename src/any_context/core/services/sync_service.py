"""
SyncService - Core Application Service for workspace indexing and background synchronization.
Pure domain logic: decoupled from terminal UI, CLI formatters, HTTP, and RPC transports.
"""

from typing import Dict, Any, Optional
from any_context.ingestion.local_folder_ingestor import (
    BackgroundSyncManager,
    check_workspace_changes,
    run_index_folder
)


class SyncService:
    """Service managing background document indexing, web crawls, and sync status."""

    def __init__(self):
        self.bg_mgr = BackgroundSyncManager()

    def start_sync(self, workspace: str = "Default", force_full: bool = False) -> Dict[str, Any]:
        """Dispatches an asynchronous background synchronization job for the workspace."""
        ws_name = (workspace or "Default").strip()
        self.bg_mgr.start_background_sync(workspace_name=ws_name, force_full=force_full, verbose=False)
        return {
            "started": True,
            "workspace": ws_name,
            "force_full": force_full,
            "message": f"Background synchronization started for workspace '{ws_name}'."
        }

    def get_sync_status(self, workspace: str = "Default") -> Dict[str, Any]:
        """Returns the current background sync progress and status string."""
        ws_name = (workspace or "Default").strip()
        is_syncing = self.bg_mgr.is_syncing(ws_name)
        progress_bar = self.bg_mgr.format_progress_bar(ws_name, width=8) if is_syncing else ""
        return {
            "workspace": ws_name,
            "is_syncing": is_syncing,
            "progress_bar": progress_bar,
            "status": f"Syncing {progress_bar}" if is_syncing else "Ready"
        }

    def check_changes(self, workspace: str = "Default") -> Dict[str, Any]:
        """Checks for diffs between disk files/web sources and the cached index."""
        ws_name = (workspace or "Default").strip()
        return check_workspace_changes(workspace_name=ws_name)
