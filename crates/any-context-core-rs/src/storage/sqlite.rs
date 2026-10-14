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
pub struct WorkspaceSourceRecord {
    pub id: String,
    pub workspace: String,
    pub source_type: String,
    pub raw_target: String,
    pub canonical_slug: String,
    pub display_name: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceSyncStatus {
    pub workspace_name: String,
    pub is_syncing: bool,
    pub pid: Option<u32>,
    pub current_item: usize,
    pub total_items: usize,
    pub stage: String,
    pub item_name: Option<String>,
    pub progress_bar: String,
    pub updated_at: String,
    pub error: Option<String>,
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
        let _ = conn.busy_timeout(std::time::Duration::from_secs(30));
        Self::apply_pragmas(&conn)?;

        let db = Self {
            db_path: p,
            conn: Mutex::new(conn),
        };
        db.ensure_tables()?;
        db.ensure_default_workspace()?;
        db.migrate_legacy_paths_json_if_needed()?;
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
        db.migrate_legacy_paths_json_if_needed()?;
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
                grounding_mode TEXT NOT NULL DEFAULT 'strict',
                search_mode TEXT NOT NULL DEFAULT 'auto',
                model TEXT NOT NULL DEFAULT 'gpt-4o-mini',
                web_search_enabled INTEGER NOT NULL DEFAULT 0,
                paths_json TEXT DEFAULT '[]',
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
                folder_id TEXT PRIMARY KEY,
                workspace_name TEXT NOT NULL,
                folder_path TEXT NOT NULL,
                added_by_email TEXT NOT NULL DEFAULT 'user',
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(workspace_name, folder_path)
            );

            CREATE TABLE IF NOT EXISTS workspace_web_urls (
                id TEXT PRIMARY KEY,
                workspace_name TEXT NOT NULL,
                url TEXT NOT NULL,
                title TEXT,
                last_hash TEXT,
                polling_interval_hours INTEGER DEFAULT 24,
                last_scraped_at TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                page_count INTEGER DEFAULT 1,
                root_url TEXT,
                scope TEXT DEFAULT 'domain',
                UNIQUE(workspace_name, url)
            );

            CREATE TABLE IF NOT EXISTS workspace_sync_status (
                workspace_name TEXT PRIMARY KEY,
                is_syncing INTEGER NOT NULL DEFAULT 0,
                pid INTEGER,
                current_item INTEGER NOT NULL DEFAULT 0,
                total_items INTEGER NOT NULL DEFAULT 0,
                stage TEXT NOT NULL DEFAULT 'idle',
                item_name TEXT,
                progress_bar TEXT,
                updated_at TEXT NOT NULL,
                error TEXT
            );

            CREATE TABLE IF NOT EXISTS workspace_files_stat_cache (
                workspace_name TEXT NOT NULL,
                file_path TEXT NOT NULL,
                last_mtime REAL NOT NULL,
                file_size INTEGER NOT NULL,
                doc_id TEXT,
                content_hash TEXT,
                indexed_at TEXT,
                PRIMARY KEY (workspace_name, file_path)
            );
            CREATE INDEX IF NOT EXISTS idx_wfsc_ws ON workspace_files_stat_cache (workspace_name);

            CREATE TABLE IF NOT EXISTS semantic_envelopes (
                content_hash TEXT PRIMARY KEY,
                file_name TEXT,
                file_path TEXT,
                url TEXT,
                summary TEXT,
                keywords_json TEXT,
                created_at TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_envelope_file ON semantic_envelopes(file_path);

            CREATE TABLE IF NOT EXISTS workspace_sync_ledger (
                workspace TEXT PRIMARY KEY,
                last_sync_timestamp TEXT NOT NULL,
                deleted_sources_json TEXT NOT NULL DEFAULT '[]',
                added_sources_json TEXT NOT NULL DEFAULT '[]',
                modified_sources_json TEXT NOT NULL DEFAULT '[]'
            );

            CREATE INDEX IF NOT EXISTS idx_file_metadata_ws ON file_metadata(workspace);
            CREATE INDEX IF NOT EXISTS idx_file_metadata_path ON file_metadata(file_path);

            CREATE TABLE IF NOT EXISTS workspace_sources (
                id TEXT PRIMARY KEY,
                workspace TEXT NOT NULL,
                source_type TEXT NOT NULL,
                raw_target TEXT NOT NULL,
                canonical_slug TEXT NOT NULL,
                display_name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(workspace, raw_target)
            );
            CREATE INDEX IF NOT EXISTS idx_workspace_sources_ws ON workspace_sources(workspace);",
        )?;

        // Ensure columns exist on legacy databases
        if !Self::check_column(&conn, "workspaces", "paths_json") {
            let _ = conn.execute("ALTER TABLE workspaces ADD COLUMN paths_json TEXT DEFAULT '[]'", []);
        }
        if !Self::check_column(&conn, "workspaces", "grounding_mode") {
            let _ = conn.execute("ALTER TABLE workspaces ADD COLUMN grounding_mode TEXT NOT NULL DEFAULT 'strict'", []);
        }
        if !Self::check_column(&conn, "workspaces", "search_mode") {
            let _ = conn.execute("ALTER TABLE workspaces ADD COLUMN search_mode TEXT NOT NULL DEFAULT 'auto'", []);
        }
        if !Self::check_column(&conn, "workspaces", "model") {
            let _ = conn.execute("ALTER TABLE workspaces ADD COLUMN model TEXT NOT NULL DEFAULT 'gpt-4o-mini'", []);
        }
        if !Self::check_column(&conn, "workspaces", "web_search_enabled") {
            let _ = conn.execute("ALTER TABLE workspaces ADD COLUMN web_search_enabled INTEGER NOT NULL DEFAULT 0", []);
        }

        if !Self::check_column(&conn, "workspace_web_urls", "title") {
            let _ = conn.execute("ALTER TABLE workspace_web_urls ADD COLUMN title TEXT", []);
        }
        if !Self::check_column(&conn, "workspace_web_urls", "last_hash") {
            let _ = conn.execute("ALTER TABLE workspace_web_urls ADD COLUMN last_hash TEXT", []);
        }
        if !Self::check_column(&conn, "workspace_web_urls", "polling_interval_hours") {
            let _ = conn.execute("ALTER TABLE workspace_web_urls ADD COLUMN polling_interval_hours INTEGER DEFAULT 24", []);
        }
        if !Self::check_column(&conn, "workspace_web_urls", "last_scraped_at") {
            let _ = conn.execute("ALTER TABLE workspace_web_urls ADD COLUMN last_scraped_at TEXT", []);
        }
        if !Self::check_column(&conn, "workspace_web_urls", "page_count") {
            let _ = conn.execute("ALTER TABLE workspace_web_urls ADD COLUMN page_count INTEGER DEFAULT 1", []);
        }
        if !Self::check_column(&conn, "workspace_web_urls", "root_url") {
            let _ = conn.execute("ALTER TABLE workspace_web_urls ADD COLUMN root_url TEXT", []);
        }
        if !Self::check_column(&conn, "workspace_web_urls", "scope") {
            let _ = conn.execute("ALTER TABLE workspace_web_urls ADD COLUMN scope TEXT DEFAULT 'domain'", []);
        }

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
                "INSERT OR IGNORE INTO workspaces (id, name, description, grounding_mode, model, web_search_enabled, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'strict', 'gpt-4o-mini', 0, ?4, ?5)",
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
                "INSERT OR IGNORE INTO workspaces (workspace_id, name, grounding_mode, model, web_search_enabled) VALUES (?1, ?2, 'strict', 'gpt-4o-mini', 0)",
                params!["ws_default", "Default"],
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
                "INSERT INTO workspaces (id, name, description, grounding_mode, model, web_search_enabled, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'strict', 'gpt-4o-mini', 0, ?4, ?5)",
                params![&id, name, description, &now, &now],
            )?;
        } else {
            conn.execute(
                "INSERT INTO workspaces (workspace_id, name, grounding_mode, model, web_search_enabled) VALUES (?1, ?2, 'strict', 'gpt-4o-mini', 0)",
                params![&id, name],
            )?;
        }
        Ok(id)
    }

    /// Migrates any legacy JSON folder lists in workspaces.paths_json to the normalized workspace_folders table,
    /// then drops the legacy column from SQLite.
    pub fn migrate_legacy_paths_json_if_needed(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Check if legacy column paths_json exists in workspaces table
        if !Self::check_column(&conn, "workspaces", "paths_json") {
            return Ok(());
        }

        // 1. Fetch all workspaces that have non-empty paths_json
        let mut stmt = match conn.prepare(
            "SELECT name, paths_json FROM workspaces WHERE paths_json IS NOT NULL AND paths_json != '[]'",
        ) {
            Ok(s) => s,
            Err(_) => return Ok(()),
        };

        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;

        let mut to_migrate: Vec<(String, Vec<String>)> = Vec::new();
        for r in rows.flatten() {
            let (ws_name, json_str) = r;
            if let Ok(paths) = serde_json::from_str::<Vec<String>>(&json_str) {
                if !paths.is_empty() {
                    to_migrate.push((ws_name, paths));
                }
            }
        }
        drop(stmt);

        // 2. Insert into workspace_folders
        let col_id = if Self::check_column(&conn, "workspace_folders", "folder_id") {
            "folder_id"
        } else {
            "id"
        };
        let has_added_by = Self::check_column(&conn, "workspace_folders", "added_by_email");
        let now = chrono::Utc::now().to_rfc3339();

        for (ws_name, paths) in to_migrate {
            for p in paths {
                let norm = normalize_path_slashes(&p);
                if norm.is_empty() {
                    continue;
                }
                let fid = format!("wf_{}", uuid_simple());
                if has_added_by {
                    let sql = format!(
                        "INSERT INTO workspace_folders ({}, workspace_name, folder_path, added_by_email, created_at)
                         VALUES (?1, ?2, ?3, 'user', ?4)
                         ON CONFLICT DO NOTHING",
                        col_id
                    );
                    let _ = conn.execute(&sql, params![&fid, &ws_name, &norm, &now]);
                } else {
                    let sql = format!(
                        "INSERT INTO workspace_folders ({}, workspace_name, folder_path, created_at)
                         VALUES (?1, ?2, ?3, ?4)
                         ON CONFLICT DO NOTHING",
                        col_id
                    );
                    let _ = conn.execute(&sql, params![&fid, &ws_name, &norm, &now]);
                }
            }
            Self::sync_paths_json(&conn, &ws_name);
        }

        Ok(())
    }

    /// Helper to synchronize workspace_folders into workspaces.paths_json for bidirectional Python parity.
    fn sync_paths_json(conn: &Connection, workspace_name: &str) {
        if !Self::check_column(conn, "workspaces", "paths_json") {
            return;
        }
        let mut stmt = match conn.prepare(
            "SELECT folder_path FROM workspace_folders WHERE workspace_name = ?1 COLLATE NOCASE ORDER BY created_at ASC",
        ) {
            Ok(s) => s,
            Err(_) => return,
        };
        let rows = stmt.query_map(params![workspace_name], |r| r.get::<_, String>(0));
        if let Ok(iter) = rows {
            let paths: Vec<String> = iter.flatten().collect();
            if let Ok(json_str) = serde_json::to_string(&paths) {
                let _ = conn.execute(
                    "UPDATE workspaces SET paths_json = ?1 WHERE name = ?2 COLLATE NOCASE",
                    params![json_str, workspace_name],
                );
            }
        }
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

        let tbl_exists: bool = conn
            .query_row(
                "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name='workspace_folders'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(false);

        if tbl_exists {
            let mut raw_paths = Vec::new();
            if let Ok(mut stmt) = conn.prepare(
                "SELECT folder_path FROM workspace_folders WHERE workspace_name = ?1 COLLATE NOCASE ORDER BY created_at ASC"
            ) {
                if let Ok(rows) = stmt.query_map(params![workspace_name], |row| row.get::<_, String>(0)) {
                    for r in rows.flatten() {
                        raw_paths.push(r);
                    }
                }
            }

            for r in raw_paths {
                let p = std::path::Path::new(&r);
                let effective_path = if !p.exists() {
                    if let Some(healed) = crate::storage::path_healer::try_heal_path(p) {
                        let healed_str = healed.to_string_lossy().replace('\\', "/");
                        if healed_str != r {
                            let _ = conn.execute(
                                "UPDATE workspace_folders SET folder_path = ?1 WHERE workspace_name = ?2 AND folder_path = ?3",
                                params![&healed_str, workspace_name, &r],
                            );
                        }
                        healed_str
                    } else {
                        r
                    }
                } else {
                    r
                };

                let norm = normalize_path_slashes(&effective_path);
                if !list.contains(&norm) && !list.contains(&effective_path) {
                    list.push(effective_path);
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

        let col_id = if Self::check_column(&conn, "workspace_folders", "folder_id") {
            "folder_id"
        } else {
            "id"
        };
        let has_added_by = Self::check_column(&conn, "workspace_folders", "added_by_email");

        if has_added_by {
            let sql = format!(
                "INSERT INTO workspace_folders ({}, workspace_name, folder_path, added_by_email, created_at)
                 VALUES (?1, ?2, ?3, 'user', ?4)
                 ON CONFLICT DO NOTHING",
                col_id
            );
            conn.execute(&sql, params![&id, workspace_name, &norm_path, &now])?;
        } else {
            let sql = format!(
                "INSERT INTO workspace_folders ({}, workspace_name, folder_path, created_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT DO NOTHING",
                col_id
            );
            conn.execute(&sql, params![&id, workspace_name, &norm_path, &now])?;
        }

        let _ = Self::insert_workspace_source_internal(&conn, workspace_name, "folder", &norm_path, None);
        Self::sync_paths_json(&conn, workspace_name);
        Ok(())
    }

    pub fn remove_workspace_folder(&self, workspace_name: &str, folder_path: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let clean_path = folder_path.trim_start_matches(r"\\?\");
        let norm_path = normalize_path_slashes(clean_path);
        let orig_norm = normalize_path_slashes(folder_path);
        let count = conn.execute(
            "DELETE FROM workspace_folders WHERE workspace_name = ?1 COLLATE NOCASE AND (
                folder_path = ?2 OR folder_path = ?3 OR folder_path = ?4 OR folder_path = ?5
            )",
            params![workspace_name, &norm_path, clean_path, folder_path, &orig_norm],
        )?;
        let _ = conn.execute(
            "DELETE FROM workspace_sources WHERE workspace = ?1 COLLATE NOCASE AND source_type = 'folder' AND (
                raw_target = ?2 OR raw_target = ?3 OR raw_target = ?4 OR raw_target = ?5
            )",
            params![workspace_name, &norm_path, clean_path, folder_path, &orig_norm],
        );
        if count > 0 {
            Self::sync_paths_json(&conn, workspace_name);
        }
        Ok(count > 0)
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
        let _ = Self::insert_workspace_source_internal(&conn, workspace_name, "web_url", url.trim(), None);
        Ok(())
    }

    pub fn remove_workspace_web_url(&self, workspace_name: &str, url: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute(
            "DELETE FROM workspace_web_urls WHERE workspace_name = ?1 AND url = ?2",
            params![workspace_name, url.trim()],
        )?;
        let _ = conn.execute(
            "DELETE FROM workspace_sources WHERE workspace = ?1 COLLATE NOCASE AND source_type = 'web_url' AND raw_target = ?2",
            params![workspace_name, url.trim()],
        );
        Ok(count > 0)
    }

    // --- Workspace Sources & Intelligent Aliasing ---

    fn insert_workspace_source_internal(
        conn: &Connection,
        workspace: &str,
        source_type: &str,
        raw_target: &str,
        custom_name: Option<&str>,
    ) -> Result<WorkspaceSourceRecord> {
        let now = chrono::Utc::now().to_rfc3339();
        let display_name = custom_name
            .map(|s| s.to_string())
            .unwrap_or_else(|| generate_intelligent_display_name(source_type, raw_target));
        let slug = generate_canonical_slug(&display_name);

        let existing: Option<(String, String, String)> = conn
            .query_row(
                "SELECT id, canonical_slug, display_name FROM workspace_sources WHERE workspace = ?1 COLLATE NOCASE AND raw_target = ?2",
                params![workspace, raw_target],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .ok();

        if let Some((id, orig_slug, existing_display)) = existing {
            let final_display = if let Some(c) = custom_name {
                conn.execute(
                    "UPDATE workspace_sources SET display_name = ?1, updated_at = ?2 WHERE id = ?3",
                    params![c, &now, &id],
                )?;
                c.to_string()
            } else {
                existing_display
            };
            return Ok(WorkspaceSourceRecord {
                id,
                workspace: workspace.to_string(),
                source_type: source_type.to_string(),
                raw_target: raw_target.to_string(),
                canonical_slug: orig_slug,
                display_name: final_display,
                created_at: now.clone(),
                updated_at: now,
            });
        }

        let id = format!("src_{}", uuid_simple());
        conn.execute(
            "INSERT INTO workspace_sources (id, workspace, source_type, raw_target, canonical_slug, display_name, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &id,
                workspace,
                source_type,
                raw_target,
                &slug,
                &display_name,
                &now,
                &now
            ],
        )?;

        Ok(WorkspaceSourceRecord {
            id,
            workspace: workspace.to_string(),
            source_type: source_type.to_string(),
            raw_target: raw_target.to_string(),
            canonical_slug: slug,
            display_name,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Adds a source with an optional custom display name.
    pub fn add_workspace_source(&self, workspace: &str, source_type: &str, raw_target: &str, custom_name: Option<&str>) -> Result<WorkspaceSourceRecord> {
        let conn = self.conn.lock().unwrap();
        Self::insert_workspace_source_internal(&conn, workspace, source_type, raw_target, custom_name)
    }

    /// Lists all sources registered in the workspace, automatically backfilling any folders/URLs from legacy tables.
    pub fn list_workspace_sources(&self, workspace: &str) -> Result<Vec<WorkspaceSourceRecord>> {
        let conn = self.conn.lock().unwrap();

        // 1. Backfill from workspace_folders if missing
        if let Ok(mut stmt) = conn.prepare("SELECT folder_path FROM workspace_folders WHERE workspace_name = ?1 COLLATE NOCASE") {
            let folder_rows: Vec<String> = stmt.query_map(params![workspace], |r| r.get(0))
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
                .unwrap_or_default();
            for fp in folder_rows {
                let _ = Self::insert_workspace_source_internal(&conn, workspace, "folder", &fp, None);
            }
        }

        // 2. Backfill from workspace_web_urls if missing
        if let Ok(mut stmt) = conn.prepare("SELECT url FROM workspace_web_urls WHERE workspace_name = ?1 COLLATE NOCASE") {
            let url_rows: Vec<String> = stmt.query_map(params![workspace], |r| r.get(0))
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
                .unwrap_or_default();
            for u in url_rows {
                let _ = Self::insert_workspace_source_internal(&conn, workspace, "web_url", &u, None);
            }
        }

        // 3. Query all workspace_sources
        let mut stmt = conn.prepare(
            "SELECT id, workspace, source_type, raw_target, canonical_slug, display_name, created_at, updated_at
             FROM workspace_sources WHERE workspace = ?1 COLLATE NOCASE ORDER BY created_at ASC"
        )?;

        let rows = stmt.query_map(params![workspace], |row| {
            Ok(WorkspaceSourceRecord {
                id: row.get(0)?,
                workspace: row.get(1)?,
                source_type: row.get(2)?,
                raw_target: row.get(3)?,
                canonical_slug: row.get(4)?,
                display_name: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// Renames a source's display_name at O(1) in SQLite without touching vector chunks.
    pub fn rename_workspace_source(&self, workspace: &str, identifier: &str, new_display_name: &str) -> Result<bool> {
        let sources = self.list_workspace_sources(workspace)?;
        let trimmed_id = identifier.trim();
        let trimmed_new = new_display_name.trim();
        if trimmed_new.is_empty() {
            return Ok(false);
        }

        // Match by 1-based index if number
        let target_source_id = if let Ok(idx) = trimmed_id.parse::<usize>() {
            if idx >= 1 && idx <= sources.len() {
                Some(sources[idx - 1].id.clone())
            } else {
                None
            }
        } else {
            None
        };

        let matched_id = target_source_id.or_else(|| {
            sources.iter().find(|s| {
                s.id == trimmed_id
                    || s.display_name.eq_ignore_ascii_case(trimmed_id)
                    || s.canonical_slug.eq_ignore_ascii_case(trimmed_id)
                    || s.raw_target == trimmed_id
                    || normalize_path_slashes(&s.raw_target) == normalize_path_slashes(trimmed_id)
            }).map(|s| s.id.clone())
        });

        if let Some(src_id) = matched_id {
            let conn = self.conn.lock().unwrap();
            let now = chrono::Utc::now().to_rfc3339();
            let count = conn.execute(
                "UPDATE workspace_sources SET display_name = ?1, updated_at = ?2 WHERE id = ?3",
                params![trimmed_new, &now, &src_id],
            )?;
            Ok(count > 0)
        } else {
            Ok(false)
        }
    }

    /// Removes a source by ID, target path/URL, or display name from both workspace_sources and physical tables.
    pub fn remove_workspace_source(&self, workspace: &str, raw_target_or_id: &str) -> Result<bool> {
        let sources = self.list_workspace_sources(workspace)?;
        let trimmed = raw_target_or_id.trim();
        let matched = sources.iter().find(|s| {
            s.id == trimmed
                || s.display_name.eq_ignore_ascii_case(trimmed)
                || s.raw_target == trimmed
                || normalize_path_slashes(&s.raw_target) == normalize_path_slashes(trimmed)
        });

        if let Some(src) = matched {
            let target = src.raw_target.clone();
            let is_web = src.source_type == "web_url";
            let conn = self.conn.lock().unwrap();
            let _ = conn.execute(
                "DELETE FROM workspace_sources WHERE workspace = ?1 COLLATE NOCASE AND raw_target = ?2",
                params![workspace, &target],
            );
            drop(conn);
            if is_web {
                let _ = self.remove_workspace_web_url(workspace, &target);
            } else {
                let _ = self.remove_workspace_folder(workspace, &target);
            }
            Ok(true)
        } else {
            let conn = self.conn.lock().unwrap();
            let count = conn.execute(
                "DELETE FROM workspace_sources WHERE workspace = ?1 COLLATE NOCASE AND (raw_target = ?2 OR id = ?2)",
                params![workspace, trimmed],
            )?;
            drop(conn);
            let _ = self.remove_workspace_folder(workspace, trimmed);
            let _ = self.remove_workspace_web_url(workspace, trimmed);
            Ok(count > 0)
        }
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
            let _ = conn.execute(
                "UPDATE workspace_sources SET workspace = ?1, updated_at = ?2 WHERE workspace = ?3 COLLATE NOCASE",
                params![new_name, &now, old_name],
            );
            let _ = conn.execute(
                "UPDATE workspace_folders SET workspace_name = ?1 WHERE workspace_name = ?2",
                params![new_name, old_name],
            );
            let _ = conn.execute(
                "UPDATE workspace_web_urls SET workspace_name = ?1 WHERE workspace_name = ?2",
                params![new_name, old_name],
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
            let _ = conn.execute("DELETE FROM workspace_sources WHERE workspace = ?1 COLLATE NOCASE", params![name]);
            let _ = conn.execute("DELETE FROM workspace_folders WHERE workspace_name = ?1 COLLATE NOCASE", params![name]);
            let _ = conn.execute("DELETE FROM workspace_web_urls WHERE workspace_name = ?1 COLLATE NOCASE", params![name]);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // --- Synchronization Telemetry ---

    pub fn get_sync_status(&self, workspace: &str) -> Result<Option<WorkspaceSyncStatus>> {
        let conn = self.conn.lock().unwrap();
        let tbl_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='workspace_sync_status'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if !tbl_exists {
            return Ok(None);
        }

        let mut stmt = conn.prepare(
            "SELECT workspace_name, is_syncing, pid, current_item, total_items, stage, item_name, progress_bar, updated_at, error
             FROM workspace_sync_status WHERE workspace_name = ?1 COLLATE NOCASE"
        )?;
        let mut rows = stmt.query(params![workspace])?;
        if let Some(row) = rows.next()? {
            let is_syncing_int: i32 = row.get(1)?;
            let is_syncing = is_syncing_int != 0;
            let pid_opt: Option<u32> = row.get(2)?;
            let current_item: i64 = row.get(3)?;
            let total_items: i64 = row.get(4)?;
            let stage: String = row.get(5)?;
            let item_name: Option<String> = row.get(6)?;
            let progress_bar: Option<String> = row.get(7)?;
            let updated_at: String = row.get(8)?;
            let error: Option<String> = row.get(9)?;

            let is_stale_bar = match &progress_bar {
                Some(b) => b == "[scanning...]" || b == "[crawling...]" || b == "[calculating...]" || b.is_empty(),
                None => true,
            };
            let bar = if is_stale_bar && (total_items > 0 || current_item > 0) {
                Self::format_sync_progress_bar(current_item as usize, total_items as usize, &stage)
            } else {
                progress_bar.unwrap_or_else(|| {
                    Self::format_sync_progress_bar(current_item as usize, total_items as usize, &stage)
                })
            };

            Ok(Some(WorkspaceSyncStatus {
                workspace_name: row.get(0)?,
                is_syncing,
                pid: pid_opt,
                current_item: current_item as usize,
                total_items: total_items as usize,
                stage,
                item_name,
                progress_bar: bar,
                updated_at,
                error,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_sync_status(
        &self,
        workspace: &str,
        is_syncing: bool,
        pid: Option<u32>,
        current_item: usize,
        total_items: usize,
        stage: &str,
        item_name: Option<&str>,
        error: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let bar = Self::format_sync_progress_bar(current_item, total_items, stage);
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO workspace_sync_status (workspace_name, is_syncing, pid, current_item, total_items, stage, item_name, progress_bar, updated_at, error)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(workspace_name) DO UPDATE SET
                is_syncing = excluded.is_syncing,
                pid = excluded.pid,
                current_item = excluded.current_item,
                total_items = excluded.total_items,
                stage = excluded.stage,
                item_name = excluded.item_name,
                progress_bar = excluded.progress_bar,
                updated_at = excluded.updated_at,
                error = excluded.error",
            params![
                workspace,
                if is_syncing { 1 } else { 0 },
                pid,
                current_item as i64,
                total_items as i64,
                stage,
                item_name,
                bar,
                now,
                error
            ],
        )?;
        Ok(())
    }

    pub fn format_sync_progress_bar(current: usize, total: usize, stage: &str) -> String {
        let width = 8;
        if total == 0 {
            if current > 0 {
                let stage_label = if stage == "pages" || stage == "web" || stage == "crawling" {
                    "urls"
                } else if stage == "files" {
                    "files"
                } else {
                    "items"
                };
                return format!("[scanning... {} {} found]", current, stage_label);
            }
            if stage == "cancelled" {
                return "[cancelled]".to_string();
            }
            if stage == "crawling" || stage == "web" || stage == "pages" {
                return "[crawling...]".to_string();
            }
            if stage == "scanning" || stage == "discovering" {
                return "[scanning...]".to_string();
            }
            return "[calculating...]".to_string();
        }
        if stage == "cancelled" {
            return "[cancelled]".to_string();
        }
        let pct = ((current as f32 / total as f32) * 100.0).round() as usize;
        let fill = ((width as f32 * current as f32) / total as f32).round() as usize;
        let fill = fill.min(width);
        let bar = "█".repeat(fill) + &"░".repeat(width - fill);
        let stage_suffix = if !stage.is_empty() && stage != "idle" {
            format!(" {}", stage)
        } else {
            String::new()
        };
        format!("[{}] {}% ({}/{}{})", bar, pct, current, total, stage_suffix)
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
        Ok(())
    }

    pub fn get_workspace_grounding_mode(&self, workspace_name: &str) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        if Self::check_column(&conn, "workspaces", "grounding_mode") {
            let ws_mode: Option<String> = conn
                .query_row(
                    "SELECT grounding_mode FROM workspaces WHERE name = ?1 COLLATE NOCASE",
                    params![workspace_name],
                    |r| r.get(0),
                )
                .ok()
                .flatten();
            if let Some(m) = ws_mode {
                let trimmed = m.trim().to_lowercase();
                if trimmed == "strict" || trimmed == "hybrid" || trimmed == "proactive" {
                    return Ok(trimmed);
                }
            }
        }
        // Strict default for new or unspecified workspaces
        Ok("strict".to_string())
    }

    pub fn set_workspace_grounding_mode(&self, workspace_name: &str, mode: &str) -> Result<()> {
        let clean = mode.trim().to_lowercase();
        let valid_mode = match clean.as_str() {
            "hybrid" => "hybrid",
            "proactive" => "proactive",
            _ => "strict",
        };
        let conn = self.conn.lock().unwrap();
        if Self::check_column(&conn, "workspaces", "grounding_mode") {
            let _ = conn.execute(
                "UPDATE workspaces SET grounding_mode = ?1 WHERE name = ?2 COLLATE NOCASE",
                params![valid_mode, workspace_name],
            );
        }
        Ok(())
    }

    pub fn get_workspace_search_mode(&self, workspace_name: &str) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        if Self::check_column(&conn, "workspaces", "search_mode") {
            let ws_mode: Option<String> = conn
                .query_row(
                    "SELECT search_mode FROM workspaces WHERE name = ?1 COLLATE NOCASE",
                    params![workspace_name],
                    |r| r.get(0),
                )
                .ok()
                .flatten();
            if let Some(m) = ws_mode {
                let trimmed = m.trim().to_lowercase();
                if trimmed == "auto" || trimmed == "fast" || trimmed == "deep" {
                    return Ok(trimmed);
                }
            }
        }
        // Auto default for new or unspecified workspaces
        Ok("auto".to_string())
    }

    pub fn set_workspace_search_mode(&self, workspace_name: &str, mode: &str) -> Result<()> {
        let clean = mode.trim().to_lowercase();
        let valid_mode = match clean.as_str() {
            "fast" => "fast",
            "deep" => "deep",
            _ => "auto",
        };
        let conn = self.conn.lock().unwrap();
        if Self::check_column(&conn, "workspaces", "search_mode") {
            let _ = conn.execute(
                "UPDATE workspaces SET search_mode = ?1 WHERE name = ?2 COLLATE NOCASE",
                params![valid_mode, workspace_name],
            );
        }
        Ok(())
    }

    pub fn get_workspace_web_search(&self, workspace_name: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        if Self::check_column(&conn, "workspaces", "web_search_enabled") {
            let enabled: Option<i64> = conn
                .query_row(
                    "SELECT web_search_enabled FROM workspaces WHERE name = ?1 COLLATE NOCASE",
                    params![workspace_name],
                    |r| r.get(0),
                )
                .ok()
                .flatten();
            if let Some(val) = enabled {
                return Ok(val != 0);
            }
        }
        Ok(false)
    }

    pub fn set_workspace_web_search(&self, workspace_name: &str, enabled: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        if Self::check_column(&conn, "workspaces", "web_search_enabled") {
            let _ = conn.execute(
                "UPDATE workspaces SET web_search_enabled = ?1 WHERE name = ?2 COLLATE NOCASE",
                params![if enabled { 1 } else { 0 }, workspace_name],
            );
        }
        Ok(())
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
            "openrouter" => "OPENROUTER_API_KEY",
            "mistral" => "MISTRAL_API_KEY",
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

    pub fn clear_workspace_file_metadata(&self, workspace: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute(
            "DELETE FROM file_metadata WHERE workspace = ?1 COLLATE NOCASE",
            params![workspace],
        )?;
        Ok(count)
    }

    /// Retrieves full file metadata records for a workspace.
    pub fn get_workspace_files_meta(&self, workspace: &str) -> Result<HashMap<String, FileMetadataRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, workspace, file_path, content_hash, last_modified, size_bytes, status, updated_at
             FROM file_metadata WHERE workspace = ?1 COLLATE NOCASE",
        )?;
        let rows = stmt.query_map(params![workspace], |row| {
            Ok(FileMetadataRecord {
                id: row.get(0)?,
                workspace: row.get(1)?,
                file_path: row.get(2)?,
                content_hash: row.get(3)?,
                last_modified: row.get(4)?,
                size_bytes: row.get(5)?,
                status: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;

        let mut map = HashMap::new();
        for r in rows {
            let rec = r?;
            map.insert(rec.file_path.clone(), rec);
        }
        Ok(map)
    }

    /// Returns a map of normalized file_path -> (last_mtime, file_size) for workspace files,
    /// checking workspace_files_stat_cache first and falling back to file_metadata.
    pub fn get_workspace_files_stat_cache(&self, workspace: &str) -> Result<HashMap<String, (f64, u64)>> {
        let conn = self.conn.lock().unwrap();
        let mut map = HashMap::new();

        let tbl_exists: bool = conn
            .query_row(
                "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name='workspace_files_stat_cache'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(false);

        if tbl_exists {
            let mut stmt = conn.prepare(
                "SELECT file_path, last_mtime, file_size FROM workspace_files_stat_cache WHERE workspace_name = ?1 COLLATE NOCASE"
            )?;
            let rows = stmt.query_map(params![workspace], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?, row.get::<_, i64>(2)?))
            })?;
            for r in rows.flatten() {
                let norm = if cfg!(windows) {
                    r.0.replace('/', "\\")
                } else {
                    normalize_path_slashes(&r.0)
                };
                map.insert(norm, (r.1, r.2.max(0) as u64));
            }
        }

        if map.is_empty() {
            let mut stmt = conn.prepare(
                "SELECT file_path, last_modified, size_bytes FROM file_metadata WHERE workspace = ?1 COLLATE NOCASE"
            )?;
            let rows = stmt.query_map(params![workspace], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
            })?;
            for r in rows.flatten() {
                let norm = if cfg!(windows) {
                    r.0.replace('/', "\\")
                } else {
                    normalize_path_slashes(&r.0)
                };
                let mtime: f64 = r.1.parse().unwrap_or(0.0);
                map.insert(norm, (mtime, r.2.max(0) as u64));
            }
        }

        Ok(map)
    }

    /// Upserts file stat records into both workspace_files_stat_cache and file_metadata.
    pub fn upsert_file_stat_cache(
        &self,
        workspace: &str,
        file_path: &str,
        last_mtime: f64,
        file_size: u64,
        content_hash: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let norm_path_fm = normalize_path_slashes(file_path);
        let orig_path = file_path;
        let now = chrono::Utc::now().to_rfc3339();
        let hash = content_hash.unwrap_or("");
        let doc_id = format!("doc_{}", uuid_simple());

        let tbl_exists: bool = conn
            .query_row(
                "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name='workspace_files_stat_cache'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(false);

        if tbl_exists {
            conn.execute(
                "INSERT INTO workspace_files_stat_cache (workspace_name, file_path, last_mtime, file_size, doc_id, content_hash, indexed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(workspace_name, file_path) DO UPDATE SET
                    last_mtime = excluded.last_mtime,
                    file_size = excluded.file_size,
                    doc_id = excluded.doc_id,
                    content_hash = excluded.content_hash,
                    indexed_at = excluded.indexed_at",
                params![workspace, orig_path, last_mtime, file_size as i64, &doc_id, hash, &now],
            )?;
        }

        let mtime_str = last_mtime.to_string();
        let fm_id = format!("fm_{}", uuid_simple());
        conn.execute(
            "INSERT INTO file_metadata (id, workspace, file_path, content_hash, last_modified, size_bytes, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'synced', ?7)
             ON CONFLICT(workspace, file_path) DO UPDATE SET
                content_hash = excluded.content_hash,
                last_modified = excluded.last_modified,
                size_bytes = excluded.size_bytes,
                status = excluded.status,
                updated_at = excluded.updated_at",
            params![&fm_id, workspace, &norm_path_fm, hash, &mtime_str, file_size as i64, &now],
        )?;

        Ok(())
    }

    /// Removes a file from both workspace_files_stat_cache and file_metadata.
    pub fn delete_file_stat_cache(&self, workspace: &str, file_path: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let norm_path = normalize_path_slashes(file_path);
        let norm_back = file_path.replace('/', "\\");
        let orig = file_path;

        let _ = conn.execute(
            "DELETE FROM workspace_files_stat_cache WHERE workspace_name = ?1 COLLATE NOCASE AND (file_path = ?2 OR file_path = ?3 OR file_path = ?4)",
            params![workspace, &norm_path, orig, &norm_back],
        );

        let count = conn.execute(
            "DELETE FROM file_metadata WHERE workspace = ?1 COLLATE NOCASE AND (file_path = ?2 OR file_path = ?3 OR file_path = ?4)",
            params![workspace, &norm_path, orig, &norm_back],
        )?;

        Ok(count > 0)
    }

    /// Clears all file cache entries for a workspace.
    pub fn clear_workspace_stat_cache(&self, workspace: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            "DELETE FROM workspace_files_stat_cache WHERE workspace_name = ?1 COLLATE NOCASE",
            params![workspace],
        );
        let count = conn.execute(
            "DELETE FROM file_metadata WHERE workspace = ?1 COLLATE NOCASE",
            params![workspace],
        )?;
        Ok(count)
    }

    /// Updates file path on rename/move with zero cost.
    pub fn rename_cached_file(&self, workspace: &str, old_path: &str, new_path: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let old_norm = normalize_path_slashes(old_path);
        let old_back = old_path.replace('/', "\\");
        let new_norm = normalize_path_slashes(new_path);
        let new_target = if cfg!(windows) { new_path.replace('/', "\\") } else { new_norm.clone() };
        let now = chrono::Utc::now().to_rfc3339();

        let _ = conn.execute(
            "UPDATE workspace_files_stat_cache SET file_path = ?1, indexed_at = ?2
             WHERE workspace_name = ?3 COLLATE NOCASE AND (file_path = ?4 OR file_path = ?5 OR file_path = ?6)",
            params![&new_target, &now, workspace, &old_norm, old_path, &old_back],
        );

        let count = conn.execute(
            "UPDATE file_metadata SET file_path = ?1, updated_at = ?2
             WHERE workspace = ?3 COLLATE NOCASE AND (file_path = ?4 OR file_path = ?5 OR file_path = ?6)",
            params![&new_norm, &now, workspace, &old_norm, old_path, &old_back],
        )?;

        Ok(count > 0)
    }

    /// Retrieves cached semantic envelope by SHA-256 content hash in sub-millisecond time.
    pub fn get_semantic_envelope(&self, content_hash: &str) -> Result<Option<crate::models::SemanticEnvelope>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT summary, keywords_json, file_name, file_path, url, created_at FROM semantic_envelopes WHERE content_hash = ?1"
        )?;
        let mut rows = stmt.query(params![content_hash])?;
        if let Some(row) = rows.next()? {
            let summary: String = row.get(0)?;
            let kw_json: Option<String> = row.get(1)?;
            let file_name: String = row.get(2)?;
            let file_path: Option<String> = row.get(3)?;
            let url: Option<String> = row.get(4)?;
            let created_at: String = row.get(5)?;
            let keywords: Vec<String> = kw_json
                .and_then(|j| serde_json::from_str(&j).ok())
                .unwrap_or_default();
            Ok(Some(crate::models::SemanticEnvelope {
                summary,
                keywords,
                content_hash: content_hash.to_string(),
                file_name,
                file_path,
                url,
                created_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Persists or updates semantic envelope in thread-safe SQLite cache.
    pub fn save_semantic_envelope(&self, envelope: &crate::models::SemanticEnvelope) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let kw_json = serde_json::to_string(&envelope.keywords).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT OR REPLACE INTO semantic_envelopes (content_hash, file_name, file_path, url, summary, keywords_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &envelope.content_hash,
                &envelope.file_name,
                &envelope.file_path,
                &envelope.url,
                &envelope.summary,
                &kw_json,
                &envelope.created_at,
            ],
        )?;
        Ok(())
    }

    /// Records the latest sync mutation diff in workspace_sync_ledger.
    pub fn record_sync_ledger(
        &self,
        workspace: &str,
        deleted_sources: &[String],
        added_sources: &[String],
        modified_sources: &[String],
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let del_json = serde_json::to_string(deleted_sources).unwrap_or_else(|_| "[]".to_string());
        let add_json = serde_json::to_string(added_sources).unwrap_or_else(|_| "[]".to_string());
        let mod_json = serde_json::to_string(modified_sources).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT INTO workspace_sync_ledger (workspace, last_sync_timestamp, deleted_sources_json, added_sources_json, modified_sources_json)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(workspace) DO UPDATE SET
                last_sync_timestamp = excluded.last_sync_timestamp,
                deleted_sources_json = excluded.deleted_sources_json,
                added_sources_json = excluded.added_sources_json,
                modified_sources_json = excluded.modified_sources_json",
            params![workspace, &now, &del_json, &add_json, &mod_json],
        )?;
        Ok(())
    }

    /// Retrieves the latest sync ledger for a workspace.
    pub fn get_sync_ledger(
        &self,
        workspace: &str,
    ) -> Result<Option<(String, Vec<String>, Vec<String>, Vec<String>)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT last_sync_timestamp, deleted_sources_json, added_sources_json, modified_sources_json
             FROM workspace_sync_ledger WHERE workspace = ?1 COLLATE NOCASE",
        )?;
        let mut rows = stmt.query(params![workspace])?;
        if let Some(row) = rows.next()? {
            let ts: String = row.get(0)?;
            let del_s: String = row.get(1)?;
            let add_s: String = row.get(2)?;
            let mod_s: String = row.get(3)?;
            let del: Vec<String> = serde_json::from_str(&del_s).unwrap_or_default();
            let add: Vec<String> = serde_json::from_str(&add_s).unwrap_or_default();
            let mdf: Vec<String> = serde_json::from_str(&mod_s).unwrap_or_default();
            Ok(Some((ts, del, add, mdf)))
        } else {
            Ok(None)
        }
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
    if std::env::var("ACTX_TEST_MODE").is_ok() {
        let sandbox = std::env::temp_dir().join("actx_test_sandbox").join("config");
        let _ = std::fs::create_dir_all(&sandbox);
        return sandbox.join("settings.db");
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

pub fn get_default_logs_dir() -> PathBuf {
    if std::env::var("ACTX_TEST_MODE").is_ok() {
        let sandbox = std::env::temp_dir().join("actx_test_sandbox").join("logs");
        let _ = std::fs::create_dir_all(&sandbox);
        return sandbox;
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local).join("AnyContext").join("logs");
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("AnyContext").join("logs");
        }
    }
    if let Some(data_dir) = dirs::data_local_dir() {
        return data_dir.join("AnyContext").join("logs");
    }
    if let Some(home) = dirs::home_dir() {
        return home.join(".local").join("share").join("any-context").join("logs");
    }
    PathBuf::from("logs")
}

pub fn get_default_models_dir() -> PathBuf {
    if let Ok(p) = std::env::var("ACTX_MODELS_DIR") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    if std::env::var("ACTX_TEST_MODE").is_ok() {
        let sandbox = std::env::temp_dir().join("actx_test_sandbox").join("models");
        let _ = std::fs::create_dir_all(&sandbox);
        return sandbox;
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local).join("AnyContext").join("models");
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("AnyContext").join("models");
        }
    }
    if let Some(data_dir) = dirs::data_local_dir() {
        return data_dir.join("AnyContext").join("models");
    }
    if let Some(home) = dirs::home_dir() {
        return home.join(".local").join("share").join("any-context").join("models");
    }
    PathBuf::from("models")
}

pub fn generate_canonical_slug(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else if c == ' ' || c == '_' || c == '-' || c == '/' || c == '.' {
                '-'
            } else {
                ' '
            }
        })
        .collect();
    let parts: Vec<&str> = clean.split('-').filter(|p| !p.trim().is_empty()).collect();
    if parts.is_empty() {
        "source".to_string()
    } else {
        parts.join("-")
    }
}

pub fn generate_intelligent_display_name(source_type: &str, raw_target: &str) -> String {
    let trimmed = raw_target.trim();
    if source_type.eq_ignore_ascii_case("web_url") || trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        let clean = trimmed
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("www.");

        // Check GitHub pattern: github.com/owner/repo...
        if clean.starts_with("github.com/") {
            let parts: Vec<&str> = clean.split('/').collect();
            if parts.len() >= 3 && !parts[2].is_empty() {
                return format!("GitHub: {}", parts[2]);
            } else if parts.len() >= 2 && !parts[1].is_empty() {
                return format!("GitHub: {}", parts[1]);
            }
        }

        // Check docs.rs pattern: docs.rs/crate_name/...
        if clean.starts_with("docs.rs/") {
            let parts: Vec<&str> = clean.split('/').collect();
            if parts.len() >= 2 && !parts[1].is_empty() {
                let crate_name = parts[1];
                let capitalized = format!("{}{}", &crate_name[..1].to_ascii_uppercase(), &crate_name[1..]);
                return format!("{} Docs", capitalized);
            }
        }

        // Check crates.io pattern: crates.io/crates/name
        if clean.starts_with("crates.io/crates/") {
            let parts: Vec<&str> = clean.split('/').collect();
            if parts.len() >= 3 && !parts[2].is_empty() {
                return format!("Crate: {}", parts[2]);
            }
        }

        // General domain + path heuristic
        let parts: Vec<&str> = clean.split('/').filter(|p| !p.is_empty()).collect();
        if parts.is_empty() {
            return "Web Source".to_string();
        }
        let domain = parts[0];
        if parts.len() == 1 {
            let domain_base = domain.split('.').next().unwrap_or(domain);
            let cap = format!("{}{}", &domain_base[..1].to_ascii_uppercase(), &domain_base[1..]);
            return format!("{} Portal", cap);
        } else {
            let domain_base = domain.split('.').next().unwrap_or(domain);
            let last_seg = parts.last().unwrap_or(&domain_base);
            let clean_seg = last_seg.trim_end_matches(".html").trim_end_matches(".htm");
            let cap_domain = format!("{}{}", &domain_base[..1].to_ascii_uppercase(), &domain_base[1..]);
            let cap_seg = format!("{}{}", &clean_seg[..1].to_ascii_uppercase(), &clean_seg[1..]);
            return format!("{}: {}", cap_domain, cap_seg);
        }
    } else {
        // Folder or local path
        let path = Path::new(trimmed);
        if path.is_dir() {
            // Check Cargo.toml
            let cargo_toml = path.join("Cargo.toml");
            if cargo_toml.exists() {
                if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                    for line in content.lines() {
                        let line_trimmed = line.trim();
                        if line_trimmed.starts_with("name") && line_trimmed.contains('=') {
                            if let Some(val) = line_trimmed.split('=').nth(1) {
                                let name = val.trim().trim_matches('"').trim_matches('\'');
                                if !name.is_empty() {
                                    return name.to_string();
                                }
                            }
                        }
                    }
                }
            }
            // Check package.json
            let pkg_json = path.join("package.json");
            if pkg_json.exists() {
                if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                    for line in content.lines() {
                        let line_trimmed = line.trim();
                        if line_trimmed.starts_with("\"name\"") && line_trimmed.contains(':') {
                            if let Some(val) = line_trimmed.split(':').nth(1) {
                                let name = val.trim().trim_matches(',').trim().trim_matches('"');
                                if !name.is_empty() {
                                    return name.to_string();
                                }
                            }
                        }
                    }
                }
            }
            // Check pyproject.toml
            let pyproject = path.join("pyproject.toml");
            if pyproject.exists() {
                if let Ok(content) = std::fs::read_to_string(&pyproject) {
                    for line in content.lines() {
                        let line_trimmed = line.trim();
                        if line_trimmed.starts_with("name") && line_trimmed.contains('=') {
                            if let Some(val) = line_trimmed.split('=').nth(1) {
                                let name = val.trim().trim_matches('"').trim_matches('\'');
                                if !name.is_empty() {
                                    return name.to_string();
                                }
                            }
                        }
                    }
                }
            }
        }

        // Leaf folder fallback
        let norm = trimmed.replace('\\', "/");
        let segments: Vec<&str> = norm.split('/').filter(|s| !s.is_empty() && *s != ".").collect();
        if segments.is_empty() {
            return "Local Folder".to_string();
        }
        let leaf = *segments.last().unwrap();
        if leaf.ends_with(':') {
            return format!("Drive {}", leaf);
        }
        let generic_names = ["src", "dist", "build", "lib", "bin", "docs", "pkg"];
        if generic_names.contains(&leaf.to_ascii_lowercase().as_str()) && segments.len() >= 2 {
            let parent = segments[segments.len() - 2];
            format!("{}/{}", parent, leaf)
        } else {
            leaf.to_string()
        }
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
            INSERT INTO workspaces (workspace_id, name, paths_json) VALUES ('ws_2', 'RustBook', '[\"C:/repos/rust-book\", \"D:/notes\"]');
            INSERT INTO workspaces (workspace_id, name) VALUES ('ws_3', 'JEVModel');",
        )
        .expect("create legacy table");

        let db = NativeConfigDb {
            db_path: PathBuf::from(":memory:"),
            conn: Mutex::new(conn),
        };
        db.ensure_tables().expect("ensure tables");
        db.ensure_default_workspace().expect("ensure default");
        db.migrate_legacy_paths_json_if_needed().expect("migrate");

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

        // Verify that paths were migrated into workspace_folders
        let rust_folders = db.get_workspace_folders("RustBook").expect("get folders");
        assert_eq!(rust_folders, vec!["C:/repos/rust-book".to_string(), "D:/notes".to_string()]);

        // Verify that the column paths_json is preserved for Python compatibility
        let col_exists = NativeConfigDb::check_column(&db.conn.lock().unwrap(), "workspaces", "paths_json");
        assert!(col_exists, "paths_json column must remain present for Python compatibility");
    }

    #[test]
    fn test_intelligent_source_naming_and_rename_crud() {
        let db = NativeConfigDb::open_in_memory().expect("open memory db");

        // 1. Test auto-naming heuristics
        let gh_name = generate_intelligent_display_name("web_url", "https://github.com/Levix-Digital/any-context");
        assert_eq!(gh_name, "GitHub: any-context");

        let tokio_docs = generate_intelligent_display_name("web_url", "https://docs.rs/tokio/latest/tokio/index.html");
        assert_eq!(tokio_docs, "Tokio Docs");

        let crate_name = generate_intelligent_display_name("web_url", "https://crates.io/crates/rusqlite");
        assert_eq!(crate_name, "Crate: rusqlite");

        let folder_name = generate_intelligent_display_name("folder", "C:\\Projects\\finance-engine\\src");
        assert_eq!(folder_name, "finance-engine/src");

        // 2. Add source via add_workspace_source
        let src1 = db.add_workspace_source("Default", "web_url", "https://docs.rs/tokio/latest/tokio/index.html", None).expect("add source");
        assert_eq!(src1.display_name, "Tokio Docs");
        assert_eq!(src1.canonical_slug, "tokio-docs");

        // 3. Add folder via add_workspace_folder, verify auto-mirroring
        db.add_workspace_folder("Default", "C:/Projects/finance-engine").expect("add folder");
        let sources = db.list_workspace_sources("Default").expect("list sources");
        assert_eq!(sources.len(), 2);
        assert!(sources.iter().any(|s| s.raw_target == "C:/Projects/finance-engine" && s.display_name == "finance-engine"));

        // 4. Rename source at O(1) in SQLite
        let renamed = db.rename_workspace_source("Default", "Tokio Docs", "Tokio Async Engine").expect("rename");
        assert!(renamed);

        let updated = db.list_workspace_sources("Default").expect("list sources after rename");
        let tokio_item = updated.iter().find(|s| s.raw_target.contains("docs.rs/tokio")).expect("found tokio");
        assert_eq!(tokio_item.display_name, "Tokio Async Engine");

        // 5. Rename using 1-based index
        let renamed_by_idx = db.rename_workspace_source("Default", "1", "Tokio Runtime Core").expect("rename by idx");
        assert!(renamed_by_idx);
        let updated2 = db.list_workspace_sources("Default").expect("list sources");
        assert_eq!(updated2[0].display_name, "Tokio Runtime Core");

        // 6. Remove source
        let removed = db.remove_workspace_source("Default", "Tokio Runtime Core").expect("remove source");
        assert!(removed);
        let sources_after = db.list_workspace_sources("Default").expect("list sources");
        assert_eq!(sources_after.len(), 1);
    }
}
