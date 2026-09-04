"""
Centralized SQLite Database Manager for AnyContext.
Provides thread-safe connections, automatic WAL mode, 10s busy timeout,
and standard path resolution across all application data stores.
"""

import os
import sqlite3
import threading
from typing import Optional
from contextlib import contextmanager
from any_context.config.paths import get_default_config_db_path


class DatabaseManager:
    """
    Centralized, thread-safe SQLite connection and transaction manager.
    Guarantees standard PRAGMAs (WAL, busy_timeout=10000, synchronous=NORMAL, foreign_keys=ON)
    and thread-local connection caching to eliminate 'database is locked' errors.
    """

    _instances = {}
    _lock = threading.Lock()

    def __new__(cls, db_path: Optional[str] = None):
        target_path = os.path.abspath(db_path) if db_path else get_default_config_db_path()
        with cls._lock:
            if target_path not in cls._instances:
                instance = super(DatabaseManager, cls).__new__(cls)
                instance._initialized = False
                instance.db_path = target_path
                cls._instances[target_path] = instance
            return cls._instances[target_path]

    def __init__(self, db_path: Optional[str] = None):
        if getattr(self, "_initialized", False):
            return
        os.makedirs(os.path.dirname(self.db_path), exist_ok=True)
        self._local = threading.local()
        self._initialized = True

    @classmethod
    def get_instance(cls, db_path: Optional[str] = None) -> "DatabaseManager":
        """Returns the singleton DatabaseManager for the specified or default database path."""
        return cls(db_path=db_path)

    def get_connection(self) -> sqlite3.Connection:
        """
        Returns a thread-local SQLite connection configured with WAL mode,
        busy timeout, and row factory.
        """
        conn = getattr(self._local, "conn", None)
        if conn is None:
            parent_dir = os.path.dirname(self.db_path)
            if parent_dir and not os.path.exists(parent_dir):
                os.makedirs(parent_dir, exist_ok=True)
            conn = sqlite3.connect(self.db_path, timeout=30.0)
            conn.row_factory = sqlite3.Row
            try:
                conn.execute("PRAGMA journal_mode=WAL;")
                conn.execute("PRAGMA busy_timeout=30000;")
                conn.execute("PRAGMA synchronous=NORMAL;")
                conn.execute("PRAGMA foreign_keys=ON;")
            except Exception:
                pass
            self._local.conn = conn
        return conn

    @contextmanager
    def connection(self):
        """
        Context manager yielding the thread-local connection and automatically
        committing on clean exit or rolling back on exception.
        """
        conn = self.get_connection()
        try:
            yield conn
            conn.commit()
        except Exception:
            try:
                conn.rollback()
            except Exception:
                pass
            raise

    # Convenient alias for transactional blocks
    transaction = connection

    def close(self):
        """Closes the current thread's connection if open."""
        conn = getattr(self._local, "conn", None)
        if conn is not None:
            try:
                conn.close()
            except Exception:
                pass
            self._local.conn = None

    @classmethod
    def close_all(cls):
        """Closes all cached connections across all managed database paths."""
        with cls._lock:
            for instance in list(cls._instances.values()):
                instance.close()
            cls._instances.clear()
