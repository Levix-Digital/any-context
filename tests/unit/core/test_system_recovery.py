import os
import sys
import unittest
import tempfile
import chromadb

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context import __version__
from any_context.config.app_settings import AppSettings
from any_context.config.db_store import ConfigDBStore
from any_context.cli.updater import (
    parse_version_tuple,
    find_active_instances,
    close_active_instances,
    prompt_multi_instance_decision
)
from tests.e2e_helpers import safe_stdout_write
from unittest.mock import patch, MagicMock

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
        ws_after_names = [w.name for w in workspaces_after]
        self.assertEqual(len(workspaces_after), 2, "Factory reset must restore Default and Shared Sources workspaces")
        self.assertIn("Default", ws_after_names)
        self.assertIn("Shared Sources", ws_after_names)

        api_key_after = store.get_api_key("openai")
        self.assertIsNone(api_key_after, "Factory reset must purge all saved API keys")
        safe_stdout_write("  [OK] Factory Reset simulated and verified!\n")

    def test_03_find_active_instances_ignores_self_and_parent(self):
        """TC-10.3: Tests find_active_instances and verifies current PID is strictly excluded."""
        safe_stdout_write(">>> [MOD 10 / TC-10.3] Testing Active Instances Discovery...\n")
        instances = find_active_instances()
        self.assertIsInstance(instances, list)
        current_pid = os.getpid()
        for inst in instances:
            self.assertIn("pid", inst)
            self.assertNotEqual(inst["pid"], current_pid, "Current process PID must never be in active instances list")
        safe_stdout_write("  [OK] Active instances discovery verified!\n")

    def test_04_close_active_instances_mock(self):
        """TC-10.4: Tests close_active_instances with mock subprocess and os.kill cross-platform."""
        safe_stdout_write(">>> [MOD 10 / TC-10.4] Testing Close Active Instances...\n")
        mock_instances = [
            {"pid": 99991, "name": "actx.exe", "type": "cli"},
            {"pid": 99992, "name": "actx.exe", "type": "server"}
        ]
        # 1. Test Windows branch
        with patch("sys.platform", "win32"), patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            closed_cnt_win = close_active_instances(mock_instances)
            self.assertEqual(closed_cnt_win, 2)
            self.assertEqual(mock_run.call_count, 2)

        # 2. Test Linux / Unix branch
        with patch("sys.platform", "linux"), patch.dict("os.environ", {"MSYSTEM": ""}, clear=False), patch("os.kill") as mock_kill:
            mock_kill.return_value = None
            closed_cnt_linux = close_active_instances(mock_instances)
            self.assertEqual(closed_cnt_linux, 2)
            self.assertEqual(mock_kill.call_count, 2)

        safe_stdout_write("  [OK] Close active instances verified across Windows and Linux!\n")

    def test_05_prompt_multi_instance_decision_logic(self):
        """TC-10.5: Tests prompt_multi_instance_decision choices and fallback."""
        safe_stdout_write(">>> [MOD 10 / TC-10.5] Testing Multi-Instance Decision Prompt Logic...\n")
        mock_instances = [{"pid": 88881, "name": "actx.exe", "type": "cli"}]
        # Test questionary background choice
        with patch("questionary.select") as mock_select:
            mock_select.return_value.ask.return_value = "background"
            res = prompt_multi_instance_decision(mock_instances)
            self.assertEqual(res, "background")

        # Test questionary close choice
        with patch("questionary.select") as mock_select:
            mock_select.return_value.ask.return_value = "close"
            res = prompt_multi_instance_decision(mock_instances)
            self.assertEqual(res, "close")

        # Test questionary cancel choice
        with patch("questionary.select") as mock_select:
            mock_select.return_value.ask.return_value = "cancel"
            res = prompt_multi_instance_decision(mock_instances)
            self.assertEqual(res, "cancel")
        safe_stdout_write("  [OK] Multi-instance decision prompt logic verified!\n")

if __name__ == "__main__":
    unittest.main()
