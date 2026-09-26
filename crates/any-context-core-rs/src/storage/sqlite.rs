//! Native SQLite Database Manager for AnyContext (Rusqlite).
//!
//! Provides thread-safe connections, automatic WAL mode, 30s busy timeout,
//! and standard schema management for workspaces, settings, and file synchronization hashes.

use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceRecord {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileMetadataRecord {
    pub id: String,
    pub workspace: String,
    pub file_path: String,
    pub content_hash: String,
    pub last_modified: String,
    pub size_bytes: i64,
    pub status: String,
    pub updated_at: String,
}

pub struct NativeConfigDb {
    db_path: PathBuf,
    conn: Mutex<Connection>,
}

impl NativeConfigDb {
    /// Opens or creates a SQLite database with standard AnyContext PRAGMAs.
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self> {
        let p = db_path.as_ref().to_path_buf();
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let conn = Connection::open(&p)?;
        Self::apply_pragmas(&conn)?;

        let db = Self {
            db_path: p,
            conn: Mutex::new(conn),
        };
        db.ensure_tables()?;
        db.ensure_default_workspace()?;
        Ok(db)
    }

    /// Opens an in-memory SQLite database (primarily for testing).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::apply_pragmas(&conn)?;
        let db = Self {
            db_path: PathBuf::from(":memory:"),
            conn: Mutex::new(conn),
        };
        db.ensure_tables()?;
        db.ensure_default_workspace()?;
        Ok(db)
    }

    /// Opens the default system configuration database.
    pub fn open_default() -> Result<Self> {
        let p = get_default_settings_db_path();
        Self::open(p)
    }

    /// Checks if a specific column exists in a SQLite table.
    fn check_column(conn: &Connection, table: &str, col: &str) -> bool {
        let mut stmt = match conn.prepare(&format!("PRAGMA table_info({})", table)) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let cols = stmt.query_map([], |row| row.get::<_, String>(1)).ok();
        if let Some(iter) = cols {
            for c in iter.flatten() {
                if c.eq_ignore_ascii_case(col) {
                    return true;
                }
            }
        }
        false
    }

    fn apply_pragmas(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 30000;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )?;
        Ok(())
    }

    /// Creates all standard schema tables if they do not exist.
    pub fn ensure_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS workspaces (
                id TEXT PRIMARY KEY,
                name TEXT UNIQUE NOT NULL,
                description TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS file_metadata (
                id TEXT PRIMARY KEY,
                workspace TEXT NOT NULL,
                file_path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                last_modified TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                status TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(workspace, file_path)
            );

            CREATE TABLE IF NOT EXISTS workspace_folders (
                id TEXT PRIMARY KEY,
                workspace_name TEXT NOT NULL,
                folder_path TEXT NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE(workspace_name, folder_path)
            );

            CREATE TABLE IF NOT EXISTS workspace_web_urls (
                id TEXT PRIMARY KEY,
                workspace_name TEXT NOT NULL,
                url TEXT NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE(workspace_name, url)
            );

            CREATE INDEX IF NOT EXISTS idx_file_metadata_ws ON file_metadata(workspace);
            CREATE INDEX IF NOT EXISTS idx_file_metadata_path ON file_metadata(file_path);",
        )?;
        Ok(())
    }

    pub fn ensure_default_workspace(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let exists: bool = conn
            .query_row(
                "SELECT count(*) > 0 FROM workspaces WHERE name = 'Default'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if exists {
            return Ok(());
        }

        let has_desc = Self::check_column(&conn, "workspaces", "description");
        let now = chrono::Utc::now().to_rfc3339();
        if has_desc {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO workspaces (id, name, description, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    "ws_default",
                    "Default",
                    "Default general-purpose workspace",
                    &now,
                    &now
                ],
            );
        } else {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO workspaces (workspace_id, name, paths_json) VALUES (?1, ?2, ?3)",
                params!["ws_default", "Default", "[]"],
            );
        }
        Ok(())
    }

    // --- Workspaces ---

    pub fn create_workspace(&self, name: &str, description: Option<&str>) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        let id = format!("ws_{}", uuid_simple());
        let now = chrono::Utc::now().to_rfc3339();
        let has_desc = Self::check_column(&conn, "workspaces", "description");
        if has_desc {
            conn.execute(
                "INSERT INTO workspaces (id, name, description, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![&id, name, description, &now, &now],
            )?;
        } else {
            conn.execute(
                "INSERT INTO workspaces (workspace_id, name, paths_json) VALUES (?1, ?2, ?3)",
                params![&id, name, "[]"],
            )?;
        }
        Ok(id)
    }

    pub fn get_workspace(&self, name: &str) -> Result<Option<WorkspaceRecord>> {
        let conn = self.conn.lock().unwrap();
        let has_desc = Self::check_column(&conn, "workspaces", "description");
        if has_desc {
            let mut stmt = conn.prepare(
                "SELECT id, name, description, created_at, updated_at FROM workspaces WHERE name = ?1",
            )?;
            let mut rows = stmt.query(params![name])?;
            if let Some(row) = rows.next()? {
                Ok(Some(WorkspaceRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                }))
            } else {
                Ok(None)
            }
        } else {
            let mut stmt = conn.prepare("SELECT id, name FROM workspaces WHERE name = ?1")?;
            let mut rows = stmt.query(params![name])?;
            if let Some(row) = rows.next()? {
                let id: String = match row.get::<_, String>(0) {
                    Ok(s) => s,
                    Err(_) => {
                        let i: i64 = row.get(0)?;
                        i.to_string()
                    }
                };
                let name: String = row.get(1)?;
                Ok(Some(WorkspaceRecord {
                    id,
                    name,
                    description: None,
                    created_at: String::new(),
                    updated_at: String::new(),
                }))
            } else {
                Ok(None)
            }
        }
    }

    pub fn list_workspace_names(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT name FROM workspaces ORDER BY name ASC")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn list_workspaces(&self) -> Result<Vec<WorkspaceRecord>> {
        let conn = self.conn.lock().unwrap();
        let has_desc = Self::check_column(&conn, "workspaces", "description");
        if has_desc {
            let mut stmt = conn.prepare(
                "SELECT id, name, description, created_at, updated_at FROM workspaces ORDER BY name ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(WorkspaceRecord {
                    id: row.get::<_, String>(0)?,
                    name: row.get::<_, String>(1)?,
                    description: row.get::<_, Option<String>>(2)?,
                    created_at: row.get::<_, String>(3)?,
                    updated_at: row.get::<_, String>(4)?,
                })
            })?;
            let mut list = Vec::new();
            for r in rows {
                list.push(r?);
            }
            Ok(list)
        } else {
            let mut stmt = conn.prepare("SELECT id, name FROM workspaces ORDER BY name ASC")?;
            let rows = stmt.query_map([], |row| {
                let id: String = match row.get::<_, String>(0) {
                    Ok(s) => s,
                    Err(_) => {
                        let i: i64 = row.get(0)?;
                        i.to_string()
                    }
                };
                let name: String = row.get(1)?;
                Ok(WorkspaceRecord {
                    id,
                    name,
                    description: None,
                    created_at: String::new(),
                    updated_at: String::new(),
                })
            })?;
            let mut list = Vec::new();
            for r in rows {
                list.push(r?);
            }
            Ok(list)
        }
    }

    pub fn get_workspace_folders(&self, workspace_name: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut list = Vec::new();

        // 1. Check workspace_folders table if it exists
        let tbl_exists: bool = conn
            .query_row(
                "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name='workspace_folders'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if tbl_exists {
            if let Ok(mut stmt) = conn.prepare("SELECT folder_path FROM workspace_folders WHERE workspace_name = ?1 COLLATE NOCASE") {
                if let Ok(rows) = stmt.query_map(params![workspace_name], |row| row.get::<_, String>(0)) {
                    for r in rows.flatten() {
                        let norm = normalize_path_slashes(&r);
                        if !list.contains(&norm) && !list.contains(&r) {
                            list.push(r);
                        }
                    }
                }
            }
        }

        // 2. Check legacy workspaces.paths_json column (stores JSON array of strings)
        if Self::check_column(&conn, "workspaces", "paths_json") {
            let paths_json_opt: Option<String> = conn
                .query_row(
                    "SELECT paths_json FROM workspaces WHERE name = ?1 COLLATE NOCASE",
                    params![workspace_name],
                    |r| r.get(0),
                )
                .ok()
                .flatten();

            if let Some(json_str) = paths_json_opt {
                if let Ok(paths) = serde_json::from_str::<Vec<String>>(&json_str) {
                    for p in paths {
                        let norm = normalize_path_slashes(&p);
                        if !list.contains(&norm) && !list.contains(&p) {
                            list.push(p);
                        }
                    }
                }
            }
        }

        Ok(list)
    }

    pub fn get_workspace_web_urls(&self, workspace_name: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let tbl_exists: bool = conn
            .query_row(
                "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name='workspace_web_urls'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if !tbl_exists {
            return Ok(Vec::new());
        }
        let mut stmt = conn.prepare("SELECT url FROM workspace_web_urls WHERE workspace_name = ?1 COLLATE NOCASE")?;
        let rows = stmt.query_map(params![workspace_name], |row| row.get::<_, String>(0))?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn add_workspace_folder(&self, workspace_name: &str, folder_path: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let norm_path = normalize_path_slashes(folder_path);
        let id = format!("wf_{}", uuid_simple());
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO workspace_folders (id, workspace_name, folder_path, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(workspace_name, folder_path) DO NOTHING",
            params![&id, workspace_name, &norm_path, &now],
        )?;

        // Also sync with workspaces.paths_json if column exists
        if Self::check_column(&conn, "workspaces", "paths_json") {
            let paths_json_opt: Option<String> = conn
                .query_row(
                    "SELECT paths_json FROM workspaces WHERE name = ?1 COLLATE NOCASE",
                    params![workspace_name],
                    |r| r.get(0),
                )
                .ok()
                .flatten();

            let mut paths: Vec<String> = paths_json_opt
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            if !paths.iter().any(|p| p == folder_path || p == &norm_path) {
                paths.push(folder_path.to_string());
                if let Ok(serialized) = serde_json::to_string(&paths) {
                    let _ = conn.execute(
                        "UPDATE workspaces SET paths_json = ?1 WHERE name = ?2 COLLATE NOCASE",
                        params![&serialized, workspace_name],
                    );
                }
            }
        }

        Ok(())
    }

    pub fn remove_workspace_folder(&self, workspace_name: &str, folder_path: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let norm_path = normalize_path_slashes(folder_path);
        let count = conn.execute(
            "DELETE FROM workspace_folders WHERE workspace_name = ?1 COLLATE NOCASE AND (folder_path = ?2 OR folder_path = ?3)",
            params![workspace_name, &norm_path, folder_path],
        )?;

        let mut paths_removed = false;
        if Self::check_column(&conn, "workspaces", "paths_json") {
            let paths_json_opt: Option<String> = conn
                .query_row(
                    "SELECT paths_json FROM workspaces WHERE name = ?1 COLLATE NOCASE",
                    params![workspace_name],
                    |r| r.get(0),
                )
                .ok()
                .flatten();

            if let Some(json_str) = paths_json_opt {
                if let Ok(mut paths) = serde_json::from_str::<Vec<String>>(&json_str) {
                    let prev_len = paths.len();
                    paths.retain(|p| p != folder_path && normalize_path_slashes(p) != norm_path);
                    if paths.len() < prev_len {
                        paths_removed = true;
                        if let Ok(serialized) = serde_json::to_string(&paths) {
                            let _ = conn.execute(
                                "UPDATE workspaces SET paths_json = ?1 WHERE name = ?2 COLLATE NOCASE",
                                params![&serialized, workspace_name],
                            );
                        }
                    }
                }
            }
        }

        Ok(count > 0 || paths_removed)
    }

    pub fn add_workspace_web_url(&self, workspace_name: &str, url: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let id = format!("wu_{}", uuid_simple());
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO workspace_web_urls (id, workspace_name, url, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(workspace_name, url) DO NOTHING",
            params![&id, workspace_name, url.trim(), &now],
        )?;
        Ok(())
    }

    pub fn remove_workspace_web_url(&self, workspace_name: &str, url: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute(
            "DELETE FROM workspace_web_urls WHERE workspace_name = ?1 AND url = ?2",
            params![workspace_name, url.trim()],
        )?;
        Ok(count > 0)
    }

    pub fn rename_workspace(&self, old_name: &str, new_name: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let count = conn.execute(
            "UPDATE workspaces SET name = ?1, updated_at = ?2 WHERE name = ?3",
            params![new_name, &now, old_name],
        )?;
        if count > 0 {
            let _ = conn.execute(
                "UPDATE file_metadata SET workspace = ?1, updated_at = ?2 WHERE workspace = ?3",
                params![new_name, &now, old_name],
            );
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn delete_workspace(&self, name: &str) -> Result<bool> {
        if name.eq_ignore_ascii_case("default") || name.eq_ignore_ascii_case("global") {
            return Ok(false); // Protected workspaces
        }
        let conn = self.conn.lock().unwrap();
        let count = conn.execute("DELETE FROM workspaces WHERE name = ?1", params![name])?;
        if count > 0 {
            let _ = conn.execute("DELETE FROM file_metadata WHERE workspace = ?1", params![name]);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // --- Settings Key-Value ---

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM app_settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, &now],
        )?;
        Ok(())
    }

    // --- Model Configuration ---

    pub fn get_default_model(&self) -> Result<String> {
        let conn = self.conn.lock().unwrap();

        // 1. Check app_settings for "active_model" or "default_model"
        let val: Option<String> = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key IN ('active_model', 'default_model') ORDER BY key ASC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .ok();
        if let Some(m) = val {
            if !m.trim().is_empty() {
                return Ok(m.trim().to_string());
            }
        }

        // 2. Check models table for inference_model
        let tbl_exists: bool = conn
            .query_row(
                "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name='models'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if tbl_exists {
            let inf_model: Option<String> = conn
                .query_row(
                    "SELECT inference_model FROM models ORDER BY id ASC LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .ok();
            if let Some(m) = inf_model {
                if !m.trim().is_empty() {
                    return Ok(m.trim().to_string());
                }
            }
        }

        // 3. Fallback
        Ok("gpt-4o-mini".to_string())
    }

    pub fn set_default_model(&self, model: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES ('active_model', ?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![model.trim(), &now],
        )?;

        let tbl_exists: bool = conn
            .query_row(
                "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name='models'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if tbl_exists {
            let count = conn.execute(
                "UPDATE models SET inference_model = ?1",
                params![model.trim()],
            )?;
            if count == 0 {
                let _ = conn.execute(
                    "INSERT INTO models (inference_model, summary_model, model_provider) VALUES (?1, ?1, 'openai')",
                    params![model.trim()],
                );
            }
        }
        Ok(())
    }

    pub fn get_workspace_model(&self, workspace_name: &str) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        if Self::check_column(&conn, "workspaces", "model") {
            let ws_model: Option<String> = conn
                .query_row(
                    "SELECT model FROM workspaces WHERE name = ?1 COLLATE NOCASE",
                    params![workspace_name],
                    |r| r.get(0),
                )
                .ok()
                .flatten();
            if let Some(m) = ws_model {
                if !m.trim().is_empty() {
                    return Ok(m.trim().to_string());
                }
            }
        }
        drop(conn);
        self.get_default_model()
    }

    pub fn set_workspace_model(&self, workspace_name: &str, model: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        if Self::check_column(&conn, "workspaces", "model") {
            let _ = conn.execute(
                "UPDATE workspaces SET model = ?1 WHERE name = ?2 COLLATE NOCASE",
                params![model.trim(), workspace_name],
            );
        }
        drop(conn);
        self.set_default_model(model)
    }

    // --- API Key Credential Resolution ---

    pub fn get_api_key(&self, provider: &str) -> Result<Option<String>> {
        let clean = provider.trim().to_lowercase();
        let env_var = match clean.as_str() {
            "openai" => "OPENAI_API_KEY",
            "anthropic" => "ANTHROPIC_API_KEY",
            "gemini" | "google" => "GEMINI_API_KEY",
            "deepseek" => "DEEPSEEK_API_KEY",
            "groq" => "GROQ_API_KEY",
            _ => "",
        };
        if !env_var.is_empty() {
            if let Ok(key) = std::env::var(env_var) {
                if !key.trim().is_empty() {
                    return Ok(Some(key.trim().to_string()));
                }
            }
        }
        if clean == "gemini" || clean == "google" {
            if let Ok(key) = std::env::var("GOOGLE_API_KEY") {
                if !key.trim().is_empty() {
                    return Ok(Some(key.trim().to_string()));
                }
            }
        }

        let conn = self.conn.lock().unwrap();
        let tbl_exists: bool = conn
            .query_row(
                "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name='api_keys'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if tbl_exists {
            let key: Option<String> = conn
                .query_row(
                    "SELECT api_key FROM api_keys WHERE provider = ?1 COLLATE NOCASE",
                    params![&clean],
                    |r| r.get(0),
                )
                .ok();
            if let Some(k) = key {
                if !k.trim().is_empty() {
                    return Ok(Some(k.trim().to_string()));
                }
            }
        }
        Ok(None)
    }

    pub fn get_all_api_keys(&self) -> Result<HashMap<String, String>> {
        let conn = self.conn.lock().unwrap();
        let mut map = HashMap::new();
        let tbl_exists: bool = conn
            .query_row(
                "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name='api_keys'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if tbl_exists {
            if let Ok(mut stmt) = conn.prepare("SELECT provider, api_key FROM api_keys") {
                if let Ok(rows) = stmt.query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                }) {
                    for r in rows.flatten() {
                        map.insert(r.0.to_lowercase(), r.1);
                    }
                }
            }
        }
        Ok(map)
    }

    pub fn set_api_key(&self, provider: &str, api_key: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let clean = provider.trim().to_lowercase();
        conn.execute(
            "INSERT INTO api_keys (provider, api_key) VALUES (?1, ?2)
             ON CONFLICT(provider) DO UPDATE SET api_key = excluded.api_key",
            params![&clean, api_key.trim()],
        )?;
        Ok(())
    }

    // --- File Synchronization Hashes ---

    pub fn get_file_hash(&self, workspace: &str, file_path: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let norm_path = normalize_path_slashes(file_path);
        let mut stmt = conn.prepare(
            "SELECT content_hash FROM file_metadata WHERE workspace = ?1 AND file_path = ?2",
        )?;
        let mut rows = stmt.query(params![workspace, &norm_path])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_file_metadata(
        &self,
        workspace: &str,
        file_path: &str,
        hash: &str,
        last_modified: &str,
        size_bytes: i64,
        status: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let norm_path = normalize_path_slashes(file_path);
        let id = format!("fm_{}", uuid_simple());
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO file_metadata (id, workspace, file_path, content_hash, last_modified, size_bytes, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(workspace, file_path) DO UPDATE SET
                content_hash = excluded.content_hash,
                last_modified = excluded.last_modified,
                size_bytes = excluded.size_bytes,
                status = excluded.status,
                updated_at = excluded.updated_at",
            params![&id, workspace, &norm_path, hash, last_modified, size_bytes, status, &now],
        )?;
        Ok(())
    }

    pub fn get_workspace_file_hashes(&self, workspace: &str) -> Result<HashMap<String, String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT file_path, content_hash FROM file_metadata WHERE workspace = ?1",
        )?;
        let rows = stmt.query_map(params![workspace], |row| {
            let path: String = row.get(0)?;
            let hash: String = row.get(1)?;
            Ok((path, hash))
        })?;

        let mut map = HashMap::new();
        for r in rows {
            let (p, h) = r?;
            map.insert(p, h);
        }
        Ok(map)
    }

    pub fn delete_file_metadata(&self, workspace: &str, file_path: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let norm_path = normalize_path_slashes(file_path);
        let count = conn.execute(
            "DELETE FROM file_metadata WHERE workspace = ?1 AND file_path = ?2",
            params![workspace, &norm_path],
        )?;
        Ok(count > 0)
    }

    pub fn get_db_path(&self) -> &Path {
        &self.db_path
    }
}

pub fn get_default_settings_db_path() -> PathBuf {
    if let Ok(p) = std::env::var("ACTX_SETTINGS_DB") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(&local).join("AnyContext").join("config").join("settings.db");
            if p.exists() {
                return p;
            }
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            let p = PathBuf::from(&appdata).join("AnyContext").join("config").join("settings.db");
            if p.exists() {
                return p;
            }
        }
    }
    if let Some(data_dir) = dirs::data_local_dir() {
        let p = data_dir.join("AnyContext").join("config").join("settings.db");
        if p.exists() {
            return p;
        }
    }
    if let Some(home) = dirs::home_dir() {
        let p = home.join(".anycontext").join("config").join("settings.db");
        if p.exists() {
            return p;
        }
    }
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var("LOCALAPPDATA")
            .unwrap_or_else(|_| "C:\\Users\\Default\\AppData\\Local".to_string());
        PathBuf::from(base).join("AnyContext").join("config").join("settings.db")
    }
    #[cfg(not(target_os = "windows"))]
    {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".local")
            .join("share")
            .join("any-context")
            .join("config")
            .join("settings.db")
    }
}

fn normalize_path_slashes(p: &str) -> String {
    p.replace('\\', "/")
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_open_in_memory_and_default_workspace() {
        let db = NativeConfigDb::open_in_memory().expect("open memory db");
        let ws = db.get_workspace("Default").expect("query Default");
        assert!(ws.is_some());
        let def = ws.unwrap();
        assert_eq!(def.name, "Default");
    }

    #[test]
    fn test_workspace_crud() {
        let db = NativeConfigDb::open_in_memory().expect("open memory db");
        let id = db.create_workspace("Research", Some("AI papers")).expect("create");
        assert!(!id.is_empty());

        let list = db.list_workspaces().expect("list");
        assert!(list.iter().any(|w| w.name == "Research"));

        let renamed = db.rename_workspace("Research", "DeepResearch").expect("rename");
        assert!(renamed);

        let queried = db.get_workspace("DeepResearch").expect("query renamed");
        assert!(queried.is_some());

        let deleted = db.delete_workspace("DeepResearch").expect("delete");
        assert!(deleted);

        let not_found = db.get_workspace("DeepResearch").expect("query after delete");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_settings_key_value() {
        let db = NativeConfigDb::open_in_memory().expect("open memory db");
        db.set_setting("active_model", "gpt-4o").expect("set");
        let val = db.get_setting("active_model").expect("get");
        assert_eq!(val, Some("gpt-4o".to_string()));

        db.set_setting("active_model", "claude-3-5-sonnet").expect("update");
        let val2 = db.get_setting("active_model").expect("get updated");
        assert_eq!(val2, Some("claude-3-5-sonnet".to_string()));
    }

    #[test]
    fn test_file_metadata_hashes() {
        let db = NativeConfigDb::open_in_memory().expect("open memory db");
        db.set_file_metadata(
            "Default",
            "C:\\docs\\report.pdf",
            "hash_abc_123",
            "2026-09-24T12:00:00Z",
            1024,
            "indexed",
        )
        .expect("set file metadata");

        // Forward and backslash normalization test
        let hash1 = db.get_file_hash("Default", "C:/docs/report.pdf").expect("get hash forward");
        assert_eq!(hash1, Some("hash_abc_123".to_string()));

        let hash2 = db.get_file_hash("Default", "C:\\docs\\report.pdf").expect("get hash backward");
        assert_eq!(hash2, Some("hash_abc_123".to_string()));

        let map = db.get_workspace_file_hashes("Default").expect("get map");
        assert_eq!(map.get("C:/docs/report.pdf"), Some(&"hash_abc_123".to_string()));

        let deleted = db.delete_file_metadata("Default", "C:/docs/report.pdf").expect("delete");
        assert!(deleted);
        let map_after = db.get_workspace_file_hashes("Default").expect("get map after");
        assert!(map_after.is_empty());
    }

    #[test]
    fn test_legacy_workspaces_schema_compatibility() {
        let conn = Connection::open_in_memory().expect("open");
        // Create table with legacy schema (from Python era)
        conn.execute_batch(
            "CREATE TABLE workspaces (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workspace_id TEXT,
                name TEXT UNIQUE NOT NULL,
                paths_json TEXT NOT NULL DEFAULT '[]',
                grounding_mode TEXT DEFAULT 'strict',
                web_search_enabled INTEGER DEFAULT 0,
                default_web_engine TEXT DEFAULT 'auto',
                model TEXT DEFAULT 'gpt-4o-mini',
                created_by TEXT DEFAULT 'user'
            );
            INSERT INTO workspaces (workspace_id, name) VALUES ('ws_1', 'Default');
            INSERT INTO workspaces (workspace_id, name) VALUES ('ws_2', 'RustBook');
            INSERT INTO workspaces (workspace_id, name) VALUES ('ws_3', 'JEVModel');",
        )
        .expect("create legacy table");

        let db = NativeConfigDb {
            db_path: PathBuf::from(":memory:"),
            conn: Mutex::new(conn),
        };
        db.ensure_default_workspace().expect("ensure default");

        let names = db.list_workspace_names().expect("list names");
        assert_eq!(
            names,
            vec![
                "Default".to_string(),
                "JEVModel".to_string(),
                "RustBook".to_string()
            ]
        );

        let records = db.list_workspaces().expect("list records");
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].name, "Default");

        // Verify legacy paths_json parsing
        db.conn.lock().unwrap().execute(
            "UPDATE workspaces SET paths_json = ?1 WHERE name = 'RustBook'",
            params![r#"["C:/repos/rust-book", "D:/notes"]"#],
        ).expect("set legacy paths");

        let rust_folders = db.get_workspace_folders("RustBook").expect("get folders");
        assert_eq!(rust_folders, vec!["C:/repos/rust-book".to_string(), "D:/notes".to_string()]);
    }
}
