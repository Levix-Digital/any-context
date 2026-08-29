"""
SourceService - Core Application Service for folder, web, and cloud source management.
Pure domain logic: decoupled from terminal UI, CLI formatters, HTTP, and RPC transports.
"""

import os
from typing import Dict, Any, List, Optional
from any_context.config.db_store import ConfigDBStore


class SourceService:
    """Service managing indexed sources (folders, web portals, shared assets) across workspaces."""

    def __init__(self, store: Optional[ConfigDBStore] = None):
        self.store = store or ConfigDBStore()

    def list_sources(self, workspace: str) -> Dict[str, Any]:
        """Returns all configured sources (folders, web URLs, cloud drives) for a workspace."""
        ws_name = (workspace or "Default").strip()
        all_sources = self.store.get_workspace_sources(workspace_name=ws_name)
        folders = all_sources.get("folders", [])
        web_sources = all_sources.get("web_sources", [])
        web_urls = all_sources.get("web_urls", [])

        # Harmonize web_urls and web_sources
        if not web_urls and web_sources:
            web_urls = [w.get("url") if isinstance(w, dict) else str(w) for w in web_sources]
        elif web_urls and not web_sources:
            web_sources = [{"url": u, "title": u, "page_count": 1} if isinstance(u, str) else u for u in web_urls]

        cloud_drives = all_sources.get("cloud_drives", [])
        total_count = len(folders) + len(web_urls or web_sources) + len(cloud_drives)
        return {
            "workspace": ws_name,
            "folders": folders,
            "web_sources": web_sources,
            "web_urls": web_urls,
            "cloud_drives": cloud_drives,
            "sources": all_sources.get("sources", []),
            "total_count": total_count
        }

    def add_folder(self, workspace: str, folder_path: str) -> Dict[str, Any]:
        """Adds a local directory path to the specified workspace."""
        from any_context.core.utils import resolve_folder_path
        ws_name = (workspace or "Default").strip()
        clean_path = resolve_folder_path(folder_path)

        if not os.path.exists(clean_path):
            raise FileNotFoundError(f"Folder path '{clean_path}' does not exist on disk.")
        if not os.path.isdir(clean_path):
            raise NotADirectoryError(f"Path '{clean_path}' is a file, not a directory.")

        # Ensure workspace exists
        if not self.store.get_workspace_meta(ws_name):
            self.store.add_workspace(ws_name, paths=[clean_path])
        else:
            self.store.add_folder_to_workspace(ws_name, clean_path)

        return {
            "added": True,
            "type": "folder",
            "workspace": ws_name,
            "path": clean_path,
            "message": f"Added folder '{clean_path}' to workspace '{ws_name}'."
        }

    def remove_folder(self, workspace: str, folder_path: str) -> Dict[str, Any]:
        """Removes a folder from the specified workspace."""
        from any_context.core.utils import resolve_folder_path
        ws_name = (workspace or "Default").strip()
        clean_path = resolve_folder_path(folder_path)

        removed = self.store.remove_folder_from_workspace(ws_name, clean_path)
        if not removed:
            raise ValueError(f"Folder '{clean_path}' was not found in workspace '{ws_name}'.")

        return {
            "removed": True,
            "type": "folder",
            "workspace": ws_name,
            "path": clean_path,
            "message": f"Removed folder '{clean_path}' from workspace '{ws_name}'."
        }

    def add_web(self, workspace: str, url: str) -> Dict[str, Any]:
        """Registers a web portal / documentation URL into the workspace."""
        ws_name = (workspace or "Default").strip()
        clean_url = url.strip()

        if not clean_url.startswith("http://") and not clean_url.startswith("https://"):
            clean_url = "https://" + clean_url

        from any_context.ingestion.web_scheduler import WebSchedulerStore
        web_store = WebSchedulerStore()
        web_store.add_web_url(workspace_name=ws_name, url=clean_url)

        return {
            "added": True,
            "type": "web",
            "workspace": ws_name,
            "url": clean_url,
            "message": f"Added web source '{clean_url}' to workspace '{ws_name}'."
        }

    def remove_web(self, workspace: str, url: str) -> Dict[str, Any]:
        """Removes a web source from the workspace."""
        ws_name = (workspace or "Default").strip()
        clean_url = url.strip()

        from any_context.ingestion.web_scheduler import WebSchedulerStore
        web_store = WebSchedulerStore()
        removed = web_store.delete_web_url_by_url(workspace_name=ws_name, url=clean_url)
        if not removed:
            raise ValueError(f"Web URL '{clean_url}' not found in workspace '{ws_name}'.")

        return {
            "removed": True,
            "type": "web",
            "workspace": ws_name,
            "url": clean_url,
            "message": f"Removed web source '{clean_url}' from workspace '{ws_name}'."
        }

    def transfer_source(self, from_ws: str, to_ws: str, item: str) -> Dict[str, Any]:
        """Transfers a source from one workspace to another with instant vector remapping."""
        from any_context.workspace_sharing.transfer_engine import transfer_source_between_workspaces
        result = transfer_source_between_workspaces(
            from_workspace=from_ws.strip(),
            to_workspace=to_ws.strip(),
            source_identifier=item.strip(),
            store=self.store
        )
        return result

    def link_source(self, source_identifier: str, target_workspace: str) -> Dict[str, Any]:
        """Links a source to the Shared Sources repository."""
        from any_context.workspace_sharing.shared_manager import link_source_to_workspace
        return link_source_to_workspace(source_identifier, target_workspace, store=self.store)

    def unlink_source(self, source_identifier: str, target_workspace: str) -> Dict[str, Any]:
        """Unlinks a shared source from a workspace."""
        from any_context.workspace_sharing.shared_manager import unlink_source_from_workspace
        return unlink_source_from_workspace(source_identifier, target_workspace, store=self.store)

    def list_shared_sources(self) -> List[Dict[str, Any]]:
        """Lists all shared reusable sources."""
        from any_context.workspace_sharing.shared_manager import list_all_shared_sources
        return list_all_shared_sources(store=self.store)
