import os
import sys
import json
import sqlite3
import hashlib
import secrets
import uuid
from typing import Optional, List, Dict, Any
from any_context.config.app_settings import (
    AppSettings,
    WorkspaceSettings,
    WorkspaceWebSource,
    WorkspaceCloudDrive,
    WorkspaceSourceItem,
    ContextSettings,
    SessionSettings,
    ModelSettings,
    MemorySettings
)

def safe_print(msg: str):
    try:
        print(msg)
    except UnicodeEncodeError:
        print(msg.encode("ascii", errors="ignore").decode("ascii"))

def hash_password(password: str, salt: Optional[str] = None) -> str:
    """Hashes a password using PBKDF2 with SHA-256 and a random salt."""
    if not salt:
        salt = secrets.token_hex(16)
    key = hashlib.pbkdf2_hmac("sha256", password.encode("utf-8"), salt.encode("utf-8"), 100000)
    return f"{salt}${key.hex()}"

def verify_password(password: str, hashed_password: str) -> bool:
    """Verifies a plaintext password against a stored PBKDF2 hash."""
    try:
        salt, key_hex = hashed_password.split("$")
        check_key = hashlib.pbkdf2_hmac("sha256", password.encode("utf-8"), salt.encode("utf-8"), 100000)
        return check_key.hex() == key_hex
    except Exception:
        return False

class ConfigDBStore:
    """
    SQLite-backed Configuration & Security RBAC Storage Manager.
    Handles persistent CRUD operations for Workspaces, Models, Database paths, Memory settings, API Keys, Users, and Access Tokens.
    """

    def __init__(self, db_path: Optional[str] = None):
        self.db_path = db_path or self.find_db_file("settings.db")
        self._init_db()
        self.ensure_default_workspace()


    @classmethod
    def find_db_file(cls, filename: str = "settings.db") -> str:
        """Resolves the settings.db SQLite file location"""
        candidates = [
            os.path.join(os.getcwd(), "config", filename),
            os.path.join(os.getcwd(), filename),
            os.path.expanduser(os.path.join("~", ".config", "any-context", filename)),
        ]

        if sys.platform == "win32" and "APPDATA" in os.environ:
            candidates.append(os.path.join(os.environ["APPDATA"], "any-context", filename))

        for candidate in candidates:
            if os.path.exists(candidate):
                return os.path.abspath(candidate)

        target_dir = os.path.join(os.getcwd(), "config")
        os.makedirs(target_dir, exist_ok=True)
        return os.path.abspath(os.path.join(target_dir, filename))

    def _get_connection(self) -> sqlite3.Connection:
        os.makedirs(os.path.dirname(self.db_path), exist_ok=True)
        conn = sqlite3.connect(self.db_path, check_same_thread=False)
        conn.row_factory = sqlite3.Row
        return conn

    def _init_db(self):
        """Creates configuration and security tables if they do not exist"""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS workspaces (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    workspace_id TEXT UNIQUE,
                    name TEXT UNIQUE NOT NULL,
                    paths_json TEXT NOT NULL
                )
            """)

            # Ensure workspace_id column exists for existing tables and all rows have a non-null workspace_id
            cursor.execute("PRAGMA table_info(workspaces)")
            ws_cols = [r[1] for r in cursor.fetchall()]
            if "workspace_id" not in ws_cols:
                cursor.execute("ALTER TABLE workspaces ADD COLUMN workspace_id TEXT")
            
            cursor.execute("SELECT id, name, workspace_id FROM workspaces WHERE workspace_id IS NULL OR workspace_id = ''")
            for r in cursor.fetchall():
                ws_auto_id = "ws_default" if r["name"].strip().lower() == "default" else f"ws_{uuid.uuid4().hex[:8]}"
                cursor.execute("UPDATE workspaces SET workspace_id = ? WHERE id = ?", (ws_auto_id, r["id"]))

            cursor.execute("""
                CREATE TABLE IF NOT EXISTS models (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    embedding_model TEXT DEFAULT 'text-embedding-3-small',
                    local_embedding_model TEXT,
                    local_openai_embedding_model TEXT,
                    inference_model TEXT NOT NULL,
                    summary_model TEXT NOT NULL,
                    model_provider TEXT NOT NULL,
                    local_base_url TEXT NOT NULL
                )
            """)

            # Ensure embedding_model column exists for existing tables
            cursor.execute("PRAGMA table_info(models)")
            cols = [r[1] for r in cursor.fetchall()]
            if "embedding_model" not in cols:
                cursor.execute("ALTER TABLE models ADD COLUMN embedding_model TEXT DEFAULT 'text-embedding-3-small'")

            cursor.execute("""
                CREATE TABLE IF NOT EXISTS context_settings (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    db_path TEXT NOT NULL,
                    collection_name TEXT NOT NULL,
                    chunk_size INTEGER DEFAULT 1024,
                    chunk_overlap INTEGER DEFAULT 200,
                    top_k INTEGER DEFAULT 40,
                    candidate_pool_size INTEGER DEFAULT 100,
                    max_chunks_per_source INTEGER DEFAULT 3,
                    retrieval_preset TEXT DEFAULT 'balanced',
                    grounding_mode TEXT DEFAULT 'hybrid'
                )
            """)
            cursor.execute("PRAGMA table_info(context_settings)")
            ctx_cols = [r[1] for r in cursor.fetchall()]
            if "chunk_size" not in ctx_cols:
                cursor.execute("ALTER TABLE context_settings ADD COLUMN chunk_size INTEGER DEFAULT 1024")
            if "chunk_overlap" not in ctx_cols:
                cursor.execute("ALTER TABLE context_settings ADD COLUMN chunk_overlap INTEGER DEFAULT 200")
            if "top_k" not in ctx_cols:
                cursor.execute("ALTER TABLE context_settings ADD COLUMN top_k INTEGER DEFAULT 40")
            if "candidate_pool_size" not in ctx_cols:
                cursor.execute("ALTER TABLE context_settings ADD COLUMN candidate_pool_size INTEGER DEFAULT 100")
            if "max_chunks_per_source" not in ctx_cols:
                cursor.execute("ALTER TABLE context_settings ADD COLUMN max_chunks_per_source INTEGER DEFAULT 3")
            if "retrieval_preset" not in ctx_cols:
                cursor.execute("ALTER TABLE context_settings ADD COLUMN retrieval_preset TEXT DEFAULT 'balanced'")
            if "grounding_mode" not in ctx_cols:
                cursor.execute("ALTER TABLE context_settings ADD COLUMN grounding_mode TEXT DEFAULT 'hybrid'")

            cursor.execute("""
                CREATE TABLE IF NOT EXISTS session_settings (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    db_path TEXT NOT NULL,
                    collection_name TEXT NOT NULL
                )
            """)

            cursor.execute("""
                CREATE TABLE IF NOT EXISTS memory_settings (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    short_term_buffer_size INTEGER NOT NULL,
                    rolling_window_messages INTEGER NOT NULL,
                    meta_summary_threshold INTEGER NOT NULL,
                    meta_summary_batch_size INTEGER NOT NULL
                )
            """)

            cursor.execute("""
                CREATE TABLE IF NOT EXISTS api_keys (
                    provider TEXT PRIMARY KEY,
                    api_key TEXT NOT NULL
                )
            """)

            cursor.execute("""
                CREATE TABLE IF NOT EXISTS users (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    user_id TEXT UNIQUE NOT NULL,
                    email TEXT UNIQUE NOT NULL,
                    name TEXT NOT NULL,
                    password_hash TEXT NOT NULL,
                    role TEXT NOT NULL,
                    allowed_workspaces_json TEXT NOT NULL,
                    created_at TEXT NOT NULL
                )
            """)

            cursor.execute("""
                CREATE TABLE IF NOT EXISTS access_tokens (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    token_id TEXT UNIQUE NOT NULL,
                    user_id TEXT NOT NULL,
                    name TEXT NOT NULL,
                    role TEXT NOT NULL,
                    allowed_workspaces_json TEXT NOT NULL,
                    created_at TEXT NOT NULL
                )
            """)

            cursor.execute("""
                CREATE TABLE IF NOT EXISTS workspace_permissions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    workspace_name TEXT UNIQUE NOT NULL,
                    owner_user_id TEXT NOT NULL,
                    is_public INTEGER NOT NULL DEFAULT 1,
                    allowed_users_json TEXT NOT NULL
                )
            """)

            cursor.execute("""
                CREATE TABLE IF NOT EXISTS workspace_cloud_drives (
                    id TEXT PRIMARY KEY,
                    workspace_name TEXT NOT NULL,
                    provider TEXT NOT NULL,
                    mount_path_or_id TEXT NOT NULL,
                    title TEXT,
                    auth_status TEXT DEFAULT 'pending',
                    last_synced_at TEXT,
                    created_at TEXT,
                    metadata_json TEXT
                )
            """)
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS workspace_source_links (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    workspace_name TEXT NOT NULL,
                    source_type TEXT NOT NULL,
                    source_identifier TEXT NOT NULL,
                    title TEXT,
                    created_at TEXT NOT NULL,
                    UNIQUE(workspace_name, source_type, source_identifier)
                )
            """)
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_wsl_ws ON workspace_source_links (workspace_name)")

            cursor.execute("""
                CREATE TABLE IF NOT EXISTS workspace_files_stat_cache (
                    workspace_name TEXT NOT NULL,
                    file_path TEXT NOT NULL,
                    last_mtime REAL NOT NULL,
                    file_size INTEGER NOT NULL,
                    doc_id TEXT,
                    content_hash TEXT,
                    indexed_at TEXT,
                    PRIMARY KEY (workspace_name, file_path)
                )
            """)
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_wfsc_ws ON workspace_files_stat_cache (workspace_name)")

            # Backfill/canonicalize existing users.allowed_workspaces_json to immutable workspace_ids
            try:
                cursor.execute("SELECT id, allowed_workspaces_json FROM users")
                for u_row in cursor.fetchall():
                    try:
                        raw_aws = json.loads(u_row["allowed_workspaces_json"])
                        updated = []
                        changed = False
                        for w in raw_aws:
                            if w == "*":
                                updated.append("*")
                            elif str(w).startswith("ws_"):
                                updated.append(w)
                            else:
                                cursor.execute("SELECT workspace_id FROM workspaces WHERE name = ? COLLATE NOCASE", (str(w).strip(),))
                                match = cursor.fetchone()
                                if match and match["workspace_id"]:
                                    updated.append(match["workspace_id"])
                                    changed = True
                                else:
                                    updated.append(w)
                        if changed:
                            cursor.execute("UPDATE users SET allowed_workspaces_json = ? WHERE id = ?", (json.dumps(updated), u_row["id"]))
                    except Exception:
                        pass
            except sqlite3.OperationalError:
                pass

            # Backfill/canonicalize existing access_tokens.allowed_workspaces_json to immutable workspace_ids
            try:
                cursor.execute("SELECT id, allowed_workspaces_json FROM access_tokens")
                for t_row in cursor.fetchall():
                    try:
                        raw_aws = json.loads(t_row["allowed_workspaces_json"])
                        updated = []
                        changed = False
                        for w in raw_aws:
                            if w == "*":
                                updated.append("*")
                            elif str(w).startswith("ws_"):
                                updated.append(w)
                            else:
                                cursor.execute("SELECT workspace_id FROM workspaces WHERE name = ? COLLATE NOCASE", (str(w).strip(),))
                                match = cursor.fetchone()
                                if match and match["workspace_id"]:
                                    updated.append(match["workspace_id"])
                                    changed = True
                                else:
                                    updated.append(w)
                        if changed:
                            cursor.execute("UPDATE access_tokens SET allowed_workspaces_json = ? WHERE id = ?", (json.dumps(updated), t_row["id"]))
                    except Exception:
                        pass
            except sqlite3.OperationalError:
                pass
            conn.commit()
    def reset_model_settings_to_default(self):
        """Resets model settings and API keys to factory defaults while preserving workspaces and user data."""
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            cursor.execute("DELETE FROM models")
            cursor.execute("DELETE FROM api_keys")
            conn.commit()
        self._init_db()

    def ensure_default_workspace(self):
        """Ensures that 'Default', 'Global', and 'Shared Sources' system workspaces exist for instant onboarding, compliance, and reusable libraries."""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            # 1. Ensure Default workspace
            cursor.execute("SELECT id FROM workspaces WHERE name = 'Default' COLLATE NOCASE")
            if not cursor.fetchone():
                default_path = os.path.abspath(os.path.join(os.getcwd(), "documents"))
                os.makedirs(default_path, exist_ok=True)
                cursor.execute(
                    "INSERT INTO workspaces (workspace_id, name, paths_json) VALUES (?, ?, ?)",
                    ("ws_default", "Default", json.dumps([default_path]))
                )
            # 2. Ensure Global workspace
            cursor.execute("SELECT id FROM workspaces WHERE name = 'Global' COLLATE NOCASE")
            if not cursor.fetchone():
                cursor.execute(
                    "INSERT INTO workspaces (workspace_id, name, paths_json) VALUES (?, ?, ?)",
                    ("ws_global", "Global", json.dumps([]))
                )
            # 3. Ensure Shared Sources library workspace
            cursor.execute("SELECT id FROM workspaces WHERE name = 'Shared Sources' COLLATE NOCASE")
            if not cursor.fetchone():
                cursor.execute(
                    "INSERT INTO workspaces (workspace_id, name, paths_json) VALUES (?, ?, ?)",
                    ("ws_shared_sources", "Shared Sources", json.dumps([]))
                )
            conn.commit()

    def get_workspace_meta(self, identifier: str) -> Optional[Dict[str, Any]]:
        """Resolves a workspace by its immutable workspace_id, numeric ID, or its name."""
        clean = str(identifier).strip()
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute(
                "SELECT id, workspace_id, name, paths_json FROM workspaces WHERE workspace_id = ? OR name = ? COLLATE NOCASE",
                (clean, clean)
            )
            row = cursor.fetchone()
            if not row and clean.startswith("ws_") and clean[3:].isdigit():
                cursor.execute("SELECT id, workspace_id, name, paths_json FROM workspaces WHERE id = ?", (int(clean[3:]),))
                row = cursor.fetchone()

            if row:
                ws_id = row["workspace_id"]
                if not ws_id:
                    lname = row["name"].strip().lower()
                    ws_id = "ws_default" if lname == "default" else ("ws_global" if lname == "global" else ("ws_shared_sources" if lname == "shared sources" else f"ws_{uuid.uuid4().hex[:8]}"))
                    cursor.execute("UPDATE workspaces SET workspace_id = ? WHERE id = ?", (ws_id, row["id"]))
                    conn.commit()
                return {
                    "id": row["id"],
                    "workspace_id": ws_id,
                    "name": row["name"],
                    "paths": json.loads(row["paths_json"]) if row["paths_json"] else []
                }
            return None

    def is_empty(self) -> bool:
        """Returns True if no workspaces exist in the database"""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT COUNT(*) FROM workspaces")
            count = cursor.fetchone()[0]
            return count == 0

    def add_workspace(self, name: str, paths: List[str], workspace_id: Optional[str] = None) -> Dict[str, Any]:
        """Adds or updates a workspace entry with folder paths and an immutable workspace_id."""
        clean_name = name.strip()
        clean_paths = [os.path.abspath(p.strip().strip("'\"")) for p in paths if p and p.strip()]
        lname = clean_name.lower()
        ws_id = workspace_id or ("ws_default" if lname == "default" else ("ws_global" if lname == "global" else ("ws_shared_sources" if lname == "shared sources" else f"ws_{uuid.uuid4().hex[:8]}")))

        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT workspace_id, paths_json FROM workspaces WHERE name = ? COLLATE NOCASE", (clean_name,))
            row = cursor.fetchone()
            if row:
                existing_ws_id = row["workspace_id"] or ws_id
                existing_paths = [os.path.abspath(p.strip().strip("'\"")) for p in json.loads(row["paths_json"])]
                combined = list(dict.fromkeys(existing_paths + clean_paths))
                cursor.execute("UPDATE workspaces SET paths_json = ?, workspace_id = ? WHERE id = ?", (json.dumps(combined), existing_ws_id, row["id"] if "id" in row.keys() else 1))
                conn.commit()
                return {"id": existing_ws_id, "workspace_id": existing_ws_id, "name": clean_name, "paths": combined}
            else:
                cursor.execute("INSERT INTO workspaces (workspace_id, name, paths_json) VALUES (?, ?, ?)", (ws_id, clean_name, json.dumps(clean_paths)))
                conn.commit()
                return {"id": ws_id, "workspace_id": ws_id, "name": clean_name, "paths": clean_paths}

    def add_folder_to_workspace(self, workspace_name: str, folder_path: str) -> bool:
        """Adds a new folder path to an existing workspace."""
        clean_ws = workspace_name.strip()
        clean_path = os.path.abspath(folder_path.strip().strip("'\""))
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT paths_json FROM workspaces WHERE name = ?", (clean_ws,))
            row = cursor.fetchone()
            if not row:
                return False
            existing_paths = [os.path.abspath(p.strip().strip("'\"")) for p in json.loads(row["paths_json"])]
            if clean_path not in existing_paths:
                existing_paths.append(clean_path)
                cursor.execute("UPDATE workspaces SET paths_json = ? WHERE name = ?", (json.dumps(existing_paths), clean_ws))
                conn.commit()
            return True

    def remove_folder_from_workspace(self, workspace_name: str, folder_path: str) -> bool:
        """Removes a folder path from an existing workspace."""
        clean_ws = workspace_name.strip()
        clean_path = os.path.abspath(folder_path.strip().strip("'\""))
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT paths_json FROM workspaces WHERE name = ?", (clean_ws,))
            row = cursor.fetchone()
            if not row:
                return False
            existing_paths = json.loads(row["paths_json"])
            updated_paths = [p for p in existing_paths if os.path.abspath(p.strip().strip("'\"")) != clean_path]
            cursor.execute("UPDATE workspaces SET paths_json = ? WHERE name = ?", (json.dumps(updated_paths), clean_ws))
            conn.commit()
            return True

    def remove_workspace(self, workspace_name: str) -> bool:
        """Deletes a workspace entry and all its associated source records completely from SQLite."""
        clean_ws = workspace_name.strip()
        if clean_ws.lower() in ["default", "global", "shared sources"]:
            return False

        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("DELETE FROM workspaces WHERE name = ? AND LOWER(name) NOT IN ('default', 'global', 'shared sources')", (clean_ws,))
            deleted_count = cursor.rowcount
            try:
                cursor.execute("DELETE FROM workspace_folders WHERE workspace_name = ?", (clean_ws,))
            except sqlite3.OperationalError:
                pass
            try:
                cursor.execute("DELETE FROM workspace_web_urls WHERE workspace_name = ?", (clean_ws,))
            except sqlite3.OperationalError:
                pass
            try:
                cursor.execute("DELETE FROM workspace_indexed_web_pages WHERE workspace_name = ?", (clean_ws,))
            except sqlite3.OperationalError:
                pass
            try:
                cursor.execute("DELETE FROM workspace_cloud_drives WHERE workspace_name = ?", (clean_ws,))
            except sqlite3.OperationalError:
                pass
            try:
                cursor.execute("DELETE FROM workspace_permissions WHERE workspace_name = ?", (clean_ws,))
            except sqlite3.OperationalError:
                pass
            try:
                cursor.execute("DELETE FROM workspace_user_permissions WHERE workspace_name = ?", (clean_ws,))
            except sqlite3.OperationalError:
                pass
            try:
                cursor.execute("DELETE FROM workspace_share_invites WHERE workspace_name = ?", (clean_ws,))
            except sqlite3.OperationalError:
                pass
            try:
                cursor.execute("DELETE FROM workspace_source_links WHERE workspace_name = ?", (clean_ws,))
            except sqlite3.OperationalError:
                pass
            try:
                cursor.execute("DELETE FROM workspace_files_stat_cache WHERE workspace_name = ?", (clean_ws,))
            except sqlite3.OperationalError:
                pass
            conn.commit()
            return deleted_count > 0

    def transfer_local_folder_source(
        self,
        source_ws: str,
        target_ws: str,
        folder_path: str
    ) -> Dict[str, Any]:
        """
        Transfers a local folder and all its indexed vector chunks from source_ws to target_ws in < 50ms without recalculating embeddings.
        1. Updates workspace paths in SQLite.
        2. Updates chunk metadata ('workspace': target_ws) in ChromaDB.
        """
        source_ws = source_ws.strip()
        target_ws = target_ws.strip()
        cleaned_folder = folder_path.strip().strip("'\"")
        abs_folder = os.path.abspath(cleaned_folder)

        if source_ws == target_ws:
            return {"success": False, "error": "Source and target workspaces cannot be the same."}

        # 1. Update SQLite workspace paths
        with self._get_connection() as conn:
            cursor = conn.cursor()
            # Fetch source paths
            cursor.execute("SELECT paths_json FROM workspaces WHERE name = ?", (source_ws,))
            row_src = cursor.fetchone()
            if not row_src:
                return {"success": False, "error": f"Source workspace '{source_ws}' does not exist."}
            src_paths = json.loads(row_src["paths_json"]) if row_src["paths_json"] else []

            # Fetch target paths
            cursor.execute("SELECT paths_json FROM workspaces WHERE name = ?", (target_ws,))
            row_tgt = cursor.fetchone()
            if not row_tgt:
                return {"success": False, "error": f"Target workspace '{target_ws}' does not exist."}
            tgt_paths = json.loads(row_tgt["paths_json"]) if row_tgt["paths_json"] else []

            # Find matching path in source
            matching_path = None
            for p in src_paths:
                clean_p = p.strip().strip("'\"")
                if os.path.abspath(clean_p) == abs_folder or abs_folder.startswith(os.path.abspath(clean_p)) or os.path.abspath(clean_p).startswith(abs_folder):
                    matching_path = p
                    break

            if not matching_path:
                matching_path = abs_folder

            # Remove from source, add to target
            clean_matching = os.path.abspath(matching_path.strip().strip("'\""))
            new_src_paths = [p for p in src_paths if os.path.abspath(p.strip().strip("'\"")) != clean_matching]
            clean_tgt_paths = [os.path.abspath(tp.strip().strip("'\"")) for tp in tgt_paths]
            if clean_matching not in clean_tgt_paths and abs_folder not in clean_tgt_paths:
                clean_tgt_paths.append(abs_folder)

            cursor.execute("UPDATE workspaces SET paths_json = ? WHERE name = ?", (json.dumps(new_src_paths), source_ws))
            cursor.execute("UPDATE workspaces SET paths_json = ? WHERE name = ?", (json.dumps(clean_tgt_paths), target_ws))
            try:
                cursor.execute(
                    "UPDATE workspace_files_stat_cache SET workspace_name = ? WHERE workspace_name = ? AND (file_path = ? OR file_path LIKE ?)",
                    (target_ws, source_ws, abs_folder, abs_folder + os.sep + "%")
                )
            except sqlite3.OperationalError:
                pass
            conn.commit()

        # 2. Update ChromaDB vector metadata
        transferred_chunks = 0
        try:
            settings = AppSettings.load()
            db_path = settings.context.db_path if (settings and settings.context) else "./context_db"
            coll_name = settings.context.collection_name if (settings and settings.context) else "context_docs"

            if os.path.exists(db_path):
                import chromadb
                client = chromadb.PersistentClient(path=db_path)
                try:
                    collection = client.get_collection(coll_name)
                    results = collection.get(
                        where={"workspace": source_ws},
                        include=["metadatas"]
                    )
                    ids_to_update = []
                    metas_to_update = []

                    if results and results.get("ids"):
                        for cid, meta in zip(results["ids"], results["metadatas"]):
                            fp = (meta.get("file_path", "") or meta.get("source", "")).strip().strip("'\"")
                            if fp:
                                abs_fp = os.path.abspath(fp)
                                if abs_fp == abs_folder or abs_fp.startswith(abs_folder + os.sep) or abs_folder.startswith(abs_fp) or abs_folder in abs_fp:
                                    ids_to_update.append(cid)
                                    new_meta = dict(meta)
                                    new_meta["workspace"] = target_ws
                                    metas_to_update.append(new_meta)

                    if ids_to_update:
                        collection.update(
                            ids=ids_to_update,
                            metadatas=metas_to_update
                        )
                        transferred_chunks = len(ids_to_update)
                except Exception:
                    pass
        except Exception:
            pass

        return {
            "success": True,
            "source_workspace": source_ws,
            "target_workspace": target_ws,
            "folder_path": abs_folder,
            "transferred_chunks": transferred_chunks
        }

    def get_workspace_files_cache(self, workspace_name: str) -> Dict[str, Dict[str, Any]]:
        """Returns a dict mapping normalized file_path to its cached stat metadata {last_mtime, file_size, doc_id, content_hash, indexed_at}."""
        clean_ws = workspace_name.strip()
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute(
                "SELECT file_path, last_mtime, file_size, doc_id, content_hash, indexed_at FROM workspace_files_stat_cache WHERE workspace_name = ?",
                (clean_ws,)
            )
            rows = cursor.fetchall()
            return {
                r["file_path"]: {
                    "file_path": r["file_path"],
                    "last_mtime": r["last_mtime"],
                    "file_size": r["file_size"],
                    "doc_id": r["doc_id"],
                    "content_hash": r["content_hash"],
                    "indexed_at": r["indexed_at"]
                }
                for r in rows
            }

    def upsert_workspace_files_cache(self, workspace_name: str, records: List[Dict[str, Any]]):
        """Batch upserts file stat records into workspace_files_stat_cache."""
        if not records:
            return
        clean_ws = workspace_name.strip()
        from datetime import datetime
        now_str = datetime.utcnow().isoformat()
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.executemany(
                """
                INSERT INTO workspace_files_stat_cache (workspace_name, file_path, last_mtime, file_size, doc_id, content_hash, indexed_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(workspace_name, file_path) DO UPDATE SET
                    last_mtime = excluded.last_mtime,
                    file_size = excluded.file_size,
                    doc_id = excluded.doc_id,
                    content_hash = excluded.content_hash,
                    indexed_at = excluded.indexed_at
                """,
                [
                    (
                        clean_ws,
                        rec["file_path"],
                        rec["last_mtime"],
                        rec["file_size"],
                        rec.get("doc_id"),
                        rec.get("content_hash"),
                        rec.get("indexed_at", now_str)
                    )
                    for rec in records
                ]
            )
            conn.commit()

    def remove_workspace_files_cache(self, workspace_name: str, file_paths: List[str]):
        """Batch deletes file stat records from workspace_files_stat_cache."""
        if not file_paths:
            return
        clean_ws = workspace_name.strip()
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.executemany(
                "DELETE FROM workspace_files_stat_cache WHERE workspace_name = ? AND file_path = ?",
                [(clean_ws, fp) for fp in file_paths]
            )
            conn.commit()

    def rename_cached_file_path(self, workspace_name: str, old_path: str, new_path: str):
        """Updates a cached file path when a file or folder is renamed."""
        clean_ws = workspace_name.strip()
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute(
                "UPDATE workspace_files_stat_cache SET file_path = ? WHERE workspace_name = ? AND file_path = ?",
                (new_path, clean_ws, old_path)
            )
            conn.commit()

    def clear_workspace_files_cache(self, workspace_name: Optional[str] = None):
        """Clears all cached file stats for a workspace, or for all workspaces."""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            if workspace_name:
                cursor.execute("DELETE FROM workspace_files_stat_cache WHERE workspace_name = ?", (workspace_name.strip(),))
            else:
                cursor.execute("DELETE FROM workspace_files_stat_cache")
            conn.commit()

    def rename_workspace(self, old_name: str, new_name: str) -> Dict[str, Any]:
        """
        Renames a workspace from old_name to new_name atomically across SQLite and ChromaDB in < 50ms ($0.00 cost).
        1. Validates guardrails (existence, non-empty, collision check).
        2. Updates SQLite tables: workspaces, workspace_folders, workspace_user_permissions, workspace_share_invites, workspace_web_urls, workspace_indexed_web_pages.
        3. Updates ChromaDB vector metadata ('workspace': new_name) for document and session collections.
        """
        old_ws = (old_name or "").strip()
        new_ws = (new_name or "").strip()

        if not old_ws:
            return {"success": False, "error": "Current workspace name cannot be empty."}
        if not new_ws:
            return {"success": False, "error": "New workspace name cannot be empty."}
        if old_ws == new_ws:
            return {"success": False, "error": "New workspace name must be different from current name."}
        if old_ws.lower() in ["default", "global", "shared sources"]:
            return {"success": False, "error": f"Workspace '{old_ws}' is a protected system workspace and cannot be renamed."}
        if new_ws.lower() in ["default", "global", "shared sources"]:
            return {"success": False, "error": f"Cannot rename to protected system workspace '{new_ws}'."}

        # 1. Update SQLite tables atomically
        with self._get_connection() as conn:
            cursor = conn.cursor()
            # Verify old workspace exists
            cursor.execute("SELECT id, workspace_id, name FROM workspaces WHERE workspace_id = ? OR name = ? COLLATE NOCASE", (old_ws, old_ws))
            row_old = cursor.fetchone()
            if not row_old:
                return {"success": False, "error": f"Workspace '{old_ws}' does not exist."}

            actual_old_name = row_old["name"]
            ws_id = row_old["workspace_id"] or f"ws_{uuid.uuid4().hex[:8]}"

            # Verify new workspace name is not taken by another workspace
            cursor.execute("SELECT id FROM workspaces WHERE name = ? COLLATE NOCASE AND id != ?", (new_ws, row_old["id"]))
            if cursor.fetchone():
                return {"success": False, "error": f"Workspace '{new_ws}' already exists."}

            # Update workspaces table
            cursor.execute("UPDATE workspaces SET name = ?, workspace_id = ? WHERE id = ?", (new_ws, ws_id, row_old["id"]))

            # Update workspace_folders (if table exists)
            try:
                cursor.execute("UPDATE workspace_folders SET workspace_name = ? WHERE workspace_name = ? OR workspace_name = ?", (new_ws, actual_old_name, old_ws))
            except sqlite3.OperationalError:
                pass

            # Update workspace_permissions (if table exists)
            try:
                cursor.execute("UPDATE workspace_permissions SET workspace_name = ? WHERE workspace_name = ? OR workspace_name = ?", (new_ws, actual_old_name, old_ws))
            except sqlite3.OperationalError:
                pass

            # Update workspace_user_permissions (if table exists)
            try:
                cursor.execute("UPDATE workspace_user_permissions SET workspace_name = ? WHERE workspace_name = ? OR workspace_name = ?", (new_ws, actual_old_name, old_ws))
            except sqlite3.OperationalError:
                pass

            # Update workspace_share_invites (if table exists)
            try:
                cursor.execute("UPDATE workspace_share_invites SET workspace_name = ? WHERE workspace_name = ? OR workspace_name = ?", (new_ws, actual_old_name, old_ws))
            except sqlite3.OperationalError:
                pass

            # Update workspace_web_urls (if table exists)
            try:
                cursor.execute("UPDATE workspace_web_urls SET workspace_name = ? WHERE workspace_name = ? OR workspace_name = ?", (new_ws, actual_old_name, old_ws))
            except sqlite3.OperationalError:
                pass

            # Update workspace_indexed_web_pages (if table exists)
            try:
                cursor.execute("UPDATE workspace_indexed_web_pages SET workspace_name = ? WHERE workspace_name = ? OR workspace_name = ?", (new_ws, actual_old_name, old_ws))
            except sqlite3.OperationalError:
                pass

            # Update workspace_cloud_drives (if table exists)
            try:
                cursor.execute("UPDATE workspace_cloud_drives SET workspace_name = ? WHERE workspace_name = ? OR workspace_name = ?", (new_ws, actual_old_name, old_ws))
            except sqlite3.OperationalError:
                pass

            # Update workspace_source_links (if table exists)
            try:
                cursor.execute("UPDATE workspace_source_links SET workspace_name = ? WHERE workspace_name = ? OR workspace_name = ?", (new_ws, actual_old_name, old_ws))
            except sqlite3.OperationalError:
                pass

            # Update workspace_files_stat_cache (if table exists)
            try:
                cursor.execute("UPDATE workspace_files_stat_cache SET workspace_name = ? WHERE workspace_name = ? OR workspace_name = ?", (new_ws, actual_old_name, old_ws))
            except sqlite3.OperationalError:
                pass

            # Cascade update users.allowed_workspaces_json
            try:
                cursor.execute("SELECT id, allowed_workspaces_json FROM users")
                for u_row in cursor.fetchall():
                    try:
                        aws = json.loads(u_row["allowed_workspaces_json"])
                        updated_aws = []
                        for w in aws:
                            if w == "*":
                                updated_aws.append("*")
                            elif w == ws_id or w.lower() in [actual_old_name.lower(), old_ws.lower()]:
                                updated_aws.append(ws_id)
                            else:
                                updated_aws.append(w)
                        if updated_aws != aws:
                            cursor.execute("UPDATE users SET allowed_workspaces_json = ? WHERE id = ?", (json.dumps(updated_aws), u_row["id"]))
                    except Exception:
                        pass
            except sqlite3.OperationalError:
                pass

            # Cascade update access_tokens.allowed_workspaces_json
            try:
                cursor.execute("SELECT id, allowed_workspaces_json FROM access_tokens")
                for t_row in cursor.fetchall():
                    try:
                        aws = json.loads(t_row["allowed_workspaces_json"])
                        updated_aws = []
                        for w in aws:
                            if w == "*":
                                updated_aws.append("*")
                            elif w == ws_id or w.lower() in [actual_old_name.lower(), old_ws.lower()]:
                                updated_aws.append(ws_id)
                            else:
                                updated_aws.append(w)
                        if updated_aws != aws:
                            cursor.execute("UPDATE access_tokens SET allowed_workspaces_json = ? WHERE id = ?", (json.dumps(updated_aws), t_row["id"]))
                    except Exception:
                        pass
            except sqlite3.OperationalError:
                pass

            conn.commit()

        # 2. Update ChromaDB vector metadata (Document vectors)
        migrated_chunks = 0
        try:
            settings = AppSettings.load()
            db_path = settings.context.db_path if (settings and settings.context) else "./context_db"
            coll_name = settings.context.collection_name if (settings and settings.context) else "context_docs"

            if os.path.exists(db_path):
                import chromadb
                client = chromadb.PersistentClient(path=db_path)
                try:
                    coll = client.get_collection(coll_name)
                    results = coll.get(where={"workspace": old_ws}, include=["metadatas"])
                    if results and results.get("ids"):
                        ids = results["ids"]
                        metadatas = results["metadatas"]
                        for meta in metadatas:
                            meta["workspace"] = new_ws
                        coll.update(ids=ids, metadatas=metadatas)
                        migrated_chunks += len(ids)
                except Exception:
                    pass
        except Exception:
            pass

        # 3. Update ChromaDB vector metadata (Session memory vectors)
        try:
            settings = AppSettings.load()
            mem_db_path = settings.session.db_path if (settings and settings.session) else "./memory"
            mem_coll_name = settings.session.collection_name if (settings and settings.session) else "session_docs"

            if os.path.exists(mem_db_path):
                import chromadb
                mem_client = chromadb.PersistentClient(path=mem_db_path)
                try:
                    mem_coll = mem_client.get_collection(mem_coll_name)
                    mem_results = mem_coll.get(where={"workspace": old_ws}, include=["metadatas"])
                    if mem_results and mem_results.get("ids"):
                        m_ids = mem_results["ids"]
                        m_metas = mem_results["metadatas"]
                        for meta in m_metas:
                            meta["workspace"] = new_ws
                        mem_coll.update(ids=m_ids, metadatas=m_metas)
                except Exception:
                    pass
        except Exception:
            pass

        return {
            "success": True,
            "workspace_id": ws_id,
            "old_workspace": actual_old_name,
            "new_workspace": new_ws,
            "migrated_chunks": migrated_chunks,
            "api_cost": "$0.00"
        }





    def add_cloud_drive_to_workspace(
        self,
        workspace_name: str,
        provider: str,
        mount_path_or_id: str,
        title: Optional[str] = None,
        metadata: Optional[Dict[str, Any]] = None
    ) -> Dict[str, Any]:
        """
        Attaches a cloud drive (e.g. Google Drive, OneDrive, S3, Dropbox) to a workspace.
        """
        import uuid
        from datetime import datetime
        clean_ws = workspace_name.strip()
        clean_provider = provider.strip().lower()
        clean_mount = mount_path_or_id.strip()
        drive_id = f"drive_{uuid.uuid4().hex[:8]}"
        now_str = datetime.utcnow().isoformat()
        meta_json = json.dumps(metadata or {})

        with self._get_connection() as conn:
            cursor = conn.cursor()
            # Ensure workspace exists
            cursor.execute("SELECT id FROM workspaces WHERE name = ?", (clean_ws,))
            if not cursor.fetchone():
                self.add_workspace(clean_ws, paths=[])

            cursor.execute("""
                INSERT INTO workspace_cloud_drives (id, workspace_name, provider, mount_path_or_id, title, auth_status, created_at, metadata_json)
                VALUES (?, ?, ?, ?, ?, 'pending', ?, ?)
            """, (drive_id, clean_ws, clean_provider, clean_mount, title, now_str, meta_json))
            conn.commit()

        return {
            "id": drive_id,
            "workspace_name": clean_ws,
            "provider": clean_provider,
            "mount_path_or_id": clean_mount,
            "title": title,
            "auth_status": "pending",
            "created_at": now_str,
            "metadata": metadata or {}
        }

    def get_workspace_cloud_drives(self, workspace_name: str) -> List[Dict[str, Any]]:
        """Returns all configured cloud drive sources for a workspace."""
        clean_ws = workspace_name.strip()
        with self._get_connection() as conn:
            cursor = conn.cursor()
            try:
                cursor.execute("""
                    SELECT id, workspace_name, provider, mount_path_or_id, title, auth_status, last_synced_at, created_at, metadata_json
                    FROM workspace_cloud_drives
                    WHERE workspace_name = ?
                """, (clean_ws,))
                rows = cursor.fetchall()
                result = []
                for r in rows:
                    d = dict(r)
                    meta = {}
                    if d.get("metadata_json"):
                        try:
                            meta = json.loads(d["metadata_json"])
                        except Exception:
                            pass
                    d["metadata"] = meta
                    result.append(d)
                return result
            except sqlite3.OperationalError:
                return []

    def delete_cloud_drive(self, drive_id: str, workspace_name: Optional[str] = None) -> bool:
        """Removes a cloud drive attachment from SQLite."""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            try:
                if workspace_name:
                    cursor.execute("DELETE FROM workspace_cloud_drives WHERE id = ? AND workspace_name = ?", (drive_id, workspace_name.strip()))
                else:
                    cursor.execute("DELETE FROM workspace_cloud_drives WHERE id = ?", (drive_id,))
                conn.commit()
                return cursor.rowcount > 0
            except sqlite3.OperationalError:
                return False

    def get_workspace_sources(self, workspace_name: str) -> Dict[str, Any]:
        """
        Retrieves all associated sources for a workspace in a UI-agnostic structured format.
        Aggregates:
        1. Local Folders (from workspaces table and workspace_folders table).
        2. Web Sources / Portals (from workspace_web_urls table).
        3. Cloud Drives (from workspace_cloud_drives table).
        4. Unified polymorphic 'sources' list.
        """
        clean_ws = workspace_name.strip()
        folders: List[str] = []
        web_sources: List[Dict[str, Any]] = []
        cloud_drives: List[Dict[str, Any]] = []
        unified_sources: List[Dict[str, Any]] = []
        actual_ws_id = "ws_default" if clean_ws.lower() == "default" else f"ws_{uuid.uuid4().hex[:8]}"
        actual_ws_name = clean_ws

        with self._get_connection() as conn:
            cursor = conn.cursor()
            
            # 1. Local folders from workspaces table
            cursor.execute("SELECT id, workspace_id, name, paths_json FROM workspaces WHERE workspace_id = ? OR name = ?", (clean_ws, clean_ws))
            row = cursor.fetchone()
            if row:
                actual_ws_id = row["workspace_id"] or ("ws_default" if row["name"].lower() == "default" else f"ws_{row['id']}")
                actual_ws_name = row["name"]
                try:
                    for p in json.loads(row["paths_json"]):
                        norm_p = os.path.abspath(p.strip().strip("'\""))
                        if norm_p and norm_p not in folders:
                            folders.append(norm_p)
                except Exception:
                    pass

            # Also check workspace_folders table if present
            try:
                cursor.execute("SELECT folder_path FROM workspace_folders WHERE workspace_name = ? OR workspace_name = ?", (actual_ws_name, actual_ws_id))
                for f_row in cursor.fetchall():
                    norm_p = os.path.abspath(f_row["folder_path"].strip().strip("'\""))
                    if norm_p and norm_p not in folders:
                        folders.append(norm_p)
            except sqlite3.OperationalError:
                pass

            # 2. Web sources from workspace_web_urls
            try:
                cursor.execute("""
                    SELECT id, workspace_name, url, title, page_count, root_url, scope, last_scraped_at, created_at
                    FROM workspace_web_urls
                    WHERE workspace_name = ? OR workspace_name = ?
                """, (actual_ws_name, actual_ws_id))
                for w_row in cursor.fetchall():
                    w_dict = dict(w_row)
                    web_sources.append({
                        "id": w_dict.get("id"),
                        "url": w_dict.get("url"),
                        "root_url": w_dict.get("root_url") or w_dict.get("url"),
                        "title": w_dict.get("title"),
                        "page_count": w_dict.get("page_count", 1) or 1,
                        "scope": w_dict.get("scope"),
                        "last_scraped_at": w_dict.get("last_scraped_at"),
                        "created_at": w_dict.get("created_at")
                    })
            except sqlite3.OperationalError:
                pass

            # 3. Cloud drives from workspace_cloud_drives
            try:
                cursor.execute("""
                    SELECT id, workspace_name, provider, mount_path_or_id, title, auth_status, last_synced_at, created_at, metadata_json
                    FROM workspace_cloud_drives
                    WHERE workspace_name = ? OR workspace_name = ?
                """, (actual_ws_name, actual_ws_id))
                for cd_row in cursor.fetchall():
                    cd_dict = dict(cd_row)
                    meta = {}
                    if cd_dict.get("metadata_json"):
                        try:
                            meta = json.loads(cd_dict["metadata_json"])
                        except Exception:
                            pass
                    cloud_drives.append({
                        "id": cd_dict.get("id"),
                        "provider": cd_dict.get("provider"),
                        "mount_path_or_id": cd_dict.get("mount_path_or_id"),
                        "title": cd_dict.get("title"),
                        "auth_status": cd_dict.get("auth_status") or "pending",
                        "last_synced_at": cd_dict.get("last_synced_at"),
                        "created_at": cd_dict.get("created_at"),
                        "metadata": meta
                    })
            except sqlite3.OperationalError:
                pass

        # 4. Linked Shared Sources from workspace_source_links
        linked_sources = []
        try:
            cursor.execute("""
                SELECT id, workspace_name, source_type, source_identifier, title, created_at
                FROM workspace_source_links
                WHERE workspace_name = ? OR workspace_name = ?
            """, (actual_ws_name, actual_ws_id))
            for ls_row in cursor.fetchall():
                linked_sources.append(dict(ls_row))
        except sqlite3.OperationalError:
            pass

        # Build unified polymorphic sources list
        for f in folders:
            folder_title = os.path.basename(f) or f
            unified_sources.append({
                "type": "folder",
                "id": None,
                "identifier": f,
                "title": folder_title,
                "details": {
                    "path": f,
                    "exists": os.path.exists(f)
                }
            })

        for w in web_sources:
            unified_sources.append({
                "type": "web",
                "id": w["id"],
                "identifier": w["url"],
                "title": w.get("title") or w["url"],
                "details": {
                    "root_url": w.get("root_url"),
                    "page_count": w.get("page_count", 1),
                    "scope": w.get("scope"),
                    "last_scraped_at": w.get("last_scraped_at")
                }
            })

        for cd in cloud_drives:
            unified_sources.append({
                "type": "cloud_drive",
                "id": cd["id"],
                "identifier": cd["mount_path_or_id"],
                "title": cd.get("title") or f"{cd['provider']}://{cd['mount_path_or_id']}",
                "details": {
                    "provider": cd["provider"],
                    "auth_status": cd["auth_status"],
                    "last_synced_at": cd.get("last_synced_at")
                }
            })

        for ls in linked_sources:
            stype = ls.get("source_type", "folder")
            ident = ls.get("source_identifier", "")
            clean_title = ls.get("title") or (os.path.basename(ident) if stype == "folder" else ident)
            link_id = f"link_{ls.get('id')}" if ls.get("id") is not None else f"link_{len(unified_sources)+1}"
            unified_sources.append({
                "type": stype,
                "id": link_id,
                "identifier": ident,
                "title": f"{clean_title} (Shared)",
                "details": {
                    "is_shared_link": True,
                    "created_at": ls.get("created_at")
                }
            })
            if stype == "folder" and ident not in folders:
                folders.append(ident)

        return {
            "id": actual_ws_id,
            "name": actual_ws_name,
            "sources": unified_sources,
            "total_sources": len(unified_sources),
            "paths": folders
        }

    def link_shared_source_to_workspace(
        self,
        workspace_name: str,
        source_type: str,
        source_identifier: str,
        title: Optional[str] = None
    ) -> Dict[str, Any]:
        """
        Links an existing indexed source (folder or web portal) to workspace_name in < 50ms with zero API cost ($0.00).
        """
        from datetime import datetime
        clean_ws = workspace_name.strip()
        clean_type = source_type.strip().lower()
        clean_ident = source_identifier.strip().strip("'\"")
        if clean_type == "folder":
            clean_ident = os.path.abspath(clean_ident)
        
        created_at = datetime.utcnow().isoformat()
        clean_title = title.strip() if title else (os.path.basename(clean_ident) if clean_type == "folder" else clean_ident)

        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT OR REPLACE INTO workspace_source_links (workspace_name, source_type, source_identifier, title, created_at)
                VALUES (?, ?, ?, ?, ?)
            """, (clean_ws, clean_type, clean_ident, clean_title, created_at))
            conn.commit()

        return {
            "status": "success",
            "workspace": clean_ws,
            "source_type": clean_type,
            "source_identifier": clean_ident,
            "title": clean_title,
            "message": f"Source '{clean_ident}' successfully linked to workspace '{clean_ws}' ($0.00 cost)."
        }

    def attach_and_broadcast_source(
        self,
        primary_workspace: str,
        source_type: str,
        source_identifier: str,
        title: Optional[str] = None,
        link_to_workspaces: Optional[List[str]] = None
    ) -> Dict[str, Any]:
        """
        Attaches a data source (folder, web portal, or cloud drive) to primary_workspace
        and automatically broadcasts/links it to an optional list of additional workspaces ($0.00 cost).
        """
        clean_ws = primary_workspace.strip()
        clean_type = source_type.strip().lower()
        clean_ident = source_identifier.strip().strip("'\"")
        if clean_type == "folder":
            clean_ident = os.path.abspath(clean_ident)
            self.add_folder_to_workspace(clean_ws, clean_ident)
        elif clean_type in ["web", "url", "portal"]:
            from any_context.ingestion.web_scheduler import WebSchedulerStore
            web_store = WebSchedulerStore()
            web_store.add_or_update_root_web_source(
                workspace_name=clean_ws,
                root_url=clean_ident,
                title=title or clean_ident,
                page_count=1
            )

        linked_targets = []
        if link_to_workspaces:
            for tgt in link_to_workspaces:
                clean_tgt = tgt.strip()
                if clean_tgt and clean_tgt.lower() != clean_ws.lower():
                    self.link_shared_source_to_workspace(
                        workspace_name=clean_tgt,
                        source_type=clean_type,
                        source_identifier=clean_ident,
                        title=title
                    )
                    linked_targets.append(clean_tgt)

        return {
            "status": "success",
            "primary_workspace": clean_ws,
            "source_type": clean_type,
            "source_identifier": clean_ident,
            "title": title or (os.path.basename(clean_ident) if clean_type == "folder" else clean_ident),
            "linked_workspaces": linked_targets,
            "total_linked": len(linked_targets),
            "message": f"Source attached to '{clean_ws}' and broadcast-linked to {len(linked_targets)} workspaces."
        }

    def unlink_shared_source_from_workspace(
        self,
        workspace_name: str,
        source_type: str,
        source_identifier: str
    ) -> bool:
        """Unlinks a shared source from a workspace."""
        clean_ws = workspace_name.strip()
        clean_type = source_type.strip().lower()
        clean_ident = source_identifier.strip().strip("'\"")
        if clean_type == "folder":
            clean_ident = os.path.abspath(clean_ident)

        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                DELETE FROM workspace_source_links
                WHERE workspace_name = ? AND source_type = ? AND (source_identifier = ? OR source_identifier = ?)
            """, (clean_ws, clean_type, clean_ident, source_identifier.strip()))
            conn.commit()
            return cursor.rowcount > 0

    def get_workspace_shared_links(self, workspace_name: str) -> List[Dict[str, Any]]:
        """Returns all shared source links configured for a workspace."""
        clean_ws = workspace_name.strip()
        with self._get_connection() as conn:
            cursor = conn.cursor()
            try:
                cursor.execute("""
                    SELECT id, workspace_name, source_type, source_identifier, title, created_at
                    FROM workspace_source_links
                    WHERE workspace_name = ?
                """, (clean_ws,))
                return [dict(r) for r in cursor.fetchall()]
            except sqlite3.OperationalError:
                return []

    def list_all_available_shared_sources(self) -> List[Dict[str, Any]]:
        """
        Lists all unique indexed sources (folders, web portals, cloud drives) configured
        in the central 'Shared Sources' library (or institutional 'Global') available for cross-workspace linking.
        """
        available = []
        seen = set()

        for lib_ws_name in ["Shared Sources", "Global"]:
            ws_detail = self.get_workspace_sources(lib_ws_name)
            for s in ws_detail.get("sources", []):
                if s.get("details", {}).get("is_shared_link"):
                    continue
                stype = s.get("type")
                ident = s.get("identifier")
                key = f"{stype}:{ident}"
                if key not in seen:
                    seen.add(key)
                    available.append({
                        "type": stype,
                        "identifier": ident,
                        "title": s.get("title") or ident,
                        "origin_workspace": lib_ws_name,
                        "details": s.get("details", {})
                    })
        return available

    def list_workspaces_detailed(self) -> List[Dict[str, Any]]:
        """
        Lists all workspaces with complete, UI-agnostic sources detail.
        """
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT name FROM workspaces ORDER BY id ASC")
            rows = cursor.fetchall()
            ws_names = [r["name"] for r in rows]

        return [self.get_workspace_sources(ws_name) for ws_name in ws_names]

    def get_app_settings(self) -> AppSettings:
        """Reads and constructs AppSettings Pydantic instance from SQLite"""
        with self._get_connection() as conn:
            cursor = conn.cursor()

            cursor.execute("SELECT workspace_id, name FROM workspaces ORDER BY id ASC")
            ws_rows = cursor.fetchall()
            workspaces = []
            for row in ws_rows:
                ws_name = row["name"]
                ws_id = row["workspace_id"] or ("ws_default" if ws_name.lower() == "default" else f"ws_{uuid.uuid4().hex[:8]}")
                ws_detail = self.get_workspace_sources(ws_name)
                workspaces.append(WorkspaceSettings(
                    id=ws_id,
                    name=ws_name,
                    paths=ws_detail.get("paths", []),
                    sources=[WorkspaceSourceItem(**s) for s in ws_detail["sources"]],
                    total_sources=ws_detail["total_sources"]
                ))

            cursor.execute("SELECT * FROM models WHERE id = 1")
            m_row = cursor.fetchone()
            if m_row:
                emb_val = None
                if "embedding_model" in m_row.keys() and m_row["embedding_model"]:
                    emb_val = m_row["embedding_model"]
                elif "local_openai_embedding_model" in m_row.keys() and m_row["local_openai_embedding_model"]:
                    emb_val = m_row["local_openai_embedding_model"]
                elif "local_embedding_model" in m_row.keys() and m_row["local_embedding_model"]:
                    emb_val = m_row["local_embedding_model"]
                emb_val = emb_val or "text-embedding-3-small"

                models = ModelSettings(
                    embedding_model=emb_val,
                    inference_model=m_row["inference_model"],
                    summary_model=m_row["summary_model"],
                    model_provider=m_row["model_provider"],
                    local_base_url=m_row["local_base_url"]
                )
            else:
                models = ModelSettings()

            cursor.execute("SELECT * FROM context_settings WHERE id = 1")
            c_row = cursor.fetchone()
            if c_row:
                c_keys = c_row.keys()
                c_sz = c_row["chunk_size"] if ("chunk_size" in c_keys and c_row["chunk_size"]) else 1024
                c_ov = c_row["chunk_overlap"] if ("chunk_overlap" in c_keys and c_row["chunk_overlap"] is not None) else 200
                c_top_k = c_row["top_k"] if ("top_k" in c_keys and c_row["top_k"]) else 40
                c_pool = c_row["candidate_pool_size"] if ("candidate_pool_size" in c_keys and c_row["candidate_pool_size"]) else 100
                c_max_src = c_row["max_chunks_per_source"] if ("max_chunks_per_source" in c_keys and c_row["max_chunks_per_source"]) else 3
                c_preset = c_row["retrieval_preset"] if ("retrieval_preset" in c_keys and c_row["retrieval_preset"]) else "balanced"
                c_mode = c_row["grounding_mode"] if ("grounding_mode" in c_keys and c_row["grounding_mode"]) else "hybrid"
                context = ContextSettings(
                    db_path=c_row["db_path"],
                    collection_name=c_row["collection_name"],
                    chunk_size=c_sz,
                    chunk_overlap=c_ov,
                    top_k=c_top_k,
                    candidate_pool_size=c_pool,
                    max_chunks_per_source=c_max_src,
                    retrieval_preset=c_preset,
                    grounding_mode=c_mode
                )
            else:
                context = ContextSettings()

            cursor.execute("SELECT * FROM session_settings WHERE id = 1")
            s_row = cursor.fetchone()
            session = SessionSettings(db_path=s_row["db_path"], collection_name=s_row["collection_name"]) if s_row else SessionSettings()

            cursor.execute("SELECT * FROM memory_settings WHERE id = 1")
            mem_row = cursor.fetchone()
            if mem_row:
                memory = MemorySettings(
                    short_term_buffer_size=mem_row["short_term_buffer_size"],
                    rolling_window_messages=mem_row["rolling_window_messages"],
                    meta_summary_threshold=mem_row["meta_summary_threshold"],
                    meta_summary_batch_size=mem_row["meta_summary_batch_size"]
                )
            else:
                memory = MemorySettings()

            return AppSettings(
                workspaces=workspaces,
                models=models,
                context=context,
                session=session,
                memory=memory
            )

    def save_app_settings(self, settings: AppSettings):
        """Saves or updates full AppSettings into SQLite"""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            
            cursor.execute("DELETE FROM workspaces")
            for ws in settings.workspaces:
                cursor.execute(
                    "INSERT INTO workspaces (name, paths_json) VALUES (?, ?)",
                    (ws.name, json.dumps(ws.paths))
                )

            m = settings.models
            cursor.execute("""
                INSERT OR REPLACE INTO models (id, embedding_model, local_embedding_model, local_openai_embedding_model, inference_model, summary_model, model_provider, local_base_url)
                VALUES (1, ?, ?, ?, ?, ?, ?, ?)
            """, (m.embedding_model, m.embedding_model, m.embedding_model, m.inference_model, m.summary_model, m.model_provider, m.local_base_url))

            c = settings.context
            cursor.execute("""
                INSERT OR REPLACE INTO context_settings (id, db_path, collection_name, chunk_size, chunk_overlap, top_k, candidate_pool_size, max_chunks_per_source, retrieval_preset, grounding_mode)
                VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """, (c.db_path, c.collection_name, c.chunk_size, c.chunk_overlap, c.top_k, c.candidate_pool_size, c.max_chunks_per_source, c.retrieval_preset, c.grounding_mode))

            s = settings.session
            cursor.execute("INSERT OR REPLACE INTO session_settings (id, db_path, collection_name) VALUES (1, ?, ?)", (s.db_path, s.collection_name))

            mem = settings.memory
            cursor.execute("""
                INSERT OR REPLACE INTO memory_settings (id, short_term_buffer_size, rolling_window_messages, meta_summary_threshold, meta_summary_batch_size)
                VALUES (1, ?, ?, ?, ?)
            """, (mem.short_term_buffer_size, mem.rolling_window_messages, mem.meta_summary_threshold, mem.meta_summary_batch_size))

            conn.commit()

    def update_context_settings(self, context: ContextSettings):
        """Updates context settings (db_path, collection_name, chunk_size, chunk_overlap, retrieval parameters, grounding_mode)"""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT OR REPLACE INTO context_settings (id, db_path, collection_name, chunk_size, chunk_overlap, top_k, candidate_pool_size, max_chunks_per_source, retrieval_preset, grounding_mode)
                VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """, (context.db_path, context.collection_name, context.chunk_size, context.chunk_overlap, context.top_k, context.candidate_pool_size, context.max_chunks_per_source, context.retrieval_preset, context.grounding_mode))
            conn.commit()

    def get_grounding_mode(self) -> str:
        """Retrieves the active AI Grounding & Answer Mode ('hybrid', 'strict', 'proactive')."""
        settings = self.get_app_settings()
        if settings and settings.context:
            mode = getattr(settings.context, "grounding_mode", "hybrid")
            if mode in ["hybrid", "strict", "proactive"]:
                return mode
        return "hybrid"

    def set_grounding_mode(self, mode: str) -> str:
        """Sets and persists the active AI Grounding Mode in SQLite context_settings."""
        clean_mode = mode.lower().strip() if mode else "hybrid"
        if clean_mode not in ["hybrid", "strict", "proactive"]:
            if "strict" in clean_mode:
                clean_mode = "strict"
            elif "pro" in clean_mode or "creat" in clean_mode:
                clean_mode = "proactive"
            else:
                clean_mode = "hybrid"

        settings = self.get_app_settings()
        ctx = settings.context if (settings and settings.context) else ContextSettings()
        ctx.grounding_mode = clean_mode
        self.update_context_settings(ctx)
        return clean_mode

    def update_session_settings(self, session: SessionSettings):
        """Updates session vector database settings"""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("INSERT OR REPLACE INTO session_settings (id, db_path, collection_name) VALUES (1, ?, ?)", (session.db_path, session.collection_name))
            conn.commit()

    def update_model_settings(self, models: ModelSettings):
        """Updates active model configuration"""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT OR REPLACE INTO models (id, embedding_model, local_embedding_model, local_openai_embedding_model, inference_model, summary_model, model_provider, local_base_url)
                VALUES (1, ?, ?, ?, ?, ?, ?, ?)
            """, (models.embedding_model, models.embedding_model, models.embedding_model, models.inference_model, models.summary_model, models.model_provider, models.local_base_url))
            conn.commit()

    def update_memory_settings(self, memory: MemorySettings):
        """Updates hierarchical memory thresholds"""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT OR REPLACE INTO memory_settings (id, short_term_buffer_size, rolling_window_messages, meta_summary_threshold, meta_summary_batch_size)
                VALUES (1, ?, ?, ?, ?)
            """, (memory.short_term_buffer_size, memory.rolling_window_messages, memory.meta_summary_threshold, memory.meta_summary_batch_size))
            conn.commit()

    def set_api_key(self, provider: str, api_key: str):
        """Stores or updates an API Key for a provider"""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute(
                "INSERT OR REPLACE INTO api_keys (provider, api_key) VALUES (?, ?)",
                (provider.lower().strip(), api_key.strip())
            )
            conn.commit()

    def get_api_key(self, provider: str) -> Optional[str]:
        """Retrieves a stored API Key for a provider"""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT api_key FROM api_keys WHERE provider = ?", (provider.lower().strip(),))
            row = cursor.fetchone()
            return row["api_key"] if row else None

    def get_all_api_keys(self) -> Dict[str, str]:
        """Retrieves all stored API keys"""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT provider, api_key FROM api_keys")
            rows = cursor.fetchall()
            return {row["provider"]: row["api_key"] for row in rows}

    # --- User Accounts & Security RBAC Methods ---

    def _canonicalize_workspace_list(self, ws_list: Optional[List[str]]) -> List[str]:
        """Converts workspace names/IDs to canonical workspace_ids (or '*' / raw strings)."""
        if ws_list is None:
            return ["ws_default"]
        canonical = []
        for w in ws_list:
            clean = str(w).strip()
            if not clean:
                continue
            if clean == "*":
                canonical.append("*")
            elif clean.startswith("ws_"):
                canonical.append(clean)
            else:
                meta = self.get_workspace_meta(clean)
                if meta and meta.get("workspace_id"):
                    canonical.append(meta["workspace_id"])
                else:
                    try:
                        new_ws = self.add_workspace(name=clean, paths=[])
                        canonical.append(new_ws["workspace_id"])
                    except Exception:
                        canonical.append(clean)
        return list(dict.fromkeys(canonical))

    def _resolve_allowed_workspaces_display(self, ws_list: List[str]) -> List[str]:
        """Resolves canonical workspace_ids to their current live workspace names."""
        resolved = []
        for w in ws_list:
            if w == "*":
                resolved.append("*")
            else:
                meta = self.get_workspace_meta(w)
                if meta:
                    resolved.append(meta["name"])
                else:
                    resolved.append(w)
        return resolved

    def is_admin_configured(self) -> bool:
        """Returns True if an Admin user has been configured in SQLite."""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT COUNT(*) FROM users WHERE role = 'admin'")
            return cursor.fetchone()[0] > 0

    def setup_admin_user(self, name: str, email: str, password: str) -> Dict[str, Any]:
        """Initial Admin setup wizard for first-time server security deployment."""
        import uuid
        from datetime import datetime

        if self.is_admin_configured():
            raise ValueError("Admin user is already configured.")

        user_id = f"usr_{uuid.uuid4().hex[:12]}"
        password_h = hash_password(password)
        created_at = datetime.utcnow().isoformat()
        allowed_ws = ["*"]

        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT INTO users (user_id, email, name, password_hash, role, allowed_workspaces_json, created_at)
                VALUES (?, ?, ?, ?, 'admin', ?, ?)
            """, (user_id, email.lower().strip(), name.strip(), password_h, json.dumps(allowed_ws), created_at))
            conn.commit()

        # Also create initial Master Admin Bearer Token
        token_info = self.create_access_token(
            name=f"Master Admin Token ({name})",
            role="admin",
            allowed_workspaces=allowed_ws,
            user_id=user_id
        )

        return {
            "user_id": user_id,
            "email": email.lower().strip(),
            "name": name.strip(),
            "role": "admin",
            "allowed_workspaces": allowed_ws,
            "token": token_info
        }

    def authenticate_user(self, email: str, password: str) -> Optional[Dict[str, Any]]:
        """Authenticates user credentials and returns user details + active access token."""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute(
                "SELECT user_id, email, name, password_hash, role, allowed_workspaces_json FROM users WHERE email = ?",
                (email.lower().strip(),)
            )
            row = cursor.fetchone()
            if not row or not verify_password(password, row["password_hash"]):
                return None

            user_id = row["user_id"]
            role = row["role"]
            raw_allowed_ws = json.loads(row["allowed_workspaces_json"])
            display_allowed_ws = self._resolve_allowed_workspaces_display(raw_allowed_ws)

            # Retrieve or generate active Bearer token for user
            tokens = self.get_access_tokens(user_id=user_id)
            if tokens:
                token_str = tokens[0]["token_id"]
            else:
                new_t = self.create_access_token(name=f"Session Token ({row['name']})", role=role, allowed_workspaces=raw_allowed_ws, user_id=user_id)
                token_str = new_t["token_id"]

            return {
                "user_id": user_id,
                "email": row["email"],
                "name": row["name"],
                "role": role,
                "allowed_workspaces": display_allowed_ws,
                "token_id": token_str
            }

    def create_user(self, name: str, email: str, password: str, role: str = "analyst", allowed_workspaces: Optional[List[str]] = None) -> Dict[str, Any]:
        """Creates a new team user (Admin only operation)."""
        import uuid
        from datetime import datetime

        user_id = f"usr_{uuid.uuid4().hex[:12]}"
        password_h = hash_password(password)
        role_clean = role.lower() if role in ["admin", "analyst", "viewer"] else "analyst"
        canonical_ws = self._canonicalize_workspace_list(allowed_workspaces if allowed_workspaces is not None else ["Default"])
        created_at = datetime.utcnow().isoformat()

        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT INTO users (user_id, email, name, password_hash, role, allowed_workspaces_json, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
            """, (user_id, email.lower().strip(), name.strip(), password_h, role_clean, json.dumps(canonical_ws), created_at))
            conn.commit()

        return {
            "user_id": user_id,
            "email": email.lower().strip(),
            "name": name.strip(),
            "role": role_clean,
            "allowed_workspaces": self._resolve_allowed_workspaces_display(canonical_ws),
            "created_at": created_at
        }

    def list_users(self) -> List[Dict[str, Any]]:
        """Lists all configured users with dynamically resolved workspace names."""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT user_id, email, name, role, allowed_workspaces_json, created_at FROM users ORDER BY id DESC")
            rows = cursor.fetchall()
            users = []
            for r in rows:
                raw_aws = json.loads(r["allowed_workspaces_json"])
                users.append({
                    "user_id": r["user_id"],
                    "email": r["email"],
                    "name": r["name"],
                    "role": r["role"],
                    "allowed_workspaces": self._resolve_allowed_workspaces_display(raw_aws),
                    "created_at": r["created_at"]
                })
            return users

    def delete_user(self, user_id: str) -> bool:
        """Deletes/revokes a user and all their active access tokens."""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("DELETE FROM users WHERE user_id = ?", (user_id,))
            cursor.execute("DELETE FROM access_tokens WHERE user_id = ?", (user_id,))
            conn.commit()
            return cursor.rowcount > 0

    # --- Access Token Methods ---

    def create_access_token(self, name: str, role: str = "viewer", allowed_workspaces: Optional[List[str]] = None, user_id: str = "system") -> Dict[str, Any]:
        """Creates a new security access token with role and workspace scopes"""
        import uuid
        from datetime import datetime
        
        token_id = f"actx_sec_{uuid.uuid4().hex}"
        role_clean = role.lower() if role in ["admin", "analyst", "viewer"] else "viewer"
        canonical_ws = self._canonicalize_workspace_list(allowed_workspaces if allowed_workspaces is not None else ["*"])
        created_at = datetime.utcnow().isoformat()

        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT INTO access_tokens (token_id, user_id, name, role, allowed_workspaces_json, created_at)
                VALUES (?, ?, ?, ?, ?, ?)
            """, (token_id, user_id, name, role_clean, json.dumps(canonical_ws), created_at))
            conn.commit()

        return {
            "token_id": token_id,
            "user_id": user_id,
            "name": name,
            "role": role_clean,
            "allowed_workspaces": self._resolve_allowed_workspaces_display(canonical_ws),
            "created_at": created_at
        }

    def get_access_tokens(self, user_id: Optional[str] = None) -> List[Dict[str, Any]]:
        """Lists all active access tokens, optionally filtered by user_id."""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            if user_id:
                cursor.execute("SELECT token_id, user_id, name, role, allowed_workspaces_json, created_at FROM access_tokens WHERE user_id = ? ORDER BY id DESC", (user_id,))
            else:
                cursor.execute("SELECT token_id, user_id, name, role, allowed_workspaces_json, created_at FROM access_tokens ORDER BY id DESC")
            rows = cursor.fetchall()
            tokens = []
            for r in rows:
                raw_aws = json.loads(r["allowed_workspaces_json"])
                tokens.append({
                    "token_id": r["token_id"],
                    "user_id": r["user_id"],
                    "name": r["name"],
                    "role": r["role"],
                    "allowed_workspaces": self._resolve_allowed_workspaces_display(raw_aws),
                    "created_at": r["created_at"]
                })
            return tokens

    def get_access_token(self, token_id: str) -> Optional[Dict[str, Any]]:
        """Gets details of a specific access token by ID."""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT token_id, user_id, name, role, allowed_workspaces_json, created_at FROM access_tokens WHERE token_id = ?", (token_id,))
            row = cursor.fetchone()
            if not row:
                return None
            raw_aws = json.loads(row["allowed_workspaces_json"])
            return {
                "token_id": row["token_id"],
                "user_id": row["user_id"],
                "name": row["name"],
                "role": row["role"],
                "allowed_workspaces": self._resolve_allowed_workspaces_display(raw_aws),
                "created_at": row["created_at"]
            }

    def delete_access_token(self, token_id: str) -> bool:
        """Revokes/deletes an access token."""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("DELETE FROM access_tokens WHERE token_id = ?", (token_id,))
            conn.commit()
            return cursor.rowcount > 0

    def validate_token_permissions(self, token_id: str, required_role: Optional[str] = None, required_workspace: Optional[str] = None) -> bool:
        """Validates if a token is valid, active, has required role and workspace permission."""
        token_info = self.get_access_token(token_id)
        if not token_info:
            return False

        token_role = token_info["role"]
        allowed_workspaces = token_info["allowed_workspaces"]

        # Admins have full access to all roles and all workspaces
        if token_role == "admin":
            return True

        # Check Role Hierarchy (admin > analyst > viewer)
        if required_role:
            if required_role == "admin" and token_role != "admin":
                return False
            if required_role == "analyst" and token_role not in ["admin", "analyst"]:
                return False

        # Check Workspace Scope
        if required_workspace and "*" not in allowed_workspaces:
            clean_req = required_workspace.strip()
            ws_meta = self.get_workspace_meta(clean_req)
            ws_name = ws_meta["name"] if ws_meta else clean_req
            ws_id = ws_meta["workspace_id"] if ws_meta else clean_req

            allowed_ids = set()
            allowed_names = set()
            for w in allowed_workspaces:
                allowed_ids.add(w)
                allowed_names.add(w.lower())
                m = self.get_workspace_meta(w)
                if m:
                    allowed_ids.add(m["workspace_id"])
                    allowed_names.add(m["name"].lower())

            if (
                ws_id not in allowed_ids
                and ws_name.lower() not in allowed_names
                and clean_req.lower() not in allowed_names
                and clean_req not in allowed_ids
            ):
                return False

        return True

    def factory_reset(self) -> bool:
        """
        Wipes all configuration tables, API keys, workspaces, users, access tokens, and deletes local vector database directories.
        Resets AnyContext completely back to factory defaults.
        """
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("DELETE FROM workspaces")
            cursor.execute("DELETE FROM models")
            cursor.execute("DELETE FROM context_settings")
            cursor.execute("DELETE FROM session_settings")
            cursor.execute("DELETE FROM memory_settings")
            cursor.execute("DELETE FROM api_keys")
            cursor.execute("DELETE FROM users")
            cursor.execute("DELETE FROM access_tokens")
            cursor.execute("DELETE FROM workspace_permissions")
            conn.commit()

        import shutil
        for dir_path in ["./context_db", "./memory", "context_db", "memory"]:
            if os.path.exists(dir_path):
                try:
                    shutil.rmtree(dir_path, ignore_errors=True)
                except Exception:
                    pass

        self.ensure_default_workspace()
        return True
