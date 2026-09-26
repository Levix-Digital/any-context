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
        let tbl_exists: bool = conn
            .query_row(
                "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name='workspace_folders'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if !tbl_exists {
            return Ok(Vec::new());
        }
        let mut stmt = conn.prepare("SELECT folder_path FROM workspace_folders WHERE workspace_name = ?1")?;
        let rows = stmt.query_map(params![workspace_name], |row| row.get::<_, String>(0))?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
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
        let mut stmt = conn.prepare("SELECT url FROM workspace_web_urls WHERE workspace_name = ?1")?;
        let rows = stmt.query_map(params![workspace_name], |row| row.get::<_, String>(0))?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
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
    }
}
