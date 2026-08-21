import os
import sys
import unittest
import tempfile

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.config.db_store import ConfigDBStore
from any_context.config.app_settings import ContextSettings
from any_context.core.utils import get_system_prompt
from tests.e2e_helpers import safe_stdout_write

class TestGroundingModes(unittest.TestCase):
    """
    Unit Test Suite: Validates AI Grounding Modes ('hybrid', 'strict', 'proactive') in Storage & Agent Prompts.
    """

    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()
        self.db_path = os.path.join(self.temp_dir.name, "test_settings.db")
        self.store = ConfigDBStore(db_path=self.db_path)

    def tearDown(self):
        try:
            self.temp_dir.cleanup()
        except Exception:
            pass

    def test_01_default_grounding_mode_is_strict(self):
        """Validates that default grounding mode is 'strict'."""
        safe_stdout_write("\n>>> [CORE UNIT] Testing Default Grounding Mode...\n")
        mode = self.store.get_grounding_mode()
        self.assertEqual(mode, "strict")
        safe_stdout_write("  [OK] Default grounding mode is 'strict'!\n")

    def test_02_set_and_persist_grounding_modes(self):
        """Validates setting and persisting 'strict', 'proactive', and 'hybrid' modes."""
        safe_stdout_write(">>> [CORE UNIT] Testing Setting & Persisting Grounding Modes...\n")
        # Strict
        mode_strict = self.store.set_grounding_mode("strict")
        self.assertEqual(mode_strict, "strict")
        self.assertEqual(self.store.get_grounding_mode(), "strict")

        # Proactive
        mode_pro = self.store.set_grounding_mode("proactive")
        self.assertEqual(mode_pro, "proactive")
        self.assertEqual(self.store.get_grounding_mode(), "proactive")

        # Hybrid
        mode_hyb = self.store.set_grounding_mode("hybrid")
        self.assertEqual(mode_hyb, "hybrid")
        self.assertEqual(self.store.get_grounding_mode(), "hybrid")
        safe_stdout_write("  [OK] Grounding modes successfully set and persisted in SQLite!\n")

    def test_03_invalid_grounding_mode_fallback(self):
        """Validates that unknown grounding mode strings fallback safely to 'strict'."""
        safe_stdout_write(">>> [CORE UNIT] Testing Invalid Grounding Mode Fallback...\n")
        mode = self.store.set_grounding_mode("unknown_mode_xyz")
        self.assertEqual(mode, "strict")
        self.assertEqual(self.store.get_grounding_mode(), "strict")
        safe_stdout_write("  [OK] Unknown grounding mode safely fell back to 'strict'!\n")

    def test_04_system_prompt_directives_injection(self):
        """Validates that get_system_prompt injects the correct directive for each mode."""
        safe_stdout_write(">>> [CORE UNIT] Testing System Prompt Directives Injection...\n")
        # Strict mode
        strict_prompt = get_system_prompt(active_workspace="LegalTest", grounding_mode="strict")
        self.assertIn("ACTIVE GROUNDING MODE: STRICT", strict_prompt)
        self.assertIn("ZERO SPECULATION / ZERO HALLUCINATION", strict_prompt)
        self.assertIn("FACTUAL ABSENCE PROTOCOL", strict_prompt)

        # Proactive mode
        pro_prompt = get_system_prompt(active_workspace="ResearchTest", grounding_mode="proactive")
        self.assertIn("ACTIVE GROUNDING MODE: PROACTIVE", pro_prompt)
        self.assertIn("FORWARD-LOOKING INSIGHTS", pro_prompt)
        self.assertIn("WEB SOURCE RECOMMENDATIONS", pro_prompt)

        # Hybrid mode (default)
        hyb_prompt = get_system_prompt(active_workspace="DefaultTest", grounding_mode="hybrid")
        self.assertIn("ACTIVE GROUNDING MODE: HYBRID", hyb_prompt)
        self.assertIn("DUAL-LAYER STRUCTURE", hyb_prompt)
        self.assertIn("Sugestões / Conhecimento Geral do Modelo", hyb_prompt)

        safe_stdout_write("  [OK] System prompt correctly injects unique behavioral directives per mode!\n")

if __name__ == "__main__":
    unittest.main()
