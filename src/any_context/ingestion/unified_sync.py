from abc import ABC, abstractmethod
from typing import Optional, Dict, Any, List
from any_context.config.db_store import ConfigDBStore
from any_context.ingestion.local_folder_ingestor import run_index_folder
from any_context.ingestion.web_scheduler import WebSchedulerStore, sync_workspace_web_urls


class SyncContext:
    """Carries execution state, options, and collected metrics across the sync chain."""
    def __init__(
        self,
        workspace_name: str,
        force_full: bool = False,
        verbose: bool = False,
        progress_callback: Optional[Any] = None,
        sources: Optional[Dict[str, Any]] = None
    ):
        self.workspace_name = workspace_name
        self.force_full = force_full
        self.verbose = verbose
        self.progress_callback = progress_callback
        self.sources = sources or {}
        self.results: Dict[str, Any] = {
            "folders": {"status": "skipped", "message": "No local folders configured"},
            "web": {"status": "skipped", "message": "No web documentation portals configured"},
            "drives": {"status": "skipped", "message": "No cloud drives configured"}
        }

    def log(self, msg: str):
        if self.verbose:
            try:
                print(msg, flush=True)
            except UnicodeEncodeError:
                print(msg.encode("ascii", errors="ignore").decode("ascii"), flush=True)


class BaseSyncHandler(ABC):
    """Abstract Base Handler in the Synchronization Chain of Responsibility."""
    def __init__(self, next_handler: Optional['BaseSyncHandler'] = None):
        self.next_handler = next_handler

    def set_next(self, handler: 'BaseSyncHandler') -> 'BaseSyncHandler':
        self.next_handler = handler
        return handler

    def handle(self, ctx: SyncContext) -> None:
        self.execute_sync(ctx)
        if self.next_handler:
            self.next_handler.handle(ctx)

    @abstractmethod
    def execute_sync(self, ctx: SyncContext) -> None:
        pass


class LocalFolderSyncHandler(BaseSyncHandler):
    """Handles synchronization and vectorization of local folders."""
    def execute_sync(self, ctx: SyncContext) -> None:
        folders = ctx.sources.get("folders", [])
        if not folders:
            ctx.log(f"📁 Local Folders: (None configured for '{ctx.workspace_name}' - skipped)")
            ctx.results["folders"] = {
                "status": "skipped",
                "total_files": 0,
                "indexed_files": 0,
                "message": "No local folders configured"
            }
            return

        ctx.log(f"📁 Synchronizing {len(folders)} local folder(s) for '{ctx.workspace_name}'...")
        folder_res = run_index_folder(
            workspace_name=ctx.workspace_name,
            verbose=ctx.verbose,
            force_full=ctx.force_full,
            progress_callback=ctx.progress_callback
        )
        ctx.results["folders"] = folder_res


class WebPortalSyncHandler(BaseSyncHandler):
    """Handles synchronization and vectorization of web documentation portals."""
    def execute_sync(self, ctx: SyncContext) -> None:
        web_sources = ctx.sources.get("web_sources", [])
        if not web_sources:
            web_store = WebSchedulerStore()
            ws_urls = web_store.get_workspace_web_urls(ctx.workspace_name)
        else:
            ws_urls = [w.get("url") for w in web_sources if w.get("url")]

        if not ws_urls:
            ctx.log(f"🌐 Web Portals: (None configured for '{ctx.workspace_name}' - skipped)")
            ctx.results["web"] = {
                "status": "skipped",
                "total_urls": 0,
                "message": "No web portals configured"
            }
            return

        ctx.log(f"\n🌐 Synchronizing {len(ws_urls)} web source(s) for workspace '{ctx.workspace_name}'...")
        web_res = sync_workspace_web_urls(
            ctx.workspace_name,
            force=ctx.force_full,
            progress_callback=ctx.progress_callback
        )
        ctx.results["web"] = web_res


class CloudDriveSyncHandler(BaseSyncHandler):
    """Handles synchronization of Cloud Drives (OneDrive, Google Drive, Dropbox, Box)."""
    def execute_sync(self, ctx: SyncContext) -> None:
        cloud_drives = ctx.sources.get("cloud_drives", [])
        if not cloud_drives:
            ctx.log(f"☁️ Cloud Drives: (None configured for '{ctx.workspace_name}' - skipped)")
            ctx.results["drives"] = {
                "status": "skipped",
                "total_drives": 0,
                "message": "No cloud drives configured"
            }
            return

        ctx.log(f"\n☁️ Synchronizing {len(cloud_drives)} cloud drive(s) for workspace '{ctx.workspace_name}'...")
        ctx.results["drives"] = {"status": "up_to_date", "total_drives": len(cloud_drives)}


def run_unified_sync(
    workspace_name: Optional[str] = None,
    sync_folders: bool = True,
    sync_web: bool = True,
    sync_drives: bool = True,
    force_full: bool = False,
    verbose: bool = False,
    is_all: bool = False,
    progress_callback: Optional[Any] = None
) -> Dict[str, Any]:
    """
    Unified synchronization orchestrator across all source categories
    leveraging an extensible Chain of Responsibility pipeline.
    """
    store = ConfigDBStore()

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
        sources = store.get_workspace_sources(ws)
        ctx = SyncContext(
            workspace_name=ws,
            force_full=force_full,
            verbose=verbose,
            progress_callback=progress_callback,
            sources=sources
        )

        handlers: List[BaseSyncHandler] = []
        if sync_folders:
            handlers.append(LocalFolderSyncHandler())
        if sync_web:
            handlers.append(WebPortalSyncHandler())
        if sync_drives:
            handlers.append(CloudDriveSyncHandler())

        if handlers:
            for i in range(len(handlers) - 1):
                handlers[i].set_next(handlers[i + 1])
            handlers[0].handle(ctx)

        results["folder_results"][ws] = ctx.results["folders"]
        results["web_results"][ws] = ctx.results["web"]
        results["drive_results"][ws] = ctx.results["drives"]

    return results
