import os
import sys
import time
import unittest

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.ingestion.orchestrator import BackgroundSyncManager


class TestCrawlerProgressAndNotifications(unittest.TestCase):
    """
    Unit Test Suite: Validates BackgroundSyncManager crawler progress bar formatting
    and real-time completion notification listeners.
    100% native unittest.TestCase without external test dependencies.
    """

    def test_background_sync_manager_crawler_progress_formatting(self):
        bg_mgr = BackgroundSyncManager()
        ws_name = "TestCrawlerWS"

        # 1. Scanning state
        bg_mgr.update_progress(ws_name, current=0, total=0, stage="crawling")
        self.assertEqual(bg_mgr.format_progress_bar(ws_name), "[crawling...]")

        # 2. Active crawling pages
        bg_mgr.update_progress(ws_name, current=15, total=30, stage="pages")
        bar = bg_mgr.format_progress_bar(ws_name, width=8)
        self.assertIn("50%", bar)
        self.assertIn("15/30 pages", bar)

    def test_background_sync_manager_notifications_and_listeners(self):
        bg_mgr = BackgroundSyncManager()
        ws_name = "TestNotifWS"

        received_events = []

        def _listener(notif):
            received_events.append(notif)

        bg_mgr.register_completion_listener(_listener)

        # Start a sync worker that finishes quickly
        t = bg_mgr.start_background_sync(
            workspace_name=ws_name,
            sync_folders=False,
            sync_web=False,
            sync_drives=False
        )
        t.join(timeout=5)

        # Verify listener fired
        self.assertGreaterEqual(len(received_events), 1)
        event = [e for e in received_events if e["workspace"] == ws_name][0]
        self.assertTrue(event["success"])
        self.assertIn("TestNotifWS", event["message"])

        # Verify pop_notifications
        notifs = bg_mgr.pop_notifications(ws_name)
        self.assertGreaterEqual(len(notifs), 1)
        self.assertEqual(notifs[0]["workspace"], ws_name)

        # Second pop must be empty
        self.assertEqual(len(bg_mgr.pop_notifications(ws_name)), 0)


if __name__ == "__main__":
    unittest.main()
