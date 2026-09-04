"""
AnyContext Ingestion Core Package.
Provides lazy-loaded indexing pipelines for local folders, session memories,
web resources, and cross-source background orchestration.
"""

from any_context.ingestion.orchestrator import (
    BackgroundSyncManager,
    check_workspace_changes,
    format_sync_status_box,
    clear_context_vector_db,
)

__all__ = [
    "index_folder",
    "run_index_folder",
    "BackgroundSyncManager",
    "check_workspace_changes",
    "format_sync_status_box",
    "clear_context_vector_db",
    "run_unified_sync",
]


def __getattr__(name: str):
    if name in ("index_folder", "run_index_folder"):
        from any_context.ingestion.local_folder_ingestor import index_folder, run_index_folder
        return {"index_folder": index_folder, "run_index_folder": run_index_folder}[name]
    if name == "run_unified_sync":
        from any_context.ingestion.unified_sync import run_unified_sync
        return run_unified_sync
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


