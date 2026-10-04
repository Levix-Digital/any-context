"""
Tests for Chain of Responsibility Multi-Source Synchronization (v0.32.11).
Verifies that workspaces with only web portals or only local folders skip irrelevant
handlers gracefully with zero false-positive error emojis.
"""
import pytest
from unittest.mock import patch, MagicMock
from any_context.ingestion.unified_sync import (
    SyncContext,
    BaseSyncHandler,
    LocalFolderSyncHandler,
    WebPortalSyncHandler,
    CloudDriveSyncHandler,
    run_unified_sync
)

def test_sync_context_initialization():
    ctx = SyncContext(workspace_name="test-ws", force_full=True, verbose=False)
    assert ctx.workspace_name == "test-ws"
    assert ctx.force_full is True
    assert ctx.results["folders"]["status"] == "skipped"
    assert ctx.results["web"]["status"] == "skipped"
    assert ctx.results["drives"]["status"] == "skipped"

def test_local_folder_sync_handler_skips_when_no_folders():
    ctx = SyncContext(workspace_name="web-only-ws", sources={"folders": [], "web_sources": [{"url": "https://example.com"}]})
    handler = LocalFolderSyncHandler()
    handler.execute_sync(ctx)
    assert ctx.results["folders"]["status"] == "skipped"
    assert ctx.results["folders"]["total_files"] == 0
    assert "No local folders" in ctx.results["folders"]["message"]

def test_web_portal_sync_handler_skips_when_no_urls():
    ctx = SyncContext(workspace_name="folder-only-ws", sources={"folders": ["/some/path"], "web_sources": []})
    with patch("any_context.ingestion.unified_sync.WebSchedulerStore") as mock_web_store_cls:
        mock_web_store = MagicMock()
        mock_web_store.get_workspace_web_urls.return_value = []
        mock_web_store_cls.return_value = mock_web_store
        
        handler = WebPortalSyncHandler()
        handler.execute_sync(ctx)
        assert ctx.results["web"]["status"] == "skipped"
        assert ctx.results["web"]["total_urls"] == 0

def test_chain_execution_flow():
    ctx = SyncContext(
        workspace_name="test-ws",
        sources={
            "folders": [],
            "web_sources": [],
            "cloud_drives": []
        }
    )
    h1 = LocalFolderSyncHandler()
    h2 = WebPortalSyncHandler()
    h3 = CloudDriveSyncHandler()
    h1.set_next(h2).set_next(h3)

    h1.handle(ctx)
    assert ctx.results["folders"]["status"] == "skipped"
    assert ctx.results["web"]["status"] == "skipped"
    assert ctx.results["drives"]["status"] == "skipped"

@patch("any_context.ingestion.unified_sync.ConfigDBStore")
def test_run_unified_sync_pipeline(mock_db_store_cls):
    mock_db = MagicMock()
    mock_db.get_workspace_sources.return_value = {
        "folders": [],
        "web_sources": [],
        "cloud_drives": []
    }
    mock_db_store_cls.return_value = mock_db

    results = run_unified_sync(workspace_name="Default", verbose=False)
    assert "workspaces" in results
    assert "Default" in results["workspaces"]
    assert results["folder_results"]["Default"]["status"] == "skipped"
    assert results["web_results"]["Default"]["status"] == "skipped"
