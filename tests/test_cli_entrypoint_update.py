import sys
import pytest
from unittest.mock import patch, MagicMock
from any_context.core.services.update_service import UpdateProgressTracker
from any_context.cli.entrypoint import entrypoint


def test_update_progress_tracker_lifecycle():
    tracker = UpdateProgressTracker.get_instance()
    tracker.reset()

    assert not tracker.is_updating
    assert tracker.stage == "idle"
    assert tracker.percent == 0

    # Start update
    tracker.set_starting("v0.30.17")
    assert tracker.is_updating
    assert tracker.stage == "downloading"
    assert tracker.target_tag == "v0.30.17"

    # Progress download (50%)
    tracker.update_download(50 * 1024 * 1024, 100 * 1024 * 1024)
    assert tracker.percent == 50
    assert "50.0/100.0MB" in tracker.update_info
    assert "50%" in tracker.update_info
    assert "[" in tracker.update_info

    # Extracting
    tracker.set_extracting()
    assert tracker.stage == "extracting"
    assert "Extracting" in tracker.update_info

    # Installing
    tracker.set_installing()
    assert tracker.stage == "installing"
    assert "Installing" in tracker.update_info

    # Completed
    tracker.set_completed("Update finished")
    assert not tracker.is_updating
    assert tracker.stage == "completed"
    assert tracker.update_info == "Update finished"

    # Status dict export
    d = tracker.get_status_dict()
    assert d["stage"] == "completed"
    assert not d["is_updating"]

    # Reset
    tracker.reset()
    assert tracker.stage == "idle"


def test_entrypoint_version_flag(capsys):
    with patch.object(sys, "argv", ["actx", "--version"]):
        with pytest.raises(SystemExit) as exc_info:
            entrypoint()
        assert exc_info.value.code == 0
        captured = capsys.readouterr()
        assert "AnyContext (actx) v" in captured.out
        assert "Levix Digital" in captured.out


def test_entrypoint_short_version_flag(capsys):
    with patch.object(sys, "argv", ["actx", "-v"]):
        with pytest.raises(SystemExit) as exc_info:
            entrypoint()
        assert exc_info.value.code == 0
        captured = capsys.readouterr()
        assert "AnyContext (actx) v" in captured.out


def test_entrypoint_update_flag_dispatches_without_tui():
    with patch.object(sys, "argv", ["actx", "--update"]):
        with patch("any_context.cli.workspace_selector.get_active_workspace") as mock_ws_sel:
            mock_ws_sel.side_effect = SystemExit(0)
            with patch("any_context.cli.tui_launcher.launch_opentui") as mock_tui:
                with pytest.raises(SystemExit) as exc_info:
                    entrypoint()
                assert exc_info.value.code == 0
                mock_ws_sel.assert_called_once()
                mock_tui.assert_not_called()


def test_entrypoint_update_targeted_flag_dispatches_without_tui():
    with patch.object(sys, "argv", ["actx", "--update@0.30.16"]):
        with patch("any_context.cli.workspace_selector.get_active_workspace") as mock_ws_sel:
            mock_ws_sel.side_effect = SystemExit(0)
            with patch("any_context.cli.tui_launcher.launch_opentui") as mock_tui:
                with pytest.raises(SystemExit) as exc_info:
                    entrypoint()
                assert exc_info.value.code == 0
                mock_ws_sel.assert_called_once()
                mock_tui.assert_not_called()


def test_entrypoint_check_update_flag_dispatches_without_tui():
    with patch.object(sys, "argv", ["actx", "--check-update"]):
        with patch("any_context.cli.workspace_selector.get_active_workspace") as mock_ws_sel:
            mock_ws_sel.side_effect = SystemExit(0)
            with patch("any_context.cli.tui_launcher.launch_opentui") as mock_tui:
                with pytest.raises(SystemExit) as exc_info:
                    entrypoint()
                assert exc_info.value.code == 0
                mock_ws_sel.assert_called_once()
                mock_tui.assert_not_called()


def test_rpc_bridge_get_state_includes_update_fields():
    from any_context.server.rpc_bridge import StdioRPCServer
    from any_context.config.db_store import ConfigDBStore

    tracker = UpdateProgressTracker.get_instance()
    tracker.reset()

    server = StdioRPCServer(default_workspace="Default", store=ConfigDBStore())
    state = server.get_state()

    assert "is_updating" in state
    assert "update_info" in state
    assert not state["is_updating"]

    # When update is active
    tracker.set_starting("v0.30.17")
    tracker.update_download(10 * 1024 * 1024, 20 * 1024 * 1024)
    state_active = server.get_state()
    assert state_active["is_updating"] is True
    assert "50%" in state_active["update_info"]

    tracker.reset()


def test_api_update_status_endpoint():
    from fastapi.testclient import TestClient
    from any_context.server.api import create_app

    tracker = UpdateProgressTracker.get_instance()
    tracker.reset()

    app = create_app()
    client = TestClient(app)

    # Test when idle
    res = client.get("/v1/system/update/status")
    assert res.status_code == 200
    data = res.json()
    assert data["is_updating"] is False
    assert data["stage"] == "idle"

    # Test when active
    tracker.set_starting("v0.30.17")
    tracker.update_download(25 * 1024 * 1024, 50 * 1024 * 1024)
    res_active = client.get("/v1/system/update/status")
    assert res_active.status_code == 200
    data_active = res_active.json()
    assert data_active["is_updating"] is True
    assert data_active["percent"] == 50
    assert "25.0/50.0MB" in data_active["update_info"]

    tracker.reset()

