"""
WorkspaceService - Core Application Service for workspace lifecycle management.
Pure domain logic: decoupled from terminal UI, CLI formatters, HTTP, and RPC transports.
"""

from typing import List, Dict, Any, Optional
from any_context.config.db_store import ConfigDBStore
from any_context.config.app_settings import AppSettings


class WorkspaceService:
    """Service managing workspace creation, deletion, renaming, and retrieval."""

    def __init__(self, store: Optional[ConfigDBStore] = None):
        self.store = store or ConfigDBStore()

    def list_workspaces(self, active_workspace: Optional[str] = None) -> List[Dict[str, Any]]:
        """Returns all configured workspaces with metadata and active flag."""
        settings = self.store.get_app_settings()
        known = settings.workspaces if settings and settings.workspaces else []
        active = (active_workspace or "Default").strip()

        result = []
        for ws in known:
            result.append({
                "name": ws.name,
                "is_active": (ws.name.lower() == active.lower()),
                "paths": getattr(ws, "paths", []) or [],
                "created_at": getattr(ws, "created_at", None),
            })
        return result

    def get_workspace_meta(self, name: str) -> Optional[Dict[str, Any]]:
        """Retrieves metadata for a specific workspace."""
        if not name:
            return None
        return self.store.get_workspace_meta(name.strip())

    def create_workspace(self, name: str, initial_paths: Optional[List[str]] = None) -> Dict[str, Any]:
        """Creates a new workspace in the SQLite store."""
        clean_name = name.strip()
        if not clean_name:
            raise ValueError("Workspace name cannot be empty.")

        existing = self.store.get_workspace_meta(clean_name)
        if existing:
            return {"created": False, "name": existing["name"], "message": f"Workspace '{existing['name']}' already exists."}

        self.store.add_workspace(name=clean_name, paths=initial_paths or [])
        return {"created": True, "name": clean_name, "message": f"Workspace '{clean_name}' created successfully."}

    def delete_workspace(self, name: str) -> Dict[str, Any]:
        """
        Deletes a workspace. Protects system workspaces ('Default', 'Shared Sources').
        """
        clean_name = name.strip()
        if not clean_name:
            raise ValueError("Workspace name cannot be empty.")

        protected = ["default", "shared sources"]
        if clean_name.lower() in protected:
            raise ValueError(f"Cannot delete protected system workspace '{clean_name}'.")

        deleted = self.store.remove_workspace(clean_name)
        if not deleted:
            raise ValueError(f"Workspace '{clean_name}' not found.")

        return {"deleted": True, "name": clean_name, "message": f"Workspace '{clean_name}' deleted successfully."}

    def rename_workspace(self, old_name: str, new_name: str) -> Dict[str, Any]:
        """
        Renames a workspace and triggers vector store metadata migration.
        """
        old_clean = old_name.strip()
        new_clean = new_name.strip()

        if not old_clean or not new_clean:
            raise ValueError("Old and new workspace names cannot be empty.")

        if old_clean.lower() == new_clean.lower():
            return {"renamed": False, "name": old_clean, "message": "New name is identical to current name."}

        protected = ["default", "shared sources"]
        if old_clean.lower() in protected:
            raise ValueError(f"Cannot rename protected system workspace '{old_clean}'.")

        # Rename in SQLite store
        meta = self.store.get_workspace_meta(old_clean)
        if not meta:
            raise ValueError(f"Workspace '{old_clean}' does not exist.")

        self.store.rename_workspace(old_clean, new_clean)

        # Migrate LanceDB vector store metadata
        vector_count = 0
        try:
            from any_context.vector_engine.store import VectorStore
            vstore = VectorStore()
            vector_count = vstore.rename_workspace_records(old_clean, new_clean)
        except Exception:
            pass

        return {
            "renamed": True,
            "old_name": old_clean,
            "new_name": new_clean,
            "migrated_records": vector_count,
            "message": f"Workspace '{old_clean}' renamed to '{new_clean}' ({vector_count} vector records updated)."
        }
