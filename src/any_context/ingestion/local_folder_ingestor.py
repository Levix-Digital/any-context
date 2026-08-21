import os
import sys
import time
import threading
import chromadb
from typing import List, Dict, Any, Optional, Tuple, Callable
from any_context.config.app_settings import AppSettings
from any_context.core.utils import get_api_key
from llama_index.core import Settings, SimpleDirectoryReader, Document
from llama_index.core.ingestion import IngestionPipeline, DocstoreStrategy
from llama_index.core.storage.docstore import SimpleDocumentStore
from llama_index.core.node_parser import SentenceSplitter
from llama_index.vector_stores.chroma import ChromaVectorStore
from llama_index.embeddings.openai import OpenAIEmbedding

from any_context.help.registry import HELP_REGISTRY
from langchain.tools import tool

def safe_print(msg: str):
    try:
        print(msg)
    except UnicodeEncodeError:
        print(msg.encode("ascii", errors="ignore").decode("ascii"))

LOCAL_API_KEY = get_api_key()

settings = AppSettings.load()

db_save_path = settings.context.db_path if settings else "./context_db"
collection_name = settings.context.collection_name if settings else "context_docs"
embedding_model = settings.models.embedding_model if settings else "text-embedding-3-small"
local_base_url = settings.models.local_base_url if settings else "http://localhost:1234/v1"

from any_context.tools.search_tools import configure_embedding_model


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

def build_help_registry_document() -> Document:
    """Constructs a comprehensive synthetic Markdown Document from the Help Module Registry."""
    text_blocks = ["# 📖 AnyContext Complete Commands & Architectural Manual\n"]
    for key, page in HELP_REGISTRY.items():
        text_blocks.append(f"## Command: {page.command} ({page.title})\n")
        text_blocks.append(f"**Aliases**: {', '.join(page.aliases)}\n")
        text_blocks.append(f"**Description**: {page.description}\n")
        text_blocks.append(f"**Syntax**: {page.syntax}\n")
        if page.parameters:
            text_blocks.append("**Parameters & Options**:\n" + "\n".join([f"- {p}" for p in page.parameters]) + "\n")
        if page.examples:
            text_blocks.append("**Usage Examples**:\n" + "\n".join([f"- {e}" for e in page.examples]) + "\n")
        if page.tips:
            text_blocks.append("**Best Practice Tips**:\n" + "\n".join([f"- {t}" for t in page.tips]) + "\n")
        text_blocks.append("\n---\n")

    full_text = "\n".join(text_blocks)
    return Document(text=full_text, metadata={"file_name": "AnyContext Command Manual & Help Registry (HELP_REGISTRY)"})

def clear_context_vector_db(verbose: bool = False):
    """
    Purges ChromaDB vector collection, docstore file, and file stat cache to prevent dimension mismatch errors
    when embedding models are changed.
    """
    try:
        current_settings = AppSettings.load() or settings
        db_path = current_settings.context.db_path if current_settings else "./context_db"
        coll_name = current_settings.context.collection_name if current_settings else "context_docs"

        db = chromadb.PersistentClient(path=db_path)
        try:
            db.delete_collection(coll_name)
        except Exception:
            pass

        docstore_path = os.path.join(db_path, "docstore.json")
        if os.path.exists(docstore_path):
            os.remove(docstore_path)

        from any_context.config.db_store import ConfigDBStore
        store = ConfigDBStore()
        store.clear_workspace_files_cache()

        if verbose:
            safe_print("│ ├─ 🧹 Context vector collection, docstore, and stat cache cleared for re-indexing")
    except Exception as e:
        if verbose:
            safe_print(f"│ ├─ ⚠️ Warning during vector db clear: {e}")

def check_workspace_changes(workspace_name: str) -> Dict[str, Any]:
    """
    Performs ultra-fast (<30ms) stat scan over workspace folders without reading file contents.
    Compares (file_path, mtime, size) against workspace_files_stat_cache in SQLite.
    """
    from any_context.config.db_store import ConfigDBStore
    store = ConfigDBStore()
    clean_ws = (workspace_name or "Default").strip()

    current_settings = AppSettings.load()
    if not current_settings or not current_settings.workspaces:
        return {
            "workspace_name": clean_ws,
            "is_up_to_date": True,
            "has_changes": False,
            "is_virgin": False,
            "new_files": [],
            "modified_files": [],
            "deleted_files": [],
            "renamed_files": [],
            "total_disk_files": 0,
            "total_cached_files": 0,
            "summary": "No workspaces configured."
        }

    ws_obj = None
    for ws in current_settings.workspaces:
        if ws.name.lower() == clean_ws.lower() or (getattr(ws, "workspace_id", None) and ws.workspace_id == clean_ws):
            ws_obj = ws
            break

    if not ws_obj or not ws_obj.paths:
        return {
            "workspace_name": clean_ws,
            "is_up_to_date": True,
            "has_changes": False,
            "is_virgin": False,
            "new_files": [],
            "modified_files": [],
            "deleted_files": [],
            "renamed_files": [],
            "total_disk_files": 0,
            "total_cached_files": 0,
            "summary": "No folder paths configured."
        }

    disk_files: Dict[str, Dict[str, Any]] = {}
    for folder_path in ws_obj.paths:
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

    summary = ", ".join(parts) if parts else "Up to date (0 changes)"

    return {
        "workspace_name": clean_ws,
        "is_up_to_date": is_up_to_date,
        "has_changes": has_changes,
        "is_virgin": is_virgin,
        "new_files": new_files,
        "modified_files": modified_files,
        "deleted_files": deleted_files,
        "renamed_files": renamed_files,
        "total_disk_files": len(disk_files),
        "total_cached_files": len(cached_files),
        "summary": summary
    }


class BackgroundSyncManager:
    """Thread-safe manager for background workspace synchronization workers."""
    _instance = None
    _lock = threading.Lock()

    def __new__(cls):
        with cls._lock:
            if cls._instance is None:
                cls._instance = super(BackgroundSyncManager, cls).__new__(cls)
                cls._instance._active_jobs = {}
            return cls._instance

    def is_syncing(self, workspace_name: str) -> bool:
        clean_ws = (workspace_name or "Default").strip()
        with self._lock:
            job = self._active_jobs.get(clean_ws)
            if job and job["thread"].is_alive():
                return True
            return False

    def get_sync_status(self, workspace_name: str) -> Dict[str, Any]:
        clean_ws = (workspace_name or "Default").strip()
        with self._lock:
            job = self._active_jobs.get(clean_ws)
            if not job:
                return {"status": "idle", "workspace_name": clean_ws}
            is_alive = job["thread"].is_alive()
            return {
                "workspace_name": clean_ws,
                "status": "syncing" if is_alive else job.get("status", "completed"),
                "start_time": job.get("start_time"),
                "result": job.get("result"),
                "error": job.get("error")
            }

    def start_background_sync(
        self,
        workspace_name: str,
        on_complete: Optional[Callable[[Dict[str, Any]], None]] = None,
        verbose: bool = False
    ) -> threading.Thread:
        clean_ws = (workspace_name or "Default").strip()
        with self._lock:
            if clean_ws in self._active_jobs and self._active_jobs[clean_ws]["thread"].is_alive():
                return self._active_jobs[clean_ws]["thread"]

        def _worker():
            try:
                res = run_index_folder(workspace_name=clean_ws, verbose=verbose)
                with self._lock:
                    self._active_jobs[clean_ws]["status"] = "completed"
                    self._active_jobs[clean_ws]["result"] = res
                if on_complete:
                    try:
                        on_complete(res)
                    except Exception:
                        pass
            except Exception as e:
                with self._lock:
                    self._active_jobs[clean_ws]["status"] = "failed"
                    self._active_jobs[clean_ws]["error"] = str(e)

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


def run_index_folder(workspace_name: str = None, verbose: bool = False, force_full: bool = False):
    """
    Index documents in the vector database incrementally across all configured workspaces,
    or a specific workspace if provided. Performs deep recursive scanning across all subdirectories.
    Automatically embeds application README and Help Module Registry as permanent system self-help context.
    Uses SQLite stat cache (mtime & size) for sub-30ms bypass when unchanged and zero-cost file path migration.
    """
    current_settings = AppSettings.load()
    if not current_settings or not current_settings.workspaces:
        if verbose:
            safe_print("❌ Error: No workspaces configured in settings.")
        return {"status": "error", "error": "No workspaces configured."}

    from any_context.config.db_store import ConfigDBStore
    store = ConfigDBStore()

    target_ws_name = workspace_name or (current_settings.workspaces[0].name if current_settings.workspaces else "Global")
    db_save_path = current_settings.context.db_path if (current_settings and current_settings.context) else "./context_db"
    collection_name = current_settings.context.collection_name if (current_settings and current_settings.context) else "context_docs"

    if verbose:
        safe_print(f"\n┌ 📦 \033[1mIngestion Pipeline: {target_ws_name}\033[0m")
        safe_print(f"│ ├─ 📂 Storage     : Vector Store ({db_save_path}/{collection_name})")

    db = chromadb.PersistentClient(path=db_save_path)
    collection = db.get_or_create_collection(collection_name)
    vector_store = ChromaVectorStore(chroma_collection=collection)

    docstore_path = os.path.join(db_save_path, "docstore.json")
    if os.path.exists(docstore_path):
        docstore = SimpleDocumentStore.from_persist_path(docstore_path)
    else:
        docstore = SimpleDocumentStore()

    # If targeting a specific workspace and not forcing full reindex, run fast stat diff check
    if workspace_name and not force_full:
        diff = check_workspace_changes(target_ws_name)
        if diff["is_up_to_date"]:
            if verbose:
                safe_print(f"│ ├─ ⚡ Stat Check  : 100% up to date ({diff['total_disk_files']} files, 0 changes)")
                safe_print("└ \033[92m✔ Ingestion completed successfully (0 changes)!\033[0m\n")
            return {"status": "up_to_date", "total_files": diff["total_disk_files"], "changes": diff}

        # Zero-cost Renamed/Moved Files Metadata Repointing ($0.00)
        if diff["renamed_files"]:
            try:
                results = collection.get(where={"workspace": target_ws_name}, include=["metadatas"])
                rename_map = {old_p: new_p for old_p, new_p in diff["renamed_files"]}
                if results and results.get("ids"):
                    ids_to_update = []
                    metas_to_update = []
                    for cid, meta in zip(results["ids"], results["metadatas"]):
                        fp = (meta.get("file_path", "") or meta.get("source", "")).strip().strip("'\"")
                        if fp in rename_map:
                            ids_to_update.append(cid)
                            new_meta = dict(meta)
                            new_meta["file_path"] = rename_map[fp]
                            new_meta["source"] = rename_map[fp]
                            metas_to_update.append(new_meta)
                    if ids_to_update:
                        collection.update(ids=ids_to_update, metadatas=metas_to_update)

                # Also update docstore nodes
                for node_id, node in list(docstore.docs.items()):
                    if node.metadata and node.metadata.get("file_path") in rename_map:
                        new_p = rename_map[node.metadata["file_path"]]
                        node.metadata["file_path"] = new_p
                        node.metadata["source"] = new_p

                for old_p, new_p in diff["renamed_files"]:
                    store.rename_cached_file_path(target_ws_name, old_p, new_p)
                if verbose:
                    safe_print(f"│ ├─ 🔄 Renamed     : {len(diff['renamed_files'])} files repointed with zero-cost ($0.00)")
            except Exception:
                pass

        # Purge Deleted Files from ChromaDB, docstore & SQLite cache
        if diff["deleted_files"]:
            deleted_set = set(diff["deleted_files"])
            del_chunk_count = 0
            for node_id, node in list(docstore.docs.items()):
                node_ws = node.metadata.get("workspace") if node.metadata else None
                node_fp = node.metadata.get("file_path") if node.metadata else None
                if node_ws == target_ws_name and node_fp in deleted_set:
                    try:
                        docstore.delete_document(node_id)
                        vector_store.delete(node_id)
                        del_chunk_count += 1
                    except Exception:
                        pass
            store.remove_workspace_files_cache(target_ws_name, diff["deleted_files"])
            if verbose:
                safe_print(f"│ ├─ 🗑️ Deleted     : {len(diff['deleted_files'])} files ({del_chunk_count} chunks purged)")

        # If only deletions and renames occurred, persist and return early
        if not diff["new_files"] and not diff["modified_files"]:
            docstore.persist(persist_path=docstore_path)
            if verbose:
                safe_print("└ \033[92m✔ Ingestion completed successfully!\033[0m\n")
            return {"status": "updated", "changes": diff}

    configure_embedding_model()

    chunk_size = current_settings.context.chunk_size if (current_settings and current_settings.context) else 1024
    chunk_overlap = current_settings.context.chunk_overlap if (current_settings and current_settings.context) else 200

    pipeline = IngestionPipeline(
        transformations = [
            SentenceSplitter(chunk_size=chunk_size, chunk_overlap=chunk_overlap),
            Settings.embed_model
        ],
        vector_store = vector_store,
        docstore = docstore,
        docstore_strategy = DocstoreStrategy.UPSERTS
    )

    all_documents = []
    total_discovered_files = 0
    scanned_file_samples = []

    # Locate application README.md for permanent system help context
    readme_path = None
    readme_candidates = [
        os.path.join(os.getcwd(), "README.md"),
        os.path.join(os.path.dirname(__file__), "..", "..", "..", "README.md"),
        os.path.join(os.path.dirname(__file__), "..", "config", "README.md")
    ]
    for cand in readme_candidates:
        if os.path.exists(cand):
            readme_path = os.path.abspath(cand)
            break

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
            try:
                reader = SimpleDirectoryReader(input_files=ws_file_paths)
                docs = reader.load_data()
                for d in docs:
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
                all_documents.extend(docs)
            except Exception:
                # Fallback: file-by-file loading if a batch contains a corrupted or locked file
                for single_file in ws_file_paths:
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

        # Auto-inject application README.md as permanent system context for Default/Global workspace
        if ws.name in ["Default", "Global"] and readme_path:
            try:
                readme_reader = SimpleDirectoryReader(input_files=[readme_path])
                readme_docs = readme_reader.load_data()
                for rd in readme_docs:
                    rd.metadata["workspace"] = ws.name
                    rd.metadata["is_system_help"] = True
                    rd.metadata["file_name"] = "AnyContext System Documentation (README.md)"
                    rd.id_ = f"system_readme_{ws.name}"
                all_documents.extend(readme_docs)
            except Exception:
                pass

        # Auto-inject Help Module Registry as permanent system self-help context for Default/Global workspace
        if ws.name in ["Default", "Global"]:
            try:
                help_doc = build_help_registry_document()
                help_doc.metadata["workspace"] = ws.name
                help_doc.metadata["is_system_help"] = True
                help_doc.id_ = f"system_help_registry_{ws.name}"
                all_documents.append(help_doc)
            except Exception:
                pass

    if verbose:
        safe_print(f"│ ├─ 🔍 Discovery   : {total_discovered_files} files scanned across configured paths")
        for sample in scanned_file_samples[:3]:
            safe_print(f"│ │    • 📄 {os.path.basename(sample)}")
        if total_discovered_files > 3:
            safe_print(f"│ │    • ... (+ {total_discovered_files - 3} more files)")
        safe_print(f"│ ├─ 📚 Chunks      : {len(all_documents)} document nodes parsed")
        safe_print(f"│ ├─ 📖 System Help : Auto-injected README.md & Command Manual (HELP_REGISTRY)")
        curr_settings = AppSettings.load() or settings
        embed_label = curr_settings.models.embedding_model if curr_settings and curr_settings.models else "text-embedding-3-small"
        safe_print(f"│ ├─ ⚡ Embeddings  : {embed_label} (incremental check)")

    if not all_documents:
        if verbose:
            safe_print("└ ❌ No valid documents found across any workspace.\n")
        return {"status": "empty", "total_files": 0}
        
    try:
        nodes = pipeline.run(documents=all_documents, show_progress=False)
    except Exception as e:
        err_str = str(e).lower()
        if "dimension" in err_str or "invalidargumenterror" in err_str or "expecting embedding" in err_str:
            if verbose:
                safe_print("│ ├─ 🧹 Auto-clearing incompatible vector database for fresh re-indexing...")
            clear_context_vector_db(verbose=verbose)

            db = chromadb.PersistentClient(path=db_save_path)
            collection = db.get_or_create_collection(collection_name)
            vector_store = ChromaVectorStore(chroma_collection=collection)
            docstore = SimpleDocumentStore()

            pipeline = IngestionPipeline(
                transformations = [
                    SentenceSplitter(chunk_size=chunk_size, chunk_overlap=chunk_overlap),
                    Settings.embed_model
                ],
                vector_store = vector_store,
                docstore = docstore,
                docstore_strategy = DocstoreStrategy.UPSERTS
            )
            nodes = pipeline.run(documents=all_documents, show_progress=False)
        elif "no embedding data" in err_str or "connection" in err_str:
            if verbose:
                safe_print("└ ❌ Error generating embeddings: endpoint did not return embedding data.\n")
            return {"status": "error", "error": str(e)}
        else:
            raise e

    current_doc_ids = {doc.doc_id for doc in all_documents}
    processed_workspace_names = {ws.name for ws in workspaces_to_process}
    
    deleted_count = 0
    for node_id, node in list(docstore.docs.items()):
        node_workspace = node.metadata.get("workspace") if node.metadata else None
        
        if not node_workspace or node_workspace in processed_workspace_names:
            if getattr(node, "ref_doc_id", None) not in current_doc_ids and node.id_ not in current_doc_ids:
                try:
                    docstore.delete_document(node_id)
                    vector_store.delete(node_id)
                    deleted_count += 1
                except Exception:
                    pass

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
        safe_print(f"│ └─ 🧹 Maintenance : {deleted_count} outdated chunks purged")
        safe_print(f"└ \033[92m✔ Ingestion completed successfully!\033[0m\n")

    docstore.persist(persist_path=docstore_path)
    return {"status": "completed", "total_files": total_discovered_files, "chunks_count": len(all_documents)}


@tool()
def index_folder(workspace_name: str = None, verbose: bool = False):
    """
    Index documents in the vector database incrementally across all configured workspaces,
    or a specific workspace if provided. Performs deep recursive scanning across all subdirectories.
    Automatically embeds application README and Help Module Registry as permanent system self-help context.
    """
    return run_index_folder(workspace_name=workspace_name, verbose=verbose)


if __name__ == "__main__":
    run_index_folder(verbose=True)
