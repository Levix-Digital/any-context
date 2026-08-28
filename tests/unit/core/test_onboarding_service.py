"""
Unit Tests for OnboardingService (v0.28.52 - First-Time Onboarding & Surface Parity).
"""

import unittest
import tempfile
import os
import shutil
from unittest.mock import MagicMock

from any_context.config.db_store import ConfigDBStore
from any_context.core.services.onboarding_service import OnboardingService, OnboardingState, OnboardingResult


class TestOnboardingService(unittest.TestCase):

    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.db_path = os.path.join(self.temp_dir, "test_settings.db")
        self.store = ConfigDBStore(db_path=self.db_path)
        self.store._init_db()
        self.svc = OnboardingService(store=self.store)

    def tearDown(self):
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_01_check_status_first_time(self):
        """Validates that a fresh database requires first-time onboarding."""
        self.assertFalse(self.store.get_onboarding_completed())
        status = self.svc.check_status()
        self.assertTrue(status.needs_onboarding)
        self.assertEqual(status.stage, "first_time")
        self.assertEqual(len(status.options_group.items), 3)
        self.assertEqual(status.options_group.items[0].id, "openai")
        self.assertEqual(status.options_group.items[1].id, "local_offline")
        self.assertEqual(status.options_group.items[2].id, "custom")

    def test_02_complete_onboarding_openai(self):
        """Validates completing onboarding with OpenAI provider and API key."""
        # Missing key should fail
        err_res = self.svc.complete_onboarding("openai", api_key="")
        self.assertFalse(err_res.success)
        self.assertIn("cannot be empty", err_res.error)

        # Valid key succeeds
        res = self.svc.complete_onboarding("openai", api_key="sk-test-key-123456789")
        self.assertTrue(res.success)
        self.assertTrue(self.store.get_onboarding_completed())
        self.assertEqual(self.store.get_api_key("openai"), "sk-test-key-123456789")

        # Re-check status should now be ready
        new_status = self.svc.check_status()
        self.assertFalse(new_status.needs_onboarding)
        self.assertEqual(new_status.stage, "ready")

    def test_03_complete_onboarding_local_offline(self):
        """Validates completing onboarding with local offline server."""
        res = self.svc.complete_onboarding("local_offline", base_url="http://localhost:1234/v1")
        self.assertTrue(res.success)
        self.assertTrue(self.store.get_onboarding_completed())
        
        new_status = self.svc.check_status()
        self.assertFalse(new_status.needs_onboarding)

    def test_04_factory_reset_resets_onboarding_flag(self):
        """Validates that factory reset clears the onboarding flag back to False."""
        self.svc.complete_onboarding("openai", api_key="sk-test-key-123456789")
        self.assertTrue(self.store.get_onboarding_completed())

        self.store.factory_reset()
        self.assertFalse(self.store.get_onboarding_completed())
        
        status = self.svc.check_status()
        self.assertTrue(status.needs_onboarding)
        self.assertEqual(status.stage, "first_time")


if __name__ == "__main__":
    unittest.main()
