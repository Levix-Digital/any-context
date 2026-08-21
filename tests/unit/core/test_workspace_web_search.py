import os
import sys
import unittest
import tempfile
from unittest.mock import patch, MagicMock

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.config.db_store import ConfigDBStore
from any_context.core.utils import get_system_prompt
from any_context.tools.web_search_tool import live_web_search, execute_web_search, _extract_domain
from tests.e2e_helpers import safe_stdout_write

class TestWorkspaceWebSearch(unittest.TestCase):
    """
    Unit Test Suite: Validates Workspace-Isolated Web Search & Dynamic Grounding Engine (v0.15.0).
    """

    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()
        self.db_path = os.path.join(self.temp_dir.name, "test_search_settings.db")
        self.store = ConfigDBStore(db_path=self.db_path)

    def tearDown(self):
        try:
            self.temp_dir.cleanup()
        except Exception:
            pass

    def test_01_default_web_search_is_disabled(self):
        """Validates that web search defaults to False (offline isolation)."""
        safe_stdout_write("\n>>> [CORE UNIT] Testing Default Web Search Status (Isolated Offline)...\n")
        self.assertFalse(self.store.get_web_search_status())
        self.assertFalse(self.store.get_web_search_status("Default"))
        safe_stdout_write("  [OK] Web search is disabled by default for zero unintended token/privacy leakage!\n")

    def test_02_workspace_isolation_for_web_search_and_modes(self):
        """Validates that web search and grounding modes are isolated per workspace."""
        safe_stdout_write(">>> [CORE UNIT] Testing Workspace Isolation for Web Search & Modes...\n")
        self.store.add_workspace("AuditWorkspace", paths=[])
        self.store.add_workspace("ResearchWorkspace", paths=[])

        # Configure AuditWorkspace: Strict + Web Search OFF
        self.store.set_grounding_mode("strict", workspace_name="AuditWorkspace")
        self.store.set_web_search_status(False, workspace_name="AuditWorkspace")

        # Configure ResearchWorkspace: Proactive + Web Search ON
        self.store.set_grounding_mode("proactive", workspace_name="ResearchWorkspace")
        self.store.set_web_search_status(True, workspace_name="ResearchWorkspace")

        # Verify AuditWorkspace
        self.assertEqual(self.store.get_grounding_mode("AuditWorkspace"), "strict")
        self.assertFalse(self.store.get_web_search_status("AuditWorkspace"))

        # Verify ResearchWorkspace
        self.assertEqual(self.store.get_grounding_mode("ResearchWorkspace"), "proactive")
        self.assertTrue(self.store.get_web_search_status("ResearchWorkspace"))

        # Verify Default remains untouched (strict default)
        self.assertEqual(self.store.get_grounding_mode("Default"), "strict")
        self.assertFalse(self.store.get_web_search_status("Default"))
        safe_stdout_write("  [OK] Web search and grounding mode settings are completely isolated per workspace!\n")

    def test_03_global_mode_and_search_application(self):
        """Validates applying settings globally across all workspaces."""
        safe_stdout_write(">>> [CORE UNIT] Testing Global Application Across All Workspaces...\n")
        self.store.add_workspace("WS1", paths=[])
        self.store.add_workspace("WS2", paths=[])

        # Apply web search ON globally
        self.store.set_web_search_status(True, apply_global=True)
        self.assertTrue(self.store.get_web_search_status())
        self.assertTrue(self.store.get_web_search_status("WS1"))
        self.assertTrue(self.store.get_web_search_status("WS2"))

        # Apply grounding mode strict globally
        self.store.set_grounding_mode("strict", apply_global=True)
        self.assertEqual(self.store.get_grounding_mode(), "strict")
        self.assertEqual(self.store.get_grounding_mode("WS1"), "strict")
        self.assertEqual(self.store.get_grounding_mode("WS2"), "strict")
        safe_stdout_write("  [OK] Global batch settings apply consistently across all workspace entries!\n")

    def test_04_domain_extractor_helper(self):
        """Validates domain extraction from URLs."""
        safe_stdout_write(">>> [CORE UNIT] Testing Domain Extraction Helper...\n")
        self.assertEqual(_extract_domain("https://www.canada.ca/en/immigration.html"), "canada.ca")
        self.assertEqual(_extract_domain("http://alberta.ca:8080/immigration"), "alberta.ca")
        self.assertEqual(_extract_domain("https://docs.python.org/3/library"), "docs.python.org")
        safe_stdout_write("  [OK] Clean domain names extracted correctly for targeted site: searches!\n")

    @patch("any_context.tools.web_search_tool.execute_web_search")
    def test_05_live_web_search_tool_formatting(self, mock_search):
        """Validates that live_web_search formats Markdown citations with source URLs."""
        safe_stdout_write(">>> [CORE UNIT] Testing live_web_search Tool Formatting...\n")
        mock_search.return_value = [
            {
                "title": "Canada Immigration Express Entry 2026",
                "url": "https://www.canada.ca/express-entry",
                "snippet": "Express Entry is Canada's flagship application management system."
            },
            {
                "title": "Alberta Advantage Immigration Program",
                "url": "https://www.alberta.ca/aaip",
                "snippet": "AAIP provides pathways for skilled workers in Alberta."
            }
        ]

        res = live_web_search.invoke({"query": "Express Entry Alberta", "workspace": "Default"})
        self.assertIn("Resultados da Busca Web", res)
        self.assertIn("https://www.canada.ca/express-entry", res)
        self.assertIn("https://www.alberta.ca/aaip", res)
        self.assertIn("Express Entry is Canada's flagship", res)
        safe_stdout_write("  [OK] live_web_search returns clean, verifiable Markdown source citations!\n")

    def test_06_system_prompt_web_search_grounding_protocols(self):
        """Validates that get_system_prompt injects the tailored protocols for Strict, Hybrid, Proactive with Web Search ON."""
        safe_stdout_write(">>> [CORE UNIT] Testing System Prompt Grounding Protocols with Web Search...\n")

        # 1. Strict + Web ON
        strict_web_prompt = get_system_prompt(active_workspace="LegalTest", grounding_mode="strict", web_search_enabled=True)
        self.assertIn("LIVE WEB SEARCH ENGINE: ACTIVE", strict_web_prompt)
        self.assertIn("STRICT PROTOCOL FOR WEB SEARCH", strict_web_prompt)
        self.assertIn("Deseja que eu faça uma busca na internet", strict_web_prompt)

        # 2. Hybrid + Web ON
        hyb_web_prompt = get_system_prompt(active_workspace="DevTest", grounding_mode="hybrid", web_search_enabled=True)
        self.assertIn("HYBRID DUAL-LAYER PROTOCOL FOR WEB SEARCH", hyb_web_prompt)
        self.assertIn("### 📂 Informações do Workspace", hyb_web_prompt)
        self.assertIn("### 🌐 Informações Complementares da Web", hyb_web_prompt)

        # 3. Proactive + Web ON
        pro_web_prompt = get_system_prompt(active_workspace="StratTest", grounding_mode="proactive", web_search_enabled=True)
        self.assertIn("PROACTIVE PROTOCOL FOR WEB SEARCH", pro_web_prompt)
        self.assertIn("[Documento: <arquivo>]", pro_web_prompt)
        self.assertIn("[Web: <URL>]", pro_web_prompt)

        # 4. Web Search OFF
        off_prompt = get_system_prompt(active_workspace="OfflineTest", grounding_mode="strict", web_search_enabled=False)
        self.assertIn("LIVE WEB SEARCH: DISABLED (OFFLINE-FIRST LOCAL ISOLATION)", off_prompt)
        safe_stdout_write("  [OK] System prompt dynamically configures strict source discrimination protocols!\n")

if __name__ == "__main__":
    unittest.main()
