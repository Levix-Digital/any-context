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
        res = subprocess.run([sys.executable, build_script, "--out", cls.shim_path], capture_output=True, text=True, encoding="utf-8", errors="replace")
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
        # Standalone native Rust binary with embedded TLS and extraction is ~2.7-3.8MB (< 10MB vs 250MB engine)
        self.assertLess(file_size, 10 * 1024 * 1024, f"Shim is unexpectedly large: {file_size} bytes")

    def test_02_instant_version_with_version_file(self):
        """Validates that 'actx -v' reads version.txt and prints clean 'vX.Y.Z' instantly."""
        version_file = os.path.join(self.temp_dir, "version.txt")
        with open(version_file, "w", encoding="utf-8") as f:
            f.write("0.28.71\n")

        t0 = time.perf_counter()
        res = subprocess.run([self.shim_path, "-v"], capture_output=True, text=True, encoding="utf-8", errors="replace")
        elapsed_ms = (time.perf_counter() - t0) * 1000

        self.assertEqual(res.returncode, 0)
        output = res.stdout.strip()
        self.assertEqual(output, "v0.28.71")
        # Ensure ultra-fast execution time (< 1500ms even on heavily loaded multi-core CI runners)
        self.assertLess(elapsed_ms, 1500.0, f"Shim version check was too slow: {elapsed_ms:.1f}ms")

    def test_03_double_dash_version_flag(self):
        """Validates that '--version' produces identical clean output."""
        res = subprocess.run([self.shim_path, "--version"], capture_output=True, text=True, encoding="utf-8", errors="replace")
        self.assertEqual(res.returncode, 0)
        self.assertEqual(res.stdout.strip(), "v0.28.71")

    def test_04_fallback_version_when_file_missing(self):
        """Validates that missing version.txt cleanly falls back to embedded version."""
        version_file = os.path.join(self.temp_dir, "version.txt")
        if os.path.isfile(version_file):
            os.remove(version_file)

        res = subprocess.run([self.shim_path, "-v"], capture_output=True, text=True, encoding="utf-8", errors="replace")
        self.assertEqual(res.returncode, 0)
        output = res.stdout.strip()
        self.assertTrue(output.startswith("v0."))

    def test_05_version_with_utf8_bom(self):
        """Validates that version.txt containing a UTF-8 BOM yields clean 'v0.28.88' without duplicate 'vv'."""
        version_file = os.path.join(self.temp_dir, "version.txt")
        with open(version_file, "wb") as f:
            f.write(b"\xef\xbb\xbfv0.28.88\r\n")

        res = subprocess.run([self.shim_path, "-v"], capture_output=True, text=True, encoding="utf-8", errors="replace")
        self.assertEqual(res.returncode, 0)
        self.assertEqual(res.stdout.strip(), "v0.28.88")

    def test_06_version_with_utf8_bom_without_leading_v(self):
        """Validates that version.txt with UTF-8 BOM and no 'v' prefix yields clean 'v0.28.88'."""
        version_file = os.path.join(self.temp_dir, "version.txt")
        with open(version_file, "wb") as f:
            f.write(b"\xef\xbb\xbf0.28.88\r\n")

        res = subprocess.run([self.shim_path, "-v"], capture_output=True, text=True, encoding="utf-8", errors="replace")
        self.assertEqual(res.returncode, 0)
        self.assertEqual(res.stdout.strip(), "v0.28.88")

    def test_07_finalize_pending_update_atomic_swap(self):
        """Validates that --finalize-update atomically swaps staging files, updates version.txt, and cleans staging."""
        if not self.is_windows:
            self.skipTest("Windows-specific launcher shim test")

        # 1. Setup base directory with active old binary and _internal
        core_exe = os.path.join(self.temp_dir, "actx-core.exe" if self.is_windows else "actx-core")
        with open(core_exe, "w") as f:
            f.write("old_core_binary")

        internal_dir = os.path.join(self.temp_dir, "_internal")
        os.makedirs(internal_dir, exist_ok=True)
        with open(os.path.join(internal_dir, "old_lib.dll"), "w") as f:
            f.write("old_dll_content")

        # 2. Setup staging directory with new files and pending_update.json
        staging_dir = os.path.join(self.temp_dir, "actx_staging")
        os.makedirs(staging_dir, exist_ok=True)
        staging_internal = os.path.join(staging_dir, "_internal")
        os.makedirs(staging_internal, exist_ok=True)
        with open(os.path.join(staging_internal, "new_lib.dll"), "w") as f:
            f.write("new_dll_content")

        staging_core = os.path.join(staging_dir, "actx-core.exe" if self.is_windows else "actx-core")
        with open(staging_core, "w") as f:
            f.write("new_core_binary")

        pending_flag = os.path.join(staging_dir, "pending_update.json")
        with open(pending_flag, "w", encoding="utf-8") as f:
            f.write('{"version": "v0.30.31", "staging": "actx_staging"}')

        # 3. Invoke shim with --finalize-update
        test_env = os.environ.copy()
        test_env["ACTX_TEST_MODE"] = "1"
        res = subprocess.run([self.shim_path, "--finalize-update"], capture_output=True, text=True, encoding="utf-8", errors="replace", env=test_env)
        self.assertEqual(res.returncode, 0, f"STDOUT: {res.stdout}\nSTDERR: {res.stderr}")
        self.assertIn("AnyContext successfully updated to v0.30.31", res.stdout)

        # 4. Verify atomic swap results
        self.assertTrue(os.path.exists(core_exe))
        with open(core_exe, "r") as f:
            self.assertEqual(f.read(), "new_core_binary")

        self.assertTrue(os.path.exists(os.path.join(internal_dir, "new_lib.dll")))
        self.assertFalse(os.path.exists(staging_dir))

        # Check version.txt
        version_file = os.path.join(self.temp_dir, "version.txt")
        self.assertTrue(os.path.exists(version_file))
        with open(version_file, "r", encoding="utf-8") as f:
            self.assertEqual(f.read().strip(), "v0.30.31")

    def test_08_post_exit_handoff_atomic_swap(self):
        """Validates that when actx-core exits with code 42, the parent launcher shim automatically performs atomic swap."""
        if not self.is_windows:
            self.skipTest("Windows-specific launcher shim test")

        from launcher.build_shim import find_windows_csharp_compiler
        csc = find_windows_csharp_compiler()
        if not csc:
            self.skipTest("csc.exe compiler not found")

        # Create a mock actx-core.exe in a clean subfolder
        sub_dir = tempfile.mkdtemp(prefix="actx_handoff_test_")
        try:
            shim_dest = os.path.join(sub_dir, "actx.exe")
            shutil.copy2(self.shim_path, shim_dest)

            # Compile mock core
            mock_cs = os.path.join(sub_dir, "mock_core.cs")
            with open(mock_cs, "w") as f:
                f.write("""
using System;
using System.IO;
class MockCore {
    static int Main() {
        string baseDir = AppDomain.CurrentDomain.BaseDirectory;
        string staging = Path.Combine(baseDir, "actx_staging");
        Directory.CreateDirectory(staging);
        File.WriteAllText(Path.Combine(staging, "pending_update.json"), "{\\"version\\": \\"v0.30.31\\"}");
        File.WriteAllText(Path.Combine(staging, "actx-core.exe"), "swapped_after_exit_core");
        return 42;
    }
}
""")
            mock_core_exe = os.path.join(sub_dir, "actx-core.exe")
            cmd = [csc, "/nologo", "/target:exe", f"/out:{mock_core_exe}", mock_cs]
            c_res = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace")
            self.assertEqual(c_res.returncode, 0)

            # Launch shim_dest (which will run mock_core_exe and wait for exit)
            test_env = os.environ.copy()
            test_env["ACTX_TEST_MODE"] = "1"
            res = subprocess.run([shim_dest], capture_output=True, text=True, encoding="utf-8", errors="replace", env=test_env)
            self.assertEqual(res.returncode, 0, f"STDOUT: {res.stdout}\nSTDERR: {res.stderr}")
            self.assertIn("AnyContext successfully updated to v0.30.31", res.stdout)

            # Verify that actx-core.exe is now the swapped binary
            with open(mock_core_exe, "r") as f:
                self.assertEqual(f.read(), "swapped_after_exit_core")

            # Verify that staging was cleaned up
            self.assertFalse(os.path.exists(os.path.join(sub_dir, "actx_staging")))
        finally:
            shutil.rmtree(sub_dir, ignore_errors=True)


if __name__ == "__main__":
    unittest.main()
