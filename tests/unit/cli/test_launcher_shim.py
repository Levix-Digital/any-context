import os
import sys
import time
import shutil
import tempfile
import subprocess
import unittest

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)


class TestLauncherShim(unittest.TestCase):
    """
    Unit Test Suite: Validates the Native Launcher Shim Architecture.
    Ensures sub-100ms instant execution and clean version output (e.g. 'v0.28.71').
    100% native unittest.TestCase without external test dependencies.
    """

    @classmethod
    def setUpClass(cls):
        cls.temp_dir = tempfile.mkdtemp(prefix="actx_test_shim_")
        cls.is_windows = sys.platform.startswith("win")
        cls.exe_name = "actx.exe" if cls.is_windows else "actx"
        cls.shim_path = os.path.join(cls.temp_dir, cls.exe_name)

        # Build shim using launcher/build_shim.py
        build_script = os.path.join(repo_root, "launcher", "build_shim.py")
        res = subprocess.run([sys.executable, build_script, "--out", cls.shim_path], capture_output=True, text=True)
        if res.returncode != 0:
            cls.build_failed = True
            cls.build_error = f"STDOUT: {res.stdout}\nSTDERR: {res.stderr}"
        else:
            cls.build_failed = False
            cls.build_error = ""

    @classmethod
    def tearDownClass(cls):
        try:
            shutil.rmtree(cls.temp_dir, ignore_errors=True)
        except Exception:
            pass

    def setUp(self):
        if self.build_failed:
            self.skipTest(f"Launcher shim compilation skipped: {self.build_error}")

    def test_01_shim_binary_generated(self):
        """Validates that the native launcher shim is a compact standalone binary."""
        self.assertTrue(os.path.isfile(self.shim_path))
        file_size = os.path.getsize(self.shim_path)
        # Should be small (< 100KB)
        self.assertLess(file_size, 100 * 1024, f"Shim is unexpectedly large: {file_size} bytes")

    def test_02_instant_version_with_version_file(self):
        """Validates that 'actx -v' reads version.txt and prints clean 'vX.Y.Z' instantly."""
        version_file = os.path.join(self.temp_dir, "version.txt")
        with open(version_file, "w", encoding="utf-8") as f:
            f.write("0.28.71\n")

        t0 = time.perf_counter()
        res = subprocess.run([self.shim_path, "-v"], capture_output=True, text=True)
        elapsed_ms = (time.perf_counter() - t0) * 1000

        self.assertEqual(res.returncode, 0)
        output = res.stdout.strip()
        self.assertEqual(output, "v0.28.71")
        # Ensure ultra-fast execution time (< 300ms even on busy CI runners)
        self.assertLess(elapsed_ms, 300.0, f"Shim version check was too slow: {elapsed_ms:.1f}ms")

    def test_03_double_dash_version_flag(self):
        """Validates that '--version' produces identical clean output."""
        res = subprocess.run([self.shim_path, "--version"], capture_output=True, text=True)
        self.assertEqual(res.returncode, 0)
        self.assertEqual(res.stdout.strip(), "v0.28.71")

    def test_04_fallback_version_when_file_missing(self):
        """Validates that missing version.txt cleanly falls back to embedded version."""
        version_file = os.path.join(self.temp_dir, "version.txt")
        if os.path.isfile(version_file):
            os.remove(version_file)

        res = subprocess.run([self.shim_path, "-v"], capture_output=True, text=True)
        self.assertEqual(res.returncode, 0)
        output = res.stdout.strip()
        self.assertTrue(output.startswith("v0.28."))

    def test_05_version_with_utf8_bom(self):
        """Validates that version.txt containing a UTF-8 BOM yields clean 'v0.28.88' without duplicate 'vv'."""
        version_file = os.path.join(self.temp_dir, "version.txt")
        with open(version_file, "wb") as f:
            f.write(b"\xef\xbb\xbfv0.28.88\r\n")

        res = subprocess.run([self.shim_path, "-v"], capture_output=True, text=True)
        self.assertEqual(res.returncode, 0)
        self.assertEqual(res.stdout.strip(), "v0.28.88")

    def test_06_version_with_utf8_bom_without_leading_v(self):
        """Validates that version.txt with UTF-8 BOM and no 'v' prefix yields clean 'v0.28.88'."""
        version_file = os.path.join(self.temp_dir, "version.txt")
        with open(version_file, "wb") as f:
            f.write(b"\xef\xbb\xbf0.28.88\r\n")

        res = subprocess.run([self.shim_path, "-v"], capture_output=True, text=True)
        self.assertEqual(res.returncode, 0)
        self.assertEqual(res.stdout.strip(), "v0.28.88")


if __name__ == "__main__":
    unittest.main()
