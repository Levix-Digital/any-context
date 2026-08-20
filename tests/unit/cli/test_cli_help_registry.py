import os
import sys
import unittest

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.help.registry import HELP_REGISTRY, get_help_page
from tests.e2e_helpers import safe_stdout_write

class TestCLIHelpRegistry(unittest.TestCase):
    """
    CLI Unit Test Suite: Validates the 17 Help Pages, Registry integrity, and Alias Resolution.
    """

    def test_01_help_registry_complete_coverage(self):
        """Validates that all 22 core commands resolve to valid, complete HelpPages."""
        safe_stdout_write("\n>>> [CLI UNIT] Testing Help Engine & Command Registry...\n")
        expected_commands = [
            "switch", "sync", "model", "api-keys", "web", "ocr",
            "billing", "update", "config", "auth", "share",
            "serve", "mcp", "reset-memory", "clear", "factory-reset", "history", "paste", "transfer",
            "density", "rename", "sources", "mode"
        ]

        for cmd in expected_commands:
            page = get_help_page(cmd)
            self.assertIsNotNone(page, f"Command '{cmd}' must exist in HELP_REGISTRY")
            self.assertTrue(len(page.title) > 0)
            self.assertTrue(len(page.description) > 0)
            self.assertTrue(len(page.syntax) > 0)
            self.assertGreater(len(page.parameters), 0)
            self.assertGreater(len(page.examples), 0)
            self.assertGreater(len(page.tips), 0)
        safe_stdout_write(f"  [OK] 100% of all {len(expected_commands)} command help pages resolved and validated!\n")

    def test_02_help_alias_resolution(self):
        """Tests resolving commands via various syntax styles (/cmd, -c, --cmd)."""
        safe_stdout_write(">>> [CLI UNIT] Testing Help Aliases Resolution...\n")
        self.assertEqual(get_help_page("/switch"), get_help_page("switch"))
        self.assertEqual(get_help_page("/sync"), get_help_page("index"))
        self.assertEqual(get_help_page("/m"), get_help_page("model"))
        self.assertEqual(get_help_page("/keys"), get_help_page("api-keys"))
        self.assertEqual(get_help_page("crawler"), get_help_page("web"))
        self.assertEqual(get_help_page("cls"), get_help_page("clear"))
        self.assertEqual(get_help_page("reset"), get_help_page("reset-memory"))
        self.assertEqual(get_help_page("/hist"), get_help_page("history"))
        self.assertEqual(get_help_page("/clear-history"), get_help_page("history"))
        self.assertEqual(get_help_page("/paste"), get_help_page("paste"))
        self.assertEqual(get_help_page("/multiline"), get_help_page("paste"))
        self.assertEqual(get_help_page("/transfer"), get_help_page("transfer"))
        self.assertEqual(get_help_page("move-source"), get_help_page("transfer"))
        self.assertEqual(get_help_page("/sources"), get_help_page("sources"))
        self.assertEqual(get_help_page("/workspace sources"), get_help_page("sources"))
        self.assertEqual(get_help_page("listsources"), get_help_page("sources"))
        self.assertEqual(get_help_page("workspace-list"), get_help_page("sources"))
        self.assertEqual(get_help_page("/mode"), get_help_page("mode"))
        self.assertEqual(get_help_page("/answer-mode"), get_help_page("mode"))
        self.assertEqual(get_help_page("grounding"), get_help_page("mode"))
        self.assertEqual(get_help_page("/am"), get_help_page("mode"))
        safe_stdout_write("  [OK] Help alias resolution verified across all shortcuts!\n")

    def test_03_prefix_and_suffix_help_interception(self):
        """Tests that prefix (/help <cmd>, actx --help <cmd>) and suffix (<cmd> --help, -h) intercept cleanly."""
        safe_stdout_write(">>> [CLI UNIT] Testing Prefix & Suffix Help Interception...\n")
        from any_context.help.manager import handle_command_help_interception
        from unittest.mock import patch

        with patch("any_context.help.manager.display_help_page") as mock_display:
            # 1. Prefix tests (/help density, help switch, actx --help sources)
            self.assertTrue(handle_command_help_interception("/help density"))
            self.assertTrue(mock_display.called)
            mock_display.reset_mock()

            self.assertTrue(handle_command_help_interception("help switch"))
            self.assertTrue(mock_display.called)
            mock_display.reset_mock()

            self.assertTrue(handle_command_help_interception("actx --help density"))
            self.assertTrue(mock_display.called)
            mock_display.reset_mock()

            self.assertTrue(handle_command_help_interception("actx help sources"))
            self.assertTrue(mock_display.called)
            mock_display.reset_mock()

            # 2. Suffix tests (/density --help, /sync -h, actx serve --help)
            self.assertTrue(handle_command_help_interception("/density --help"))
            self.assertTrue(mock_display.called)
            mock_display.reset_mock()

            self.assertTrue(handle_command_help_interception("/sources -h"))
            self.assertTrue(mock_display.called)
            mock_display.reset_mock()

            self.assertTrue(handle_command_help_interception("actx serve --help"))
            self.assertTrue(mock_display.called)
            mock_display.reset_mock()

        safe_stdout_write("  [OK] Prefix and suffix help interception verified for all command styles!\n")

if __name__ == "__main__":
    unittest.main()
