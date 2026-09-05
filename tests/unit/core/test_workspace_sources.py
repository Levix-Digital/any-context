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
        cls._orig_db = os.environ.get("ACTX_SETTINGS_DB")
        cls._orig_test_mode = os.environ.get("ACTX_TEST_MODE")
        os.environ["ACTX_SETTINGS_DB"] = cls.db_path
        os.environ["ACTX_TEST_MODE"] = "1"
        cls.store = ConfigDBStore(db_path=cls.db_path)
        cls.web_store = WebSchedulerStore(db_path=cls.db_path)

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
        deletable_after = [w.name for w in settings_after.workspaces if w.name.lower() not in ["default", "shared sources"]]
        self.assertNotIn("all", deletable_after)
        safe_stdout_write("  [OK] Workspace lifecycle and deletion discovery verified!\n")

    def test_09_shared_sources_linking(self):
        """Validates Shared Sources provisioning, linking, listing and unlinking."""
        safe_stdout_write(">>> [UNIT] Testing Shared Sources Linking...\n")
        # 1. Ensure default and shared sources
        self.store.ensure_default_workspace()
        meta_shared = self.store.get_workspace_meta("Shared Sources")
        self.assertIsNotNone(meta_shared)
        self.assertEqual(meta_shared["workspace_id"], "ws_shared_sources")

        # 2. Add an origin source in workspace 'Shared Sources'
        folder_shared = os.path.join(self.temp_dir, "shared_framework")
        os.makedirs(folder_shared, exist_ok=True)
        self.store.add_folder_to_workspace("Shared Sources", folder_shared)

        # 3. List available shared sources
        available = self.store.list_all_available_shared_sources()
        found = next((s for s in available if s["identifier"] == os.path.abspath(folder_shared)), None)
        self.assertIsNotNone(found, "Origin folder must be discoverable in list_all_available_shared_sources")

        # 4. Link shared source to workspace B
        ws_target = "Unit_Target_WS"
        self.store.add_workspace(ws_target, paths=[])
        link_res = self.store.link_shared_source_to_workspace(
            workspace_name=ws_target,
            source_type="folder",
            source_identifier=folder_shared,
            title="Shared Framework"
        )
        self.assertEqual(link_res["status"], "success")

        # 5. Verify target workspace sources contains the shared source
        target_sources = self.store.get_workspace_sources(ws_target)
        self.assertEqual(target_sources["total_sources"], 1)
        self.assertTrue(target_sources["sources"][0]["details"].get("is_shared_link"))
        self.assertIn("Shared", target_sources["sources"][0]["title"])

        # 6. Unlink shared source
        unlinked = self.store.unlink_shared_source_from_workspace(
            workspace_name=ws_target,
            source_type="folder",
            source_identifier=folder_shared
        )
        self.assertTrue(unlinked)
        target_sources_after = self.store.get_workspace_sources(ws_target)
        self.assertEqual(target_sources_after["total_sources"], 0)
        safe_stdout_write("  [OK] Shared Sources linking verified!\n")

    def test_10_attach_and_broadcast_source_modular_core(self):
        """Validates attach_and_broadcast_source modular core API attaching to primary and linking to multiple targets."""
        safe_stdout_write(">>> [UNIT] Testing Modular attach_and_broadcast_source Core API...\n")
        ws_primary = "Unit_Broadcast_Primary"
        ws_sub1 = "Unit_Broadcast_Sub1"
        ws_sub2 = "Unit_Broadcast_Sub2"
        folder_shared = os.path.join(self.temp_dir, "broadcast_folder")
        os.makedirs(folder_shared, exist_ok=True)

        self.store.add_workspace(ws_primary, paths=[])
        self.store.add_workspace(ws_sub1, paths=[])
        self.store.add_workspace(ws_sub2, paths=[])

        # Attach to primary and broadcast to sub1 and sub2
        res = self.store.attach_and_broadcast_source(
            primary_workspace=ws_primary,
            source_type="folder",
            source_identifier=folder_shared,
            title="Broadcast Component",
            link_to_workspaces=[ws_sub1, ws_sub2]
        )
        self.assertEqual(res["status"], "success")
        self.assertEqual(res["total_linked"], 2)

        # Verify primary has direct folder
        src_primary = self.store.get_workspace_sources(ws_primary)
        self.assertEqual(src_primary["total_sources"], 1)
        self.assertFalse(src_primary["sources"][0]["details"].get("is_shared_link", False))

        # Verify sub1 and sub2 have linked shared source
        src_sub1 = self.store.get_workspace_sources(ws_sub1)
        self.assertEqual(src_sub1["total_sources"], 1)
        self.assertTrue(src_sub1["sources"][0]["details"].get("is_shared_link"))

        src_sub2 = self.store.get_workspace_sources(ws_sub2)
        self.assertEqual(src_sub2["total_sources"], 1)
        self.assertTrue(src_sub2["sources"][0]["details"].get("is_shared_link"))

        safe_stdout_write("  [OK] attach_and_broadcast_source modular core API verified!\n")

    def test_11_path_resolution_and_dispatcher_folder_web(self):
        """Validates resolve_folder_path and CommandDispatcher /folder --add, /add, /web --add handling."""
        safe_stdout_write(">>> [UNIT] Testing Path Resolution and CommandDispatcher Source Add...\n")
        from any_context.core.utils import resolve_folder_path
        from any_context.commands.dispatcher import CommandDispatcher

        # 1. Path resolution tests
        resolved_win = resolve_folder_path('"/mnt/c/Users/Test"')
        if sys.platform == "win32" or os.name == "nt":
            self.assertEqual(resolved_win, os.path.abspath("C:\\Users\\Test"))

        resolved_wsl = resolve_folder_path('"G:\\My Drive\\Docs"')
        if sys.platform != "win32" and os.name != "nt":
            self.assertEqual(resolved_wsl, os.path.abspath("/mnt/g/My Drive/Docs"))

        # 2. Add folder with spaces via dispatcher
        ws_test = "Unit_Dispatch_WS"
        self.store.add_workspace(ws_test, paths=[])
        space_folder = os.path.join(self.temp_dir, "Folder With Spaces")
        os.makedirs(space_folder, exist_ok=True)

        dispatcher = CommandDispatcher(store=self.store)
        res_folder = dispatcher.dispatch(f"/folder --add {space_folder}", active_workspace=ws_test)
        self.assertTrue(res_folder.success, f"Failed adding folder with spaces: {res_folder.message}")

        # 3. Add via /add shortcut
        space_folder2 = os.path.join(self.temp_dir, "Folder With Spaces Two")
        os.makedirs(space_folder2, exist_ok=True)
        res_add = dispatcher.dispatch(f"/add {space_folder2}", active_workspace=ws_test)
        self.assertTrue(res_add.success, f"Failed adding folder via /add: {res_add.message}")

        # 4. Add web portal via dispatcher
        res_web = dispatcher.dispatch("/web --add https://docs.example.com", active_workspace=ws_test)
        self.assertTrue(res_web.success, f"Failed adding web portal: {res_web.message}")

        safe_stdout_write("  [OK] Path resolution and CommandDispatcher source commands verified!\n")


if __name__ == "__main__":
    unittest.main()
