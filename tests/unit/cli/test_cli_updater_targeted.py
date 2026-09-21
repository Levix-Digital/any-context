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

        with tempfile.TemporaryDirectory() as tmp_dir:
            orig_env = os.environ.get("ACTX_UPDATE_DIR")
            os.environ["ACTX_UPDATE_DIR"] = tmp_dir
            try:
                with patch("sys.exit"):
                    with patch("subprocess.Popen"):
                        run_self_update(target_version="0.15.2", force=True, force_background=True)
                        mock_urlopen.assert_called()
                        call_arg = mock_urlopen.call_args[0][0]
                        self.assertIn("v0.15.2", call_arg.full_url)
            finally:
                if orig_env is None:
                    os.environ.pop("ACTX_UPDATE_DIR", None)
                else:
                    os.environ["ACTX_UPDATE_DIR"] = orig_env
        safe_stdout_write("  [OK] Targeted download for release tag verified!\n")

    def test_05_merge_directory_contents_overwrites_without_nesting(self):
        """Validates that merge_directory_contents merges files without nesting _internal inside _internal."""
        safe_stdout_write(">>> [UPDATER UNIT] Testing merge_directory_contents...\n")
        from any_context.cli.updater import merge_directory_contents

        with tempfile.TemporaryDirectory() as tmp_root:
            target_internal = os.path.join(tmp_root, "bin", "_internal")
            os.makedirs(target_internal, exist_ok=True)
            with open(os.path.join(target_internal, "python311.dll"), "w") as f:
                f.write("old_dll")
            with open(os.path.join(target_internal, "stale.txt"), "w") as f:
                f.write("stale_content")

            staging_internal = os.path.join(tmp_root, "staging", "_internal")
            os.makedirs(os.path.join(staging_internal, "any_context"), exist_ok=True)
            with open(os.path.join(staging_internal, "python311.dll"), "w") as f:
                f.write("new_dll")
            with open(os.path.join(staging_internal, "any_context", "__init__.py"), "w") as f:
                f.write("pkg_init")

            # Execute merge
            merge_directory_contents(staging_internal, target_internal)

            # Assertions
            self.assertTrue(os.path.exists(os.path.join(target_internal, "python311.dll")))
            with open(os.path.join(target_internal, "python311.dll"), "r") as f:
                self.assertEqual(f.read(), "new_dll")
            self.assertTrue(os.path.exists(os.path.join(target_internal, "any_context", "__init__.py")))

            # Critical assertion: NO nested _internal directory
            nested_internal = os.path.join(target_internal, "_internal")
            self.assertFalse(os.path.exists(nested_internal), "_internal must NOT be nested inside _internal!")
            self.assertFalse(os.path.exists(staging_internal), "staging_internal must be cleaned up after merge!")
            safe_stdout_write("  [OK] merge_directory_contents merges cleanly with zero nesting!\n")

    def test_06_cleanup_stale_internal_backups(self):
        """Validates that cleanup_stale_internal_backups removes all _internal_old* folders."""
        safe_stdout_write(">>> [UPDATER UNIT] Testing cleanup_stale_internal_backups...\n")
        from any_context.cli.updater import cleanup_stale_internal_backups

        with tempfile.TemporaryDirectory() as tmp_root:
            old_1 = os.path.join(tmp_root, "_internal_old")
            old_2 = os.path.join(tmp_root, "_internal_old_1726000000_1234")
            normal_dir = os.path.join(tmp_root, "_internal")
            os.makedirs(old_1, exist_ok=True)
            os.makedirs(old_2, exist_ok=True)
            os.makedirs(normal_dir, exist_ok=True)

            cleanup_stale_internal_backups(tmp_root)

            self.assertFalse(os.path.exists(old_1))
            self.assertFalse(os.path.exists(old_2))
            self.assertTrue(os.path.exists(normal_dir))
            safe_stdout_write("  [OK] cleanup_stale_internal_backups cleans all backup directories!\n")

if __name__ == "__main__":
    unittest.main()

