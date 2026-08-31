import time
import pytest
from any_context.ingestion.orchestrator import BackgroundSyncManager

def test_background_sync_manager_crawler_progress_formatting():
    bg_mgr = BackgroundSyncManager()
    ws_name = "TestCrawlerWS"
    
    # 1. Scanning state
    bg_mgr.update_progress(ws_name, current=0, total=0, stage="crawling")
    assert bg_mgr.format_progress_bar(ws_name) == "[crawling...]"
    
    # 2. Active crawling pages
    bg_mgr.update_progress(ws_name, current=15, total=30, stage="pages")
    bar = bg_mgr.format_progress_bar(ws_name, width=8)
    assert "50%" in bar
    assert "15/30 pages" in bar

def test_background_sync_manager_notifications_and_listeners():
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
    assert len(received_events) >= 1
    event = [e for e in received_events if e["workspace"] == ws_name][0]
    assert event["success"] is True
    assert "TestNotifWS" in event["message"]
    
    # Verify pop_notifications
    notifs = bg_mgr.pop_notifications(ws_name)
    assert len(notifs) >= 1
    assert notifs[0]["workspace"] == ws_name
    
    # Second pop must be empty
    assert len(bg_mgr.pop_notifications(ws_name)) == 0
