"""
Unit and Integration Tests for AnyContext Interactive /update & Auto-Restart Architecture.
"""
import pytest
from any_context.core.services.update_service import UpdateService
from any_context.core.interaction.options_engine import OptionsEngine
from any_context.commands.dispatcher import dispatch_command


def test_update_service_query_and_instances():
    svc = UpdateService()
    current = svc.get_current_version()
    assert current is not None
    instances = svc.find_active_instances()
    assert isinstance(instances, list)


def test_options_engine_update_options():
    engine = OptionsEngine()
    opts = engine.get_update_options(target_version="0.28.16")
    assert opts.type == "update"
    assert "0.28.16" in opts.title
    assert len(opts.items) == 3
    assert opts.items[0].id == "background"
    assert opts.items[1].id == "close"
    assert opts.items[2].id == "cancel"


def test_options_engine_cancel_update():
    engine = OptionsEngine()
    res = engine.execute_update_option("cancel")
    assert res.success is True
    assert "cancelled" in res.message.lower()


def test_dispatcher_check_update_command():
    res = dispatch_command("/check-update", active_workspace="Default")
    assert res.success is True
    assert "AnyContext" in res.message


def test_dispatcher_update_command_modal():
    res = dispatch_command("/update@0.28.99", active_workspace="Default")
    assert res.success is True
    assert res.action == "open_update_modal"
