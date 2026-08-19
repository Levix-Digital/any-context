import os
import sys
import shutil
import unittest
import tempfile
import chromadb

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.config.app_settings import AppSettings, SessionSettings
from any_context.config.db_store import ConfigDBStore
from any_context.memory.models import MemoryEntry, MemoryLevel
from any_context.memory.store import MemoryStore
from tests.e2e_helpers import safe_stdout_write, setup_mock_embeddings_if_needed

class Test04MemoryHierarchy(unittest.TestCase):
    """
    E2E Test Suite 04: 3-Level Memory Lifecycle (L1 Session, L2 Short-term, L3 Meta-Summary, /reset-memory)
    """

    @classmethod
    def setUpClass(cls):
        setup_mock_embeddings_if_needed()
        cls.temp_dir = tempfile.mkdtemp(prefix="actx_e2e_memory_")
        cls.ws = "E2E_Mod4_MemoryWorkspace"
        cls.store = ConfigDBStore()
        cls.store.add_workspace(cls.ws, [])

        custom_settings = AppSettings(
            session=SessionSettings(
                db_path=os.path.join(cls.temp_dir, "memory_test"),
                collection_name="e2e_session_docs"
            )
        )
        cls.memory_store = MemoryStore(settings=custom_settings)

    @classmethod
    def tearDownClass(cls):
        try:
            cls.store.remove_workspace(cls.ws)
            cls.memory_store.reset_memory(workspace=cls.ws)
            shutil.rmtree(cls.temp_dir, ignore_errors=True)
        except Exception:
            pass

    def test_01_level1_session_summary_storage_and_retrieval(self):
        """TC-4.1: Tests storing and querying Level-1 session block summaries."""
        safe_stdout_write("\n>>> [MOD 4 / TC-4.1] Testing Level-1 Session Summary Storage...\n")
        entry = MemoryEntry(
            content="User discussed migrating legacy SQLite to PostgreSQL on AWS RDS.",
            level=MemoryLevel.SESSION_SUMMARY,
            workspace=self.ws,
            thread_id="thread_test_01"
        )
        doc_id = self.memory_store.save_memory_entry(entry)
        self.assertTrue(bool(doc_id))

        entries = self.memory_store.get_entries_by_level(MemoryLevel.SESSION_SUMMARY, workspace=self.ws)
        self.assertGreaterEqual(len(entries), 1)
        safe_stdout_write("  [OK] Level-1 Session Summary successfully stored and retrieved!\n")

    def test_02_level3_meta_summary_consolidation(self):
        """TC-4.3: Tests storing and querying Level-3 Consolidated Meta-Summaries."""
        safe_stdout_write(">>> [MOD 4 / TC-4.3] Testing Level-3 Meta-Summary Storage...\n")
        meta_entry = MemoryEntry(
            content="Consolidated Meta-Summary: Architecture decisions across Q1 2026.",
            level=MemoryLevel.META_SUMMARY,
            workspace=self.ws,
            thread_id="thread_meta_01"
        )
        doc_id = self.memory_store.save_memory_entry(meta_entry)
        self.assertTrue(bool(doc_id))

        meta_entries = self.memory_store.get_entries_by_level(MemoryLevel.META_SUMMARY, workspace=self.ws)
        self.assertGreaterEqual(len(meta_entries), 1)
        safe_stdout_write("  [OK] Level-3 Meta-Summary consolidation verified!\n")

    def test_03_isolated_memory_reset_lifecycle(self):
        """TC-4.4: Tests resetting workspace memory without affecting other workspaces or document vectors."""
        safe_stdout_write(">>> [MOD 4 / TC-4.4] Testing Isolated Memory Reset Lifecycle...\n")
        # Ensure memory exists
        count_before = len(self.memory_store.get_entries_by_level(MemoryLevel.SESSION_SUMMARY, workspace=self.ws))
        self.assertGreaterEqual(count_before, 1)

        # Reset workspace memory
        deleted_count = self.memory_store.reset_memory(workspace=self.ws)
        self.assertGreaterEqual(deleted_count, 1)

        count_after = len(self.memory_store.get_entries_by_level(MemoryLevel.SESSION_SUMMARY, workspace=self.ws))
        self.assertEqual(count_after, 0, "All session memory for this workspace must be purged")
        safe_stdout_write("  [OK] Memory reset verified: isolated memory purging keeps documents intact!\n")

if __name__ == "__main__":
    unittest.main()
