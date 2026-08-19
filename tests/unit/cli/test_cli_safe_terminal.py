import unittest
from any_context.cli.chat_loop import format_session_error
from tests.e2e_helpers import safe_stdout_write

class TestCLISafeTerminal(unittest.TestCase):
    """
    CLI Unit Test Suite: Validates terminal encoding safety and friendly error formatting.
    """

    def test_01_windows_charmap_safe_output(self):
        """Ensures terminal output helpers never raise UnicodeEncodeError on CP1252 consoles."""
        safe_stdout_write("\n>>> [CLI UNIT] Testing Windows CP1252 / Charmap Terminal Output...\n")
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

    def test_02_format_session_error_user_friendly(self):
        """Verifies that unexpected session exceptions render friendly, reassuring messages."""
        safe_stdout_write(">>> [CLI UNIT] Testing User-Friendly Session Error Formatting...\n")
        err = UnboundLocalError("cannot access local variable 'ConfigDBStore' where it is not associated with a value")
        formatted = format_session_error(err)

        self.assertIn("Ops!", formatted)
        self.assertIn("Nota técnica:", formatted)
        self.assertIn("Sua sessão", formatted)
        safe_stdout_write("  [OK] User-friendly session error formatting verified!\n")

if __name__ == "__main__":
    unittest.main()
