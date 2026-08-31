import time
import pytest


def test_core_services_import_speed():
    t0 = time.perf_counter()
    import any_context.core.services
    t1 = time.perf_counter()
    elapsed_ms = (t1 - t0) * 1000
    # Core services should import fast (< 500ms) thanks to lazy loading
    assert elapsed_ms < 500.0, f"Core services import too slow: {elapsed_ms:.1f}ms"


def test_command_dispatcher_import_speed():
    t0 = time.perf_counter()
    from any_context.commands.dispatcher import CommandDispatcher
    t1 = time.perf_counter()
    elapsed_ms = (t1 - t0) * 1000
    assert elapsed_ms < 500.0, f"Dispatcher import too slow: {elapsed_ms:.1f}ms"


def test_entrypoint_import_speed():
    t0 = time.perf_counter()
    import any_context.cli.entrypoint
    t1 = time.perf_counter()
    elapsed_ms = (t1 - t0) * 1000
    assert elapsed_ms < 500.0, f"Entrypoint import too slow: {elapsed_ms:.1f}ms"
