import os
import sys
import unittest
import tempfile
import shutil

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.config.db_store import ConfigDBStore
from any_context.vector_engine.store import LanceDBStore
from any_context.core.services.source_service import SourceService
from any_context.commands.dispatcher import dispatch_command
from tests.e2e_helpers import safe_stdout_write, setup_mock_embeddings_if_needed


class TestLanceDBInventory(unittest.TestCase):
    """
    Unit Test Suite: LanceDB as Single Source of Truth for Workspace Inventory & Silent Switch.
    """

    @classmethod
    def setUpClass(cls):
        setup_mock_embeddings_if_needed()
        cls.temp_dir = tempfile.mkdtemp()
        cls.db_path = os.path.join(cls.temp_dir, "test_settings.db")
        cls.lance_dir = os.path.join(cls.temp_dir, "test_lancedb")
        os.makedirs(cls.lance_dir, exist_ok=True)

        cls._orig_db = os.environ.get("ACTX_SETTINGS_DB")
        cls._orig_test_mode = os.environ.get("ACTX_TEST_MODE")
        cls._orig_ctx = os.environ.get("ACTX_CONTEXT_DB")

        os.environ["ACTX_SETTINGS_DB"] = cls.db_path
        os.environ["ACTX_TEST_MODE"] = "1"
        os.environ["ACTX_CONTEXT_DB"] = cls.lance_dir

        cls.store = ConfigDBStore(db_path=cls.db_path)
        cls.lance_store = LanceDBStore.get_instance(db_path=cls.lance_dir)

    @classmethod
    def tearDownClass(cls):
        if cls._orig_db is not None:
            os.environ["ACTX_SETTINGS_DB"] = cls._orig_db
        else:
            os.environ.pop("ACTX_SETTINGS_DB", None)
        if cls._orig_test_mode is not None:
            os.environ["ACTX_TEST_MODE"] = cls._orig_test_mode
        else:
            os.environ.pop("ACTX_TEST_MODE", None)
        if cls._orig_ctx is not None:
            os.environ["ACTX_CONTEXT_DB"] = cls._orig_ctx
        else:
            os.environ.pop("ACTX_CONTEXT_DB", None)

        try:
            shutil.rmtree(cls.temp_dir, ignore_errors=True)
        except Exception:
            pass

    def test_01_lancedb_web_inventory_multi_content_types(self):
        """Validates that LanceDBStore counts all web pages regardless of content_type classification."""
        safe_stdout_write("\n>>> [UNIT] Testing LanceDB Web Inventory with Multi Content Types...\n")
        ws = "Test_Web_Inventory_WS"

        # Mock vector of dimension 1536
        mock_vec = [0.0] * 1536
        records = [
            {
                "id": "c1",
                "vector": mock_vec,
                "text": "Canada immigration home",
                "file_name": "[Web] Immigration Canada",
                "file_path": "https://www.canada.ca/en/immigration.html",
                "workspace": ws,
                "last_modified": "2026-09-05",
                "content_type": "Canonical Service / Documentation",
                "document_summary": "Summary",
                "keywords": "canada",
                "content_hash": "h1"
            },
            {
                "id": "c2",
                "vector": mock_vec,
                "text": "Canada news 2026",
                "file_name": "[Web] Press Release",
                "file_path": "https://www.canada.ca/en/news/2026.html",
                "workspace": ws,
                "last_modified": "2026-09-05",
                "content_type": "Historical News / Press Release",
                "document_summary": "News",
                "keywords": "news",
                "content_hash": "h2"
            },
            {
                "id": "c3",
                "vector": mock_vec,
                "text": "Canada guide",
                "file_name": "[Web] Guide",
                "file_path": "https://www.canada.ca/en/guide.html",
                "workspace": ws,
                "last_modified": "2026-09-05",
                "content_type": "Web Documentation",
                "document_summary": "Guide",
                "keywords": "guide",
                "content_hash": "h3"
            },
            {
                "id": "c4",
                "vector": mock_vec,
                "text": "Canada guide part 2",
                "file_name": "[Web] Guide Part 2",
                "file_path": "https://www.canada.ca/en/guide.html",
                "workspace": ws,
                "last_modified": "2026-09-05",
                "content_type": "Web Documentation",
                "document_summary": "Guide 2",
                "keywords": "guide",
                "content_hash": "h3"
            }
        ]
        self.lance_store.upsert_records(records)

        pages_map = self.lance_store.get_indexed_pages_map(ws, domain_or_prefix="canada.ca")
        self.assertEqual(len(pages_map), 3, "Expected exactly 3 distinct web URLs")
        self.assertIn("https://www.canada.ca/en/immigration.html", pages_map)
        self.assertIn("https://www.canada.ca/en/news/2026.html", pages_map)
        self.assertIn("https://www.canada.ca/en/guide.html", pages_map)

        cnt = self.lance_store.get_indexed_pages_count(ws, domain_or_prefix="canada.ca")
        self.assertEqual(cnt, 3)
        safe_stdout_write("  [OK] LanceDB web inventory correctly recognized all rich content types!\n")

    def test_02_lancedb_local_folder_inventory(self):
        """Validates that LanceDBStore computes distinct file counts by local folder prefix."""
        safe_stdout_write("\n>>> [UNIT] Testing LanceDB Local Folder File Counting...\n")
        ws = "Test_Folder_Inventory_WS"
        mock_vec = [0.0] * 1536

        f1 = os.path.abspath(os.path.join(self.temp_dir, "folder_a", "doc1.txt"))
        f2 = os.path.abspath(os.path.join(self.temp_dir, "folder_a", "doc2.txt"))
        f3 = os.path.abspath(os.path.join(self.temp_dir, "folder_b", "doc3.txt"))

        records = [
            {"id": "lf1", "vector": mock_vec, "text": "Doc 1", "file_name": "doc1.txt", "file_path": f1, "workspace": ws, "last_modified": "2026-09-05", "content_type": "Local Document", "document_summary": "", "keywords": "", "content_hash": "d1"},
            {"id": "lf1_chunk2", "vector": mock_vec, "text": "Doc 1 Part 2", "file_name": "doc1.txt", "file_path": f1, "workspace": ws, "last_modified": "2026-09-05", "content_type": "Local Document", "document_summary": "", "keywords": "", "content_hash": "d1"},
            {"id": "lf2", "vector": mock_vec, "text": "Doc 2", "file_name": "doc2.txt", "file_path": f2, "workspace": ws, "last_modified": "2026-09-05", "content_type": "Local Document", "document_summary": "", "keywords": "", "content_hash": "d2"},
            {"id": "lf3", "vector": mock_vec, "text": "Doc 3", "file_name": "doc3.txt", "file_path": f3, "workspace": ws, "last_modified": "2026-09-05", "content_type": "Local Document", "document_summary": "", "keywords": "", "content_hash": "d3"},
        ]
        self.lance_store.upsert_records(records)

        fa_cnt = self.lance_store.get_indexed_folder_files_count(ws, folder_path=os.path.join(self.temp_dir, "folder_a"))
        fb_cnt = self.lance_store.get_indexed_folder_files_count(ws, folder_path=os.path.join(self.temp_dir, "folder_b"))
        all_cnt = self.lance_store.get_indexed_folder_files_count(ws)

        self.assertEqual(fa_cnt, 2, "folder_a must have 2 distinct files")
        self.assertEqual(fb_cnt, 1, "folder_b must have 1 distinct file")
        self.assertEqual(all_cnt, 3, "Total local files must be 3")

        summary = self.lance_store.get_workspace_inventory_summary(ws)
        self.assertEqual(summary["total_files"], 3)
        self.assertEqual(summary["total_chunks"], 4)
        self.assertEqual(summary["total_web_pages"], 0)
        safe_stdout_write("  [OK] LanceDB local folder inventory verified!\n")

    def test_03_source_service_dynamic_enrichment_and_auto_heal(self):
        """Validates that SourceService enriches sources from LanceDB and heals SQLite cache."""
        safe_stdout_write("\n>>> [UNIT] Testing SourceService Dynamic Enrichment & Cache Auto-Heal...\n")
        ws = "Test_Auto_Heal_WS"
        self.store.add_workspace(ws, paths=[])

        # Register web source with stale page_count = 1 in SQLite
        from any_context.ingestion.web_scheduler import WebSchedulerStore
        web_store = WebSchedulerStore(db_path=self.db_path)
        root_url = "https://rust-lang.org/book"
        web_store.add_or_update_root_web_source(
            workspace_name=ws,
            root_url=root_url,
            title="Rust Book",
            page_count=1,
            scope="domain"
        )

        # Insert 4 distinct pages in LanceDB
        mock_vec = [0.0] * 1536
        records = [
            {"id": f"rb_{i}", "vector": mock_vec, "text": f"Ch {i}", "file_name": f"Ch {i}", "file_path": f"https://rust-lang.org/book/ch{i}.html", "workspace": ws, "last_modified": "2026-09-05", "content_type": "Canonical Service / Documentation", "document_summary": "", "keywords": "", "content_hash": f"h{i}"}
            for i in range(1, 5)
        ]
        self.lance_store.upsert_records(records)

        # SourceService should resolve 4 pages from LanceDB and heal SQLite
        svc = SourceService(store=self.store, lance_store=self.lance_store)
        res = svc.list_sources(ws)

        self.assertEqual(len(res["web_sources"]), 1)
        self.assertEqual(res["web_sources"][0]["page_count"], 4, "Live count from LanceDB must be 4")
        self.assertEqual(res["total_indexed_pages"], 4)

        # Verify SQLite cache was auto-healed to 4
        db_urls = web_store.get_workspace_web_urls(ws)
        self.assertEqual(db_urls[0]["page_count"], 4, "SQLite page_count must be auto-healed to 4")
        safe_stdout_write("  [OK] SourceService dynamic enrichment and SQLite cache auto-healing verified!\n")

    def test_04_switch_silence(self):
        """Validates that /switch returns empty message to avoid polluting chat history."""
        safe_stdout_write("\n>>> [UNIT] Testing /switch Zero-Noise Message Suppression...\n")
        res_modal = dispatch_command("/switch", active_workspace="Default", store=self.store)
        self.assertEqual(res_modal.action, "open_switch_modal")
        self.assertEqual(res_modal.message, "", "/switch modal must produce empty message")

        res_switch = dispatch_command("/switch NewTargetWS", active_workspace="Default", store=self.store)
        self.assertEqual(res_switch.action, "switch_workspace")
        self.assertEqual(res_switch.message, "", "/switch <name> must produce empty message")
        self.assertEqual(res_switch.state_updates.get("workspace"), "NewTargetWS")
        safe_stdout_write("  [OK] /switch zero-noise silence verified!\n")


if __name__ == "__main__":
    unittest.main()
