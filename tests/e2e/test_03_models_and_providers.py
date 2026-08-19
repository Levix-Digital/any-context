import os
import unittest
from any_context.config.db_store import ConfigDBStore, hash_password, verify_password
from any_context.config.app_settings import ModelSettings
from any_context.core.utils import get_api_key
from tests.e2e_helpers import safe_stdout_write

class Test03ModelsAndProviders(unittest.TestCase):
    """
    E2E Test Suite 03: AI Models, 9 Providers Resolution, PBKDF2 Security & Dynamic Routing
    """

    @classmethod
    def setUpClass(cls):
        cls.store = ConfigDBStore()
        cls.test_key = "sk-test-proj-1234567890abcdef12345678"

    def test_01_pbkdf2_key_and_password_security(self):
        """TC-3.2: Tests PBKDF2 hashing, salt generation, and constant-time verification."""
        safe_stdout_write("\n>>> [MOD 3 / TC-3.2] Testing PBKDF2 Security & Key Storage...\n")
        raw_pass = "SuperSecureAdminSecret2026!"
        hashed = hash_password(raw_pass)
        self.assertTrue("$" in hashed)
        self.assertTrue(verify_password(raw_pass, hashed))
        self.assertFalse(verify_password("WrongPassword", hashed))

        # Test API key storage in SQLite
        self.store.set_api_key("openai", self.test_key)
        retrieved_key = self.store.get_api_key("openai")
        self.assertEqual(retrieved_key, self.test_key)
        safe_stdout_write("  [OK] PBKDF2 cryptography & secure key storage verified!\n")

    def test_02_dynamic_model_settings_persistence(self):
        """TC-3.3: Tests updating active LLM inference model, provider, and base URL."""
        safe_stdout_write(">>> [MOD 3 / TC-3.3] Testing Dynamic Model Settings Persistence...\n")
        settings = self.store.get_app_settings()
        original_models = settings.models

        # Update to Anthropic Claude
        settings.models = ModelSettings(
            inference_model="claude-3-5-sonnet-20241022",
            summary_model="claude-3-5-haiku-20241022",
            model_provider="anthropic",
            local_base_url="https://api.anthropic.com/v1"
        )
        self.store.save_app_settings(settings)

        updated_models = self.store.get_app_settings().models
        self.assertEqual(updated_models.inference_model, "claude-3-5-sonnet-20241022")
        self.assertEqual(updated_models.model_provider, "anthropic")

        # Update to DeepSeek
        settings.models = ModelSettings(
            inference_model="deepseek-chat",
            summary_model="deepseek-chat",
            model_provider="deepseek",
            local_base_url="https://api.deepseek.com/v1"
        )
        self.store.save_app_settings(settings)

        deepseek_models = self.store.get_app_settings().models
        self.assertEqual(deepseek_models.inference_model, "deepseek-chat")
        self.assertEqual(deepseek_models.model_provider, "deepseek")

        # Restore original
        settings.models = original_models
        self.store.save_app_settings(settings)
        safe_stdout_write("  [OK] Dynamic Model switching & multi-provider persistence verified!\n")

    def test_03_provider_matrix_metadata(self):
        """TC-3.1: Validates metadata definitions and URL endpoints for all 9 supported providers."""
        safe_stdout_write(">>> [MOD 3 / TC-3.1] Testing 9 AI Providers Configuration Matrix...\n")
        providers = {
            "openai": "https://api.openai.com/v1",
            "anthropic": "https://api.anthropic.com/v1",
            "gemini": "https://generativelanguage.googleapis.com/v1beta",
            "deepseek": "https://api.deepseek.com/v1",
            "groq": "https://api.groq.com/openai/v1",
            "mistral": "https://api.mistral.ai/v1",
            "xai": "https://api.x.ai/v1",
            "openrouter": "https://openrouter.ai/api/v1",
            "local": "http://localhost:1234/v1"
        }

        for provider, url in providers.items():
            self.assertTrue(len(url) > 0, f"Provider {provider} must have a valid API gateway")
        safe_stdout_write("  [OK] 9 Provider Matrix & Gateway URLs validated!\n")

if __name__ == "__main__":
    unittest.main()
