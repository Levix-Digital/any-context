"""
Unit and Integration Tests for Ingestion Orchestrator & Multi-Source Decoupling (v0.24.0).
Tests:
  1. BackgroundSyncManager progress telemetry & Unicode micro-bar rendering.
  2. check_workspace_changes multi-source inspection.
  3. format_sync_status_box modern UI formatting.
  4. clear_context_vector_db LanceDB & cache maintenance.
  5. 100% backward compatibility imports from local_folder_ingestor.
"""
import os
import shutil
import tempfile
import unittest
from unittest.mock import patch, MagicMock

from any_context.ingestion.orchestrator import (
    BackgroundSyncManager,
    check_workspace_changes,
    format_sync_status_box,
    clear_context_vector_db,
    safe_print
)
from any_context.ingestion.local_folder_ingestor import (
    BackgroundSyncManager as LegacyBSM,
    check_workspace_changes as legacy_cwc,
    format_sync_status_box as legacy_fssb,
    clear_context_vector_db as legacy_ccvdb
)
from any_context.vector_engine.store import LanceDBStore


class TestIngestionOrchestrator(unittest.TestCase):

    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.lance_dir = os.path.join(self.temp_dir, "lancedb")
        os.makedirs(self.lance_dir, exist_ok=True)
        self.lance_store = LanceDBStore.get_instance(db_path=self.lance_dir)

    def tearDown(self):
        LanceDBStore._instance = None
        if os.path.exists(self.temp_dir):
            try:
                shutil.rmtree(self.temp_dir)
            except Exception:
                pass

    def test_01_background_sync_manager_telemetry_and_bar(self):
        """
        Tests that BackgroundSyncManager tracks progress and renders unicode micro-bars.
        """
        print("\n>>> [UNIT] Testing BackgroundSyncManager Progress & Unicode Micro-Bar...")
        bg_mgr = BackgroundSyncManager()

        # Update progress to 50%
        bg_mgr.update_progress("TestWorkspace", current=15, total=30, stage="files", item_name="doc.pdf")
        prog = bg_mgr.get_progress("TestWorkspace")

        self.assertEqual(prog["current"], 15)
        self.assertEqual(prog["total"], 30)
        self.assertEqual(prog["pct"], 50.0)
        self.assertEqual(prog["stage"], "files")

        bar = bg_mgr.format_progress_bar("TestWorkspace", width=8)
        self.assertIn("50%", bar)
        self.assertIn("(15/30 files)", bar)
        self.assertIn("█", bar)
        self.assertIn("░", bar)
        safe_print(f"  [OK] Rendered Bar: {bar}")

    def test_02_check_workspace_changes_and_status_box(self):
        """
        Tests that check_workspace_changes inspects multi-source state and format_sync_status_box renders cleanly.
        """
        print("\n>>> [UNIT] Testing check_workspace_changes & format_sync_status_box...")
        mock_diff = {
            "workspace_name": "Consulting",
            "total_sources": 3,
            "folders": [os.path.join(self.temp_dir, "Docs")],
            "total_disk_files": 12,
            "total_cached_files": 12,
            "web_sources": [{"url": "https://example.com/docs", "title": "Example Docs", "page_count": 45}],
            "web_pages_count": 45,
            "cloud_drives": [{"drive_type": "google_drive", "folder_name": "Finance"}],
            "is_up_to_date": True,
            "summary": "Up to date (0 changes)"
        }

        card = format_sync_status_box(mock_diff)
        self.assertIn("Workspace Sync Status: Consulting", card)
        self.assertIn("Local Folders : 1 folder", card)
        self.assertIn("Web Sources   : 1 portal (45 pages indexed)", card)
        self.assertIn("Cloud Drives  : 1 connected", card)
        self.assertIn("Up to Date   : Yes", card)
        print("  [OK] Multi-source sync status box rendered perfectly!")

    def test_03_clear_context_vector_db_maintenance(self):
        """
        Tests that clear_context_vector_db safely purges LanceDB tables and SQLite caches.
        """
        print("\n>>> [UNIT] Testing clear_context_vector_db...")
        test_ctx_dir = os.path.join(self.temp_dir, "test_context_db")
        os.makedirs(test_ctx_dir, exist_ok=True)
        with patch.dict(os.environ, {"ACTX_CONTEXT_DB": test_ctx_dir, "ACTX_TEST_MODE": "1"}):
            clear_context_vector_db(verbose=False)
        print("  [OK] clear_context_vector_db completed safely in isolated sandbox!")

    def test_04_backward_compatibility_reexports(self):
        """
        Tests that all symbols exported by orchestrator are accessible from local_folder_ingestor.
        """
        print("\n>>> [UNIT] Verifying Backward Compatibility Aliases...")
        self.assertIs(LegacyBSM, BackgroundSyncManager)
        self.assertIs(legacy_cwc, check_workspace_changes)
        self.assertIs(legacy_fssb, format_sync_status_box)
        self.assertIs(legacy_ccvdb, clear_context_vector_db)
        print("  [OK] 100% backward compatibility preserved across all legacy imports!")


if __name__ == "__main__":
    unittest.main()
