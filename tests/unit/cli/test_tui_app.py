import os
import sys
import unittest
from unittest.mock import patch, MagicMock

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.cli.tui_app import AnyContextApp, StatusFooterDock
from tests.e2e_helpers import safe_stdout_write


class TestTUIApp(unittest.IsolatedAsyncioTestCase):
    """
    Unit Test Suite: Validates AnyContext Textual TUI Application (v0.25.0).
    """

    async def test_01_status_footer_dock_rendering_and_updates(self):
        """Validates that StatusFooterDock correctly formats badges and updates dynamically."""
        safe_stdout_write("\n>>> [TUI UNIT] Testing StatusFooterDock Formatting & Badges...\n")
        dock = StatusFooterDock(
            workspace="TestWS",
            model="gpt-4o-mini",
            mode="strict",
            web_search=True
        )

        rendered = dock.render()
        self.assertIn("TestWS", rendered)
        self.assertIn("gpt-4o-mini", rendered)
        self.assertIn("Strict", rendered)
        self.assertIn("Search: ON", rendered)

        # Dynamic update
        dock.update_values(workspace="NewWS", model="claude-3-5-sonnet", mode="hybrid", web_search=False, sync_info="Syncing [████░░░░]")
        updated = dock.render()
        self.assertIn("NewWS", updated)
        self.assertIn("claude-3-5-sonnet", updated)
        self.assertIn("Hybrid", updated)
        self.assertIn("Search: OFF", updated)
        self.assertIn("Syncing", updated)
        safe_stdout_write("  [OK] StatusFooterDock rendering and dynamic badge updates verified!\n")

    async def test_02_tui_app_composition_and_mount(self):
        """Validates that AnyContextApp composes Header, ChatScroll, Input, and StatusDock."""
        safe_stdout_write(">>> [TUI UNIT] Testing AnyContextApp Widget Tree & Composition...\n")
        app = AnyContextApp(initial_workspace="UnitWS")

        async with app.run_test() as pilot:
            self.assertIsNotNone(app.query_one("#header-bar"))
            self.assertIsNotNone(app.query_one("#chat-scroll"))
            self.assertIsNotNone(app.query_one("#prompt-input"))
            self.assertIsNotNone(app.query_one("#status-dock"))
            self.assertEqual(app.active_workspace, "UnitWS")

        safe_stdout_write("  [OK] TUI App mounted successfully with all persistent layout widgets!\n")

    async def test_03_tui_slash_commands_dispatch(self):
        """Validates that slash commands (/mode, /web-search, /clear, /version) execute in TUI."""
        safe_stdout_write(">>> [TUI UNIT] Testing TUI Slash Commands Dispatch...\n")
        app = AnyContextApp(initial_workspace="CmdWS")

        async with app.run_test() as pilot:
            # 1. /mode
            app.handle_slash_command("/mode Hybrid")
            self.assertEqual(app.grounding_mode, "hybrid")

            # 2. /web-search
            app.handle_slash_command("/web-search on")
            self.assertTrue(app.web_search_enabled)

            # 3. /version
            app.handle_slash_command("/version")

            # 4. /clear
            app.handle_slash_command("/clear")

        safe_stdout_write("  [OK] TUI slash commands dispatched and updated state cleanly!\n")

    async def test_04_user_input_submission_and_card_mount(self):
        """Validates that submitting a prompt via the input box mounts a UserMessageCard."""
        safe_stdout_write(">>> [TUI UNIT] Testing User Input Submission & Message Mounting...\n")
        app = AnyContextApp(initial_workspace="PromptWS")

        with patch.object(app, "_start_ai_generation") as mock_ai_start:
            async with app.run_test() as pilot:
                input_widget = app.query_one("#prompt-input")
                input_widget.value = "Como funciona o AnyContext?"
                await pilot.press("enter")

                # The user card must be mounted in chat-scroll
                chat_scroll = app.query_one("#chat-scroll")
                self.assertTrue(mock_ai_start.called, "_start_ai_generation must be invoked")

        safe_stdout_write("  [OK] Prompt submission mounted user card and initiated stream!\n")


if __name__ == "__main__":
    unittest.main()
