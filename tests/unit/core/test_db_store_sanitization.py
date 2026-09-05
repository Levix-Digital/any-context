import os
import sys
import tempfile
import shutil
import sqlite3
import unittest

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.config.db_store import ConfigDBStore
from any_context.core.services.model_service import ModelService
from any_context.server.rpc_bridge import StdioRPCServer
from tests.e2e_helpers import safe_stdout_write


class TestDBStoreSanitization(unittest.TestCase):
    """
    Unit Test Suite: Validates auto-purge of legacy/test workspaces during startup
    and strict per-workspace model isolation across Core and RPC bridge.
    """

    def setUp(self):
        self.temp_dir = tempfile.mkdtemp(prefix="actx_test_sanitize_")
        self.db_path = os.path.join(self.temp_dir, "test_settings.db")
        self._orig_env = os.environ.get("ACTX_SETTINGS_DB")
        self._orig_test_mode = os.environ.get("ACTX_TEST_MODE")
        os.environ["ACTX_SETTINGS_DB"] = self.db_path
        os.environ["ACTX_TEST_MODE"] = "1"
        self.store = ConfigDBStore(db_path=self.db_path)

    def tearDown(self):
        if self._orig_env is not None:
            os.environ["ACTX_SETTINGS_DB"] = self._orig_env
        else:
            os.environ.pop("ACTX_SETTINGS_DB", None)
        if self._orig_test_mode is not None:
            os.environ["ACTX_TEST_MODE"] = self._orig_test_mode
        else:
            os.environ.pop("ACTX_TEST_MODE", None)
        if hasattr(self, "temp_dir") and os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_01_purge_legacy_test_workspaces_on_init(self):
        """Validates that created_by='test' workspaces (even with sources) and lingering test fixtures are cascade-purged, preserving user workspaces even if named 'TestWorkspace'."""
        safe_stdout_write("\n>>> [SANITIZATION UNIT] Testing Provenance-Aware Workspace Preservation...\n")
        # Artificially insert workspaces into the SQLite database with different provenance tags
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            # 1. Ephemeral test workspace with no sources (should be purged)
            cursor.execute("INSERT OR IGNORE INTO workspaces (workspace_id, name, paths_json, model, created_by) VALUES ('ws_rpc', 'RpcUnitTestWS', '[]', 'gpt-4o-mini', 'test')")
            # 2. User workspace explicitly named 'TestWorkspace' (MUST BE PRESERVED)
            cursor.execute("INSERT OR IGNORE INTO workspaces (workspace_id, name, paths_json, model, created_by) VALUES ('ws_tw', 'TestWorkspace', '[]', 'gpt-4o-mini', 'user')")
            # 3. User workspace starting with 'test_' (MUST BE PRESERVED)
            cursor.execute("INSERT OR IGNORE INTO workspaces (workspace_id, name, paths_json, model, created_by) VALUES ('ws_sub', 'test_my_feature', '[]', 'gpt-4o-mini', 'user')")
            # 4. Another user workspace (MUST BE PRESERVED)
            cursor.execute("INSERT OR IGNORE INTO workspaces (workspace_id, name, paths_json, model, created_by) VALUES ('ws_user', 'MyPersonalVault', '[]', 'gpt-4o-mini', 'user')")
            # 5. Test workspace that has an associated source URL (MUST BE CASCADE PURGED along with its source URLs)
            cursor.execute("INSERT OR IGNORE INTO workspaces (workspace_id, name, paths_json, model, created_by) VALUES ('ws_test_with_src', 'TestWithSource', '[]', 'gpt-4o-mini', 'test')")
            cursor.execute("INSERT OR IGNORE INTO workspace_web_urls (id, workspace_name, url) VALUES ('web_test_1', 'TestWithSource', 'https://example.com')")
            # 6. Lingering legacy test fixtures (MUST BE PURGED)
            cursor.execute("INSERT OR IGNORE INTO workspaces (workspace_id, name, paths_json, model, created_by) VALUES ('ws_ud', 'Unit_Dispatch_WS', '[]', 'gpt-4o-mini', 'test')")
            cursor.execute("INSERT OR IGNORE INTO workspaces (workspace_id, name, paths_json, model, created_by) VALUES ('ws_tws', 'TestWS', '[]', 'gpt-4o-mini', 'user')")
            conn.commit()

        # Re-initialize DB store simulating an app restart or upgrade in production mode (ACTX_TEST_MODE not set)
        orig_test_mode = os.environ.pop("ACTX_TEST_MODE", None)
        try:
            reloaded_store = ConfigDBStore(db_path=self.db_path)
            with sqlite3.connect(self.db_path) as conn:
                cursor = conn.cursor()
                cursor.execute("SELECT name, created_by FROM workspaces")
                rows = dict(cursor.fetchall())

                # Verify child tables were cascade purged for test workspaces
                cursor.execute("SELECT * FROM workspace_web_urls WHERE workspace_name = 'TestWithSource'")
                orphaned_urls = cursor.fetchall()
        finally:
            if orig_test_mode is not None:
                os.environ["ACTX_TEST_MODE"] = orig_test_mode

        self.assertNotIn("RpcUnitTestWS", rows)
        self.assertNotIn("TestWithSource", rows)
        self.assertNotIn("Unit_Dispatch_WS", rows)
        self.assertNotIn("TestWS", rows)
        self.assertEqual(len(orphaned_urls), 0, "Test workspace web URLs must be cascade-purged!")
        self.assertIn("TestWorkspace", rows)
        self.assertEqual(rows["TestWorkspace"], "user")
        self.assertIn("test_my_feature", rows)
        self.assertEqual(rows["test_my_feature"], "user")
        self.assertIn("MyPersonalVault", rows)
        self.assertEqual(rows["MyPersonalVault"], "user")
        self.assertIn("Default", rows)
        self.assertEqual(rows["Default"], "system")
        self.assertIn("Shared Sources", rows)
        self.assertEqual(rows["Shared Sources"], "system")
        safe_stdout_write("  [OK] Provenance tracking & cascade test purge verified: user workspaces 100% immune!\n")

    def test_03_installer_and_update_logging_paths(self):
        """Validates that log paths for install, update, and migrations are resolved and writable."""
        safe_stdout_write(">>> [SANITIZATION UNIT] Testing Installer and Update Log Paths...\n")
        from any_context.config.paths import get_install_log_path, get_update_log_path, get_migration_log_path
        from any_context.cli.updater import log_update_event
        from any_context.config.db_store import _log_migration

        inst_path = get_install_log_path()
        upd_path = get_update_log_path()
        mig_path = get_migration_log_path()

        self.assertTrue(inst_path.endswith("install.log"))
        self.assertTrue(upd_path.endswith("update.log"))
        self.assertTrue(mig_path.endswith("migration.log"))

        # Test writing log events
        log_update_event("Unit test update log event")
        _log_migration("Unit test migration log event")

        self.assertTrue(os.path.exists(upd_path))
        self.assertTrue(os.path.exists(mig_path))
        with open(upd_path, "r", encoding="utf-8") as f:
            content = f.read()
            self.assertIn("Unit test update log event", content)
        with open(mig_path, "r", encoding="utf-8") as f:
            content = f.read()
            self.assertIn("Unit test migration log event", content)
        safe_stdout_write("  [OK] Persistent log paths verified and operational!\n")


    def test_02_rpc_bridge_workspace_model_parity(self):
        """Validates that StdioRPCServer returns the model configured for the active workspace."""
        safe_stdout_write(">>> [SANITIZATION UNIT] Testing RPC Bridge Workspace Model Parity...\n")
        model_svc = ModelService(store=self.store)

        # Ensure Default is gpt-4o-mini
        curr_default = model_svc.get_current_model(workspace_name="Default")
        self.assertEqual(curr_default, "gpt-4o-mini")

        # Create a workspace with a custom model
        self.store.add_workspace("CustomWS", paths=[])
        model_svc.set_model("deepseek-chat", workspace_name="CustomWS")

        # RPC server on Default should show gpt-4o-mini
        server_default = StdioRPCServer(default_workspace="Default", store=self.store)
        state_default = server_default.get_state()
        self.assertEqual(state_default["model"], "gpt-4o-mini")
        self.assertEqual(state_default["model_display"], "GPT-4o Mini")

        # RPC server on CustomWS should show deepseek-chat
        server_custom = StdioRPCServer(default_workspace="CustomWS", store=self.store)
        state_custom = server_custom.get_state()
        self.assertEqual(state_custom["model"], "deepseek-chat")
        self.assertEqual(state_custom["model_display"], "DeepSeek V3")

        # RPC set_model should update the current workspace without contaminating other workspaces
        server_default.handle_request({"id": 1, "method": "set_model", "params": {"model": "gemini-flash-latest"}})
        state_after = server_default.get_state()
        self.assertEqual(state_after["model"], "gemini-flash-latest")
        self.assertEqual(self.store.get_workspace_model("Default"), "gemini-flash-latest")
        self.assertEqual(self.store.get_workspace_model("CustomWS"), "deepseek-chat")
        safe_stdout_write("  [OK] RPC Bridge per-workspace model parity verified!\n")


if __name__ == "__main__":
    unittest.main()
