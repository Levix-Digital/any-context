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

    def test_05_onboarding_persists_across_save_app_settings(self):
        """Validates that save_app_settings does not wipe onboarding_completed or workspaces."""
        self.svc.complete_onboarding("openai", api_key="sk-test-persist-123")
        self.assertTrue(self.store.get_onboarding_completed())

        settings = self.store.get_app_settings()
        self.assertTrue(settings.context.onboarding_completed)

        # Trigger full save
        self.store.save_app_settings(settings)
        self.assertTrue(self.store.get_onboarding_completed())
        
        updated_settings = self.store.get_app_settings()
        self.assertTrue(updated_settings.context.onboarding_completed)

    def test_06_onboarding_persists_across_update_context_settings(self):
        """Validates that update_context_settings does not wipe onboarding_completed."""
        self.svc.complete_onboarding("openai", api_key="sk-test-persist-123")
        self.assertTrue(self.store.get_onboarding_completed())

        settings = self.store.get_app_settings()
        settings.context.chunk_size = 2048
        self.store.update_context_settings(settings.context)

        self.assertTrue(self.store.get_onboarding_completed())
        updated_settings = self.store.get_app_settings()
        self.assertEqual(updated_settings.context.chunk_size, 2048)
        self.assertTrue(updated_settings.context.onboarding_completed)

    def test_07_existing_api_key_auto_heals_and_bypasses_onboarding(self):
        """Validates that an existing API key in SQLite auto-heals onboarding status without prompting."""
        # Directly insert API key simulate prior version upgrade
        self.store.set_api_key("openai", "sk-prior-version-key-999")
        
        # Explicitly set context_settings.onboarding_completed = 0
        with self.store._get_connection() as conn:
            conn.execute("UPDATE context_settings SET onboarding_completed = 0 WHERE id = 1")
            conn.execute("DELETE FROM system_config WHERE key = 'onboarding_completed'")
            conn.commit()

        # check_status should detect key, auto-heal to True, and NOT require onboarding
        status = self.svc.check_status()
        self.assertFalse(status.needs_onboarding)
        self.assertEqual(status.stage, "ready")
        self.assertTrue(self.store.get_onboarding_completed())

    def test_08_system_config_table_isolation(self):
        """Validates get_system_config and set_system_config key-value isolation."""
        self.store.set_system_config("app_theme", "dracula")
        self.assertEqual(self.store.get_system_config("app_theme"), "dracula")
        self.assertIsNone(self.store.get_system_config("nonexistent_key"))
        self.assertEqual(self.store.get_system_config("nonexistent_key", "default_val"), "default_val")


if __name__ == "__main__":
    unittest.main()
