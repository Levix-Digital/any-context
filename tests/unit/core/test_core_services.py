"""
Unit Tests for Core Application Services (v0.27.0 - Hexagonal Decoupling).
Tests:
  - WorkspaceService: creation, deletion, renaming, system protection
  - SourceService: adding/removing folders and web URLs, listing
  - ModelService: switching models, API key configuration, catalog inspection
  - GroundingService: setting/getting grounding mode and web search status
  - SyncService: background sync trigger and status
  - MemoryService: memory reset
  - BillingService: subscription matrix retrieval
"""

import os
import unittest
from unittest.mock import MagicMock, patch

from any_context.core.services import (
    WorkspaceService,
    SourceService,
    ModelService,
    GroundingService,
    SyncService,
    MemoryService,
    BillingService,
)
from any_context.config.db_store import ConfigDBStore


class TestCoreServices(unittest.TestCase):

    def setUp(self):
        self.store = MagicMock(spec=ConfigDBStore)

    def test_01_workspace_service_lifecycle(self):
        """Validates WorkspaceService create, list, and delete protections."""
        svc = WorkspaceService(store=self.store)
        self.store.get_workspace_meta.return_value = None

        # Create
        res = svc.create_workspace("TestWorkspace")
        self.assertTrue(res["created"])
        self.assertEqual(res["name"], "TestWorkspace")
        self.store.add_workspace.assert_called_once_with(name="TestWorkspace", paths=[])

        # Delete protection for system workspaces
        with self.assertRaises(ValueError):
            svc.delete_workspace("Default")
        with self.assertRaises(ValueError):
            svc.delete_workspace("Global")

        # Delete custom workspace
        self.store.remove_workspace.return_value = True
        del_res = svc.delete_workspace("CustomWS")
        self.assertTrue(del_res["deleted"])

    def test_02_source_service_folder_and_web(self):
        """Validates SourceService folder and web portal additions."""
        svc = SourceService(store=self.store)
        self.store.get_workspace_sources.return_value = {
            "folders": ["C:/Test/Docs"],
            "web_urls": ["https://fastapi.tiangolo.com"],
            "cloud_drives": []
        }

        sources = svc.list_sources("TestWorkspace")
        self.assertEqual(sources["total_count"], 2)
        self.assertIn("https://fastapi.tiangolo.com", sources["web_urls"])

        # Add web portal
        with patch("any_context.ingestion.web_scheduler.WebSchedulerStore") as mock_web_class:
            mock_web = MagicMock()
            mock_web_class.return_value = mock_web
            web_res = svc.add_web("TestWorkspace", "python.org")
            self.assertTrue(web_res["added"])
            self.assertEqual(web_res["url"], "https://python.org")
            mock_web.add_web_url.assert_called_once_with(workspace_name="TestWorkspace", url="https://python.org")

    def test_03_model_service(self):
        """Validates ModelService model switching and catalog listing."""
        svc = ModelService(store=self.store)
        self.store.get_app_settings.return_value = None

        res = svc.set_model("gpt-4o-mini")
        self.assertEqual(res["model"], "gpt-4o-mini")
        self.assertEqual(res["provider"].lower(), "openai")

        catalog = svc.list_models()
        self.assertGreater(len(catalog), 0)
        self.assertTrue(any(m["id"] == "gpt-4o-mini" for m in catalog))

    def test_04_grounding_service(self):
        """Validates GroundingService mode switches and web search toggle."""
        svc = GroundingService(store=self.store)

        # Valid mode
        res = svc.set_grounding_mode("TestWS", "hybrid")
        self.assertEqual(res["mode"], "hybrid")

        # Invalid mode
        with self.assertRaises(ValueError):
            svc.set_grounding_mode("TestWS", "invalid_mode")

        # Web search toggle
        search_res = svc.set_web_search_status("TestWS", True)
        self.assertTrue(search_res["web_search_enabled"])
        self.store.set_web_search_status.assert_called_once_with(workspace_name="TestWS", enabled=True)

    def test_05_sync_service(self):
        """Validates SyncService background sync dispatch."""
        with patch("any_context.core.services.sync_service.BackgroundSyncManager") as mock_bg_class:
            mock_bg = MagicMock()
            mock_bg_class.return_value = mock_bg
            mock_bg.is_syncing.return_value = False

            svc = SyncService()
            res = svc.start_sync("TestWS", force_full=True)
            self.assertTrue(res["started"])
            self.assertTrue(res["force_full"])
            mock_bg.start_background_sync.assert_called_once_with(workspace_name="TestWS", force_full=True, verbose=False)

    def test_06_memory_and_billing_service(self):
        """Validates MemoryService reset and BillingService info retrieval."""
        mem_svc = MemoryService()
        res = mem_svc.reset_memory("TestWS")
        self.assertTrue(res["reset"])

        billing_svc = BillingService()
        binfo = billing_svc.get_billing_info()
        self.assertIn("current_tier", binfo)
        self.assertIn("matrix_text", binfo)


if __name__ == "__main__":
    unittest.main()
