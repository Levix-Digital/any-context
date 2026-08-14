import os
import sys
import json
import sqlite3
import hashlib
import secrets
from typing import Optional, List, Dict, Any
from any_context.config.app_settings import (
    AppSettings,
    WorkspaceSettings,
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
                    name TEXT UNIQUE NOT NULL,
                    paths_json TEXT NOT NULL
                )
            """)

            cursor.execute("""
                CREATE TABLE IF NOT EXISTS models (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    local_embedding_model TEXT NOT NULL,
                    local_openai_embedding_model TEXT NOT NULL,
                    inference_model TEXT NOT NULL,
                    summary_model TEXT NOT NULL,
                    model_provider TEXT NOT NULL,
                    local_base_url TEXT NOT NULL
                )
            """)

            cursor.execute("""
                CREATE TABLE IF NOT EXISTS context_settings (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    db_path TEXT NOT NULL,
                    collection_name TEXT NOT NULL
                )
            """)

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
    def reset_model_settings_to_default(self):
        """Resets model settings and API keys to factory defaults while preserving workspaces and user data."""
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            cursor.execute("DELETE FROM models")
            cursor.execute("DELETE FROM api_keys")
            conn.commit()
        self._init_db()

    def ensure_default_workspace(self):

        """Ensures that at least a 'Default' workspace exists for instant friction-free onboarding."""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT COUNT(*) FROM workspaces")
            if cursor.fetchone()[0] == 0:
                default_path = os.path.abspath(os.path.join(os.getcwd(), "documents"))
                os.makedirs(default_path, exist_ok=True)
                cursor.execute(
                    "INSERT INTO workspaces (name, paths_json) VALUES (?, ?)",
                    ("Default", json.dumps([default_path]))
                )
                conn.commit()

    def is_empty(self) -> bool:
        """Returns True if no workspaces other than 'Default' exist"""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT COUNT(*) FROM workspaces")
            count = cursor.fetchone()[0]
            return count == 0

    def add_workspace(self, name: str, paths: List[str]):
        """Adds or updates a workspace entry with folder paths."""
        clean_name = name.strip()
        clean_paths = [os.path.abspath(p.strip()) for p in paths if p.strip()]
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT paths_json FROM workspaces WHERE name = ?", (clean_name,))
            row = cursor.fetchone()
            if row:
                existing_paths = json.loads(row["paths_json"])
                combined = list(dict.fromkeys(existing_paths + clean_paths))
                cursor.execute("UPDATE workspaces SET paths_json = ? WHERE name = ?", (json.dumps(combined), clean_name))
            else:
                cursor.execute("INSERT INTO workspaces (name, paths_json) VALUES (?, ?)", (clean_name, json.dumps(clean_paths)))
            conn.commit()

    def add_folder_to_workspace(self, workspace_name: str, folder_path: str) -> bool:
        """Adds a new folder path to an existing workspace."""
        clean_ws = workspace_name.strip()
        clean_path = os.path.abspath(folder_path.strip())
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT paths_json FROM workspaces WHERE name = ?", (clean_ws,))
            row = cursor.fetchone()
            if not row:
                return False
            existing_paths = json.loads(row["paths_json"])
            if clean_path not in existing_paths:
                existing_paths.append(clean_path)
                cursor.execute("UPDATE workspaces SET paths_json = ? WHERE name = ?", (json.dumps(existing_paths), clean_ws))
                conn.commit()
            return True

    def remove_folder_from_workspace(self, workspace_name: str, folder_path: str) -> bool:
        """Removes a folder path from an existing workspace."""
        clean_ws = workspace_name.strip()
        clean_path = os.path.abspath(folder_path.strip())
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT paths_json FROM workspaces WHERE name = ?", (clean_ws,))
            row = cursor.fetchone()
            if not row:
                return False
            existing_paths = json.loads(row["paths_json"])
            updated_paths = [p for p in existing_paths if os.path.abspath(p) != clean_path]
            cursor.execute("UPDATE workspaces SET paths_json = ? WHERE name = ?", (json.dumps(updated_paths), clean_ws))
            conn.commit()
            return True

    def remove_workspace(self, workspace_name: str) -> bool:
        """Deletes a workspace entry completely from SQLite."""
        clean_ws = workspace_name.strip()
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("DELETE FROM workspaces WHERE name = ?", (clean_ws,))
            conn.commit()
            return cursor.rowcount > 0




    def get_app_settings(self) -> AppSettings:
        """Reads and constructs AppSettings Pydantic instance from SQLite"""
        with self._get_connection() as conn:
            cursor = conn.cursor()

            cursor.execute("SELECT name, paths_json FROM workspaces")
            ws_rows = cursor.fetchall()
            workspaces = [
                WorkspaceSettings(name=row["name"], paths=json.loads(row["paths_json"]))
                for row in ws_rows
            ]

            cursor.execute("SELECT * FROM models WHERE id = 1")
            m_row = cursor.fetchone()
            if m_row:
                models = ModelSettings(
                    local_embedding_model=m_row["local_embedding_model"],
                    local_openai_embedding_model=m_row["local_openai_embedding_model"],
                    inference_model=m_row["inference_model"],
                    summary_model=m_row["summary_model"],
                    model_provider=m_row["model_provider"],
                    local_base_url=m_row["local_base_url"]
                )
            else:
                models = ModelSettings()

            cursor.execute("SELECT * FROM context_settings WHERE id = 1")
            c_row = cursor.fetchone()
            context = ContextSettings(db_path=c_row["db_path"], collection_name=c_row["collection_name"]) if c_row else ContextSettings()

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
                INSERT OR REPLACE INTO models (id, local_embedding_model, local_openai_embedding_model, inference_model, summary_model, model_provider, local_base_url)
                VALUES (1, ?, ?, ?, ?, ?, ?)
            """, (m.local_embedding_model, m.local_openai_embedding_model, m.inference_model, m.summary_model, m.model_provider, m.local_base_url))

            c = settings.context
            cursor.execute("INSERT OR REPLACE INTO context_settings (id, db_path, collection_name) VALUES (1, ?, ?)", (c.db_path, c.collection_name))

            s = settings.session
            cursor.execute("INSERT OR REPLACE INTO session_settings (id, db_path, collection_name) VALUES (1, ?, ?)", (s.db_path, s.collection_name))

            mem = settings.memory
            cursor.execute("""
                INSERT OR REPLACE INTO memory_settings (id, short_term_buffer_size, rolling_window_messages, meta_summary_threshold, meta_summary_batch_size)
                VALUES (1, ?, ?, ?, ?)
            """, (mem.short_term_buffer_size, mem.rolling_window_messages, mem.meta_summary_threshold, mem.meta_summary_batch_size))

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
            allowed_ws = json.loads(row["allowed_workspaces_json"])

            # Retrieve or generate active Bearer token for user
            tokens = self.get_access_tokens(user_id=user_id)
            if tokens:
                token_str = tokens[0]["token_id"]
            else:
                new_t = self.create_access_token(name=f"Session Token ({row['name']})", role=role, allowed_workspaces=allowed_ws, user_id=user_id)
                token_str = new_t["token_id"]

            return {
                "user_id": user_id,
                "email": row["email"],
                "name": row["name"],
                "role": role,
                "allowed_workspaces": allowed_ws,
                "token_id": token_str
            }

    def create_user(self, name: str, email: str, password: str, role: str = "analyst", allowed_workspaces: Optional[List[str]] = None) -> Dict[str, Any]:
        """Creates a new team user (Admin only operation)."""
        import uuid
        from datetime import datetime

        user_id = f"usr_{uuid.uuid4().hex[:12]}"
        password_h = hash_password(password)
        role_clean = role.lower() if role in ["admin", "analyst", "viewer"] else "analyst"
        allowed_ws = allowed_workspaces if allowed_workspaces is not None else ["Default"]
        created_at = datetime.utcnow().isoformat()

        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT INTO users (user_id, email, name, password_hash, role, allowed_workspaces_json, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
            """, (user_id, email.lower().strip(), name.strip(), password_h, role_clean, json.dumps(allowed_ws), created_at))
            conn.commit()

        return {
            "user_id": user_id,
            "email": email.lower().strip(),
            "name": name.strip(),
            "role": role_clean,
            "allowed_workspaces": allowed_ws,
            "created_at": created_at
        }

    def list_users(self) -> List[Dict[str, Any]]:
        """Lists all configured users."""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT user_id, email, name, role, allowed_workspaces_json, created_at FROM users ORDER BY id DESC")
            rows = cursor.fetchall()
            users = []
            for r in rows:
                users.append({
                    "user_id": r["user_id"],
                    "email": r["email"],
                    "name": r["name"],
                    "role": r["role"],
                    "allowed_workspaces": json.loads(r["allowed_workspaces_json"]),
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
        ws_list = allowed_workspaces if allowed_workspaces is not None else ["*"]
        created_at = datetime.utcnow().isoformat()

        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT INTO access_tokens (token_id, user_id, name, role, allowed_workspaces_json, created_at)
                VALUES (?, ?, ?, ?, ?, ?)
            """, (token_id, user_id, name, role_clean, json.dumps(ws_list), created_at))
            conn.commit()

        return {
            "token_id": token_id,
            "user_id": user_id,
            "name": name,
            "role": role_clean,
            "allowed_workspaces": ws_list,
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
                tokens.append({
                    "token_id": r["token_id"],
                    "user_id": r["user_id"],
                    "name": r["name"],
                    "role": r["role"],
                    "allowed_workspaces": json.loads(r["allowed_workspaces_json"]),
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
            return {
                "token_id": row["token_id"],
                "user_id": row["user_id"],
                "name": row["name"],
                "role": row["role"],
                "allowed_workspaces": json.loads(row["allowed_workspaces_json"]),
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
            if required_workspace not in allowed_workspaces:
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
