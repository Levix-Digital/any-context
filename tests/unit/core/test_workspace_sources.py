import os
import sys
import unittest
import tempfile
import shutil

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.config.db_store import ConfigDBStore
from any_context.ingestion.web_scheduler import WebSchedulerStore
from tests.e2e_helpers import safe_stdout_write, setup_mock_embeddings_if_needed

class TestWorkspaceSources(unittest.TestCase):
    """
    Unit Test Suite: UI-Agnostic Workspace & Multi-Source Management (Folders, Web Portals, Cloud Drives).
    """

    @classmethod
    def setUpClass(cls):
        setup_mock_embeddings_if_needed()
        cls.temp_dir = tempfile.mkdtemp()
        cls.db_path = os.path.join(cls.temp_dir, "test_sources_settings.db")
        cls.store = ConfigDBStore(db_path=cls.db_path)
        cls.web_store = WebSchedulerStore(db_path=cls.db_path)

    @classmethod
    def tearDownClass(cls):
        try:
            shutil.rmtree(cls.temp_dir, ignore_errors=True)
        except Exception:
            pass

    def test_01_empty_workspace_sources(self):
        """Validates that a new empty workspace has 0 sources and empty lists."""
        safe_stdout_write("\n>>> [UNIT] Testing Empty Workspace Sources...\n")
        ws_name = "Unit_Empty_WS"
        self.store.add_workspace(ws_name, paths=[])

        detail = self.store.get_workspace_sources(ws_name)
        self.assertEqual(detail["name"], ws_name)
        self.assertIn("id", detail)
        self.assertEqual(detail["sources"], [])
        self.assertEqual(detail["total_sources"], 0)
        safe_stdout_write("  [OK] Empty workspace sources verified!\n")

    def test_02_workspace_with_folders(self):
        """Validates that attached folder paths are structured into unified sources."""
        safe_stdout_write(">>> [UNIT] Testing Workspace with Folder Sources...\n")
        ws_name = "Unit_Folder_WS"
        test_folder = os.path.join(self.temp_dir, "docs_folder")
        os.makedirs(test_folder, exist_ok=True)
        self.store.add_workspace(ws_name, paths=[test_folder])

        detail = self.store.get_workspace_sources(ws_name)
        self.assertEqual(detail["name"], ws_name)
        self.assertIn("id", detail)
        self.assertEqual(detail["total_sources"], 1)
        
        source_item = detail["sources"][0]
        self.assertEqual(source_item["type"], "folder")
        self.assertEqual(source_item["identifier"], os.path.abspath(test_folder))
        self.assertTrue(source_item["details"]["exists"])
        safe_stdout_write("  [OK] Folder sources verified!\n")

    def test_03_workspace_with_web_sources(self):
        """Validates that web portals and URLs are properly aggregated into workspace sources."""
        safe_stdout_write(">>> [UNIT] Testing Workspace with Web Sources...\n")
        ws_name = "Unit_Web_WS"
        self.store.add_workspace(ws_name, paths=[])
        self.web_store.add_or_update_root_web_source(
            workspace_name=ws_name,
            root_url="https://docs.python.org",
            title="Python Documentation",
            page_count=42,
            scope="domain"
        )

        detail = self.store.get_workspace_sources(ws_name)
        self.assertEqual(detail["total_sources"], 1)
        unified = detail["sources"][0]
        self.assertEqual(unified["type"], "web")
        self.assertEqual(unified["identifier"], "https://docs.python.org")
        self.assertEqual(unified["title"], "Python Documentation")
        self.assertEqual(unified["details"]["page_count"], 42)
        safe_stdout_write("  [OK] Web portal sources verified!\n")

    def test_04_cloud_drives_crud_and_sources(self):
        """Validates cloud drive attachments (Google Drive, S3, OneDrive, Dropbox)."""
        safe_stdout_write(">>> [UNIT] Testing Cloud Drive Sources CRUD...\n")
        ws_name = "Unit_Cloud_WS"
        self.store.add_workspace(ws_name, paths=[])

        # 1. Add Google Drive source
        drive_res = self.store.add_cloud_drive_to_workspace(
            workspace_name=ws_name,
            provider="google_drive",
            mount_path_or_id="gdrive://folder-abc123xyz",
            title="Google Drive - Legal Contracts",
            metadata={"shared_drive": True, "mime": "application/vnd.google-apps.folder"}
        )
        drive_id = drive_res["id"]
        self.assertTrue(drive_id.startswith("drive_"))

        # 2. Get workspace cloud drives
        drives = self.store.get_workspace_cloud_drives(ws_name)
        self.assertEqual(len(drives), 1)
        self.assertEqual(drives[0]["provider"], "google_drive")
        self.assertEqual(drives[0]["title"], "Google Drive - Legal Contracts")
        self.assertTrue(drives[0]["metadata"]["shared_drive"])

        # 3. Check workspace sources aggregation
        detail = self.store.get_workspace_sources(ws_name)
        self.assertEqual(detail["total_sources"], 1)
        self.assertEqual(len(detail["sources"]), 1)
        unified = detail["sources"][0]
        self.assertEqual(unified["type"], "cloud_drive")
        self.assertEqual(unified["identifier"], "gdrive://folder-abc123xyz")
        self.assertEqual(unified["details"]["provider"], "google_drive")

        # 4. Delete cloud drive
        deleted = self.store.delete_cloud_drive(drive_id=drive_id, workspace_name=ws_name)
        self.assertTrue(deleted)
        self.assertEqual(len(self.store.get_workspace_cloud_drives(ws_name)), 0)
        safe_stdout_write("  [OK] Cloud drive CRUD and source aggregation verified!\n")

    def test_05_list_workspaces_detailed_multi_source(self):
        """Validates list_workspaces_detailed returns all workspaces with complete source breakdown."""
        safe_stdout_write(">>> [UNIT] Testing list_workspaces_detailed Multi-Source Aggregation...\n")
        ws_multi = "Unit_Multi_WS"
        folder_p = os.path.join(self.temp_dir, "multi_folder")
        os.makedirs(folder_p, exist_ok=True)
        self.store.add_workspace(ws_multi, paths=[folder_p])
        self.web_store.add_or_update_root_web_source(
            workspace_name=ws_multi,
            root_url="https://canada.ca/immigration",
            title="Canada Immigration",
            page_count=15
        )
        self.store.add_cloud_drive_to_workspace(
            workspace_name=ws_multi,
            provider="s3",
            mount_path_or_id="s3://company-invoices/2026",
            title="S3 Invoices"
        )

        all_ws = self.store.list_workspaces_detailed()
        target = next((w for w in all_ws if w["name"] == ws_multi), None)
        self.assertIsNotNone(target)
        self.assertIn("id", target)
        self.assertEqual(target["total_sources"], 3)
        self.assertEqual(len(target["sources"]), 3)

        source_types = {s["type"] for s in target["sources"]}
        self.assertEqual(source_types, {"folder", "web", "cloud_drive"})
        safe_stdout_write("  [OK] list_workspaces_detailed verified across all 3 source types!\n")

    def test_06_atomic_rename_migrates_all_sources(self):
        """Validates that rename_workspace atomically migrates sources, users, and tokens permissions."""
        safe_stdout_write(">>> [UNIT] Testing Workspace Rename Source Migration & RBAC Cascade...\n")
        old_ws = "Unit_Old_WS"
        new_ws = "Unit_New_WS"
        self.store.add_workspace(old_ws, paths=[os.path.join(self.temp_dir, "old_folder")])
        self.web_store.add_or_update_root_web_source(
            workspace_name=old_ws,
            root_url="https://docs.anthropic.com",
            title="Claude Docs",
            page_count=20
        )
        self.store.add_cloud_drive_to_workspace(
            workspace_name=old_ws,
            provider="onedrive",
            mount_path_or_id="onedrive://sharepoint/docs",
            title="OneDrive Docs"
        )

        # Create user and token assigned to old_ws
        u = self.store.create_user(name="Cascade User", email="cascade@test.com", password="pwd", role="analyst", allowed_workspaces=[old_ws])
        t = self.store.create_access_token(name="Cascade Token", role="analyst", allowed_workspaces=[old_ws])

        # Rename workspace
        res = self.store.rename_workspace(old_name=old_ws, new_name=new_ws)
        self.assertTrue(res["success"])
        self.assertIn("workspace_id", res)

        # Old workspace should have 0 sources
        old_detail = self.store.get_workspace_sources(old_ws)
        self.assertEqual(old_detail["total_sources"], 0)

        # New workspace should have all 3 sources
        new_detail = self.store.get_workspace_sources(new_ws)
        self.assertEqual(new_detail["total_sources"], 3)
        self.assertEqual(len(new_detail["sources"]), 3)

        # Verify RBAC Cascade in users and access_tokens
        users = self.store.list_users()
        cascaded_u = next((usr for usr in users if usr["email"] == "cascade@test.com"), None)
        self.assertIsNotNone(cascaded_u)
        self.assertIn(new_ws, cascaded_u["allowed_workspaces"])
        self.assertNotIn(old_ws, cascaded_u["allowed_workspaces"])

        tokens = self.store.get_access_tokens()
        cascaded_t = next((tok for tok in tokens if tok["token_id"] == t["token_id"]), None)
        self.assertIsNotNone(cascaded_t)
        self.assertIn(new_ws, cascaded_t["allowed_workspaces"])
        self.assertNotIn(old_ws, cascaded_t["allowed_workspaces"])

        # Token validation allows access to new_ws
        self.assertTrue(self.store.validate_token_permissions(t["token_id"], required_workspace=new_ws))
        safe_stdout_write("  [OK] Workspace rename atomically migrated all source types and cascaded RBAC permissions!\n")

    def test_07_remove_workspace_cleans_all_source_tables(self):
        """Validates that deleting a workspace purges associated records across all SQLite tables."""
        safe_stdout_write(">>> [UNIT] Testing Workspace Deletion SQLite Cleanup...\n")
        del_ws = "Unit_To_Delete_WS"
        self.store.add_workspace(del_ws, paths=[os.path.join(self.temp_dir, "del_folder")])
        self.web_store.add_or_update_root_web_source(
            workspace_name=del_ws,
            root_url="https://example.com",
            title="Example Site"
        )
        self.store.add_cloud_drive_to_workspace(
            workspace_name=del_ws,
            provider="dropbox",
            mount_path_or_id="dropbox://team-folder",
            title="Dropbox"
        )

        # Verify before deletion
        self.assertEqual(self.store.get_workspace_sources(del_ws)["total_sources"], 3)

        # Remove workspace
        deleted = self.store.remove_workspace(del_ws)
        self.assertTrue(deleted)

        # Verify all source records purged
        self.assertEqual(self.store.get_workspace_sources(del_ws)["total_sources"], 0)
        self.assertEqual(len(self.store.get_workspace_cloud_drives(del_ws)), 0)
        self.assertEqual(len(self.web_store.get_workspace_web_urls(del_ws)), 0)
        safe_stdout_write("  [OK] Workspace deletion SQLite cleanup verified!\n")

    def test_08_workspace_lifecycle_and_deletion_discovery(self):
        """Validates that newly created workspaces like 'all' are immediately discoverable for deletion."""
        safe_stdout_write(">>> [UNIT] Testing Workspace Lifecycle & Deletion Discovery...\n")
        ws_name = "all"
        
        # 1. Create workspace 'all'
        self.store.add_workspace(ws_name, paths=[])
        
        # 2. Verify discoverable in AppSettings
        settings = self.store.get_app_settings()
        deletable = [w.name for w in settings.workspaces if w.name.lower() not in ["default", "global"]]
        self.assertIn("all", deletable, "Workspace 'all' must be present in deletable list")

        # 3. Delete workspace 'all'
        deleted = self.store.remove_workspace(ws_name)
        self.assertTrue(deleted)

        # 4. Verify removed
        settings_after = self.store.get_app_settings()
        deletable_after = [w.name for w in settings_after.workspaces if w.name.lower() not in ["default", "global"]]
        self.assertNotIn("all", deletable_after)
        safe_stdout_write("  [OK] Workspace lifecycle and deletion discovery verified!\n")

if __name__ == "__main__":
    unittest.main()
