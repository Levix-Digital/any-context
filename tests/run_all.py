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
    AnyContext Master Unified Test Suite Orchestrator
    Executes separated layers:
      1. Unit Tests - Core (Ingestion, RAG, Memory, Auth, System)
      2. Unit Tests - CLI / UI (Command Dispatch, Help, History, Terminal Safety)
      3. E2E Tests - (REST API, MCP Server, Full Lifecycle)
    """
    tests_dir = os.path.dirname(os.path.abspath(__file__))
    unit_core_dir = os.path.join(tests_dir, "unit", "core")
    unit_cli_dir = os.path.join(tests_dir, "unit", "cli")
    unit_server_dir = os.path.join(tests_dir, "unit", "server")
    unit_ingestion_dir = os.path.join(tests_dir, "unit", "ingestion")
    e2e_dir = os.path.join(tests_dir, "e2e")

    safe_print("\n" + "=" * 80)
    safe_print(">> AnyContext Master Test Suite (Core Unit + CLI UI Unit + Server Unit + Ingestion Unit + E2E)")
    safe_print("=" * 80)

    loader = unittest.TestLoader()
    suite = unittest.TestSuite()

    # 1. Discover Core Unit Tests
    if os.path.exists(unit_core_dir):
        discovered_core = loader.discover(start_dir=unit_core_dir, top_level_dir=repo_root, pattern="test_*.py")
        suite.addTests(discovered_core)

    # 2. Discover CLI / UI Unit Tests
    if os.path.exists(unit_cli_dir):
        discovered_cli = loader.discover(start_dir=unit_cli_dir, top_level_dir=repo_root, pattern="test_*.py")
        suite.addTests(discovered_cli)

    # 3. Discover Server Unit Tests
    if os.path.exists(unit_server_dir):
        discovered_server = loader.discover(start_dir=unit_server_dir, top_level_dir=repo_root, pattern="test_*.py")
        suite.addTests(discovered_server)

    # 4. Discover Ingestion Unit Tests
    if os.path.exists(unit_ingestion_dir):
        discovered_ingestion = loader.discover(start_dir=unit_ingestion_dir, top_level_dir=repo_root, pattern="test_*.py")
        suite.addTests(discovered_ingestion)

    # 5. Discover E2E Server & Protocol Tests
    if os.path.exists(e2e_dir):
        discovered_e2e = loader.discover(start_dir=e2e_dir, top_level_dir=repo_root, pattern="test_*.py")
        suite.addTests(discovered_e2e)

    # 4. Add full lifecycle integration master test
    full_lifecycle_path = os.path.join(tests_dir, "test_e2e_full_lifecycle.py")
    if os.path.exists(full_lifecycle_path):
        discovered_lifecycle = loader.discover(start_dir=tests_dir, top_level_dir=repo_root, pattern="test_e2e_full_lifecycle.py")
        suite.addTests(discovered_lifecycle)

    # 5. Add interactive update tests
    update_interactive_path = os.path.join(tests_dir, "test_update_interactive.py")
    if os.path.exists(update_interactive_path):
        discovered_update = loader.discover(start_dir=tests_dir, top_level_dir=repo_root, pattern="test_update_interactive.py")
        suite.addTests(discovered_update)

    total_tests = suite.countTestCases()
    safe_print(f"Discovered {total_tests} automated test cases across Core, CLI, and E2E layers.\n")

    import tempfile
    import shutil

    temp_sandbox_dir = tempfile.mkdtemp(prefix="actx_test_sandbox_")
    test_db_path = os.path.join(temp_sandbox_dir, "test_settings.db")
    test_context_dir = os.path.join(temp_sandbox_dir, "context_db")
    test_memory_dir = os.path.join(temp_sandbox_dir, "memory")
    os.makedirs(test_context_dir, exist_ok=True)
    os.makedirs(test_memory_dir, exist_ok=True)

    orig_env_db = os.environ.get("ACTX_SETTINGS_DB")
    orig_env_ctx = os.environ.get("ACTX_CONTEXT_DB")
    orig_env_mem = os.environ.get("ACTX_MEMORY_DB")
    orig_test_mode = os.environ.get("ACTX_TEST_MODE")

    os.environ["ACTX_SETTINGS_DB"] = test_db_path
    os.environ["ACTX_CONTEXT_DB"] = test_context_dir
    os.environ["ACTX_MEMORY_DB"] = test_memory_dir
    os.environ["ACTX_TEST_MODE"] = "1"

    try:
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
    finally:
        if orig_env_db is not None:
            os.environ["ACTX_SETTINGS_DB"] = orig_env_db
        else:
            os.environ.pop("ACTX_SETTINGS_DB", None)
        if orig_env_ctx is not None:
            os.environ["ACTX_CONTEXT_DB"] = orig_env_ctx
        else:
            os.environ.pop("ACTX_CONTEXT_DB", None)
        if orig_env_mem is not None:
            os.environ["ACTX_MEMORY_DB"] = orig_env_mem
        else:
            os.environ.pop("ACTX_MEMORY_DB", None)
        if orig_test_mode is not None:
            os.environ["ACTX_TEST_MODE"] = orig_test_mode
        else:
            os.environ.pop("ACTX_TEST_MODE", None)
        try:
            shutil.rmtree(temp_sandbox_dir, ignore_errors=True)
        except Exception:
            pass


if __name__ == "__main__":
    main()
