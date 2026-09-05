"""
SourceService - Core Application Service for folder, web, and cloud source management.
Pure domain logic: decoupled from terminal UI, CLI formatters, HTTP, and RPC transports.
"""

import os
from typing import Dict, Any, List, Optional
from any_context.config.db_store import ConfigDBStore


class SourceService:
    """Service managing indexed sources (folders, web portals, shared assets) across workspaces."""

    def __init__(self, store: Optional[ConfigDBStore] = None, lance_store: Optional[Any] = None):
        self.store = store or ConfigDBStore()
        self.lance_store = lance_store

    def list_sources(self, workspace: str) -> Dict[str, Any]:
        """Returns all configured sources (folders, web URLs, cloud drives) for a workspace enriched with live LanceDB metrics."""
        ws_name = (workspace or "Default").strip()
        all_sources = self.store.get_workspace_sources(workspace_name=ws_name)
        raw_folders = all_sources.get("folders", [])
        web_sources = all_sources.get("web_sources", [])
        web_urls = all_sources.get("web_urls", [])

        # Harmonize web_urls and web_sources
        if not web_urls and web_sources:
            web_urls = [w.get("url") if isinstance(w, dict) else str(w) for w in web_sources]
        elif web_urls and not web_sources:
            web_sources = [{"url": u, "title": u, "page_count": 1} if isinstance(u, str) else u for u in web_urls]

        cloud_drives = all_sources.get("cloud_drives", [])

        # Enrich metrics directly from LanceDB (Single Source of Truth)
        if self.lance_store is not None:
            lance_store = self.lance_store
        else:
            from any_context.vector_engine.store import LanceDBStore
            lance_store = LanceDBStore.get_instance()

        # 1. Enrich folders with indexed file counts
        folder_details: List[Dict[str, Any]] = []
        for f in raw_folders:
            f_count = lance_store.get_indexed_folder_files_count(ws_name, folder_path=f)
            folder_details.append({
                "path": f,
                "file_count": f_count
            })

        # 2. Enrich web sources with live LanceDB page counts and auto-heal SQLite cache
        enriched_web: List[Dict[str, Any]] = []
        for w in web_sources:
            w_copy = dict(w) if isinstance(w, dict) else {"url": str(w), "title": str(w)}
            root_u = w_copy.get("root_url") or w_copy.get("url", "")
            live_count = lance_store.get_indexed_pages_count(ws_name, domain_or_prefix=root_u)
            if live_count > 0:
                w_copy["page_count"] = live_count
                # Auto-heal SQLite cache if desynchronized
                try:
                    target_id = w_copy.get("id") or root_u
                    if target_id and w.get("page_count") != live_count:
                        self.store.update_web_url_page_count(target_id, live_count)
                except Exception:
                    pass
            elif not w_copy.get("page_count"):
                w_copy["page_count"] = 1
            enriched_web.append(w_copy)

        summary = lance_store.get_workspace_inventory_summary(ws_name)
        total_count = len(raw_folders) + len(web_urls or web_sources) + len(cloud_drives)
        return {
            "workspace": ws_name,
            "folders": raw_folders,
            "folder_details": folder_details,
            "web_sources": enriched_web,
            "web_urls": web_urls,
            "cloud_drives": cloud_drives,
            "sources": all_sources.get("sources", []),
            "total_count": total_count,
            "total_indexed_files": summary.get("total_files", 0),
            "total_indexed_pages": summary.get("total_web_pages", 0),
            "total_chunks": summary.get("total_chunks", 0)
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

        try:
            from any_context.ingestion.web_scheduler import WebSchedulerStore
            web_store = WebSchedulerStore()
            web_store.add_web_url(workspace_name=ws_name, url=clean_url)
        except Exception as err:
            err_str = str(err).lower()
            if "decompress" in err_str or "truncated stream" in err_str or "-5" in err_str or "zlib" in err_str:
                import time
                time.sleep(0.1)
                from any_context.ingestion.web_scheduler import WebSchedulerStore
                web_store = WebSchedulerStore()
                web_store.add_web_url(workspace_name=ws_name, url=clean_url)
            else:
                raise

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
        clean_from = from_ws.strip()
        clean_to = to_ws.strip()
        clean_item = item.strip()
        if clean_item.lower().startswith("http://") or clean_item.lower().startswith("https://"):
            from any_context.ingestion.web_scheduler import WebSchedulerStore
            web_store = WebSchedulerStore()
            return web_store.transfer_web_source(source_ws=clean_from, target_ws=clean_to, url_or_root=clean_item)
        else:
            return self.store.transfer_local_folder_source(source_ws=clean_from, target_ws=clean_to, folder_path=clean_item)

    def link_source(self, source_identifier: str, target_workspace: str = "Default") -> Dict[str, Any]:
        """Links a folder source to target workspace."""
        clean_source = source_identifier.strip()
        clean_ws = target_workspace.strip()
        return self.add_folder(clean_ws, clean_source)

    def unlink_source(self, source_identifier: str, target_workspace: str = "Default") -> Dict[str, Any]:
        """Unlinks a folder source from workspace."""
        clean_source = source_identifier.strip()
        clean_ws = target_workspace.strip()
        return self.remove_folder(clean_ws, clean_source)

    def list_shared_sources(self) -> List[Dict[str, Any]]:
        """Lists shared sources in the Shared Sources workspace."""
        shared_ws = self.list_sources("Shared Sources")
        results = []
        for f in shared_ws.get("folders", []):
            results.append({"identifier": f, "source_type": "folder"})
        for w in shared_ws.get("web_sources", []):
            results.append({"identifier": w.get("url") or w.get("root_url", ""), "source_type": "web"})
        return results
