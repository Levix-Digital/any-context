"""
Observability Storage Layer.
Provides SQLite persistence for system logs, metrics, and trace spans
with automatic table initialization, thread-safe access, and rolling retention.
"""

import os
import json
import sqlite3
import threading
from typing import List, Optional, Dict, Any
from any_context.config.paths import get_app_data_root
from any_context.observability.schemas import LogEvent, MetricEvent, TraceSpan, LogLevel


class ObservabilityStorage:
    """Thread-safe SQLite storage for system observability."""

    _instance = None
    _lock = threading.Lock()

    def __new__(cls, db_path: Optional[str] = None):
        with cls._lock:
            if cls._instance is None:
                cls._instance = super(ObservabilityStorage, cls).__new__(cls)
                cls._instance._initialized = False
            return cls._instance

    def __init__(self, db_path: Optional[str] = None):
        if getattr(self, "_initialized", False):
            return

        if db_path:
            self.db_path = db_path
        else:
            app_root = get_app_data_root()
            os.makedirs(os.path.join(app_root, "config"), exist_ok=True)
            self.db_path = os.path.join(app_root, "config", "settings.db")

        self._local = threading.local()
        self._init_tables()
        self._initialized = True

    def _get_connection(self) -> sqlite3.Connection:
        if not hasattr(self._local, "conn") or self._local.conn is None:
            self._local.conn = sqlite3.connect(self.db_path, timeout=10.0)
            self._local.conn.execute("PRAGMA journal_mode=WAL;")
        return self._local.conn

    def _init_tables(self):
        """Initializes observability tables if they do not exist."""
        try:
            conn = self._get_connection()
            with conn:
                conn.execute("""
                    CREATE TABLE IF NOT EXISTS system_logs (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        timestamp TEXT NOT NULL,
                        component TEXT NOT NULL,
                        level TEXT NOT NULL,
                        message TEXT NOT NULL,
                        metadata TEXT,
                        traceback TEXT
                    );
                """)
                conn.execute("CREATE INDEX IF NOT EXISTS idx_system_logs_ts ON system_logs(timestamp);")
                conn.execute("CREATE INDEX IF NOT EXISTS idx_system_logs_comp ON system_logs(component);")
                conn.execute("CREATE INDEX IF NOT EXISTS idx_system_logs_lvl ON system_logs(level);")

                conn.execute("""
                    CREATE TABLE IF NOT EXISTS system_metrics (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        timestamp TEXT NOT NULL,
                        metric_name TEXT NOT NULL,
                        value REAL NOT NULL,
                        unit TEXT NOT NULL,
                        tags TEXT
                    );
                """)
                conn.execute("CREATE INDEX IF NOT EXISTS idx_system_metrics_name ON system_metrics(metric_name);")

                conn.execute("""
                    CREATE TABLE IF NOT EXISTS trace_spans (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        span_id TEXT NOT NULL,
                        parent_id TEXT,
                        name TEXT NOT NULL,
                        start_time TEXT NOT NULL,
                        end_time TEXT,
                        duration_ms REAL,
                        metadata TEXT
                    );
                """)
                conn.execute("CREATE INDEX IF NOT EXISTS idx_trace_spans_id ON trace_spans(span_id);")
        except Exception:
            pass

    def insert_log(self, event: LogEvent):
        """Inserts a structured log event into SQLite."""
        try:
            conn = self._get_connection()
            with conn:
                conn.execute(
                    """
                    INSERT INTO system_logs (timestamp, component, level, message, metadata, traceback)
                    VALUES (?, ?, ?, ?, ?, ?)
                    """,
                    (
                        event.timestamp,
                        event.component,
                        event.level.value if isinstance(event.level, LogLevel) else str(event.level),
                        event.message,
                        json.dumps(event.metadata, ensure_ascii=False) if event.metadata else None,
                        event.traceback,
                    ),
                )
        except Exception:
            pass

    def get_recent_logs(
        self,
        limit: int = 50,
        component: Optional[str] = None,
        level: Optional[str] = None
    ) -> List[LogEvent]:
        """Retrieves recent logs sorted chronologically."""
        try:
            conn = self._get_connection()
            query = "SELECT timestamp, component, level, message, metadata, traceback FROM system_logs"
            params: List[Any] = []
            clauses: List[str] = []

            if component:
                clauses.append("component = ?")
                params.append(component)
            if level:
                clauses.append("level = ?")
                params.append(level)

            if clauses:
                query += " WHERE " + " AND ".join(clauses)

            query += " ORDER BY id DESC LIMIT ?"
            params.append(limit)

            cursor = conn.cursor()
            cursor.execute(query, params)
            rows = cursor.fetchall()

            results = []
            for row in reversed(rows):
                meta = {}
                if row[4]:
                    try:
                        meta = json.loads(row[4])
                    except Exception:
                        pass
                results.append(
                    LogEvent(
                        timestamp=row[0],
                        component=row[1],
                        level=LogLevel(row[2]) if row[2] in LogLevel.__members__ else LogLevel.INFO,
                        message=row[3],
                        metadata=meta,
                        traceback=row[5],
                    )
                )
            return results
        except Exception:
            return []

    def insert_metric(self, event: MetricEvent):
        """Inserts a performance metric into SQLite."""
        try:
            conn = self._get_connection()
            with conn:
                conn.execute(
                    """
                    INSERT INTO system_metrics (timestamp, metric_name, value, unit, tags)
                    VALUES (?, ?, ?, ?, ?)
                    """,
                    (
                        event.timestamp,
                        event.metric_name,
                        event.value,
                        event.unit,
                        json.dumps(event.tags, ensure_ascii=False) if event.tags else None,
                    ),
                )
        except Exception:
            pass

    def prune_old_logs(self, max_entries: int = 5000):
        """Keeps the system_logs table capped to prevent unbounded database growth."""
        try:
            conn = self._get_connection()
            with conn:
                conn.execute(
                    """
                    DELETE FROM system_logs WHERE id NOT IN (
                        SELECT id FROM system_logs ORDER BY id DESC LIMIT ?
                    )
                    """,
                    (max_entries,),
                )
        except Exception:
            pass
