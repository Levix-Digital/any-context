"""
Encapsulated LanceDB Storage Driver (Fase 2 & Unificação Completa).
Provides thread-safe Apache Arrow columnar vector storage in Rust,
eliminating SQLite write locks, docstores, and delivering zero-copy sub-millisecond retrieval.
"""
import os
import threading
from typing import List, Dict, Any, Optional
import pyarrow as pa
import lancedb

from any_context.config.app_settings import AppSettings
from any_context.vector_engine.models import ScoredChunk


class LanceDBStore:
    """
    Unified encapsulated driver for LanceDB columnar storage.
    Guarantees that all vector I/O is isolated, thread-safe, and zero-copy.
    Supports both 'workspace_chunks' (document/web context) and 'session_memory'.
    """
    _instances: Dict[str, "LanceDBStore"] = {}
    _lock = threading.Lock()

    def __init__(self, db_path: Optional[str] = None):
        if db_path:
            self._db_path = os.path.abspath(db_path)
        else:
            settings = AppSettings.load()
            base_dir = settings.context.db_path if (settings and settings.context) else os.path.expanduser("~/.anycontext/lancedb")
            self._db_path = os.path.abspath(os.path.join(base_dir, "lancedb"))

        os.makedirs(self._db_path, exist_ok=True)
        self._db = lancedb.connect(self._db_path)
        self._table_lock = threading.Lock()
        self._default_table_name = "workspace_chunks"

    @classmethod
    def get_instance(cls, db_path: Optional[str] = None) -> "LanceDBStore":
        """Singleton instance provider per database path."""
        if not db_path:
            settings = AppSettings.load()
            base_dir = settings.context.db_path if (settings and settings.context) else os.path.expanduser("~/.anycontext/lancedb")
            target_path = os.path.abspath(os.path.join(base_dir, "lancedb"))
        else:
            target_path = os.path.abspath(db_path)

        with cls._lock:
            if target_path not in cls._instances:
                cls._instances[target_path] = cls(db_path=target_path)
            return cls._instances[target_path]

    def delete_all_records(self, table_name: str = "workspace_chunks"):
        """Drops and purges the entire table."""
        with self._table_lock:
            try:
                if self._has_table(table_name):
                    self._db.drop_table(table_name)
            except Exception:
                pass

    def _get_schema(self, dim: int = 1536) -> pa.Schema:
        """Returns standard PyArrow schema for vector records with dynamic dimensions."""
        return pa.schema([
            pa.field("id", pa.string()),
            pa.field("vector", pa.list_(pa.float32(), dim)),
            pa.field("text", pa.string()),
            pa.field("file_name", pa.string()),
            pa.field("file_path", pa.string()),
            pa.field("workspace", pa.string()),
            pa.field("last_modified", pa.string()),
            pa.field("content_type", pa.string()),
            pa.field("document_summary", pa.string()),
            pa.field("keywords", pa.string()),
            pa.field("content_hash", pa.string()),
        ])

    def _has_table(self, name: str) -> bool:
        """Helper to check if table exists across LanceDB versions."""
        try:
            res = self._db.list_tables()
            t_list = getattr(res, "tables", None)
            if t_list is not None and isinstance(t_list, (list, tuple, set)):
                return name in t_list
            if isinstance(res, (list, tuple, set)):
                return name in res
            self._db.open_table(name)
            return True
        except Exception:
            try:
                self._db.open_table(name)
                return True
            except Exception:
                return False

    def get_table(self, table_name: str = "workspace_chunks", dim: int = 1536):
        """Retrieves or creates the requested table."""
        with self._table_lock:
            if self._has_table(table_name):
                return self._db.open_table(table_name)
            schema = self._get_schema(dim=dim)
            return self._db.create_table(table_name, schema=schema, mode="create")

    @staticmethod
    def _norm_path(p: Optional[str]) -> str:
        """Normalizes filesystem path to forward slashes for cross-platform LanceDB SQL compatibility."""
        return p.replace("\\", "/") if p else ""

    def upsert_records(self, records: List[Dict[str, Any]], table_name: str = "workspace_chunks", dim: int = 1536):
        """
        Inserts or updates vector records into LanceDB using high-speed columnar Arrow batches.
        Payload text, summaries, and paths are encrypted on disk with hardware-bound AES-GCM-256.
        """
        if not records:
            return

        from any_context.core.security_engine import SecurityEngine
        sec = SecurityEngine.get_instance()

        prepared_records = []
        for r in records:
            item = dict(r)
            if "file_path" in item:
                item["file_path"] = self._norm_path(item["file_path"])
            prepared_records.append(sec.encrypt_record(item))

        with self._table_lock:
            if not self._has_table(table_name):
                schema = self._get_schema(dim=dim)
                table = self._db.create_table(table_name, schema=schema, mode="create")
            else:
                table = self._db.open_table(table_name)

            table.add(prepared_records)

    def search_vector(
        self,
        query_vector: List[float],
        limit: int = 50,
        workspace: Optional[str] = None,
        workspaces: Optional[List[str]] = None,
        filter_expr: Optional[str] = None,
        table_name: str = "workspace_chunks"
    ) -> List[ScoredChunk]:
        """
        Executes Rust-powered vector similarity search with optional workspace filtering.
        Decrypts top-K matched chunk payloads on-the-fly and converts into ScoredChunk contracts.
        """
        if not self._has_table(table_name):
            return []

        try:
            table = self._db.open_table(table_name)
            query = table.search(query_vector).limit(limit)

            # Build where clause for workspace isolation
            where_clauses = []
            if workspace and not workspaces:
                clean_ws = workspace.replace("'", "''")
                where_clauses.append(f"workspace = '{clean_ws}'")
            elif workspaces:
                clean_workspaces = [w.replace("'", "''") for w in workspaces]
                ws_in = ", ".join("'" + w + "'" for w in clean_workspaces)
                where_clauses.append(f"workspace IN ({ws_in})")

            if filter_expr:
                where_clauses.append(filter_expr)

            if where_clauses:
                query = query.where(" AND ".join(where_clauses))

            raw_results = query.to_list()
            scored_chunks: List[ScoredChunk] = []

            from any_context.core.security_engine import SecurityEngine
            sec = SecurityEngine.get_instance()

            for r in raw_results:
                r_dec = sec.decrypt_record(r)
                dist = float(r.get("_distance", 0.0))
                # Convert Euclidean / Cosine L2 distance to normalized similarity score [0.0, 1.0]
                similarity = 1.0 / (1.0 + max(0.0, dist))

                chunk = ScoredChunk(
                    text=r_dec.get("text", ""),
                    file_name=r_dec.get("file_name", "Unknown"),
                    file_path=r_dec.get("file_path", ""),
                    workspace=r_dec.get("workspace", "Default"),
                    score=similarity,
                    last_modified=r_dec.get("last_modified"),
                    content_type=r_dec.get("content_type", "Local Document"),
                    document_summary=r_dec.get("document_summary"),
                    keywords=r_dec.get("keywords"),
                    chunk_id=r_dec.get("id"),
                    metadata={
                        "file_name": r_dec.get("file_name", "Unknown"),
                        "file_path": r_dec.get("file_path", ""),
                        "workspace": r_dec.get("workspace", "Default"),
                        "last_modified": r_dec.get("last_modified"),
                        "content_type": r_dec.get("content_type", "Local Document"),
                        "document_summary": r_dec.get("document_summary"),
                        "keywords": r_dec.get("keywords"),
                        "content_hash": r_dec.get("content_hash", "")
                    }
                )
                scored_chunks.append(chunk)

            return scored_chunks

        except Exception:
            return []

    def delete_by_workspace(self, workspace_name: str, table_name: str = "workspace_chunks"):
        """Purges all chunks associated with a specific workspace."""
        if not self._has_table(table_name):
            return
        with self._table_lock:
            try:
                table = self._db.open_table(table_name)
                clean_ws = workspace_name.replace("'", "''")
                table.delete(f"workspace = '{clean_ws}'")
            except Exception:
                pass

    def delete_local_documents_by_workspace(self, workspace_name: str, table_name: str = "workspace_chunks"):
        """Purges only local document chunks associated with a specific workspace, preserving web sources."""
        if not self._has_table(table_name):
            return
        with self._table_lock:
            try:
                table = self._db.open_table(table_name)
                clean_ws = workspace_name.replace("'", "''")
                table.delete(f"workspace = '{clean_ws}' AND content_type = 'Local Document'")
            except Exception:
                pass

    def delete_by_id(self, chunk_id: str, table_name: str = "workspace_chunks"):
        """Purges a single chunk by ID."""
        if not self._has_table(table_name):
            return
        with self._table_lock:
            try:
                table = self._db.open_table(table_name)
                clean_id = chunk_id.replace("'", "''")
                table.delete(f"id = '{clean_id}'")
            except Exception:
                pass

    def delete_by_file(self, file_path: str, workspace_name: Optional[str] = None, table_name: str = "workspace_chunks"):
        """Purges all chunks for a specific file path or URL with proper path normalization and prefix support."""
        if not self._has_table(table_name):
            return
        with self._table_lock:
            try:
                table = self._db.open_table(table_name)
                clean_fp = self._norm_path(file_path).replace("'", "''")
                where_clause = f"(file_path = '{clean_fp}' OR file_path LIKE '{clean_fp}/%' OR file_path LIKE '{clean_fp}%')"
                if workspace_name:
                    clean_ws = workspace_name.replace("'", "''")
                    where_clause = f"workspace = '{clean_ws}' AND ({where_clause})"
                table.delete(where_clause)
            except Exception:
                pass

    def update_workspace_name(self, old_workspace: str, new_workspace: str, table_name: str = "workspace_chunks") -> int:
        """
        Renames workspace on all matching vector records in LanceDB ($0.00 cost).
        Reads matching records, updates workspace metadata, and re-adds them.
        """
        if not self._has_table(table_name):
            return 0
        with self._table_lock:
            try:
                table = self._db.open_table(table_name)
                clean_old = old_workspace.replace("'", "''")
                matching = table.search().where(f"workspace = '{clean_old}'").limit(100000).to_list()
                if not matching:
                    return 0
                table.delete(f"workspace = '{clean_old}'")
                for r in matching:
                    r["workspace"] = new_workspace
                    if "_distance" in r:
                        del r["_distance"]
                dim = len(matching[0]["vector"]) if matching and "vector" in matching[0] else 1536
                table.add(matching)
                return len(matching)
            except Exception:
                return 0

    def transfer_file(self, source_ws: str, target_ws: str, file_path: str, table_name: str = "workspace_chunks") -> int:
        """
        Transfers vector records for a specific file, folder, or URL from source_ws to target_ws ($0.00 cost).
        """
        if not self._has_table(table_name):
            return 0
        with self._table_lock:
            try:
                table = self._db.open_table(table_name)
                clean_src = source_ws.replace("'", "''")
                clean_fp = self._norm_path(file_path).replace("'", "''")
                where_clause = f"workspace = '{clean_src}' AND (file_path = '{clean_fp}' OR file_path LIKE '{clean_fp}/%' OR file_path LIKE '%{clean_fp}%')"
                matching = table.search().where(where_clause).limit(50000).to_list()
                if not matching:
                    return 0
                table.delete(where_clause)
                for r in matching:
                    r["workspace"] = target_ws
                    if "_distance" in r:
                        del r["_distance"]
                table.add(matching)
                return len(matching)
            except Exception:
                return 0

    def count_records(self, workspace_name: Optional[str] = None, table_name: str = "workspace_chunks") -> int:
        """Returns total record count in table or scoped to workspace using zero-copy projection."""
        if not self._has_table(table_name):
            return 0
        try:
            table = self._db.open_table(table_name)
            if workspace_name:
                clean_ws = workspace_name.replace("'", "''")
                return len(table.search().where(f"workspace = '{clean_ws}'").select(["id"]).limit(100000).to_arrow())
            return table.count_rows()
        except Exception:
            return 0

    def get_indexed_pages_map(
        self,
        workspace_name: str,
        domain_or_prefix: Optional[str] = None,
        table_name: str = "workspace_chunks"
    ) -> Dict[str, Dict[str, Any]]:
        """
        Returns a dictionary mapping url -> {url, title, content_hash, last_modified}
        for all web documentation pages indexed in this workspace.
        Uses high-speed zero-copy columnar projection on LanceDB without loading vectors or full texts.
        """
        if not self._has_table(table_name):
            return {}
        try:
            import urllib.parse
            table = self._db.open_table(table_name)
            clean_ws = workspace_name.replace("'", "''")
            where_clauses = [
                f"workspace = '{clean_ws}'",
                "(file_path LIKE 'http://%' OR file_path LIKE 'https://%')"
            ]
            if domain_or_prefix:
                raw_dom = domain_or_prefix.strip()
                if raw_dom.startswith("http://") or raw_dom.startswith("https://"):
                    parsed = urllib.parse.urlparse(raw_dom)
                    host = parsed.netloc.lower()
                    path = parsed.path
                    if path and ("." in os.path.basename(path)):
                        path = os.path.dirname(path)
                    path = path.rstrip("/")
                    clean_dom = f"{host}{path}" if path else host
                else:
                    clean_dom = raw_dom
                clean_dom = clean_dom.replace("'", "''")
                where_clauses.append(f"file_path LIKE '%{clean_dom}%'")

            where_str = " AND ".join(where_clauses)
            arrow_tbl = table.search().where(where_str).select(["file_path", "file_name", "content_hash", "last_modified"]).limit(100000).to_arrow()
            if arrow_tbl.num_rows == 0:
                return {}

            url_col = arrow_tbl.column("file_path").to_pylist()
            name_col = arrow_tbl.column("file_name").to_pylist()
            hash_col = arrow_tbl.column("content_hash").to_pylist() if "content_hash" in arrow_tbl.column_names else [""] * len(url_col)
            lm_col = arrow_tbl.column("last_modified").to_pylist() if "last_modified" in arrow_tbl.column_names else [""] * len(url_col)

            pages_map: Dict[str, Dict[str, Any]] = {}
            for u, n, h, lm in zip(url_col, name_col, hash_col, lm_col):
                if u and u not in pages_map:
                    clean_title = n[6:].strip() if (n and n.startswith("[Web] ")) else (n or "")
                    pages_map[u] = {
                        "url": u,
                        "title": clean_title,
                        "content_hash": h or "",
                        "char_count": 0,
                        "scraped_at": lm or "",
                        "last_modified": lm or "",
                        "root_url": u
                    }
            return pages_map
        except Exception:
            return {}

    def get_indexed_pages_count(
        self,
        workspace_name: str,
        domain_or_prefix: Optional[str] = None,
        table_name: str = "workspace_chunks"
    ) -> int:
        """
        Returns total count of distinct web pages indexed for this workspace.
        """
        return len(self.get_indexed_pages_map(workspace_name, domain_or_prefix=domain_or_prefix, table_name=table_name))

    def get_indexed_folder_files_count(
        self,
        workspace_name: str,
        folder_path: Optional[str] = None,
        table_name: str = "workspace_chunks"
    ) -> int:
        """
        Returns the count of distinct non-web local files indexed in this workspace.
        Optionally filtered to files located within folder_path.
        """
        if not self._has_table(table_name):
            return 0
        try:
            table = self._db.open_table(table_name)
            clean_ws = workspace_name.replace("'", "''")
            where_clauses = [
                f"workspace = '{clean_ws}'",
                "NOT (file_path LIKE 'http://%' OR file_path LIKE 'https://%')"
            ]
            if folder_path:
                norm_p = os.path.abspath(folder_path.strip().strip("'\""))
                norm_p_fwd = norm_p.replace("\\", "/").replace("'", "''")
                norm_p_bck = norm_p.replace("'", "''")
                where_clauses.append(f"(file_path LIKE '{norm_p_fwd}%' OR file_path LIKE '{norm_p_bck}%')")

            where_str = " AND ".join(where_clauses)
            arrow_tbl = table.search().where(where_str).select(["file_path"]).limit(100000).to_arrow()
            if arrow_tbl.num_rows == 0:
                return 0
            distinct_files = set(arrow_tbl.column("file_path").to_pylist())
            return len(distinct_files)
        except Exception:
            return 0

    def get_workspace_inventory_summary(
        self,
        workspace_name: str,
        table_name: str = "workspace_chunks"
    ) -> Dict[str, Any]:
        """
        Retrieves the full telemetry inventory for a workspace directly from LanceDB:
        total chunks, distinct local files count, distinct web pages count.
        """
        if not self._has_table(table_name):
            return {"total_chunks": 0, "total_files": 0, "total_web_pages": 0}
        try:
            table = self._db.open_table(table_name)
            clean_ws = workspace_name.replace("'", "''")
            where_str = f"workspace = '{clean_ws}'"
            arrow_tbl = table.search().where(where_str).select(["file_path"]).limit(200000).to_arrow()
            if arrow_tbl.num_rows == 0:
                return {"total_chunks": 0, "total_files": 0, "total_web_pages": 0}
            file_paths = arrow_tbl.column("file_path").to_pylist()
            total_chunks = len(file_paths)
            web_urls = set(p for p in file_paths if p and (p.startswith("http://") or p.startswith("https://")))
            local_files = set(p for p in file_paths if p and not (p.startswith("http://") or p.startswith("https://")))
            return {
                "total_chunks": total_chunks,
                "total_files": len(local_files),
                "total_web_pages": len(web_urls)
            }
        except Exception:
            return {"total_chunks": 0, "total_files": 0, "total_web_pages": 0}
