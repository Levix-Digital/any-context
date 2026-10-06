import os
import sys
import unittest
import time
import tempfile

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.config.db_store import ConfigDBStore
from any_context.core.services.sync_service import SyncService


class TestSyncTelemetryStatus(unittest.TestCase):
    """
    Tests SQLite workspace_sync_status telemetry table and SyncService multi-surface integration.
    """

    def setUp(self):
        self.store = ConfigDBStore()
        self.ws = f"TelemetryTestWS_{os.getpid()}_{int(time.time()*1000)}"
        self.store.add_workspace(name=self.ws, paths=[])

    def tearDown(self):
        try:
            with self.store._get_connection() as conn:
                cursor = conn.cursor()
                cursor.execute("DELETE FROM workspace_sync_status WHERE workspace = ?", (self.ws,))
                cursor.execute("DELETE FROM workspaces WHERE name = ?", (self.ws,))
                conn.commit()
        except Exception:
            pass

    def test_sync_status_lifecycle_and_service_reflection(self):
        # 1. Initially no status or idle
        svc = SyncService()
        initial = self.store.get_sync_status(self.ws)
        self.assertIsNone(initial)

        # 2. Update to active syncing
        self.store.update_sync_status(
            workspace_name=self.ws,
            is_syncing=True,
            pid=9999,
            current_item=5,
            total_items=10,
            stage="files",
            item_name="test_doc.md",
            progress_bar="[████░░░░] 50% (5/10 files)"
        )

        active = self.store.get_sync_status(self.ws)
        self.assertIsNotNone(active)
        self.assertTrue(active["is_syncing"])
        self.assertEqual(active["current_item"], 5)
        self.assertEqual(active["total_items"], 10)
        self.assertIn("50%", active["progress_bar"])

        # Service reflects SQLite state
        svc_status = svc.get_sync_status(self.ws)
        self.assertTrue(svc_status["is_syncing"])
        self.assertIn("Syncing", svc_status["status"])
        self.assertIn("50%", svc_status["progress_bar"])

        # 3. Complete synchronization
        self.store.update_sync_status(
            workspace_name=self.ws,
            is_syncing=False,
            pid=9999,
            current_item=10,
            total_items=10,
            stage="completed",
            progress_bar="✔ Up to date"
        )

        completed = self.store.get_sync_status(self.ws)
        self.assertIsNotNone(completed)
        self.assertFalse(completed["is_syncing"])
        self.assertEqual(completed["stage"], "completed")

        svc_status_done = svc.get_sync_status(self.ws)
        self.assertFalse(svc_status_done["is_syncing"])
        self.assertEqual(svc_status_done["status"], "Ready")


if __name__ == "__main__":
    unittest.main()
