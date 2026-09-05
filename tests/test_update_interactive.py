"""
Unit and Integration Tests for AnyContext Interactive /update & Auto-Restart Architecture.
100% native unittest.TestCase without pytest dependencies.
"""
import os
import sys
import unittest

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.core.services.update_service import UpdateService
from any_context.core.interaction.options_engine import OptionsEngine
from any_context.commands.dispatcher import dispatch_command


class TestUpdateInteractive(unittest.TestCase):
    def test_update_service_query_and_instances(self):
        svc = UpdateService()
        current = svc.get_current_version()
        self.assertIsNotNone(current)
        instances = svc.find_active_instances()
        self.assertIsInstance(instances, list)

    def test_options_engine_update_options(self):
        from unittest.mock import patch
        engine = OptionsEngine()
        with patch.object(UpdateService, "find_active_instances", return_value=[{"pid": 9999, "name": "actx.exe"}]):
            opts = engine.get_update_options(target_version="0.28.16")
            self.assertEqual(opts.type, "update")
            self.assertIn("0.28.16", opts.title)
            self.assertEqual(len(opts.items), 3)
            self.assertEqual(opts.items[0].id, "background")
            self.assertEqual(opts.items[1].id, "close")
            self.assertIn("Close all AnyContext sessions", opts.items[1].title)
            self.assertEqual(opts.items[2].id, "cancel")

    def test_options_engine_update_options_zero_instances(self):
        from unittest.mock import patch
        engine = OptionsEngine()
        with patch.object(UpdateService, "find_active_instances", return_value=[]):
            opts = engine.get_update_options(target_version="0.28.16")
            self.assertEqual(opts.type, "update")
            self.assertIn("0.28.16", opts.title)
            self.assertEqual(len(opts.items), 3)
            self.assertEqual(opts.items[0].id, "background")
            self.assertEqual(opts.items[1].id, "close")
            self.assertIn("Close session and update now", opts.items[1].title)
            self.assertEqual(opts.items[2].id, "cancel")

    def test_options_engine_cancel_update(self):
        engine = OptionsEngine()
        res = engine.execute_update_option("cancel")
        self.assertTrue(res.success)
        self.assertIn("cancelled", res.message.lower())

    def test_dispatcher_check_update_command(self):
        res = dispatch_command("/check-update", active_workspace="Default")
        self.assertTrue(res.success)
        self.assertIn("AnyContext", res.message)

    def test_options_engine_execute_close_update(self):
        from unittest.mock import patch
        engine = OptionsEngine()
        with patch.object(UpdateService, "execute_binary_update", return_value=(True, "Updated successfully", {"action": "exit_update", "version": "v0.28.82"})):
            res = engine.execute_update_option("close", is_tui=True)
            self.assertTrue(res.success)
            self.assertEqual(res.state_updates.get("action"), "exit_update")

    def test_options_engine_execute_background_update(self):
        from unittest.mock import patch
        engine = OptionsEngine()
        with patch.object(UpdateService, "execute_binary_update", return_value=(True, "Updated in bg", {"action": "none", "version": "v0.28.82"})):
            res = engine.execute_update_option("background", is_tui=True)
            self.assertTrue(res.success)
            self.assertEqual(res.state_updates.get("action"), "none")

    def test_update_service_download_failure_safeguard(self):
        from unittest.mock import patch
        svc = UpdateService()
        with patch.object(svc, "fetch_latest_release_tag", return_value="v0.28.99"):
            with patch("urllib.request.urlopen", side_effect=Exception("Network error")):
                with patch("subprocess.run", return_value=type("Res", (), {"returncode": 1, "stdout": ""})()):
                    with patch.object(svc, "close_active_instances") as mock_close:
                        success, msg, updates = svc.execute_binary_update(target_tag="0.28.99", auto_close_instances=True)
                        self.assertFalse(success)
                        mock_close.assert_not_called()

    def test_update_service_auto_close_exit_update(self):
        import io
        import zipfile
        import tempfile
        from unittest.mock import patch, MagicMock
        svc = UpdateService()
        with tempfile.TemporaryDirectory() as tmpdir:
            orig_env = os.environ.get("ACTX_UPDATE_DIR")
            os.environ["ACTX_UPDATE_DIR"] = tmpdir
            try:
                zip_buf = io.BytesIO()
                with zipfile.ZipFile(zip_buf, "w") as zf:
                    zf.writestr("actx-core.exe", b"test_core_binary")
                    zf.writestr("_internal/test.dll", b"test_dll")
                valid_zip = zip_buf.getvalue()

                mock_resp = MagicMock()
                mock_resp.status = 200
                mock_resp.read.side_effect = [valid_zip, b""]
                with patch("urllib.request.urlopen", return_value=MagicMock(__enter__=MagicMock(return_value=mock_resp))):
                    with patch("subprocess.Popen"):
                        with patch.object(svc, "close_active_instances", return_value=2):
                            with patch.object(svc, "find_active_instances", return_value=[{"pid": 1234, "name": "actx.exe"}]):
                                success, msg, updates = svc.execute_binary_update(
                                    target_tag="0.28.82",
                                    auto_close_instances=True,
                                    is_tui=True
                                )
                                self.assertTrue(success)
                                self.assertEqual(updates.get("action"), "exit_update")
                                self.assertEqual(updates.get("version"), "v0.28.82")
            finally:
                if orig_env is None:
                    os.environ.pop("ACTX_UPDATE_DIR", None)
                else:
                    os.environ["ACTX_UPDATE_DIR"] = orig_env

    def test_update_service_fallback_to_single_binary(self):
        import tempfile
        from unittest.mock import patch, MagicMock
        svc = UpdateService()
        with tempfile.TemporaryDirectory() as tmpdir:
            orig_env = os.environ.get("ACTX_UPDATE_DIR")
            os.environ["ACTX_UPDATE_DIR"] = tmpdir
            try:
                # First call (archive) raises 404, second call (single binary) succeeds
                mock_resp = MagicMock()
                mock_resp.status = 200
                mock_resp.read.side_effect = [b"fallback_bin_data", b""]

                def side_effect(req, *args, **kwargs):
                    url = req.full_url if hasattr(req, "full_url") else str(req)
                    if url.endswith(".zip") or url.endswith(".tar.gz"):
                        raise Exception("404 Not Found")
                    m = MagicMock()
                    m.__enter__.return_value = mock_resp
                    return m

                with patch("urllib.request.urlopen", side_effect=side_effect):
                    with patch("subprocess.Popen"):
                        success, msg, updates = svc.execute_binary_update(
                            target_tag="0.28.82",
                            auto_close_instances=False
                        )
                        self.assertTrue(success)
                        self.assertIn("v0.28.82", msg)
            finally:
                if orig_env is None:
                    os.environ.pop("ACTX_UPDATE_DIR", None)
                else:
                    os.environ["ACTX_UPDATE_DIR"] = orig_env


if __name__ == "__main__":
    unittest.main()
