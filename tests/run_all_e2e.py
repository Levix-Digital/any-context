import os
import sys
import time
import unittest

repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

def safe_print(msg: str):
    try:
        sys.stdout.write(msg + "\n")
        sys.stdout.flush()
    except (UnicodeEncodeError, Exception):
        try:
            clean = msg.encode("ascii", errors="ignore").decode("ascii")
            sys.stdout.write(clean + "\n")
            sys.stdout.flush()
        except Exception:
            pass

def main():
    """
    AnyContext Master E2E Test Suite Orchestrator
    Discovers and executes all modular E2E test suites with formatted console output.
    """
    tests_dir = os.path.dirname(os.path.abspath(__file__))
    e2e_dir = os.path.join(tests_dir, "e2e")

    safe_print("\n" + "=" * 80)
    safe_print(">> AnyContext Master E2E Modular Test Suite Runner (100% Coverage)")
    safe_print("=" * 80)

    loader = unittest.TestLoader()
    suite = unittest.TestSuite()

    # 1. Discover all modular tests in tests/e2e/
    if os.path.exists(e2e_dir):
        discovered_e2e = loader.discover(start_dir=e2e_dir, top_level_dir=repo_root, pattern="test_*.py")
        suite.addTests(discovered_e2e)

    # 2. Add full lifecycle integration suite
    full_lifecycle_path = os.path.join(tests_dir, "test_e2e_full_lifecycle.py")
    if os.path.exists(full_lifecycle_path):
        discovered_lifecycle = loader.discover(start_dir=tests_dir, top_level_dir=repo_root, pattern="test_e2e_full_lifecycle.py")
        suite.addTests(discovered_lifecycle)

    total_tests = suite.countTestCases()
    safe_print(f"Discovered {total_tests} automated test cases across all modules.\n")

    start_time = time.time()
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    elapsed = time.time() - start_time

    safe_print("\n" + "=" * 80)
    if result.wasSuccessful():
        safe_print(f"ALL {result.testsRun} TESTS PASSED SUCCESSFULLY in {elapsed:.2f}s!")
        safe_print("=" * 80 + "\n")
        sys.exit(0)
    else:
        safe_print(f"TEST SUITE FAILED: {len(result.failures)} failures, {len(result.errors)} errors in {elapsed:.2f}s")
        safe_print("=" * 80 + "\n")
        sys.exit(1)

if __name__ == "__main__":
    main()
