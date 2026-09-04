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
        os.environ["ACTX_SETTINGS_DB"] = self.db_path
        self.store = ConfigDBStore(db_path=self.db_path)

    def tearDown(self):
        if self._orig_env is not None:
            os.environ["ACTX_SETTINGS_DB"] = self._orig_env
        else:
            os.environ.pop("ACTX_SETTINGS_DB", None)
        if hasattr(self, "temp_dir") and os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_01_purge_legacy_test_workspaces_on_init(self):
        """Validates that test-named workspaces are automatically purged during _init_db."""
        safe_stdout_write("\n>>> [SANITIZATION UNIT] Testing Auto-Purge of Test Workspaces...\n")
        # Artificially insert test workspaces into the SQLite database and remove purge flag to simulate upgrade
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            cursor.execute("DELETE FROM system_config WHERE key = 'legacy_test_workspaces_purged'")
            cursor.execute("INSERT OR IGNORE INTO workspaces (workspace_id, name, paths_json, model) VALUES ('ws_rpc', 'RpcUnitTestWS', '[]', 'gpt-4o-mini')")
            cursor.execute("INSERT OR IGNORE INTO workspaces (workspace_id, name, paths_json, model) VALUES ('ws_new', 'NewRPCWS', '[]', 'gpt-4o-mini')")
            cursor.execute("INSERT OR IGNORE INTO workspaces (workspace_id, name, paths_json, model) VALUES ('ws_e2e', 'E2E_Empty_Workspace', '[]', 'gpt-4o-mini')")
            cursor.execute("INSERT OR IGNORE INTO workspaces (workspace_id, name, paths_json, model) VALUES ('ws_user', 'MyPersonalVault', '[]', 'gpt-4o-mini')")
            conn.commit()

        # Re-initialize DB store (simulating an app restart or upgrade)
        reloaded_store = ConfigDBStore(db_path=self.db_path)
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT name FROM workspaces")
            names = [r[0] for r in cursor.fetchall()]

        self.assertNotIn("RpcUnitTestWS", names)
        self.assertNotIn("NewRPCWS", names)
        self.assertNotIn("E2E_Empty_Workspace", names)
        self.assertIn("Default", names)
        self.assertIn("Shared Sources", names)
        self.assertIn("MyPersonalVault", names)
        safe_stdout_write("  [OK] Legacy test workspaces purged and user workspaces safely preserved!\n")

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
        server_default = StdioRPCServer(default_workspace="Default")
        state_default = server_default.get_state()
        self.assertEqual(state_default["model"], "gpt-4o-mini")
        self.assertEqual(state_default["model_display"], "GPT-4o Mini")

        # RPC server on CustomWS should show deepseek-chat
        server_custom = StdioRPCServer(default_workspace="CustomWS")
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
