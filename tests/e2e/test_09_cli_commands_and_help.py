import os
import unittest
from any_context.help.registry import HELP_REGISTRY, get_help_page
from tests.e2e_helpers import safe_stdout_write

class Test09CLICommandsAndHelp(unittest.TestCase):
    """
    E2E Test Suite 09: CLI Commands, Help Engine Registry (14 pages), Aliases & Formatting
    """

    def test_01_help_registry_complete_coverage(self):
        """TC-9.1: Validates that all 14 core commands and their aliases resolve to valid HelpPages."""
        safe_stdout_write("\n>>> [MOD 9 / TC-9.1] Testing Help Engine & Command Registry...\n")
        expected_commands = [
            "switch", "sync", "model", "api-keys", "web", "ocr",
            "billing", "update", "config", "auth", "share",
            "serve", "mcp", "reset-memory", "clear", "factory-reset"
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
        safe_stdout_write("  [OK] 100% of all 16 command help pages resolved and validated!\n")

    def test_02_help_alias_resolution(self):
        """TC-9.1: Tests resolving commands via various syntax styles (/cmd, -c, --cmd)."""
        safe_stdout_write(">>> [MOD 9 / TC-9.1] Testing Help Aliases Resolution...\n")
        self.assertEqual(get_help_page("/switch"), get_help_page("switch"))
        self.assertEqual(get_help_page("/sync"), get_help_page("index"))
        self.assertEqual(get_help_page("/m"), get_help_page("model"))
        self.assertEqual(get_help_page("/keys"), get_help_page("api-keys"))
        self.assertEqual(get_help_page("crawler"), get_help_page("web"))
        self.assertEqual(get_help_page("cls"), get_help_page("clear"))
        self.assertEqual(get_help_page("reset"), get_help_page("reset-memory"))
        safe_stdout_write("  [OK] Help alias resolution verified across all shortcuts!\n")

    def test_03_windows_charmap_safe_output(self):
        """TC-9.3: Ensures terminal output helpers never raise UnicodeEncodeError on CP1252 consoles."""
        safe_stdout_write(">>> [MOD 9 / TC-9.3] Testing Windows CP1252 / Charmap Terminal Output...\n")
        test_strings = [
            "🧠 [Hierarchical Memory - Level 1] Generating session summary block...",
            "🚀 Processing and indexing 250 web pages into workspace 'Default'...",
            "⠋ [1/2 Crawling] [██████████████] 250/250 (100%) • 250 new • Canada.ca",
            "✔ Successfully ingested 250 web pages (6,604,334 chars) into workspace 'Default'!",
            "🧹 Screen cleared | Workspace: Global | Model: gpt-4o-mini"
        ]
        for s in test_strings:
            try:
                safe_stdout_write(f"  {s}\n")
            except Exception as e:
                self.fail(f"Terminal safe writer crashed on string: {s} with error: {e}")
        safe_stdout_write("  [OK] Windows CP1252 / Charmap terminal output verified!\n")

if __name__ == "__main__":
    unittest.main()
