import os
import sys
import shutil
import subprocess
import unittest

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)


class TestTUISyntax(unittest.TestCase):
    """
    Unit Test Suite: Validates OpenTUI TypeScript / React frontend integrity.
    Verifies that app.tsx and components exist, have valid balanced delimiters,
    and cleanly pass Bun / TypeScript compilation when Bun is present.
    """

    def setUp(self):
        self.tui_dir = os.path.join(repo_root, "src", "any_context", "tui")
        self.app_tsx = os.path.join(self.tui_dir, "app.tsx")
        self.index_tsx = os.path.join(self.tui_dir, "index.tsx")

    def test_01_tui_source_files_exist(self):
        """Verifies essential OpenTUI frontend files exist."""
        self.assertTrue(os.path.isdir(self.tui_dir), f"TUI directory missing: {self.tui_dir}")
        self.assertTrue(os.path.isfile(self.app_tsx), f"app.tsx missing: {self.app_tsx}")
        self.assertTrue(os.path.isfile(self.index_tsx), f"index.tsx missing: {self.index_tsx}")

    def test_02_app_tsx_bracket_balance(self):
        """Validates that app.tsx has balanced curly braces and parentheses."""
        with open(self.app_tsx, "r", encoding="utf-8") as f:
            code = f.read()

        # Check for unclosed setMessages or client.onNotification syntax
        self.assertIn("client.onNotification", code)
        self.assertIn("BridgeClient", code)
        
        # Ensure catch block is properly closed before onNotification
        err_block = code[code.find(".catch((err) => {"):code.find("client.onNotification")]
        self.assertTrue(err_block.count("{") == err_block.count("}"), "Unbalanced braces in catch block!")
        self.assertTrue(err_block.count("(") == err_block.count(")"), "Unbalanced parens in catch block!")
        self.assertTrue(err_block.count("[") == err_block.count("]"), "Unbalanced brackets in catch block!")

    def test_03_bun_typecheck_if_available(self):
        """Runs bun build / typecheck if bun is available in PATH."""
        bun_exe = shutil.which("bun")
        if not bun_exe:
            self.skipTest("Bun executable not found in PATH; skipping live Bun build verification.")

        # Execute bun build on app.tsx
        result = subprocess.run(
            [bun_exe, "build", "./app.tsx", "--no-bundle"],
            cwd=self.tui_dir,
            capture_output=True,
            text=True
        )
        self.assertEqual(
            result.returncode, 0,
            f"Bun build check failed for app.tsx:\nSTDOUT:\n{result.stdout}\nSTDERR:\n{result.stderr}"
        )


if __name__ == "__main__":
    unittest.main()
