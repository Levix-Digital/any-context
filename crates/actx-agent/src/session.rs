//! Native session persistence and conversation history stores
//! Implements both in-memory and SQLite-backed session storage with deterministic rolling windows.

use crate::error::AgentError;
use actx_lm::types::{ChatMessage, Role};
use async_trait::async_trait;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

/// Universal contract for session conversation stores
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Retrieve the ordered history of messages for a session
    async fn get_messages(&self, session_id: &str) -> Result<Vec<ChatMessage>, AgentError>;

    /// Append a single message to the session history
    async fn append_message(&self, session_id: &str, message: ChatMessage) -> Result<(), AgentError>;

    /// Append multiple messages atomically
    async fn append_messages(&self, session_id: &str, messages: &[ChatMessage]) -> Result<(), AgentError>;

    /// Clear all conversation history for a specific session
    async fn clear_session(&self, session_id: &str) -> Result<(), AgentError>;
}

/// Ephemeral in-memory session store (ideal for tests and stateless API invocations)
#[derive(Default, Clone)]
pub struct InMemorySessionStore {
    sessions: Arc<RwLock<HashMap<String, Vec<ChatMessage>>>>,
    max_history: usize,
}

impl InMemorySessionStore {
    pub fn new(max_history: usize) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            max_history: if max_history == 0 { 30 } else { max_history },
        }
    }
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn get_messages(&self, session_id: &str) -> Result<Vec<ChatMessage>, AgentError> {
        let map = self.sessions.read().await;
        Ok(map.get(session_id).cloned().unwrap_or_default())
    }

    async fn append_message(&self, session_id: &str, message: ChatMessage) -> Result<(), AgentError> {
        self.append_messages(session_id, &[message]).await
    }

    async fn append_messages(&self, session_id: &str, messages: &[ChatMessage]) -> Result<(), AgentError> {
        let mut map = self.sessions.write().await;
        let entry = map.entry(session_id.to_string()).or_default();
        entry.extend_from_slice(messages);

        // Enforce rolling window
        if entry.len() > self.max_history {
            let drain_count = entry.len() - self.max_history;
            entry.drain(0..drain_count);
        }
        Ok(())
    }

    async fn clear_session(&self, session_id: &str) -> Result<(), AgentError> {
        let mut map = self.sessions.write().await;
        map.remove(session_id);
        Ok(())
    }
}

/// Resilient SQLite-backed session store with zero zlib compression (100% clean columns)
pub struct SqliteSessionStore {
    conn: Arc<Mutex<Connection>>,
    max_history: usize,
    #[allow(dead_code)]
    db_path: PathBuf,
}

impl SqliteSessionStore {
    pub fn open(path: impl AsRef<Path>, max_history: usize) -> Result<Self, AgentError> {
        let p = path.as_ref().to_path_buf();
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let conn = Connection::open(&p)
            .map_err(|e| AgentError::SessionStoreError(format!("Failed to open SQLite db: {}", e)))?;

        // Initialize schema
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;

            CREATE TABLE IF NOT EXISTS actx_sessions (
                session_id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS actx_session_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                name TEXT,
                tool_call_id TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES actx_sessions(session_id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_actx_messages_sess ON actx_session_messages(session_id, id);
            "#,
        )
        .map_err(|e| AgentError::SessionStoreError(format!("Schema initialization failed: {}", e)))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            max_history: if max_history == 0 { 30 } else { max_history },
            db_path: p,
        })
    }
}

#[async_trait]
impl SessionStore for SqliteSessionStore {
    async fn get_messages(&self, session_id: &str) -> Result<Vec<ChatMessage>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT role, content, name, tool_call_id FROM actx_session_messages WHERE session_id = ? ORDER BY id ASC",
            )
            .map_err(|e| AgentError::SessionStoreError(e.to_string()))?;

        let rows = stmt
            .query_map(params![session_id], |row| {
                let role_str: String = row.get(0)?;
                let content: String = row.get(1)?;
                let name: Option<String> = row.get(2)?;
                let tool_call_id: Option<String> = row.get(3)?;

                let role = match role_str.as_str() {
                    "system" => Role::System,
                    "user" => Role::User,
                    "assistant" => Role::Assistant,
                    "tool" => Role::Tool,
                    _ => Role::User,
                };

                Ok(ChatMessage {
                    role,
                    content,
                    name,
                    tool_call_id,
                    tool_calls: None,
                })
            })
            .map_err(|e| AgentError::SessionStoreError(e.to_string()))?;

        let mut msgs = Vec::new();
        for r in rows {
            msgs.push(r.map_err(|e| AgentError::SessionStoreError(e.to_string()))?);
        }

        // Return latest max_history
        if msgs.len() > self.max_history {
            let offset = msgs.len() - self.max_history;
            Ok(msgs[offset..].to_vec())
        } else {
            Ok(msgs)
        }
    }

    async fn append_message(&self, session_id: &str, message: ChatMessage) -> Result<(), AgentError> {
        self.append_messages(session_id, &[message]).await
    }

    async fn append_messages(&self, session_id: &str, messages: &[ChatMessage]) -> Result<(), AgentError> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| AgentError::SessionStoreError(e.to_string()))?;

        tx.execute(
            r#"INSERT INTO actx_sessions (session_id, created_at, updated_at) 
               VALUES (?, ?, ?)
               ON CONFLICT(session_id) DO UPDATE SET updated_at = excluded.updated_at"#,
            params![session_id, now, now],
        )
        .map_err(|e| AgentError::SessionStoreError(e.to_string()))?;

        for msg in messages {
            tx.execute(
                r#"INSERT INTO actx_session_messages (session_id, role, content, name, tool_call_id, created_at)
                   VALUES (?, ?, ?, ?, ?, ?)"#,
                params![
                    session_id,
                    msg.role.to_string(),
                    msg.content,
                    msg.name,
                    msg.tool_call_id,
                    now
                ],
            )
            .map_err(|e| AgentError::SessionStoreError(e.to_string()))?;
        }

        tx.commit()
            .map_err(|e| AgentError::SessionStoreError(e.to_string()))?;
        Ok(())
    }

    async fn clear_session(&self, session_id: &str) -> Result<(), AgentError> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM actx_sessions WHERE session_id = ?", params![session_id])
            .map_err(|e| AgentError::SessionStoreError(e.to_string()))?;
        conn.execute("DELETE FROM actx_session_messages WHERE session_id = ?", params![session_id])
            .map_err(|e| AgentError::SessionStoreError(e.to_string()))?;
        Ok(())
    }
}
