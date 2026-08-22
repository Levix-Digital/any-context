import os
from typing import Optional, Dict, Any, List
from any_context.config.db_store import ConfigDBStore
from any_context.ingestion.local_folder_ingestor import run_index_folder, check_workspace_changes
from any_context.ingestion.web_scheduler import WebSchedulerStore, sync_workspace_web_urls

def run_unified_sync(
    workspace_name: Optional[str] = None,
    sync_folders: bool = True,
    sync_web: bool = True,
    sync_drives: bool = True,
    force_full: bool = False,
    verbose: bool = False,
    is_all: bool = False
) -> Dict[str, Any]:
    """
    Unified synchronization orchestrator across all source categories:
    - 📁 Local Folders (run_index_folder)
    - 🌐 Web Portals (sync_workspace_web_urls)
    - ☁️ Cloud Drives (when configured)
    """
    store = ConfigDBStore()
    web_store = WebSchedulerStore()

    if is_all:
        settings = store.get_app_settings()
        if settings and settings.workspaces:
            target_ws_list = [ws.name for ws in settings.workspaces if ws.name]
        else:
            target_ws_list = ["Default"]
    else:
        target_ws_list = [workspace_name] if workspace_name else ["Default"]

    results: Dict[str, Any] = {
        "workspaces": target_ws_list,
        "folder_results": {},
        "web_results": {},
        "drive_results": {}
    }

    for ws in target_ws_list:
        # 1. Synchronize Local Folders
        if sync_folders:
            folder_res = run_index_folder(workspace_name=ws, verbose=verbose, force_full=force_full)
            results["folder_results"][ws] = folder_res

        # 2. Synchronize Web Sources
        if sync_web:
            ws_urls = web_store.get_workspace_web_urls(ws)
            if ws_urls:
                if verbose:
                    try:
                        print(f"\n🌐 Synchronizing {len(ws_urls)} web source(s) for workspace '{ws}'...")
                    except UnicodeEncodeError:
                        print(f"\n[Web] Synchronizing {len(ws_urls)} web source(s) for workspace '{ws}'...")
                web_res = sync_workspace_web_urls(ws, force=force_full)
                results["web_results"][ws] = web_res
            else:
                results["web_results"][ws] = {"status": "empty", "total_urls": 0}

        # 3. Synchronize Cloud Drives
        if sync_drives:
            results["drive_results"][ws] = {"status": "up_to_date", "total_drives": 0}

    return results
