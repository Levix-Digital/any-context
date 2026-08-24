import os
import sys
import unittest
from unittest.mock import patch, MagicMock
import io

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.cli.viewport import PinnedBottomDock, _visible_len, _char_width
from tests.e2e_helpers import safe_stdout_write

class TestViewportDock(unittest.TestCase):
    """
    Unit Test Suite: Validates PinnedBottomDock and Terminal Viewport Management (v0.24.6).
    """

    def test_01_visible_len_and_char_width(self):
        """Validates visible length calculation ignoring ANSI escape codes."""
        safe_stdout_write("\n>>> [CLI UNIT] Testing ANSI Visible Length Calculation...\n")
        raw = "\033[1;33m📂 TestWorkspace\033[0m"
        self.assertEqual(_visible_len(raw), 16) # '📂 TestWorkspace' (emoji=2, chars=14)
        safe_stdout_write("  [OK] Visible string length calculated accurately!\n")

    def test_02_render_dock_lines_structure(self):
        """Validates that _render_dock_lines builds valid divider and status dock strings."""
        safe_stdout_write(">>> [CLI UNIT] Testing Dock Lines Rendering...\n")
        dock = PinnedBottomDock(
            workspace_name="LegalDocs",
            model_name="gpt-4o-mini",
            grounding_mode="strict",
            web_search_enabled=True
        )
        dock._cols = 100
        divider, status_line = dock._render_dock_lines(status_override="⚡ Syncing...")

        self.assertIn("─", divider)
        self.assertIn("LegalDocs", status_line)
        self.assertIn("gpt-4o-mini", status_line)
        self.assertIn("Strict", status_line)
        self.assertIn("🌐 Search: ON", status_line)
        self.assertIn("⚡ Syncing...", status_line)
        self.assertIn("/exit", status_line)
        safe_stdout_write("  [OK] Dock lines rendered with exact formatting and status badges!\n")

    def test_03_non_tty_safe_passthrough(self):
        """Validates that non-TTY environments operate in safe pass-through mode without ANSI margin locks."""
        safe_stdout_write(">>> [CLI UNIT] Testing Non-TTY Safe Fallback...\n")
        dock = PinnedBottomDock()
        with patch("sys.stdout.isatty", return_value=False):
            with dock as active_dock:
                self.assertFalse(active_dock._is_active)
                active_dock.write("Streaming test tokens...")
                active_dock.update_status("Searching...")
            self.assertFalse(active_dock._is_active)
        safe_stdout_write("  [OK] Non-TTY mode falls back safely to pass-through without errors!\n")

    def test_04_tty_decstbm_lifecycle_and_exception_safety(self):
        """Validates that TTY environment sets DECSTBM scroll region and ALWAYS resets margins on exit."""
        safe_stdout_write(">>> [CLI UNIT] Testing TTY DECSTBM Scroll Margin Lifecycle & Safety...\n")
        fake_stdout = io.StringIO()
        fake_stdout.isatty = MagicMock(return_value=True)

        with patch("sys.stdout", fake_stdout):
            with patch("shutil.get_terminal_size", return_value=os.terminal_size((100, 30))):
                try:
                    with PinnedBottomDock(workspace_name="DevWS", model_name="gpt-4o", grounding_mode="hybrid") as dock:
                        self.assertTrue(dock._is_active)
                        dock.write("Token 1 ")
                        dock.update_status("Reading files...")
                        dock.write("Token 2")
                        # Simulate unexpected interruption
                        raise RuntimeError("Simulated mid-stream error")
                except RuntimeError:
                    pass

                output = fake_stdout.getvalue()
                # Must contain the scroll region set \033[1;28r
                self.assertIn("\033[1;28r", output)
                # Must contain the reset command \033[r
                self.assertIn("\033[r", output)

        safe_stdout_write("  [OK] DECSTBM scroll margin set and guaranteed clean reset on exit!\n")

if __name__ == "__main__":
    unittest.main()
