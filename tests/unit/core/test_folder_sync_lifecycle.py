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
    run_index_folder
)
from any_context.vector_engine.store import LanceDBStore
from tests.e2e_helpers import safe_stdout_write, setup_mock_embeddings_if_needed


class TestFolderSyncLifecycle(unittest.TestCase):
    """
    Unit Test Suite for v0.29.2:
    - Purge-Before-Embed
    - At-Once Deletion on /sync
    - Clean Idempotency on /sync --force (Zero-Ghosts & Zero-Duplicates)
    - Prevention of False Rename Heuristics Across Subdirectories
    - Windows Backslash vs Forward Slash Deletion in LanceDB
    """

    @classmethod
    def setUpClass(cls):
        cls.test_dir = tempfile.mkdtemp(prefix="actx_sync_lifecycle_test_")
        cls.docs_dir = os.path.join(cls.test_dir, "shipment_docs")
        os.makedirs(cls.docs_dir, exist_ok=True)

        cls.db_dir = os.path.join(cls.test_dir, "context_db")
        cls.store = ConfigDBStore()
        cls.orig_settings = cls.store.get_app_settings()
        cls.store.update_context_settings(ContextSettings(db_path=cls.db_dir, collection_name="lifecycle_test_chunks"))
        cls.ws_name = "Test_Shipments_WS"
        cls.store.remove_workspace(cls.ws_name)
        cls.store.add_workspace(cls.ws_name, [cls.docs_dir])

        setup_mock_embeddings_if_needed()

        cls.lance_store = LanceDBStore.get_instance(db_path=os.path.join(cls.db_dir, "lancedb"))

    @classmethod
    def tearDownClass(cls):
        try:
            cls.store.remove_workspace(cls.ws_name)
            if hasattr(cls, "orig_settings") and cls.orig_settings and cls.orig_settings.context:
                cls.store.update_context_settings(cls.orig_settings.context)
            shutil.rmtree(cls.test_dir, ignore_errors=True)
        except Exception:
            pass

    def test_01_initial_indexing(self):
        """TC-LC.1: Create 2 files, index them, and verify both exist in LanceDB without duplicates."""
        f1 = os.path.join(self.docs_dir, "shipment_01.txt")
        f2 = os.path.join(self.docs_dir, "shipment_02.txt")
        with open(f1, "w", encoding="utf-8") as f:
            f.write("Shipment 015-TSO-100: Status Confirmed for September 1st.\nPallet count: 12.")
        with open(f2, "w", encoding="utf-8") as f:
            f.write("Shipment 015-TSO-200: Status Scheduled for September 2nd.\nPallet count: 24.")

        res = run_index_folder(workspace_name=self.ws_name, verbose=False)
        self.assertIn(res.get("status"), ["completed", "updated"])

        tbl = self.lance_store._db.open_table("workspace_chunks")
        records = tbl.search().where(f"workspace = '{self.ws_name}'").limit(100).to_arrow()
        self.assertGreaterEqual(records.num_rows, 2)

        fps = set(records.column("file_path").to_pylist())
        # Both files should be recorded
        self.assertEqual(len(fps), 2)

    def test_02_incremental_deletion_purges_immediately(self):
        """TC-LC.2: Delete shipment_01.txt on disk, run /sync, verify it is immediately gone from LanceDB."""
        f1 = os.path.join(self.docs_dir, "shipment_01.txt")
        f2 = os.path.join(self.docs_dir, "shipment_02.txt")

        # Delete file 1 on disk
        if os.path.exists(f1):
            os.remove(f1)

        diff = check_workspace_changes(self.ws_name)
        self.assertTrue(diff["has_changes"])
        self.assertIn(f1, diff["deleted_files"])

        # Incremental sync
        res = run_index_folder(workspace_name=self.ws_name, force_full=False, verbose=False)
        self.assertIn(res.get("status"), ["completed", "updated"])

        # Verify f1 is purged from LanceDB
        tbl = self.lance_store._db.open_table("workspace_chunks")
        records = tbl.search().where(f"workspace = '{self.ws_name}'").limit(100).to_arrow()
        fps = records.column("file_path").to_pylist()

        # Check that f1 is completely gone
        for fp in fps:
            self.assertNotIn("shipment_01.txt", fp)

        # Check that f2 is still present
        self.assertTrue(any("shipment_02.txt" in fp for fp in fps))

        # Check SQLite stat cache
        cached = self.store.get_workspace_files_cache(self.ws_name)
        self.assertNotIn(f1, cached)
        self.assertIn(f2, cached)

    def test_03_modification_purge_before_embed(self):
        """TC-LC.3: Modify shipment_02.txt, run /sync, verify chunks are replaced without duplicating."""
        f2 = os.path.join(self.docs_dir, "shipment_02.txt")

        # Initial chunk count for f2
        tbl = self.lance_store._db.open_table("workspace_chunks")
        recs_before = tbl.search().where(f"workspace = '{self.ws_name}'").limit(100).to_arrow()
        count_before = recs_before.num_rows

        # Modify f2
        time.sleep(0.05)
        with open(f2, "w", encoding="utf-8") as f:
            f.write("Shipment 015-TSO-200 REVISED: Status Dispatched on September 2nd.\nPallet count: 28.\nExtra notes.")

        diff = check_workspace_changes(self.ws_name)
        self.assertIn(f2, diff["modified_files"])

        res = run_index_folder(workspace_name=self.ws_name, force_full=False, verbose=False)
        self.assertIn(res.get("status"), ["completed", "updated"])

        # Verify chunk count did NOT inflate/duplicate
        recs_after = tbl.search().where(f"workspace = '{self.ws_name}'").limit(100).to_arrow()
        self.assertEqual(recs_after.num_rows, count_before)

    def test_04_force_full_sync_is_clean_and_idempotent(self):
        """TC-LC.4: Run /sync --force multiple times, verify chunk count remains perfectly stable (zero duplicate chunks)."""
        f3 = os.path.join(self.docs_dir, "shipment_03.txt")
        with open(f3, "w", encoding="utf-8") as f:
            f.write("Shipment 015-TSO-300: Arrived September 3rd.")

        # Run force full 1st time
        res1 = run_index_folder(workspace_name=self.ws_name, force_full=True, verbose=False)
        tbl = self.lance_store._db.open_table("workspace_chunks")
        count1 = tbl.search().where(f"workspace = '{self.ws_name}'").limit(100).to_arrow().num_rows

        # Run force full 2nd time
        res2 = run_index_folder(workspace_name=self.ws_name, force_full=True, verbose=False)
        count2 = tbl.search().where(f"workspace = '{self.ws_name}'").limit(100).to_arrow().num_rows

        # Run force full 3rd time
        res3 = run_index_folder(workspace_name=self.ws_name, force_full=True, verbose=False)
        count3 = tbl.search().where(f"workspace = '{self.ws_name}'").limit(100).to_arrow().num_rows

        self.assertEqual(count1, count2)
        self.assertEqual(count2, count3)

    def test_05_unzip_different_subdirectory_not_false_rename(self):
        """TC-LC.5: Delete file in root, add file of same ext in subfolder (unzip scenario) -> not a rename."""
        root_f = os.path.join(self.docs_dir, "consolidated_manifest.csv")
        with open(root_f, "w", encoding="utf-8") as f:
            f.write("id,sku,qty\n100,chair,4")

        run_index_folder(workspace_name=self.ws_name, force_full=False, verbose=False)

        # Delete root file and create extracted file in subfolder with same size
        content = "id,sku,qty\n100,table,4"
        os.remove(root_f)

        sub_dir = os.path.join(self.docs_dir, "extracted_archive")
        os.makedirs(sub_dir, exist_ok=True)
        sub_f = os.path.join(sub_dir, "manifest_extracted.csv")
        with open(sub_f, "w", encoding="utf-8") as f:
            f.write(content)

        diff = check_workspace_changes(self.ws_name)
        self.assertEqual(len(diff["renamed_files"]), 0)
        self.assertIn(root_f, diff["deleted_files"])
        self.assertIn(sub_f, diff["new_files"])

        # Sync and verify
        run_index_folder(workspace_name=self.ws_name, force_full=False, verbose=False)
        tbl = self.lance_store._db.open_table("workspace_chunks")
        fps = tbl.search().where(f"workspace = '{self.ws_name}'").limit(100).to_arrow().column("file_path").to_pylist()

        for fp in fps:
            self.assertNotIn("consolidated_manifest.csv", fp)
        self.assertTrue(any("manifest_extracted.csv" in fp for fp in fps))

    def test_06_windows_backslash_delete_by_file(self):
        """TC-LC.6: Ensure delete_by_file works whether passed Windows backslashes or forward slashes."""
        test_win_path = r"C:\Users\guilh\test\sample_win_doc.pdf"
        record = {
            "id": "test_win_chunk_001",
            "workspace": self.ws_name,
            "file_path": test_win_path,
            "file_name": "sample_win_doc.pdf",
            "text": "Testing Windows backslash deletion",
            "vector": [0.01] * 1536,
            "last_modified": "2026-09-05",
            "content_type": "Local Document",
            "document_summary": "Test Summary",
            "keywords": "test",
            "content_hash": "hash123"
        }
        self.lance_store.upsert_records([record], dim=1536)

        tbl = self.lance_store._db.open_table("workspace_chunks")
        rows_before = tbl.search().where(f"workspace = '{self.ws_name}' AND id = 'test_win_chunk_001'").limit(10).to_arrow().num_rows
        self.assertEqual(rows_before, 1)

        # Delete with Windows backslash path
        self.lance_store.delete_by_file(test_win_path, workspace_name=self.ws_name)

        tbl_after = self.lance_store._db.open_table("workspace_chunks")
        rows_after = tbl_after.search().where(f"workspace = '{self.ws_name}' AND id = 'test_win_chunk_001'").limit(10).to_arrow().num_rows
        self.assertEqual(rows_after, 0)


if __name__ == "__main__":
    unittest.main()
