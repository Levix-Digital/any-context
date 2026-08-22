import os
import shutil
import tempfile
import unittest
from unittest.mock import patch, MagicMock

from any_context.config.app_settings import AppSettings, ContextSettings
from any_context.config.db_store import ConfigDBStore
from any_context.ingestion.unified_sync import run_unified_sync
from any_context.ingestion.web_scheduler import WebSchedulerStore

class TestUnifiedSyncArchitecture(unittest.TestCase):
    """
    Unit & Integration Tests for Unified Synchronization Architecture:
    - /sync (all sources: folders + web + drives)
    - /sync --folder (folder only)
    - /sync --web (web only)
    - /sync --drive (drive only)
    - /sync --all (across all workspaces)
    """

    def setUp(self):
        self.test_dir = tempfile.mkdtemp()
        self.db_path = os.path.join(self.test_dir, "context_db")
        self.sqlite_db = os.path.join(self.test_dir, "config.db")
        self.folder_a = os.path.join(self.test_dir, "docs_a")
        os.makedirs(self.folder_a, exist_ok=True)

        with open(os.path.join(self.folder_a, "sample.md"), "w", encoding="utf-8") as f:
            f.write("# Sample Doc\nSome test content.")

        self.store = ConfigDBStore(db_path=self.sqlite_db)
        ConfigDBStore._instance = self.store

        self.store.add_workspace("TestWS", paths=[self.folder_a])

        self.web_store = WebSchedulerStore(db_path=self.sqlite_db)
        WebSchedulerStore._instance = self.web_store
        self.web_store.add_web_url("TestWS", "https://example.com/docs")

    def tearDown(self):
        shutil.rmtree(self.test_dir, ignore_errors=True)

    @patch("any_context.ingestion.unified_sync.run_index_folder")
    @patch("any_context.ingestion.unified_sync.sync_workspace_web_urls")
    def test_01_unified_sync_all_sources(self, mock_web_sync, mock_folder_sync):
        """Verify /sync triggers both folders and web sync by default."""
        mock_folder_sync.return_value = {"status": "ok"}
        mock_web_sync.return_value = {"status": "ok", "total_urls": 1}

        res = run_unified_sync(
            workspace_name="TestWS",
            sync_folders=True,
            sync_web=True,
            sync_drives=True
        )

        self.assertIn("TestWS", res["workspaces"])
        mock_folder_sync.assert_called_once()
        mock_web_sync.assert_called_once()

    @patch("any_context.ingestion.unified_sync.run_index_folder")
    @patch("any_context.ingestion.unified_sync.sync_workspace_web_urls")
    def test_02_sync_folder_only(self, mock_web_sync, mock_folder_sync):
        """Verify /sync --folder only triggers folder sync."""
        mock_folder_sync.return_value = {"status": "ok"}

        res = run_unified_sync(
            workspace_name="TestWS",
            sync_folders=True,
            sync_web=False,
            sync_drives=False
        )

        mock_folder_sync.assert_called_once()
        mock_web_sync.assert_not_called()

    @patch("any_context.ingestion.unified_sync.run_index_folder")
    @patch("any_context.ingestion.unified_sync.sync_workspace_web_urls")
    def test_03_sync_web_only(self, mock_web_sync, mock_folder_sync):
        """Verify /sync --web only triggers web sync."""
        mock_web_sync.return_value = {"status": "ok", "total_urls": 1}

        res = run_unified_sync(
            workspace_name="TestWS",
            sync_folders=False,
            sync_web=True,
            sync_drives=False
        )

        mock_folder_sync.assert_not_called()
        mock_web_sync.assert_called_once()

    @patch("any_context.ingestion.unified_sync.run_index_folder")
    @patch("any_context.ingestion.unified_sync.sync_workspace_web_urls")
    def test_04_sync_all_workspaces(self, mock_web_sync, mock_folder_sync):
        """Verify /sync --all iterates over all workspaces."""
        self.store.add_workspace("SecondWS", paths=[self.folder_a])
        mock_folder_sync.return_value = {"status": "ok"}
        mock_web_sync.return_value = {"status": "ok"}

        res = run_unified_sync(
            sync_folders=True,
            sync_web=True,
            sync_drives=True,
            is_all=True
        )

        self.assertIn("TestWS", res["workspaces"])
        self.assertIn("SecondWS", res["workspaces"])
        self.assertGreaterEqual(mock_folder_sync.call_count, 2)

if __name__ == "__main__":
    unittest.main()
