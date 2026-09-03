import os
import sys
import time
import unittest

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)


class TestStartupPerformance(unittest.TestCase):
    """
    Performance verification for critical subsystem imports.
    100% native unittest.TestCase without pytest dependencies.
    """

    def test_core_services_import_speed(self):
        t0 = time.perf_counter()
        import any_context.core.services
        t1 = time.perf_counter()
        elapsed_ms = (t1 - t0) * 1000
        # Core services should import fast (< 500ms) thanks to lazy loading
        self.assertLess(elapsed_ms, 500.0, f"Core services import too slow: {elapsed_ms:.1f}ms")

    def test_command_dispatcher_import_speed(self):
        t0 = time.perf_counter()
        from any_context.commands.dispatcher import CommandDispatcher
        t1 = time.perf_counter()
        elapsed_ms = (t1 - t0) * 1000
        self.assertLess(elapsed_ms, 500.0, f"Dispatcher import too slow: {elapsed_ms:.1f}ms")

    def test_entrypoint_import_speed(self):
        t0 = time.perf_counter()
        import any_context.cli.entrypoint
        t1 = time.perf_counter()
        elapsed_ms = (t1 - t0) * 1000
        self.assertLess(elapsed_ms, 500.0, f"Entrypoint import too slow: {elapsed_ms:.1f}ms")


if __name__ == "__main__":
    unittest.main()
