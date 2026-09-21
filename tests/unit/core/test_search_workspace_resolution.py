import unittest
from unittest.mock import patch, MagicMock
from any_context.tools.search_tools import (
    get_resolved_workspace,
    set_active_workspace_context,
    reset_active_workspace_context,
    search_db
)
from any_context.config.db_store import ConfigDBStore

class TestSearchWorkspaceResolution(unittest.TestCase):
    def setUp(self):
        self.store = ConfigDBStore()

    def test_explicit_workspace_highest_priority(self):
        tok = set_active_workspace_context("ContextWorkspace")
        try:
            resolved = get_resolved_workspace("ExplicitWorkspace")
            self.assertEqual(resolved, "ExplicitWorkspace")
        finally:
            reset_active_workspace_context(tok)

    def test_contextvar_second_priority(self):
        tok = set_active_workspace_context("ContextWorkspace")
        try:
            resolved = get_resolved_workspace(None)
            self.assertEqual(resolved, "ContextWorkspace")

            resolved_empty = get_resolved_workspace("   ")
            self.assertEqual(resolved_empty, "ContextWorkspace")
        finally:
            reset_active_workspace_context(tok)

    def test_config_store_fallback(self):
        tok = set_active_workspace_context(None)
        try:
            with patch.object(ConfigDBStore, "get_active_workspace", return_value="StoredWorkspace"):
                resolved = get_resolved_workspace(None)
                self.assertEqual(resolved, "StoredWorkspace")
        finally:
            reset_active_workspace_context(tok)

    def test_default_fallback_when_nothing_set(self):
        tok = set_active_workspace_context(None)
        try:
            with patch.object(ConfigDBStore, "get_active_workspace", return_value=""):
                resolved = get_resolved_workspace(None)
                self.assertEqual(resolved, "Default")
        finally:
            reset_active_workspace_context(tok)

    @patch("any_context.tools.search_tools._execute_search_context")
    def test_search_db_invocation_with_query_and_prompt_text(self, mock_exec):
        mock_exec.return_value = "Mocked Search Output"

        # 1. Via prompt_text
        res1 = search_db.invoke({"prompt_text": "hello world", "workspace": "CustomWS"})
        self.assertEqual(res1, "Mocked Search Output")
        mock_exec.assert_called_with(
            prompt_text="hello world",
            workspace="CustomWS",
            top_k=40,
            search_session_memory=False
        )

        # 2. Via query alias
        res2 = search_db.invoke({"query": "hello query", "workspace": None})
        self.assertEqual(res2, "Mocked Search Output")
        mock_exec.assert_called_with(
            prompt_text="hello query",
            workspace=None,
            top_k=40,
            search_session_memory=False
        )

if __name__ == "__main__":
    unittest.main()
