import os
import unittest
import tempfile
import chromadb
from any_context import __version__
from any_context.config.app_settings import AppSettings
from any_context.config.db_store import ConfigDBStore
from any_context.cli.updater import parse_version_tuple
from tests.e2e_helpers import safe_stdout_write

class Test10SystemLifecycleRecovery(unittest.TestCase):
    """
    E2E Test Suite 10: System Lifecycle, Factory Reset Atomics & Self-Updater Version Resolution
    """

    def test_01_version_comparison_logic(self):
        """TC-10.2: Tests semantic version tuple parsing and comparison logic for updater."""
        safe_stdout_write("\n>>> [MOD 10 / TC-10.2] Testing Version Parsing & Comparison...\n")
        self.assertEqual(parse_version_tuple("v0.11.21"), (0, 11, 21))
        self.assertEqual(parse_version_tuple("0.12.0"), (0, 12, 0))
        self.assertTrue(parse_version_tuple("v0.12.0") > parse_version_tuple("v0.11.21"))
        self.assertTrue(parse_version_tuple("v1.0.0") > parse_version_tuple("v0.12.5"))
        self.assertFalse(parse_version_tuple("v0.11.20") > parse_version_tuple("v0.11.21"))
        safe_stdout_write("  [OK] Version parsing & comparison logic verified!\n")

    def test_02_factory_reset_simulation(self):
        """TC-10.1: Tests factory reset wiping SQLite tables and restoring clean state."""
        safe_stdout_write(">>> [MOD 10 / TC-10.1] Testing Factory Reset Simulation...\n")
        temp_dir = tempfile.mkdtemp(prefix="actx_factory_reset_")
        temp_db = os.path.join(temp_dir, "test_settings.db")

        store = ConfigDBStore(db_path=temp_db)
        store.add_workspace("Ephemeral_WS_1", [])
        store.add_workspace("Ephemeral_WS_2", [])
        store.set_api_key("openai", "sk-test-fake-key-12345678")

        workspaces_before = store.get_app_settings().workspaces
        self.assertGreaterEqual(len(workspaces_before), 2)

        # Execute Factory Reset
        store.factory_reset()

        workspaces_after = store.get_app_settings().workspaces
        self.assertEqual(len(workspaces_after), 1, "Factory reset must restore only 1 Default workspace")
        self.assertEqual(workspaces_after[0].name, "Default")

        api_key_after = store.get_api_key("openai")
        self.assertIsNone(api_key_after, "Factory reset must purge all saved API keys")
        safe_stdout_write("  [OK] Factory Reset simulated and verified!\n")

if __name__ == "__main__":
    unittest.main()
