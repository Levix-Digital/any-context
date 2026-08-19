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
            "serve", "mcp", "reset-memory", "clear", "factory-reset", "history"
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
        """TC-9.1: Tests resolving commands via various syntax styles (/cmd, -c, --cmd)."""
        safe_stdout_write(">>> [MOD 9 / TC-9.1] Testing Help Aliases Resolution...\n")
        self.assertEqual(get_help_page("/switch"), get_help_page("switch"))
        self.assertEqual(get_help_page("/sync"), get_help_page("index"))
        self.assertEqual(get_help_page("/m"), get_help_page("model"))
        self.assertEqual(get_help_page("/keys"), get_help_page("api-keys"))
        self.assertEqual(get_help_page("crawler"), get_help_page("web"))
        self.assertEqual(get_help_page("cls"), get_help_page("clear"))
        self.assertEqual(get_help_page("reset"), get_help_page("reset-memory"))
        self.assertEqual(get_help_page("/hist"), get_help_page("history"))
        self.assertEqual(get_help_page("/clear-history"), get_help_page("history"))
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

    def test_04_workspace_history_isolation(self):
        """TC-9.4: Verifies strict per-workspace prompt history file isolation and retrieval."""
        safe_stdout_write(">>> [MOD 9 / TC-9.4] Testing Workspace Input History Isolation & Persistence...\n")
        from any_context.cli.history import (
            get_workspace_history_file,
            get_workspace_history_entries,
            clear_workspace_history,
            WorkspaceHistoryManager
        )
        from prompt_toolkit.history import FileHistory

        ws_a = "E2E_History_Legal"
        ws_b = "E2E_History_Mercado"

        # Clean any preexisting history
        clear_workspace_history(ws_a)
        clear_workspace_history(ws_b)

        file_a = get_workspace_history_file(ws_a)
        file_b = get_workspace_history_file(ws_b)
        self.assertNotEqual(file_a, file_b, "Each workspace must have a separate history file")

        # Append prompts to Workspace A
        hist_a = FileHistory(file_a)
        hist_a.append_string("Qual o prazo do contrato?")
        hist_a.append_string("Qual a multa por rescisão?")

        # Append prompts to Workspace B
        hist_b = FileHistory(file_b)
        hist_b.append_string("Qual o preço do detergente?")

        entries_a = get_workspace_history_entries(ws_a)
        entries_b = get_workspace_history_entries(ws_b)

        self.assertEqual(len(entries_a), 2)
        self.assertIn("Qual o prazo do contrato?", entries_a)
        self.assertNotIn("Qual o preço do detergente?", entries_a, "Workspace A must NOT contain prompts from Workspace B")

        self.assertEqual(len(entries_b), 1)
        self.assertEqual(entries_b[0], "Qual o preço do detergente?")

        # Clean up
        clear_workspace_history(ws_a)
        clear_workspace_history(ws_b)
        self.assertEqual(len(get_workspace_history_entries(ws_a)), 0)
        safe_stdout_write("  [OK] Workspace history isolation & persistence verified!\n")

if __name__ == "__main__":
    unittest.main()
