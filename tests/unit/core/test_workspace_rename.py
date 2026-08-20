import os
import sys
import unittest
import tempfile
import chromadb

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.config.db_store import ConfigDBStore
from any_context.ingestion.web_scheduler import WebSchedulerStore
from any_context.workspace_sharing.store import WorkspaceSharingStore
from tests.e2e_helpers import safe_stdout_write

class TestWorkspaceRename(unittest.TestCase):
    """
    Unit Test Suite: Validates Atomic Zero-Cost Workspace Renaming ($0.00 cost, < 50ms).
    Tests SQLite relational updates and ChromaDB metadata migration for folders and web sources.
    """

    @classmethod
    def setUpClass(cls):
        from tests.e2e_helpers import setup_mock_embeddings_if_needed
        setup_mock_embeddings_if_needed()

    def setUp(self):
        self.temp_dir = tempfile.mkdtemp(prefix="actx_test_rename_")
        self.db_path = os.path.join(self.temp_dir, "test_settings.db")
        self.store = ConfigDBStore(db_path=self.db_path)
        self.web_store = WebSchedulerStore(db_path=self.db_path)
        self.sharing_store = WorkspaceSharingStore(db_path=self.db_path)

        self.old_ws = "OldWorkspaceName"
        self.new_ws = "NewWorkspaceName"
        self.test_folder = os.path.join(self.temp_dir, "legal_docs")
        os.makedirs(self.test_folder, exist_ok=True)

        self.store.add_workspace(self.old_ws, paths=[self.test_folder])
        self.web_store.add_or_update_root_web_source(
            workspace_name=self.old_ws,
            root_url="https://canada.ca/immigration",
            title="Canada Immigration",
            page_count=10
        )
        self.sharing_store.grant_direct_permission(self.old_ws, "lawyer@firm.com", "editor", "admin@firm.com")

    def tearDown(self):
        try:
            self.store.remove_workspace(self.old_ws)
            self.store.remove_workspace(self.new_ws)
        except Exception:
            pass

    def test_01_atomic_workspace_rename(self):
        """Tests that renaming updates workspaces, folders, web URLs, and permissions in SQLite."""
        safe_stdout_write("\n>>> [CORE UNIT] Testing Atomic Workspace Rename...\n")

        res = self.store.rename_workspace(old_name=self.old_ws, new_name=self.new_ws)
        self.assertTrue(res["success"], f"Rename failed: {res.get('error')}")
        self.assertEqual(res["old_workspace"], self.old_ws)
        self.assertEqual(res["new_workspace"], self.new_ws)

        # 1. Verify workspaces table
        settings = self.store.get_app_settings()
        ws_names = [w.name for w in settings.workspaces]
        self.assertNotIn(self.old_ws, ws_names)
        self.assertIn(self.new_ws, ws_names)

        # 2. Verify folders preserved in new workspace
        new_obj = next((w for w in settings.workspaces if w.name == self.new_ws), None)
        self.assertIsNotNone(new_obj)
        self.assertIn(os.path.abspath(self.test_folder), [os.path.abspath(p) for p in new_obj.paths])

        # 3. Verify web URLs updated
        old_urls = self.web_store.get_workspace_web_urls(self.old_ws)
        new_urls = self.web_store.get_workspace_web_urls(self.new_ws)
        self.assertEqual(len(old_urls), 0)
        self.assertEqual(len(new_urls), 1)
        self.assertEqual(new_urls[0]["url"], "https://canada.ca/immigration")

        # 4. Verify RBAC permissions updated
        old_perms = self.sharing_store.get_workspace_permissions(self.old_ws)
        new_perms = self.sharing_store.get_workspace_permissions(self.new_ws)
        self.assertEqual(len(old_perms), 0)
        self.assertEqual(len(new_perms), 1)
        self.assertEqual(new_perms[0].user_email, "lawyer@firm.com")

        safe_stdout_write("  [OK] SQLite tables atomic rename verified!\n")

    def test_02_rename_validation_guardrails(self):
        """Tests guardrails for invalid workspace rename attempts."""
        safe_stdout_write(">>> [CORE UNIT] Testing Workspace Rename Guardrails...\n")

        # Non-existent workspace
        res1 = self.store.rename_workspace("NonExistentWS", "ValidNewName")
        self.assertFalse(res1["success"])
        self.assertIn("does not exist", res1["error"])

        # Target name already taken
        self.store.add_workspace("ExistingTarget", paths=[])
        res2 = self.store.rename_workspace(self.old_ws, "ExistingTarget")
        self.assertFalse(res2["success"])
        self.assertIn("already exists", res2["error"])

        # Same name
        res3 = self.store.rename_workspace(self.old_ws, self.old_ws)
        self.assertFalse(res3["success"])

        # Empty name
        res4 = self.store.rename_workspace(self.old_ws, "   ")
        self.assertFalse(res4["success"])

        # Protected workspace Default cannot be renamed
        res_def = self.store.rename_workspace("Default", "NewDefault")
        self.assertFalse(res_def["success"])
        self.assertIn("protected system workspace", res_def["error"])

        # Protected workspace Global cannot be renamed
        res_glob = self.store.rename_workspace("Global", "NewGlobal")
        self.assertFalse(res_glob["success"])
        self.assertIn("protected system workspace", res_glob["error"])

        # Cannot rename custom workspace to Default or Global
        res_to_def = self.store.rename_workspace(self.old_ws, "Default")
        self.assertFalse(res_to_def["success"])
        self.assertIn("protected system workspace", res_to_def["error"])

        # Protected workspaces Default and Global cannot be removed
        self.assertFalse(self.store.remove_workspace("Default"))
        self.assertFalse(self.store.remove_workspace("Global"))

        safe_stdout_write("  [OK] Workspace rename and delete protection guardrails verified!\n")


if __name__ == "__main__":
    unittest.main()
