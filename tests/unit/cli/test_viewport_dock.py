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
    Unit Test Suite: Validates Cursor-Aware PinnedBottomDock and Terminal Viewport Management (v0.24.8).
    """

    def test_01_visible_len_and_char_width(self):
        """Validates visible length calculation ignoring ANSI escape codes."""
        safe_stdout_write("\n>>> [CLI UNIT] Testing ANSI Visible Length Calculation...\n")
        raw = "\033[1;33m📂 TestWorkspace\033[0m"
        self.assertEqual(_visible_len(raw), 16)  # '📂 TestWorkspace' (emoji=2, chars=14)
        safe_stdout_write("  [OK] Visible string length calculated accurately!\n")

    def test_02_cursor_aware_write_and_stream(self):
        """Validates that write() outputs stream content safely to stdout."""
        safe_stdout_write(">>> [CLI UNIT] Testing Cursor-Aware Write & Stream...\n")
        fake_stdout = io.StringIO()
        with patch("sys.stdout", fake_stdout):
            with PinnedBottomDock(workspace_name="LegalDocs", model_name="gpt-4o-mini") as dock:
                dock.write("Hello ")
                dock.write("World!")

        output = fake_stdout.getvalue()
        self.assertIn("Hello World!", output)
        safe_stdout_write("  [OK] Stream text outputted cleanly without disruption!\n")

    def test_03_inline_status_ticker_updates(self):
        """Validates that update_status() renders transient inline ticker and clears on next write."""
        safe_stdout_write(">>> [CLI UNIT] Testing Inline Status Ticker & Auto-Clear...\n")
        fake_stdout = io.StringIO()
        with patch("sys.stdout", fake_stdout):
            with PinnedBottomDock() as dock:
                dock.update_status("Searching online...")
                self.assertTrue(dock._has_active_status_line)
                dock.write("Found answer:")
                self.assertFalse(dock._has_active_status_line)

        output = fake_stdout.getvalue()
        self.assertIn("Searching online...", output)
        self.assertIn("\r\033[K", output)
        self.assertIn("Found answer:", output)
        safe_stdout_write("  [OK] Inline status ticker rendered and gracefully replaced by output!\n")

    def test_04_non_tty_safe_passthrough(self):
        """Validates that non-TTY environments operate safely without errors."""
        safe_stdout_write(">>> [CLI UNIT] Testing Non-TTY Safe Passthrough...\n")
        fake_stdout = io.StringIO()
        fake_stdout.isatty = MagicMock(return_value=False)
        with patch("sys.stdout", fake_stdout):
            with PinnedBottomDock() as dock:
                dock.write("Non-TTY chunk 1")
                dock.update_status("Processing...")
                dock.write(" Non-TTY chunk 2")

        output = fake_stdout.getvalue()
        self.assertIn("Non-TTY chunk 1", output)
        self.assertIn("Non-TTY chunk 2", output)
        safe_stdout_write("  [OK] Non-TTY environment handled safely in pass-through mode!\n")

    def test_05_no_decstbm_pollution_and_clean_exit(self):
        """Validates that no DECSTBM scrolling margin codes are emitted, protecting terminal scrollback."""
        safe_stdout_write(">>> [CLI UNIT] Testing Absence of DECSTBM Buffer Corruption Sequences...\n")
        fake_stdout = io.StringIO()
        with patch("sys.stdout", fake_stdout):
            with PinnedBottomDock(workspace_name="DevWS", model_name="gpt-4o") as dock:
                dock.write("Testing clean stream")
                dock.update_status("Loading...")

        output = fake_stdout.getvalue()
        # Ensure no DECSTBM scroll region code like \033[1;28r or \033[r was outputted
        self.assertNotIn("\033[1;28r", output)
        self.assertNotIn("\033[r", output)
        safe_stdout_write("  [OK] DECSTBM-free stream verified, 100% terminal scrollback preserved!\n")


if __name__ == "__main__":
    unittest.main()

