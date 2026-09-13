"""
Unit Tests for Anti-Virtualenv Safeguard in Binary Target Directory Resolution.
Ensures that running updates inside a development environment (.venv) never
corrupts the virtualenv with standalone compiled binaries.
"""
import os
import sys
import tempfile
import unittest
from unittest.mock import patch

from any_context.config.paths import (
    get_canonical_bin_dir,
    is_virtualenv_dir,
    resolve_binary_target_dir,
)


class TestBinaryTargetResolver(unittest.TestCase):

    def test_01_is_virtualenv_detection(self):
        """Validates virtual environment path detection."""
        with tempfile.TemporaryDirectory() as td:
            # Empty dir is not venv
            self.assertFalse(is_virtualenv_dir(td))

            # Dir with python.exe is venv
            py_exe = os.path.join(td, "python.exe")
            with open(py_exe, "w") as f:
                f.write("")
            self.assertTrue(is_virtualenv_dir(td))
            os.remove(py_exe)

            # Dir with pyvenv.cfg in parent is venv
            scripts_dir = os.path.join(td, "Scripts")
            os.makedirs(scripts_dir, exist_ok=True)
            with open(os.path.join(td, "pyvenv.cfg"), "w") as f:
                f.write("home = C:\\Python\n")
            self.assertTrue(is_virtualenv_dir(scripts_dir))

    def test_02_resolve_rejects_virtualenv(self):
        """Validates that resolve_binary_target_dir refuses to target a virtualenv directory."""
        with tempfile.TemporaryDirectory() as td:
            venv_scripts = os.path.join(td, ".venv", "Scripts")
            os.makedirs(venv_scripts, exist_ok=True)
            fake_actx = os.path.join(venv_scripts, "actx.exe")
            with open(fake_actx, "w") as f:
                f.write("")
            with open(os.path.join(td, ".venv", "pyvenv.cfg"), "w") as f:
                f.write("")

            with patch("shutil.which", return_value=fake_actx), \
                 patch.dict(os.environ, {}, clear=True):
                # Should NOT return venv_scripts!
                resolved = resolve_binary_target_dir(is_windows=True)
                self.assertNotEqual(resolved, os.path.abspath(venv_scripts))
                self.assertEqual(resolved, get_canonical_bin_dir())

    def test_03_resolve_respects_actx_update_dir(self):
        """Validates explicit ACTX_UPDATE_DIR override."""
        with tempfile.TemporaryDirectory() as td:
            with patch.dict(os.environ, {"ACTX_UPDATE_DIR": td}):
                resolved = resolve_binary_target_dir()
                self.assertEqual(resolved, os.path.abspath(td))


if __name__ == "__main__":
    unittest.main()
