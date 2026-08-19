import os
import sys
import unittest

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.cli.history import (
    get_workspace_history_file,
    get_workspace_history_entries,
    clear_workspace_history,
)
from prompt_toolkit.history import FileHistory
from tests.e2e_helpers import safe_stdout_write

class TestCLIHistory(unittest.TestCase):
    """
    CLI Unit Test Suite: Validates per-workspace prompt history file isolation & persistence.
    """

    def test_01_workspace_history_isolation(self):
        """Verifies strict per-workspace prompt history file isolation and retrieval."""
        safe_stdout_write("\n>>> [CLI UNIT] Testing Workspace Input History Isolation & Persistence...\n")
        ws_a = "Unit_History_A"
        ws_b = "Unit_History_B"

        clear_workspace_history(ws_a)
        clear_workspace_history(ws_b)

        file_a = get_workspace_history_file(ws_a)
        file_b = get_workspace_history_file(ws_b)
        self.assertNotEqual(file_a, file_b, "Each workspace must have a separate history file")

        hist_a = FileHistory(file_a)
        hist_a.append_string("Pergunta A1")
        hist_a.append_string("Pergunta A2")

        hist_b = FileHistory(file_b)
        hist_b.append_string("Pergunta B1")

        entries_a = get_workspace_history_entries(ws_a)
        entries_b = get_workspace_history_entries(ws_b)

        self.assertEqual(len(entries_a), 2)
        self.assertIn("Pergunta A1", entries_a)
        self.assertNotIn("Pergunta B1", entries_a)

        self.assertEqual(len(entries_b), 1)
        self.assertEqual(entries_b[0], "Pergunta B1")

        clear_workspace_history(ws_a)
        clear_workspace_history(ws_b)
        self.assertEqual(len(get_workspace_history_entries(ws_a)), 0)
        safe_stdout_write("  [OK] Workspace history isolation & persistence verified!\n")

if __name__ == "__main__":
    unittest.main()
