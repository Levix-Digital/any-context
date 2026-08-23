"""
AnyContext Local Folder Ingestion Engine (v0.24.0).
Provides specialized recursive discovery and incremental parsing for local filesystem documents
(PDFs, Word documents, text files, source code, spreadsheets, and images).

Decoupled Architecture:
- Cross-source orchestration, background threading, and workspace inspection are located in orchestrator.py.
- Vector embeddings and columnar storage are handled exclusively by ParallelIndexer & LanceDBStore.
"""
import os
import sys
import time
from typing import List, Dict, Any, Optional, Callable

from any_context.config.app_settings import AppSettings
from any_context.config.db_store import ConfigDBStore
from any_context.core.utils import get_api_key
from any_context.tools.search_tools import configure_embedding_model
from any_context.vector_engine.store import LanceDBStore
from any_context.vector_engine.indexer import ParallelIndexer
from any_context.vector_engine.models import IngestionConfig
from any_context.ingestion.orchestrator import (
    safe_print,
    clear_context_vector_db,
    check_workspace_changes,
    format_sync_status_box,
    BackgroundSyncManager
)
from llama_index.core import SimpleDirectoryReader, Document
from langchain.tools import tool

SUPPORTED_EXTENSIONS = {
    # Documents & Text
    ".pdf", ".docx", ".doc", ".txt", ".md", ".rtf", ".odt", ".pages", ".epub", ".eml", ".msg",
    # Data & Spreadsheets
    ".csv", ".tsv", ".json", ".jsonl", ".xlsx", ".xls", ".ods",
    # Presentations
    ".pptx", ".ppt", ".key",
    # Code & Tech
    ".py", ".js", ".ts", ".tsx", ".jsx", ".html", ".htm", ".css", ".xml", ".yaml", ".yml", ".toml", ".sql",
    ".c", ".cpp", ".cs", ".java", ".go", ".rs", ".sh", ".ps1", ".bat", ".cmd",
    # Images
    ".png", ".jpg", ".jpeg", ".webp"
}

IGNORED_DIRS = {
    ".git", ".svn", "node_modules", "__pycache__", ".venv", "venv", "env",
    ".vs", ".idea", ".vscode", "$recycle.bin", ".tmp"
}


def discover_workspace_files(root_folder: str) -> List[str]:
    """
    Recursively crawls all subfolders starting from root_folder using os.walk.
    Finds ALL supported files, handling case-insensitive extensions and ignoring lock/temp files.
    """
    valid_file_paths = []
    for root, dirs, files in os.walk(root_folder):
        dirs[:] = [d for d in dirs if d.lower() not in IGNORED_DIRS and not d.startswith(".")]

        for file_name in files:
            if file_name.startswith("~$") or file_name.startswith("._"):
                continue  # Skip Microsoft Office temporary lock files & macOS metadata files

            ext = os.path.splitext(file_name)[1].lower()
            if ext in SUPPORTED_EXTENSIONS:
                full_path = os.path.abspath(os.path.join(root, file_name))
                valid_file_paths.append(full_path)

    return valid_file_paths


def run_index_folder(
    workspace_name: str = None,
    verbose: bool = False,
    force_full: bool = False,
    progress_callback: Optional[Callable[[int, int, str, str], None]] = None
) -> Dict[str, Any]:
    """
    Index documents in the vector database incrementally across all configured workspaces,
    or a specific workspace if provided. Performs deep recursive scanning across all subdirectories.
    Uses SQLite stat cache (mtime & size) for sub-30ms bypass when unchanged and zero-cost file path migration.
    """
    if progress_callback:
        progress_callback(0, 0, "scanning", "")

    current_settings = AppSettings.load()
    if not current_settings or not current_settings.workspaces:
        if verbose:
            safe_print("❌ Error: No workspaces configured in settings.")
        return {"status": "error", "error": "No workspaces configured."}

    store = ConfigDBStore()
    target_ws_name = workspace_name or (current_settings.workspaces[0].name if current_settings.workspaces else "Global")
    db_save_path = current_settings.context.db_path if (current_settings and current_settings.context) else "./context_db"
    lance_store = LanceDBStore.get_instance(db_path=os.path.join(db_save_path, "lancedb"))

    if verbose:
        safe_print(f"\n[Ingestion Pipeline: {target_ws_name}]")
        safe_print(f"  • Storage: LanceDB Columnar Vector Store ({db_save_path}/lancedb)")

    # 1. Fast SQLite Stat Cache Diff Check
    if workspace_name and not force_full:
        diff = check_workspace_changes(target_ws_name)
        if diff["is_up_to_date"]:
            if verbose:
                safe_print(f"  • Stat Check: 100% up to date ({diff['total_disk_files']} files, 0 changes)")
                safe_print("✔ Ingestion completed successfully (0 changes)!\n")
            return {"status": "up_to_date", "total_files": diff["total_disk_files"], "changes": diff}

        # Zero-cost Renamed/Moved Files Metadata Repointing ($0.00)
        if diff["renamed_files"]:
            try:
                for old_p, new_p in diff["renamed_files"]:
                    store.rename_cached_file_path(target_ws_name, old_p, new_p)
                    lance_store.delete_by_file(old_p, workspace_name=target_ws_name)
                if verbose:
                    safe_print(f"  • Renamed: {len(diff['renamed_files'])} files repointed with zero-cost ($0.00)")
            except Exception:
                pass

        # Purge Deleted Files from LanceDB & SQLite cache
        if diff["deleted_files"]:
            try:
                for dfp in diff["deleted_files"]:
                    lance_store.delete_by_file(dfp, workspace_name=target_ws_name)
            except Exception:
                pass
            store.remove_workspace_files_cache(target_ws_name, diff["deleted_files"])
            if verbose:
                safe_print(f"  • Deleted: {len(diff['deleted_files'])} files purged")

        # If only deletions and renames occurred, return early
        if not diff["new_files"] and not diff["modified_files"]:
            if verbose:
                safe_print("✔ Ingestion completed successfully!\n")
            return {"status": "updated", "changes": diff}

    configure_embedding_model()

    chunk_size = current_settings.context.chunk_size if (current_settings and current_settings.context) else 1024
    chunk_overlap = current_settings.context.chunk_overlap if (current_settings and current_settings.context) else 200

    all_documents = []
    total_discovered_files = 0
    scanned_file_samples = []
    workspaces_to_process = []
    all_processed_files_for_cache = []

    for ws in current_settings.workspaces:
        if workspace_name and ws.name != workspace_name:
            continue
        workspaces_to_process.append(ws)

        ws_file_paths = []
        for folder_path in ws.paths:
            if not os.path.exists(folder_path):
                if verbose:
                    safe_print(f"│ ├─ ⚠️ Directory missing: {folder_path}")
                continue

            discovered_files = discover_workspace_files(folder_path)
            ws_file_paths.extend(discovered_files)
            total_discovered_files += len(discovered_files)
            scanned_file_samples.extend(discovered_files[:4])

        all_processed_files_for_cache.extend(ws_file_paths)

        # Load discovered files safely
        if ws_file_paths:
            total_ws_files = len(ws_file_paths)
            if progress_callback:
                progress_callback(1, total_ws_files, "files", os.path.basename(ws_file_paths[0]))
            try:
                reader = SimpleDirectoryReader(input_files=ws_file_paths)
                docs = reader.load_data()
                for idx, d in enumerate(docs):
                    d.metadata["workspace"] = ws.name
                    if "file_path" in d.metadata:
                        fp = d.metadata["file_path"]
                        d.id_ = fp
                        try:
                            mtime = os.path.getmtime(fp)
                            ctime = os.path.getctime(fp)
                            d.metadata["last_modified_date"] = time.strftime("%Y-%m-%d", time.localtime(mtime))
                            d.metadata["creation_date"] = time.strftime("%Y-%m-%d", time.localtime(ctime))
                            d.metadata["content_type"] = "Local Document"
                            d.metadata["date_confidence"] = "filesystem_timestamp"
                        except Exception:
                            pass
                    if progress_callback and idx % 5 == 0:
                        progress_callback(min(idx + 1, total_ws_files), total_ws_files, "files", d.metadata.get("file_name", ""))
                all_documents.extend(docs)
            except Exception:
                # Fallback: file-by-file loading if a batch contains a corrupted or locked file
                for f_idx, single_file in enumerate(ws_file_paths):
                    if progress_callback:
                        progress_callback(f_idx + 1, total_ws_files, "files", os.path.basename(single_file))
                    try:
                        single_reader = SimpleDirectoryReader(input_files=[single_file])
                        s_docs = single_reader.load_data()
                        for d in s_docs:
                            d.metadata["workspace"] = ws.name
                            if "file_path" in d.metadata:
                                fp = d.metadata["file_path"]
                                d.id_ = fp
                                try:
                                    mtime = os.path.getmtime(fp)
                                    ctime = os.path.getctime(fp)
                                    d.metadata["last_modified_date"] = time.strftime("%Y-%m-%d", time.localtime(mtime))
                                    d.metadata["creation_date"] = time.strftime("%Y-%m-%d", time.localtime(ctime))
                                    d.metadata["content_type"] = "Local Document"
                                    d.metadata["date_confidence"] = "filesystem_timestamp"
                                    d.metadata["source"] = fp
                                except Exception:
                                    pass
                        all_documents.extend(s_docs)
                    except Exception:
                        pass
            if progress_callback:
                progress_callback(total_ws_files, total_ws_files, "files", "")

    if verbose:
        safe_print(f"  • Discovery: {total_discovered_files} files scanned across configured paths")
        for sample in scanned_file_samples[:3]:
            safe_print(f"    - {os.path.basename(sample)}")
        if total_discovered_files > 3:
            safe_print(f"    - ... (+ {total_discovered_files - 3} more files)")
        safe_print(f"  • Chunks: {len(all_documents)} document nodes parsed")
        embed_label = current_settings.models.embedding_model if (current_settings and current_settings.models) else "text-embedding-3-small"
        safe_print(f"  • Embeddings: {embed_label} (incremental check)")

    if not all_documents:
        for ws in workspaces_to_process:
            try:
                lance_store.delete_local_documents_by_workspace(ws.name)
            except Exception:
                pass
        if verbose:
            safe_print("❌ No valid documents found across any workspace.\n")
        return {"status": "empty", "total_files": 0}

    # Vectorize and Index via ParallelIndexer into LanceDBStore
    indexer = ParallelIndexer(store=lance_store)
    cfg = IngestionConfig(chunk_size=chunk_size, chunk_overlap=chunk_overlap, max_workers=6)
    idx_res = indexer.index_documents(
        documents=all_documents,
        workspace_name=target_ws_name,
        config=cfg,
        progress_callback=progress_callback
    )

    # Batch upsert file stat records into SQLite cache
    cache_records = []
    for fp in all_processed_files_for_cache:
        if os.path.exists(fp):
            try:
                st = os.stat(fp)
                cache_records.append({
                    "file_path": fp,
                    "last_mtime": st.st_mtime,
                    "file_size": st.st_size,
                    "doc_id": fp,
                    "content_hash": None
                })
            except Exception:
                pass

    for ws in workspaces_to_process:
        ws_recs = [r for r in cache_records if any(r["file_path"].startswith(os.path.abspath(p)) for p in ws.paths if os.path.exists(p))]
        if ws_recs:
            store.upsert_workspace_files_cache(ws.name, ws_recs)

    if verbose:
        safe_print(f"  • Vectorized: {idx_res.get('indexed_chunks', len(all_documents))} chunks indexed in LanceDB")
        safe_print("✔ Ingestion completed successfully!\n")

    return {
        "status": "completed",
        "total_files": total_discovered_files,
        "chunks_count": idx_res.get("indexed_chunks", len(all_documents))
    }


@tool()
def index_folder(workspace_name: str = None, verbose: bool = False):
    """
    Index documents in the vector database incrementally across all configured workspaces,
    or a specific workspace if provided. Performs deep recursive scanning across all subdirectories.
    """
    return run_index_folder(workspace_name=workspace_name, verbose=verbose)


if __name__ == "__main__":
    run_index_folder(verbose=True)
