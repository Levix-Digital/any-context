import os
import sys
import unittest
import tempfile
import shutil

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.config.db_store import ConfigDBStore
from any_context.core.services.model_service import ModelService
from any_context.commands.dispatcher import CommandDispatcher


class TestWorkspaceDefaultModel(unittest.TestCase):
    """
    Unit Test Suite: Validates per-workspace default model isolation (gpt-4o-mini).
    100% native unittest.TestCase without external test framework dependencies.
    """

    def setUp(self):
        self.temp_dir = tempfile.mkdtemp(prefix="actx_test_ws_model_")
        self.db_file = os.path.join(self.temp_dir, "test_settings.db")
        self.store = ConfigDBStore(db_path=self.db_file)
        self.svc = ModelService(store=self.store)

    def tearDown(self):
        try:
            shutil.rmtree(self.temp_dir)
        except Exception:
            pass

    def test_default_workspaces_have_gpt_4o_mini(self):
        self.assertEqual(self.store.get_workspace_model("Default"), "gpt-4o-mini")
        self.assertEqual(self.store.get_workspace_model("Shared Sources"), "gpt-4o-mini")

    def test_new_workspace_creation_strictly_defaults_to_gpt_4o_mini(self):
        self.store.add_workspace(name="ProjectAlpha", paths=[])
        self.assertEqual(self.store.get_workspace_model("ProjectAlpha"), "gpt-4o-mini")
        self.assertEqual(self.svc.get_current_model(workspace_name="ProjectAlpha"), "gpt-4o-mini")

    def test_workspace_model_isolation_prevents_contamination(self):
        # 1. Create Workspace A and set model to gpt-4o
        self.store.add_workspace(name="WorkspaceA", paths=[])
        self.svc.set_model("gpt-4o", workspace_name="WorkspaceA")
        self.assertEqual(self.svc.get_current_model(workspace_name="WorkspaceA"), "gpt-4o")

        # 2. Create Workspace B -> must strictly be factory default gpt-4o-mini
        self.store.add_workspace(name="WorkspaceB", paths=[])
        self.assertEqual(self.svc.get_current_model(workspace_name="WorkspaceB"), "gpt-4o-mini")

        # 3. Switching with dispatcher returns gpt-4o-mini for new workspace
        dispatcher = CommandDispatcher(store=self.store)
        res = dispatcher.dispatch("/switch WorkspaceB", active_workspace="WorkspaceA")
        self.assertTrue(res.success)
        self.assertEqual(res.state_updates.get("model"), "gpt-4o-mini")


if __name__ == "__main__":
    unittest.main()
