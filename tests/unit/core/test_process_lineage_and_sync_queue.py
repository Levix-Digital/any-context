import os
import unittest
from unittest.mock import patch, MagicMock
from any_context.core.services.update_service import get_current_session_pids, UpdateService
from any_context.core.interaction.options_engine import OptionsEngine
from any_context.ingestion.orchestrator import BackgroundSyncManager


class TestProcessLineageAndSyncQueue(unittest.TestCase):

    def test_get_current_session_pids_contains_self_and_env(self):
        os.environ["ACTX_LAUNCHER_PID"] = "99991"
        os.environ["ACTX_ROOT_PID"] = "99992"
        os.environ["ACTX_TUI_PID"] = "99993"
        try:
            pids = get_current_session_pids()
            self.assertIn(os.getpid(), pids)
            self.assertIn(99991, pids)
            self.assertIn(99992, pids)
            self.assertIn(99993, pids)
        finally:
            os.environ.pop("ACTX_LAUNCHER_PID", None)
            os.environ.pop("ACTX_ROOT_PID", None)
            os.environ.pop("ACTX_TUI_PID", None)

    def test_find_active_instances_excludes_current_session(self):
        update_svc = UpdateService()
        curr_pids = get_current_session_pids()
        instances = update_svc.find_active_instances()
        for inst in instances:
            self.assertNotIn(inst["pid"], curr_pids, f"Instance {inst['pid']} should have been ignored!")

    def test_close_active_instances_protects_immune_session_pids(self):
        update_svc = UpdateService()
        immune_pid = os.getpid()
        fake_instances = [
            {"pid": immune_pid, "name": "actx.exe", "title": "Self"},
            {"pid": 999999, "name": "actx.exe", "title": "Other"}
        ]
        with patch("subprocess.run") as mock_sub:
            mock_sub.return_value = MagicMock(returncode=0)
            closed = update_svc.close_active_instances(fake_instances)
            # Should only attempt to close the non-immune PID
            for call in mock_sub.call_args_list:
                args = call[0][0]
                self.assertNotIn(str(immune_pid), args, "Self PID should never be passed to taskkill!")

    def test_options_engine_shows_close_option_when_zero_instances(self):
        engine = OptionsEngine()
        with patch.object(UpdateService, "check_for_updates", return_value=(True, "v0.28.74")), \
             patch.object(UpdateService, "find_active_instances", return_value=[]):
            group = engine.get_update_options(target_version="v0.28.74")
            item_ids = [item.id for item in group.items]
            self.assertIn("background", item_ids)
            self.assertIn("close", item_ids)
            self.assertIn("cancel", item_ids)
            close_item = next(i for i in group.items if i.id == "close")
            self.assertIn("Close session and update now", close_item.title)
            self.assertIn("Terminates this session", close_item.description)

    def test_options_engine_shows_close_option_when_instances_exist(self):
        engine = OptionsEngine()
        fake_instances = [{"pid": 1234, "name": "actx.exe", "title": "Other"}]
        with patch.object(UpdateService, "check_for_updates", return_value=(True, "v0.28.74")), \
             patch.object(UpdateService, "find_active_instances", return_value=fake_instances):
            group = engine.get_update_options(target_version="v0.28.74")
            item_ids = [item.id for item in group.items]
            self.assertIn("background", item_ids)
            self.assertIn("close", item_ids)
            self.assertIn("cancel", item_ids)
            close_item = next(i for i in group.items if i.id == "close")
            self.assertIn("Close all AnyContext sessions and update now", close_item.title)
            self.assertIn("Terminates all 2 active sessions", close_item.description)

    def test_background_sync_manager_pending_queue(self):
        mgr = BackgroundSyncManager()
        clean_ws = "TestQueueWS"
        fake_thread = MagicMock()
        fake_thread.is_alive.return_value = True

        with mgr._lock:
            mgr._active_jobs[clean_ws] = {"thread": fake_thread, "status": "syncing"}
            if clean_ws in mgr._pending_syncs:
                mgr._pending_syncs.remove(clean_ws)

        # Call start_background_sync while active
        thread = mgr.start_background_sync(clean_ws)
        self.assertEqual(thread, fake_thread)
        with mgr._lock:
            self.assertIn(clean_ws, mgr._pending_syncs, "Workspace should be marked in _pending_syncs")
            # Clean up
            del mgr._active_jobs[clean_ws]
            mgr._pending_syncs.remove(clean_ws)


if __name__ == "__main__":
    unittest.main()
