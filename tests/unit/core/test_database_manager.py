import os
import sys
import unittest
import tempfile
import threading
import concurrent.futures

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.config.database import DatabaseManager
from any_context.config.db_store import ConfigDBStore
from any_context.ingestion.web_scheduler import WebSchedulerStore
from any_context.billing.store import BillingStore
from any_context.observability.storage import ObservabilityStorage


class TestDatabaseManager(unittest.TestCase):
    """
    Unit Test Suite: Validates DatabaseManager connection pooling,
    thread-local caching, WAL pragmas, transaction safety, and high-concurrency resilience.
    """

    def setUp(self):
        self.temp_dir = tempfile.mkdtemp(prefix="actx_db_mgr_test_")
        self.db_path = os.path.join(self.temp_dir, "test_settings.db")
        self.mgr = DatabaseManager.get_instance(self.db_path)

    def tearDown(self):
        DatabaseManager.close_all()

    def test_01_singleton_per_path(self):
        """Validates that DatabaseManager returns singleton instances keyed by normalized path."""
        mgr1 = DatabaseManager.get_instance(self.db_path)
        mgr2 = DatabaseManager.get_instance(self.db_path)
        self.assertIs(mgr1, mgr2)
        self.assertIs(mgr1, self.mgr)

    def test_02_connection_pragmas(self):
        """Validates that connections have WAL mode, busy_timeout, foreign keys, and Row factory."""
        conn = self.mgr.get_connection()
        self.assertIsNotNone(conn)

        # Check WAL mode
        cursor = conn.cursor()
        cursor.execute("PRAGMA journal_mode")
        mode = cursor.fetchone()[0].lower()
        self.assertEqual(mode, "wal")

        # Check foreign keys
        cursor.execute("PRAGMA foreign_keys")
        fk = cursor.fetchone()[0]
        self.assertEqual(fk, 1)

        # Check busy_timeout (>= 5000ms)
        cursor.execute("PRAGMA busy_timeout")
        timeout = cursor.fetchone()[0]
        self.assertGreaterEqual(timeout, 5000)

    def test_03_thread_local_connections(self):
        """Validates that the same thread reuses its connection while different threads get distinct connections."""
        conn_main = self.mgr.get_connection()
        conn_main_repeat = self.mgr.get_connection()
        self.assertIs(conn_main, conn_main_repeat)

        thread_conns = []

        def worker():
            c = self.mgr.get_connection()
            thread_conns.append(c)

        t = threading.Thread(target=worker)
        t.start()
        t.join()

        self.assertEqual(len(thread_conns), 1)
        self.assertIsNot(conn_main, thread_conns[0])

    def test_04_transaction_context_manager_commit_and_rollback(self):
        """Validates atomic transaction commit on normal exit and rollback on exception."""
        with self.mgr.transaction() as conn:
            conn.execute("CREATE TABLE test_tx (id INTEGER PRIMARY KEY, val TEXT)")
            conn.execute("INSERT INTO test_tx (val) VALUES ('committed')")

        # Verify committed
        conn = self.mgr.get_connection()
        row = conn.execute("SELECT val FROM test_tx WHERE id = 1").fetchone()
        self.assertIsNotNone(row)
        self.assertEqual(row["val"], "committed")

        # Verify rollback on exception
        try:
            with self.mgr.transaction() as conn:
                conn.execute("INSERT INTO test_tx (val) VALUES ('rolled_back')")
                raise ValueError("Forced error to test rollback")
        except ValueError:
            pass

        count = conn.execute("SELECT COUNT(*) AS cnt FROM test_tx").fetchone()["cnt"]
        self.assertEqual(count, 1)

    def test_05_concurrent_multi_threaded_reads_and_writes(self):
        """Validates that 10 concurrent threads can write and read simultaneously without 'database is locked' errors."""
        with self.mgr.transaction() as conn:
            conn.execute("CREATE TABLE concurrent_records (id INTEGER PRIMARY KEY AUTOINCREMENT, thread_id INTEGER, value TEXT)")

        num_threads = 10
        records_per_thread = 20
        errors = []

        def worker_task(worker_id):
            try:
                for i in range(records_per_thread):
                    with self.mgr.transaction() as conn:
                        conn.execute(
                            "INSERT INTO concurrent_records (thread_id, value) VALUES (?, ?)",
                            (worker_id, f"msg_{worker_id}_{i}")
                        )
                    # Also perform a read
                    conn_read = self.mgr.get_connection()
                    rows = conn_read.execute(
                        "SELECT COUNT(*) AS cnt FROM concurrent_records WHERE thread_id = ?",
                        (worker_id,)
                    ).fetchone()
                    assert rows["cnt"] > 0
            except Exception as e:
                errors.append(f"Thread {worker_id} error: {e}")

        with concurrent.futures.ThreadPoolExecutor(max_workers=num_threads) as executor:
            futures = [executor.submit(worker_task, tid) for tid in range(num_threads)]
            concurrent.futures.wait(futures)

        self.assertEqual(len(errors), 0, f"Concurrency errors occurred: {errors}")

        conn = self.mgr.get_connection()
        total_count = conn.execute("SELECT COUNT(*) AS cnt FROM concurrent_records").fetchone()["cnt"]
        self.assertEqual(total_count, num_threads * records_per_thread)

    def test_06_unified_stores_coexistence(self):
        """Validates that ConfigDBStore, WebSchedulerStore, BillingStore, and ObservabilityStorage share the pool safely."""
        config_store = ConfigDBStore(db_path=self.db_path)
        web_store = WebSchedulerStore(db_path=self.db_path)
        billing_store = BillingStore(db_path=self.db_path)
        obs_store = ObservabilityStorage(db_path=self.db_path)

        # 1. Config store operation
        config_store.add_workspace("CoexistenceWS", paths=[])
        settings = config_store.get_app_settings()
        ws_names = [w.name for w in settings.workspaces]
        self.assertIn("CoexistenceWS", ws_names)

        # 2. Web store operation
        web_store.add_or_update_root_web_source(
            workspace_name="CoexistenceWS",
            root_url="https://example.com",
            title="Example Docs",
            page_count=5
        )
        sources = web_store.get_workspace_web_urls("CoexistenceWS")
        self.assertEqual(len(sources), 1)

        # 3. Billing store operation
        sub = billing_store.get_subscription_status()
        self.assertIsNotNone(sub)

        # 4. Observability store operation
        from any_context.observability.schemas import LogEvent, LogLevel
        obs_store.insert_log(LogEvent(
            timestamp="2026-09-03T12:00:00Z",
            component="test",
            level=LogLevel.INFO,
            message="test coexistence"
        ))
        logs = obs_store.get_recent_logs(limit=5)
        self.assertGreaterEqual(len(logs), 1)


if __name__ == "__main__":
    unittest.main()
