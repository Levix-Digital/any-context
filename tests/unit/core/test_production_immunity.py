"""
Unit tests asserting 100% immunity of the user's production vector database and memory
against automated test runners (v0.28.81).
"""
import os
import tempfile
import unittest
from unittest.mock import patch

from any_context.config.paths import (
    get_default_vector_db_path,
    get_default_session_db_path,
    get_app_data_root
)
from any_context.ingestion.orchestrator import clear_context_vector_db


class TestProductionImmunity(unittest.TestCase):

    def test_vector_db_path_sandbox_safety(self):
        """Verifies that in test mode without ACTX_CONTEXT_DB, the vector path is isolated to temp."""
        with patch.dict(os.environ, {"ACTX_TEST_MODE": "1"}, clear=False):
            if "ACTX_CONTEXT_DB" in os.environ:
                del os.environ["ACTX_CONTEXT_DB"]

            path = get_default_vector_db_path()
            prod_root = get_app_data_root()

            self.assertFalse(
                path.lower().startswith(prod_root.lower()),
                f"Test mode vector path must NEVER point to production AppData! Got: {path}"
            )
            self.assertTrue(
                path.lower().startswith(tempfile.gettempdir().lower()),
                f"Test mode vector path must be in temp directory! Got: {path}"
            )

    def test_session_db_path_sandbox_safety(self):
        """Verifies that in test mode without ACTX_MEMORY_DB, the session path is isolated to temp."""
        with patch.dict(os.environ, {"ACTX_TEST_MODE": "1"}, clear=False):
            if "ACTX_MEMORY_DB" in os.environ:
                del os.environ["ACTX_MEMORY_DB"]

            path = get_default_session_db_path()
            prod_root = get_app_data_root()

            self.assertFalse(
                path.lower().startswith(prod_root.lower()),
                f"Test mode session path must NEVER point to production AppData! Got: {path}"
            )
            self.assertTrue(
                path.lower().startswith(tempfile.gettempdir().lower()),
                f"Test mode session path must be in temp directory! Got: {path}"
            )

    def test_clear_context_vector_db_prod_shield(self):
        """Verifies clear_context_vector_db aborts and never purges production data in test mode."""
        prod_dir = os.path.join(get_app_data_root(), "data", "context_db")
        with patch.dict(os.environ, {"ACTX_TEST_MODE": "1"}):
            with patch("any_context.config.app_settings.AppSettings.load") as mock_load:
                from any_context.config.app_settings import AppSettings, ContextSettings
                dummy_settings = AppSettings(context=ContextSettings(db_path=prod_dir))
                mock_load.return_value = dummy_settings

                with patch("any_context.vector_engine.store.LanceDBStore.get_instance") as mock_lance:
                    clear_context_vector_db(verbose=False)
                    mock_lance.assert_not_called()


if __name__ == "__main__":
    unittest.main()
