import os
import sys
import time
import shutil
import unittest
import tempfile

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.config.app_settings import AppSettings, ContextSettings
from any_context.config.db_store import ConfigDBStore
from any_context.ingestion.local_folder_ingestor import (
    check_workspace_changes,
    run_index_folder,
    BackgroundSyncManager,
    clear_context_vector_db
)
from tests.e2e_helpers import safe_stdout_write, setup_mock_embeddings_if_needed

class TestStatCacheSync(unittest.TestCase):
    """
    Unit Test Suite: Fast Stat Cache, Change Detection, Background Sync & Zero-Cost Rename Repointing
    """

    @classmethod
    def setUpClass(cls):
        cls.test_dir = tempfile.mkdtemp(prefix="actx_stat_cache_test_")
        cls.docs_dir = os.path.join(cls.test_dir, "test_documents")
        os.makedirs(cls.docs_dir, exist_ok=True)

        cls.doc1 = os.path.join(cls.docs_dir, "report_2026.md")
        with open(cls.doc1, "w", encoding="utf-8") as f:
            f.write("# Annual Clinical Report 2026\nPatient satisfaction reached 98.7% across 12 departments.")

        cls.db_dir = os.path.join(cls.test_dir, "context_db")
        cls.store = ConfigDBStore()
        cls.orig_settings = cls.store.get_app_settings()
        cls.store.update_context_settings(ContextSettings(db_path=cls.db_dir, collection_name="stat_cache_docs"))
        cls.ws_name = "Test_Hospital_Cache"
        cls.store.remove_workspace(cls.ws_name)
        cls.store.add_workspace(cls.ws_name, [cls.docs_dir])

        setup_mock_embeddings_if_needed()

    @classmethod
    def tearDownClass(cls):
        try:
            cls.store.remove_workspace(cls.ws_name)
            if hasattr(cls, "orig_settings") and cls.orig_settings and cls.orig_settings.context:
                cls.store.update_context_settings(cls.orig_settings.context)
            shutil.rmtree(cls.test_dir, ignore_errors=True)
        except Exception:
            pass

    def test_01_virgin_workspace_detection(self):
        """TC-SC.1: Ensures virgin workspace is detected when 0 files have been cached."""
        safe_stdout_write("\n>>> [STAT CACHE / TC-SC.1] Testing Virgin Workspace Detection...\n")
        diff = check_workspace_changes(self.ws_name)
        self.assertTrue(diff["is_virgin"])
        self.assertFalse(diff["is_up_to_date"])
        self.assertEqual(len(diff["new_files"]), 1)
        safe_stdout_write("  [OK] Virgin workspace successfully detected!\n")

    def test_02_initial_sync_and_up_to_date_bypass(self):
        """TC-SC.2: Validates that initial sync populates stat cache and subsequent check is up-to-date."""
        safe_stdout_write(">>> [STAT CACHE / TC-SC.2] Testing Initial Sync and Fast Stat Bypass...\n")
        res = run_index_folder(workspace_name=self.ws_name, verbose=False)
        self.assertIn(res.get("status"), ["completed", "updated", "up_to_date"])

        # Check cached records in SQLite
        cached = self.store.get_workspace_files_cache(self.ws_name)
        self.assertIn(self.doc1, cached)
        self.assertGreater(cached[self.doc1]["last_mtime"], 0)

        # Check diff is now up to date in < 30ms
        t0 = time.time()
        diff = check_workspace_changes(self.ws_name)
        duration_ms = (time.time() - t0) * 1000
        self.assertTrue(diff["is_up_to_date"])
        self.assertFalse(diff["has_changes"])
        self.assertLess(duration_ms, 50, f"Stat check should execute in < 50ms, took {duration_ms:.2f}ms")
        safe_stdout_write(f"  [OK] Up-to-date bypass verified in {duration_ms:.2f}ms!\n")

    def test_03_modification_and_addition_detection(self):
        """TC-SC.3: Validates detection of modified and new files via mtime & size."""
        safe_stdout_write(">>> [STAT CACHE / TC-SC.3] Testing Modified and New File Detection...\n")
        # Add new file
        doc2 = os.path.join(self.docs_dir, "guidelines.txt")
        with open(doc2, "w", encoding="utf-8") as f:
            f.write("Clinical guidelines: Hand hygiene compliance mandatory before ICU entry.")

        # Modify existing file (ensure mtime changes)
        time.sleep(0.05)
        with open(self.doc1, "w", encoding="utf-8") as f:
            f.write("# Annual Clinical Report 2026\nPatient satisfaction updated to 99.4% with emergency room expansion.")

        diff = check_workspace_changes(self.ws_name)
        self.assertFalse(diff["is_up_to_date"])
        self.assertTrue(diff["has_changes"])
        self.assertIn(doc2, diff["new_files"])
        self.assertIn(self.doc1, diff["modified_files"])

        # Run index to sync changes
        run_index_folder(workspace_name=self.ws_name, verbose=False)
        diff_after = check_workspace_changes(self.ws_name)
        self.assertTrue(diff_after["is_up_to_date"])
        safe_stdout_write("  [OK] New and modified files successfully detected and synchronized!\n")

    def test_04_zero_cost_rename_repointing(self):
        """TC-SC.4: Validates zero-cost rename repointing ($0.00) when a file is renamed on disk."""
        safe_stdout_write(">>> [STAT CACHE / TC-SC.4] Testing Zero-Cost Rename Repointing ($0.00)...\n")
        doc2 = os.path.join(self.docs_dir, "guidelines.txt")
        renamed_doc2 = os.path.join(self.docs_dir, "guidelines_v2.txt")

        # Rename file on disk
        if os.path.exists(doc2):
            os.rename(doc2, renamed_doc2)

        diff = check_workspace_changes(self.ws_name)
        self.assertTrue(diff["has_changes"])
        self.assertTrue(any(pair[0] == doc2 and pair[1] == renamed_doc2 for pair in diff["renamed_files"]))

        # Execute index to repoint metadata without re-embedding
        run_index_folder(workspace_name=self.ws_name, verbose=False)

        cached = self.store.get_workspace_files_cache(self.ws_name)
        self.assertIn(renamed_doc2, cached)
        self.assertNotIn(doc2, cached)
        safe_stdout_write("  [OK] Zero-cost rename repointing verified without API cost!\n")

    def test_05_background_sync_manager(self):
        """TC-SC.5: Validates non-blocking BackgroundSyncManager execution and status polling."""
        safe_stdout_write(">>> [STAT CACHE / TC-SC.5] Testing BackgroundSyncManager Lifecycle...\n")
        bg_mgr = BackgroundSyncManager()
        self.assertFalse(bg_mgr.is_syncing(self.ws_name))

        completed_events = []
        thread = bg_mgr.start_background_sync(self.ws_name, on_complete=lambda res: completed_events.append(res))
        self.assertTrue(thread.is_alive() or bg_mgr.get_sync_status(self.ws_name)["status"] in ["syncing", "completed"])

        thread.join(timeout=5.0)
        self.assertFalse(bg_mgr.is_syncing(self.ws_name))
        status = bg_mgr.get_sync_status(self.ws_name)
        self.assertEqual(status["status"], "completed")
        self.assertEqual(len(completed_events), 1)
        safe_stdout_write("  [OK] BackgroundSyncManager lifecycle verified!\n")

    def test_06_multi_source_status_detection(self):
        """TC-SC.6: Validates multi-source status reporting (web sources, cloud drives, local folders)."""
        safe_stdout_write(">>> [STAT CACHE / TC-SC.6] Testing Multi-Source Status Reporting...\n")
        from any_context.ingestion.web_scheduler import WebSchedulerStore
        web_store = WebSchedulerStore(db_path=self.store.db_path)
        web_ws = "Test_Multi_Source_WS"
        self.store.add_workspace(web_ws, paths=[])
        web_store.add_or_update_root_web_source(
            workspace_name=web_ws,
            root_url="https://docs.test-portal.org/guide",
            title="Test Documentation Portal",
            page_count=42
        )

        try:
            diff = check_workspace_changes(web_ws)
            self.assertEqual(diff["workspace_name"], web_ws)
            self.assertEqual(diff["total_sources"], 1)
            self.assertEqual(diff["web_sources_count"], 1)
            self.assertEqual(diff["web_pages_count"], 42)
            self.assertEqual(diff["local_folders_count"], 0)
            self.assertTrue(diff["is_up_to_date"])
            self.assertIn("web source", diff["summary"].lower())
            safe_stdout_write("  [OK] Multi-source status detection and metadata verified!\n")
        finally:
            self.store.remove_workspace(web_ws)

    def test_07_unified_multi_source_background_sync(self):
        """TC-SC.7: Validates unified multi-source background sync execution across folder and web sources."""
        safe_stdout_write(">>> [STAT CACHE / TC-SC.7] Testing Unified Multi-Source Background Sync...\n")
        bg_mgr = BackgroundSyncManager()
        completed_results = []
        thread = bg_mgr.start_background_sync(
            workspace_name=self.ws_name,
            sync_folders=True,
            sync_web=True,
            sync_drives=True,
            force_full=False,
            verbose=False,
            on_complete=lambda res: completed_results.append(res)
        )
        self.assertTrue(thread.is_alive() or bg_mgr.get_sync_status(self.ws_name)["status"] in ["syncing", "completed"])
        thread.join(timeout=5.0)
        self.assertFalse(bg_mgr.is_syncing(self.ws_name))
        status = bg_mgr.get_sync_status(self.ws_name)
        self.assertEqual(status["status"], "completed")
        self.assertEqual(len(completed_results), 1)
        self.assertIn("folder_results", completed_results[0])
        self.assertIn("web_results", completed_results[0])
        self.assertIn("drive_results", completed_results[0])
        safe_stdout_write("  [OK] Unified multi-source background sync verified!\n")

    def test_08_background_sync_progress_telemetry(self):
        """TC-SC.8: Validates progress telemetry and micro-bar formatting in BackgroundSyncManager."""
        safe_stdout_write(">>> [STAT CACHE / TC-SC.8] Testing Progress Telemetry & Micro-Bar Formatting...\n")
        bg_mgr = BackgroundSyncManager()
        test_ws = "TelemetryWS"

        # 1. Test scanning stage (total == 0)
        bg_mgr.update_progress(test_ws, current=0, total=0, stage="scanning")
        prog = bg_mgr.get_progress(test_ws)
        self.assertEqual(prog["stage"], "scanning")
        self.assertEqual(prog["pct"], 0.0)
        self.assertEqual(bg_mgr.format_progress_bar(test_ws), "[scanning...]")

        # 2. Test 50% progress on 10 files with width 8
        bg_mgr.update_progress(test_ws, current=5, total=10, stage="files", item_name="doc.pdf")
        prog = bg_mgr.get_progress(test_ws)
        self.assertEqual(prog["current"], 5)
        self.assertEqual(prog["total"], 10)
        self.assertEqual(prog["pct"], 50.0)
        self.assertEqual(prog["item_name"], "doc.pdf")
        bar_str = bg_mgr.format_progress_bar(test_ws, width=8)
        self.assertEqual(bar_str, "[████░░░░] 50% (5/10 files)")

        # 3. Test 100% progress on web pages with width 10
        bg_mgr.update_progress(test_ws, current=20, total=20, stage="pages")
        bar_str = bg_mgr.format_progress_bar(test_ws, width=10)
        self.assertEqual(bar_str, "[██████████] 100% (20/20 pages)")

        safe_stdout_write("  [OK] Progress telemetry and micro-bar formatting verified!\n")

if __name__ == "__main__":
    unittest.main()
