"""
AnyContext Ingestion & Workspace Multi-Source Orchestrator (v0.24.0).
Provides shared cross-source orchestration, non-blocking background thread management,
atomic progress telemetry with Unicode micro-bar rendering, and multi-source workspace state inspection.
"""
import os
import sys
import time
import threading
from typing import Dict, Any, List, Optional, Callable

from any_context.config.app_settings import AppSettings
from any_context.config.db_store import ConfigDBStore


def safe_print(msg: str):
    """Prints strings safely avoiding UnicodeEncodeError on legacy Windows consoles."""
    try:
        print(msg)
    except UnicodeEncodeError:
        print(msg.encode("ascii", errors="ignore").decode("ascii"))


def clear_context_vector_db(verbose: bool = False):
    """
    Purges LanceDB vector tables, docstore cache, and SQLite file stat cache
    when embedding models are changed or when full reset is requested.
    """
    try:
        current_settings = AppSettings.load()
        db_path = current_settings.context.db_path if current_settings and current_settings.context else "./context_db"

        # 1. Clear SQLite stat cache
        store = ConfigDBStore()
        store.clear_workspace_files_cache()

        # 2. Delete LanceDB vector records
        try:
            from any_context.vector_engine.store import LanceDBStore
            lance_store = LanceDBStore.get_instance(db_path=os.path.join(db_path, "lancedb"))
            lance_store.delete_all_records()
        except Exception:
            pass

        # 3. Clean any legacy docstore.json if present
        docstore_path = os.path.join(db_path, "docstore.json")
        if os.path.exists(docstore_path):
            try:
                os.remove(docstore_path)
            except Exception:
                pass

        if verbose:
            safe_print("│ ├─ 🧹 LanceDB vector table, caches, and filesystem stat cache cleared for re-indexing")
    except Exception as e:
        if verbose:
            safe_print(f"│ ├─ ⚠️ Warning during vector db clear: {e}")


def check_workspace_changes(workspace_name: str) -> Dict[str, Any]:
    """
    Performs ultra-fast (<30ms) holistic scan over all workspace sources:
    1. Local Folders: Compares (file_path, mtime, size) against workspace_files_stat_cache in SQLite.
    2. Web Sources: Queries registered web URLs, total page counts, and last scrape dates.
    3. Cloud Drives: Queries registered cloud drives and sync state.
    4. Shared Links: Reusable sources linked to this workspace.
    """
    store = ConfigDBStore()
    clean_ws = (workspace_name or "Default").strip()

    # Query multi-source details from SQLite
    ws_sources = store.get_workspace_sources(clean_ws)
    folders = list(ws_sources.get("folders", []))
    web_sources = ws_sources.get("web_sources", [])
    cloud_drives = ws_sources.get("cloud_drives", [])
    unified_sources = ws_sources.get("sources", [])
    total_web_pages = sum(w.get("page_count", 1) or 1 for w in web_sources)

    # Also merge folders from AppSettings if present
    try:
        current_settings = AppSettings.load()
        if current_settings and current_settings.workspaces:
            for ws in current_settings.workspaces:
                if ws.name.lower() == clean_ws.lower() or (getattr(ws, "workspace_id", None) and ws.workspace_id == clean_ws):
                    for p in (ws.paths or []):
                        norm_p = os.path.abspath(p.strip().strip("'\""))
                        if norm_p and norm_p not in folders:
                            folders.append(norm_p)
                    break
    except Exception:
        pass

    # Discover local disk files
    from any_context.ingestion.local_folder_ingestor import discover_workspace_files
    disk_files: Dict[str, Dict[str, Any]] = {}
    for folder_path in folders:
        if os.path.exists(folder_path):
            discovered = discover_workspace_files(folder_path)
            for f in discovered:
                try:
                    st = os.stat(f)
                    disk_files[f] = {
                        "file_path": f,
                        "last_mtime": st.st_mtime,
                        "file_size": st.st_size
                    }
                except Exception:
                    pass

    cached_files = store.get_workspace_files_cache(clean_ws)
    is_virgin = len(cached_files) == 0 and len(disk_files) > 0

    # Self-healing check: verify if the vector table actually has indexed chunks for this workspace
    if len(disk_files) > 0 and not is_virgin:
        try:
            from any_context.vector_engine.store import LanceDBStore
            lance = LanceDBStore.get_instance()
            ws_chunks = lance.count_records(clean_ws)
            if ws_chunks == 0:
                is_virgin = True
                cached_files = {}
        except Exception:
            pass

    new_files = []
    modified_files = []
    deleted_files = []

    for fp, d_info in disk_files.items():
        if fp not in cached_files:
            new_files.append(fp)
        else:
            c_info = cached_files[fp]
            if abs(c_info["last_mtime"] - d_info["last_mtime"]) > 0.001 or c_info["file_size"] != d_info["file_size"]:
                modified_files.append(fp)

    for fp in cached_files.keys():
        if fp not in disk_files:
            deleted_files.append(fp)

    # Detect zero-cost renamed/moved files
    renamed_files = []
    remaining_new = list(new_files)
    remaining_deleted = list(deleted_files)

    for del_f in list(deleted_files):
        del_size = cached_files[del_f]["file_size"]
        del_base = os.path.basename(del_f)
        del_ext = os.path.splitext(del_f)[1].lower()
        for new_f in list(remaining_new):
            new_size = disk_files[new_f]["file_size"]
            new_base = os.path.basename(new_f)
            new_ext = os.path.splitext(new_f)[1].lower()

            is_match = False
            if del_size == new_size:
                if del_base == new_base:
                    is_match = True
                elif del_ext == new_ext and (os.path.dirname(del_f) == os.path.dirname(new_f) or len(deleted_files) == 1 or len(new_files) == 1):
                    is_match = True

            if is_match:
                renamed_files.append((del_f, new_f))
                if del_f in remaining_deleted:
                    remaining_deleted.remove(del_f)
                if new_f in remaining_new:
                    remaining_new.remove(new_f)
                break

    new_files = remaining_new
    deleted_files = remaining_deleted

    has_changes = bool(new_files or modified_files or deleted_files or renamed_files)
    is_up_to_date = not has_changes and not is_virgin

    parts = []
    if new_files:
        parts.append(f"{len(new_files)} new file{'s' if len(new_files) > 1 else ''}")
    if modified_files:
        parts.append(f"{len(modified_files)} modified file{'s' if len(modified_files) > 1 else ''}")
    if deleted_files:
        parts.append(f"{len(deleted_files)} deleted file{'s' if len(deleted_files) > 1 else ''}")
    if renamed_files:
        parts.append(f"{len(renamed_files)} renamed file{'s' if len(renamed_files) > 1 else ''}")

    if parts:
        summary = ", ".join(parts)
    elif len(unified_sources) == 0:
        summary = "No sources configured."
    elif len(folders) == 0 and len(web_sources) > 0:
        summary = f"All {len(web_sources)} web sources ({total_web_pages} pages) up to date."
    elif len(folders) == 0 and len(cloud_drives) > 0:
        summary = f"All {len(cloud_drives)} cloud drives connected."
    else:
        summary = "Up to date (0 changes)"

    return {
        "workspace_name": clean_ws,
        "workspace_id": ws_sources.get("workspace_id", clean_ws),
        "total_sources": len(unified_sources),
        "sources": unified_sources,
        "folders": folders,
        "web_sources": web_sources,
        "cloud_drives": cloud_drives,
        "local_folders_count": len(folders),
        "total_disk_files": len(disk_files),
        "total_cached_files": len(cached_files),
        "web_sources_count": len(web_sources),
        "web_pages_count": total_web_pages,
        "cloud_drives_count": len(cloud_drives),
        "is_up_to_date": is_up_to_date,
        "has_changes": has_changes,
        "is_virgin": is_virgin,
        "new_files": new_files,
        "modified_files": modified_files,
        "deleted_files": deleted_files,
        "renamed_files": renamed_files,
        "summary": summary
    }


def format_sync_status_box(diff: Dict[str, Any]) -> str:
    """
    Proxy for backward compatibility.
    Terminal visual status card formatting is part of the CLI presentation layer (any_context.cli.formatters).
    """
    from any_context.cli.formatters import format_sync_status_box as _cli_format_sync_status_box
    return _cli_format_sync_status_box(diff)



class BackgroundSyncManager:
    """Thread-safe manager for background workspace synchronization workers."""
    _instance = None
    _lock = threading.RLock()

    def __new__(cls):
        with cls._lock:
            if cls._instance is None:
                cls._instance = super(BackgroundSyncManager, cls).__new__(cls)
                cls._instance._active_jobs = {}
                cls._instance._progress = {}
                cls._instance._notifications = {}
                cls._instance._completion_listeners = []
                cls._instance._pending_syncs = set()
            return cls._instance

    def register_completion_listener(self, callback: Callable[[Dict[str, Any]], None]) -> None:
        """Registers a global listener called when any background sync/crawl job finishes."""
        with self._lock:
            if callback not in self._completion_listeners:
                self._completion_listeners.append(callback)

    def unregister_completion_listener(self, callback: Callable[[Dict[str, Any]], None]) -> None:
        """Unregisters a global completion listener."""
        with self._lock:
            if callback in self._completion_listeners:
                self._completion_listeners.remove(callback)

    def pop_notifications(self, workspace_name: Optional[str] = None) -> List[Dict[str, Any]]:
        """Pops and returns pending completion notifications for a workspace or all."""
        with self._lock:
            if workspace_name:
                clean_ws = workspace_name.strip()
                return self._notifications.pop(clean_ws, [])
            all_notifs = []
            for ws in list(self._notifications.keys()):
                all_notifs.extend(self._notifications.pop(ws, []))
            return all_notifs

    def is_syncing(self, workspace_name: str) -> bool:
        clean_ws = (workspace_name or "Default").strip()
        with self._lock:
            job = self._active_jobs.get(clean_ws)
            if job and job["thread"].is_alive():
                return True
            return False

    def update_progress(
        self,
        workspace_name: str,
        current: int,
        total: int,
        stage: str = "files",
        item_name: str = ""
    ) -> None:
        """Atomically updates the synchronization progress telemetry for a workspace."""
        clean_ws = (workspace_name or "Default").strip()
        pct = round((current / total * 100), 1) if total > 0 else 0.0
        with self._lock:
            self._progress[clean_ws] = {
                "current": current,
                "total": total,
                "pct": pct,
                "stage": stage,
                "item_name": item_name
            }

    def get_progress(self, workspace_name: str) -> Dict[str, Any]:
        """Returns the current synchronization progress telemetry for a workspace."""
        clean_ws = (workspace_name or "Default").strip()
        with self._lock:
            return self._progress.get(
                clean_ws,
                {"current": 0, "total": 0, "pct": 0.0, "stage": "idle", "item_name": ""}
            )

    def format_progress_bar(self, workspace_name: str, width: int = 8) -> str:
        """
        Formats a compact Unicode block progress bar:
        [████░░░░] 50% (15/30 files) or [████░░░░] 50% (15/30 pages) or [crawling...]
        """
        clean_ws = (workspace_name or "Default").strip()
        prog = self.get_progress(clean_ws)
        total = prog.get("total", 0)
        current = prog.get("current", 0)
        pct = prog.get("pct", 0.0)
        stage = prog.get("stage", "files")

        if total <= 0:
            if stage in ["pages", "web", "crawling"]:
                return "[crawling...]"
            if stage == "scanning":
                return "[scanning...]"
            return "[calculating...]"

        fill = int(round(width * (current / total))) if total > 0 else 0
        fill = min(width, max(0, fill))
        bar = "█" * fill + "░" * (width - fill)
        stage_suffix = f" {stage}" if stage in ["files", "pages", "drives", "web"] else ""
        return f"[{bar}] {int(pct)}% ({current}/{total}{stage_suffix})"

    def get_sync_status(self, workspace_name: str) -> Dict[str, Any]:
        clean_ws = (workspace_name or "Default").strip()
        with self._lock:
            job = self._active_jobs.get(clean_ws)
            if not job:
                return {"status": "idle", "workspace_name": clean_ws}
            is_alive = job["thread"].is_alive()
            prog = self._progress.get(
                clean_ws,
                {"current": 0, "total": 0, "pct": 0.0, "stage": "idle", "item_name": ""}
            )
            return {
                "workspace_name": clean_ws,
                "status": "syncing" if is_alive else job.get("status", "completed"),
                "start_time": job.get("start_time"),
                "progress": prog,
                "progress_bar": self.format_progress_bar(clean_ws),
                "result": job.get("result"),
                "error": job.get("error")
            }

    def start_background_sync(
        self,
        workspace_name: str,
        sync_folders: bool = True,
        sync_web: bool = True,
        sync_drives: bool = True,
        force_full: bool = False,
        verbose: bool = False,
        is_all: bool = False,
        on_complete: Optional[Callable[[Dict[str, Any]], None]] = None
    ) -> threading.Thread:
        clean_ws = (workspace_name or "Default").strip()
        with self._lock:
            if clean_ws in self._active_jobs and self._active_jobs[clean_ws]["thread"].is_alive():
                self._pending_syncs.add(clean_ws)
                return self._active_jobs[clean_ws]["thread"]

        self.update_progress(clean_ws, current=0, total=0, stage="scanning")

        def _worker():
            try:
                from any_context.ingestion.unified_sync import run_unified_sync

                def _prog_cb(curr: int, tot: int, stg: str = "files", itm: str = ""):
                    self.update_progress(clean_ws, current=curr, total=tot, stage=stg, item_name=itm)

                while True:
                    res = run_unified_sync(
                        workspace_name=clean_ws if not is_all else None,
                        sync_folders=sync_folders,
                        sync_web=sync_web,
                        sync_drives=sync_drives,
                        force_full=force_full,
                        verbose=verbose,
                        is_all=is_all,
                        progress_callback=_prog_cb
                    )

                    with self._lock:
                        if clean_ws in self._pending_syncs:
                            self._pending_syncs.remove(clean_ws)
                            continue
                        break
                
                # Format completion summary
                folder_res = res.get("folder_results", {}).get(clean_ws, {}) if isinstance(res, dict) else {}
                web_res = res.get("web_results", {}).get(clean_ws, {}) if isinstance(res, dict) else {}
                indexed_files = folder_res.get("indexed_files", 0) if isinstance(folder_res, dict) else 0
                total_web_pages = 0
                if isinstance(web_res, dict) and "synced" in web_res:
                    for item in web_res["synced"]:
                        r = item.get("result", {})
                        if isinstance(r, dict):
                            total_web_pages += r.get("indexed_count", r.get("char_count", 1 if r.get("status") == "success" else 0))

                summary_parts = []
                if indexed_files > 0:
                    summary_parts.append(f"{indexed_files} local file(s) indexed")
                if total_web_pages > 0:
                    summary_parts.append(f"{total_web_pages} web page(s) crawled and indexed")

                if summary_parts:
                    summary_str = f"✔ Workspace '{clean_ws}' synchronization completed ({', '.join(summary_parts)})."
                else:
                    summary_str = f"✔ Workspace '{clean_ws}' is up to date."

                notif = {
                    "workspace": clean_ws,
                    "type": "sync_completed",
                    "message": summary_str,
                    "timestamp": time.time(),
                    "success": True,
                    "result": res
                }

                with self._lock:
                    self._active_jobs[clean_ws]["status"] = "completed"
                    self._active_jobs[clean_ws]["result"] = res
                    self._progress[clean_ws] = {
                        "current": self._progress.get(clean_ws, {}).get("total", 1),
                        "total": self._progress.get(clean_ws, {}).get("total", 1),
                        "pct": 100.0,
                        "stage": "done",
                        "item_name": ""
                    }
                    if clean_ws not in self._notifications:
                        self._notifications[clean_ws] = []
                    self._notifications[clean_ws].append(notif)
                    listeners = list(self._completion_listeners)

                # Dispatch notifications to listeners
                for listener in listeners:
                    try:
                        listener(notif)
                    except Exception:
                        pass

                if on_complete:
                    try:
                        on_complete(res)
                    except Exception:
                        pass
            except Exception as e:
                err_msg = f"❌ Background synchronization failed for '{clean_ws}': {str(e)}"
                notif = {
                    "workspace": clean_ws,
                    "type": "sync_failed",
                    "message": err_msg,
                    "timestamp": time.time(),
                    "success": False,
                    "error": str(e)
                }
                with self._lock:
                    self._active_jobs[clean_ws]["status"] = "failed"
                    self._active_jobs[clean_ws]["error"] = str(e)
                    if clean_ws not in self._notifications:
                        self._notifications[clean_ws] = []
                    self._notifications[clean_ws].append(notif)
                    listeners = list(self._completion_listeners)

                for listener in listeners:
                    try:
                        listener(notif)
                    except Exception:
                        pass

        t = threading.Thread(target=_worker, daemon=True, name=f"SyncWorker-{clean_ws}")
        with self._lock:
            self._active_jobs[clean_ws] = {
                "thread": t,
                "status": "syncing",
                "start_time": time.time(),
                "result": None,
                "error": None
            }
        t.start()
        return t

