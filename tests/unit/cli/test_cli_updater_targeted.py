import os
import sys
import unittest
import tempfile
from unittest.mock import patch, MagicMock

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.cli.updater import (
    normalize_version_tag,
    parse_version_tuple,
    fetch_available_releases,
    display_available_releases,
    run_self_update
)
from tests.e2e_helpers import safe_stdout_write

class TestTargetedUpdater(unittest.TestCase):
    """
    Unit Test Suite: Validates Targeted Version Updates (@0.15.2), Rollbacks, and Release Listing.
    """

    def test_01_normalize_version_tag(self):
        """Validates that normalize_version_tag handles @, v, and latest prefixes correctly."""
        safe_stdout_write("\n>>> [UPDATER UNIT] Testing normalize_version_tag...\n")
        self.assertEqual(normalize_version_tag("@0.15.2"), "v0.15.2")
        self.assertEqual(normalize_version_tag("0.15.2"), "v0.15.2")
        self.assertEqual(normalize_version_tag("v0.15.2"), "v0.15.2")
        self.assertEqual(normalize_version_tag("@latest"), "latest")
        self.assertEqual(normalize_version_tag("latest"), "latest")
        self.assertEqual(normalize_version_tag("v1.0.0"), "v1.0.0")
        safe_stdout_write("  [OK] Version normalization parses all formats cleanly!\n")

    def test_02_parse_version_tuple(self):
        """Validates version tuple integer comparison."""
        safe_stdout_write(">>> [UPDATER UNIT] Testing parse_version_tuple...\n")
        self.assertEqual(parse_version_tuple("0.15.2"), (0, 15, 2))
        self.assertEqual(parse_version_tuple("v0.15.6"), (0, 15, 6))
        self.assertTrue(parse_version_tuple("0.15.6") > parse_version_tuple("0.15.2"))
        self.assertTrue(parse_version_tuple("0.14.0") < parse_version_tuple("0.15.0"))
        safe_stdout_write("  [OK] Version tuple comparison verified!\n")

    @patch("any_context.cli.updater.fetch_available_releases")
    def test_03_display_available_releases(self, mock_fetch):
        """Validates formatting of available release tags from GitHub."""
        safe_stdout_write(">>> [UPDATER UNIT] Testing display_available_releases...\n")
        mock_fetch.return_value = [
            {"tag": "v0.15.6", "name": "Release v0.15.6", "published_at": "2026-08-21", "prerelease": False, "body": ""},
            {"tag": "v0.15.2", "name": "Release v0.15.2", "published_at": "2026-08-20", "prerelease": False, "body": ""}
        ]
        res = display_available_releases(interactive_select=False)
        self.assertIsNone(res)
        safe_stdout_write("  [OK] Release listing rendered cleanly!\n")

    @patch("urllib.request.urlopen")
    def test_04_run_self_update_targeted_download(self, mock_urlopen):
        """Validates that targeted version triggers download for specific tag."""
        safe_stdout_write(">>> [UPDATER UNIT] Testing run_self_update for targeted tag...\n")
        mock_resp = MagicMock()
        mock_resp.status = 200
        mock_resp.headers = {"Content-Length": "100"}
        mock_resp.read.side_effect = [b"mock_binary_data", b""]
        mock_urlopen.return_value.__enter__.return_value = mock_resp

        with patch("sys.exit"):
            with patch("subprocess.Popen"):
                run_self_update(target_version="0.15.2", force=True, force_background=True)
                mock_urlopen.assert_called()
                call_arg = mock_urlopen.call_args[0][0]
                self.assertIn("v0.15.2", call_arg.full_url)
        safe_stdout_write("  [OK] Targeted download for release tag verified!\n")

if __name__ == "__main__":
    unittest.main()
