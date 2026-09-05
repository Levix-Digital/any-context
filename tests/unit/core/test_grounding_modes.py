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
        self._orig_db = os.environ.get("ACTX_SETTINGS_DB")
        self._orig_test_mode = os.environ.get("ACTX_TEST_MODE")
        os.environ["ACTX_SETTINGS_DB"] = self.db_path
        os.environ["ACTX_TEST_MODE"] = "1"
        self.store = ConfigDBStore(db_path=self.db_path)

    def tearDown(self):
        if self._orig_db is not None:
            os.environ["ACTX_SETTINGS_DB"] = self._orig_db
        else:
            os.environ.pop("ACTX_SETTINGS_DB", None)
        if self._orig_test_mode is not None:
            os.environ["ACTX_TEST_MODE"] = self._orig_test_mode
        else:
            os.environ.pop("ACTX_TEST_MODE", None)
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
        """Validates that get_system_prompt injects the correct directive for each mode and web search state."""
        safe_stdout_write(">>> [CORE UNIT] Testing System Prompt Directives Injection & Priority Matrix...\n")
        # Universal Temporal Recency Rule must always be present
        base_prompt = get_system_prompt(active_workspace="LegalTest", grounding_mode="strict", web_search_enabled=False)
        self.assertIn("UNIVERSAL TEMPORAL RECENCY RULE (MANDATORY TIE-BREAKER FOR SAME-PRIORITY SOURCES)", base_prompt)
        self.assertIn("THE MOST RECENT SOURCE ALWAYS PREVAILS AND SUPERSEDES OLDER DATA", base_prompt)

        # 1. Strict + Web OFF
        strict_off = get_system_prompt(active_workspace="LegalTest", grounding_mode="strict", web_search_enabled=False)
        self.assertIn("ACTIVE GROUNDING MODE: STRICT", strict_off)
        self.assertIn("ZERO SPECULATION / ZERO HALLUCINATION", strict_off)
        self.assertIn("LIVE WEB SEARCH: DISABLED", strict_off)
        self.assertIn("⚠️ Essa informação não consta nos documentos deste workspace.", strict_off)
        self.assertNotIn("Deseja que eu faça uma busca na internet", strict_off)

        # 2. Strict + Web ON
        strict_on = get_system_prompt(active_workspace="LegalTest", grounding_mode="strict", web_search_enabled=True)
        self.assertIn("ACTIVE GROUNDING MODE: STRICT", strict_on)
        self.assertIn("LIVE WEB SEARCH ENGINE: ACTIVE", strict_on)
        self.assertIn("FACTUAL ABSENCE & WEB SEARCH PERMISSION PROTOCOL (MANDATORY)", strict_on)
        self.assertIn("⚠️ Essa informação não consta nos documentos deste workspace. Deseja que eu faça uma busca na internet sobre '[tópico]'?", strict_on)

        # 3. Hybrid + Web OFF
        hyb_off = get_system_prompt(active_workspace="DefaultTest", grounding_mode="hybrid", web_search_enabled=False)
        self.assertIn("ACTIVE GROUNDING MODE: HYBRID", hyb_off)
        self.assertIn("DUAL-LAYER STRUCTURE", hyb_off)
        self.assertIn("LIVE WEB SEARCH: DISABLED", hyb_off)
        self.assertIn("Sugestões / Conhecimento Geral do Modelo", hyb_off)

        # 4. Hybrid + Web ON
        hyb_on = get_system_prompt(active_workspace="DefaultTest", grounding_mode="hybrid", web_search_enabled=True)
        self.assertIn("ACTIVE GROUNDING MODE: HYBRID", hyb_on)
        self.assertIn("LIVE WEB SEARCH ENGINE: ACTIVE", hyb_on)
        self.assertIn("HYBRID DUAL-LAYER PROTOCOL FOR WEB SEARCH (AUTONOMOUS EXECUTION)", hyb_on)

        # 5. Proactive + Web OFF
        pro_off = get_system_prompt(active_workspace="ResearchTest", grounding_mode="proactive", web_search_enabled=False)
        self.assertIn("ACTIVE GROUNDING MODE: PROACTIVE", pro_off)
        self.assertIn("FORWARD-LOOKING INSIGHTS", pro_off)
        self.assertIn("LIVE WEB SEARCH: DISABLED", pro_off)

        # 6. Proactive + Web ON
        pro_on = get_system_prompt(active_workspace="ResearchTest", grounding_mode="proactive", web_search_enabled=True)
        self.assertIn("ACTIVE GROUNDING MODE: PROACTIVE", pro_on)
        self.assertIn("LIVE WEB SEARCH ENGINE: ACTIVE", pro_on)
        self.assertIn("PROACTIVE PROTOCOL FOR WEB SEARCH (AUTONOMOUS & COMPREHENSIVE)", pro_on)

        safe_stdout_write("  [OK] System prompt correctly injects unique behavioral directives per mode and web search state!\n")

    def test_05_grounding_strategies_formatting_and_recency_matrix(self):
        """Validates that GroundingStrategy implementations produce exact priority matrices and recency directives."""
        safe_stdout_write(">>> [CORE UNIT] Testing Grounding Strategy Pattern Formatting & Recency Matrix...\n")
        from any_context.core.grounding_strategies import (
            get_grounding_strategy,
            StrictGroundingStrategy,
            HybridGroundingStrategy,
            ProactiveGroundingStrategy,
        )
        from any_context.core.utils import format_turn_grounding_header

        # Factory resolution
        self.assertIsInstance(get_grounding_strategy("strict"), StrictGroundingStrategy)
        self.assertIsInstance(get_grounding_strategy("hybrid"), HybridGroundingStrategy)
        self.assertIsInstance(get_grounding_strategy("proactive"), ProactiveGroundingStrategy)
        self.assertIsInstance(get_grounding_strategy("unknown_fallback"), HybridGroundingStrategy)

        # Strict Strategy (Web OFF / Web ON)
        strict_off = format_turn_grounding_header("LegalDoc", "strict", web_search_enabled=False)
        self.assertIn("GROUNDING: STRICT", strict_off)
        self.assertIn("Priority 0: VectorDB ONLY", strict_off)
        self.assertIn("Parametric Memory: FORBIDDEN", strict_off)
        self.assertIn("Web Search: DISABLED", strict_off)
        self.assertIn("RECENCY RULE (SAME PRIORITY)", strict_off)

        strict_on = format_turn_grounding_header("LegalDoc", "strict", web_search_enabled=True)
        self.assertIn("Web Search: PERMISSION-GATED", strict_on)
        self.assertIn("NEVER call live_web_search autonomously", strict_on)
        self.assertIn("Deseja que eu faça uma busca na internet sobre \"[tópico]\"?", strict_on)
        self.assertIn("RECENCY RULE (SAME PRIORITY)", strict_on)

        # Hybrid Strategy (Web OFF / Web ON)
        hyb_off = format_turn_grounding_header("DevDoc", "hybrid", web_search_enabled=False)
        self.assertIn("GROUNDING: HYBRID", hyb_off)
        self.assertIn("Priority 0: VectorDB", hyb_off)
        self.assertIn("Priority 1: Parametric Memory", hyb_off)
        self.assertIn("RECENCY RULE (SAME PRIORITY)", hyb_off)

        hyb_on = format_turn_grounding_header("DevDoc", "hybrid", web_search_enabled=True)
        self.assertIn("Priority 0: VectorDB", hyb_on)
        self.assertIn("Priority 1: Open Web & Parametric", hyb_on)
        self.assertIn("RECENCY RULE (SAME PRIORITY)", hyb_on)

        # Proactive Strategy (Web OFF / Web ON)
        pro_off = format_turn_grounding_header("StratDoc", "proactive", web_search_enabled=False)
        self.assertIn("GROUNDING: PROACTIVE", pro_off)
        self.assertIn("All Sources Priority 0", pro_off)
        self.assertIn("RECENCY RULE (SAME PRIORITY)", pro_off)

        pro_on = format_turn_grounding_header("StratDoc", "proactive", web_search_enabled=True)
        self.assertIn("All Sources Priority 0", pro_on)
        self.assertIn("Total real-time fusion", pro_on)
        self.assertIn("RECENCY RULE (SAME PRIORITY)", pro_on)

        safe_stdout_write("  [OK] Strategy pattern formats precise, token-efficient priority matrices with universal recency rules!\n")

    def test_06_turn_level_header_injection_in_prune_messages(self):
        """Validates that _prune_messages_for_llm injects the strategy header ONLY on the active HumanMessage."""
        safe_stdout_write(">>> [CORE UNIT] Testing Turn-Level Header Injection in _prune_messages_for_llm...\n")
        from langchain_core.messages import HumanMessage, AIMessage, ToolMessage
        from any_context.core.agent import _prune_messages_for_llm

        messages = [
            HumanMessage(content="Turn 1: Qual o faturamento de 2024?"),
            AIMessage(content="O faturamento de 2024 foi de R$ 10 milhões."),
            HumanMessage(content="Turn 2: E a projeção para 2025?"),
            ToolMessage(content="[Chunk 1] Projeção 2025 é de R$ 15 milhões.", tool_call_id="call_1"),
        ]

        pruned = _prune_messages_for_llm(
            messages,
            active_workspace="FinanceWS",
            grounding_mode="strict",
            web_search_enabled=True
        )

        # Turn 1 HumanMessage should NOT have the turn grounding header injected
        self.assertEqual(pruned[0].content, "Turn 1: Qual o faturamento de 2024?")
        # Turn 1 AIMessage should be untouched
        self.assertEqual(pruned[1].content, "O faturamento de 2024 foi de R$ 10 milhões.")
        # Turn 2 HumanMessage (the active turn) MUST have the turn grounding header injected
        self.assertIn("[GROUNDING: STRICT", pruned[2].content)
        self.assertIn("Turn 2: E a projeção para 2025?", pruned[2].content)
        self.assertIn("Workspace: 'FinanceWS'", pruned[2].content)

        safe_stdout_write("  [OK] Strategy turn header injected strictly on the active turn without history pollution!\n")

    def test_07_is_web_search_authorized_with_injected_header(self):
        """Validates that _is_web_search_authorized_by_prompt accurately parses prompts even with injected headers."""
        safe_stdout_write(">>> [CORE UNIT] Testing Web Search Authorization with Injected Headers...\n")
        from any_context.core.agent import _is_web_search_authorized_by_prompt
        from any_context.core.utils import format_turn_grounding_header

        header = format_turn_grounding_header("LegalDoc", "strict", web_search_enabled=True)

        unauthorized_prompt = f"{header}\n\nQual a multa rescisória?"
        self.assertFalse(_is_web_search_authorized_by_prompt(unauthorized_prompt))

        authorized_confirm = f"{header}\n\nSim, pode pesquisar na web"
        self.assertTrue(_is_web_search_authorized_by_prompt(authorized_confirm))

        authorized_short = f"{header}\n\nsim"
        self.assertTrue(_is_web_search_authorized_by_prompt(authorized_short))

        safe_stdout_write("  [OK] Web search authorization helper is 100% resilient to header-prefixed prompts!\n")

    def test_08_registered_workspace_portals_priority_and_recency_rule(self):
        """Validates that registered workspace portals receive Priority 0 and recency rules are injected."""
        safe_stdout_write(">>> [CORE UNIT] Testing Registered Workspace Portals Priority & Recency Rule...\n")
        from unittest.mock import patch
        from any_context.core.utils import format_turn_grounding_header

        mock_urls = [{"url": "https://www.canada.ca/en/immigration.html"}]
        with patch("any_context.ingestion.web_scheduler.WebSchedulerStore.get_workspace_web_urls", return_value=mock_urls):
            # Hybrid with registered portal
            hyb_portal = format_turn_grounding_header("CanadaWS", "hybrid", web_search_enabled=True)
            self.assertIn("Priority 0: VectorDB & Registered Portals (canada.ca)", hyb_portal)
            self.assertIn("query registered workspace portal (canada.ca) FIRST", hyb_portal)
            self.assertIn("RECENCY RULE (SAME PRIORITY)", hyb_portal)

            # Strict with registered portal
            strict_portal = format_turn_grounding_header("CanadaWS", "strict", web_search_enabled=True)
            self.assertIn("Priority 0: VectorDB & Registered Portals (canada.ca)", strict_portal)
            self.assertIn("query registered portal (canada.ca) FIRST before open web", strict_portal)
            self.assertIn("RECENCY RULE (SAME PRIORITY)", strict_portal)

        safe_stdout_write("  [OK] Registered workspace portals mapped to Priority 0 with universal recency rules!\n")

    def test_09_rpc_bridge_agent_invalidation_on_web_search_toggle(self):
        """Validates that rpc_bridge invalidates cached agent_instance when web_search_enabled is toggled."""
        safe_stdout_write(">>> [CORE UNIT] Testing RPC Bridge Agent Invalidation on Web Search Toggle...\n")
        from unittest.mock import MagicMock
        from any_context.server.rpc_bridge import StdioRPCServer

        server = StdioRPCServer(default_workspace="TestWS", store=self.store)
        # Mock agent instance
        dummy_agent = MagicMock()
        server.agent_instance = dummy_agent
        server._agent_sig = ("TestWS", "gpt-4o-mini", "strict", False)
        server._web_search_enabled = False

        # Execute /web-search on
        server.handle_request({
            "id": 100,
            "method": "execute_command",
            "params": {"command": "/web-search on"}
        })

        # agent_instance MUST be invalidated to None
        self.assertIsNone(server.agent_instance)
        self.assertTrue(server._web_search_enabled)

        safe_stdout_write("  [OK] RPC Bridge safely invalidates agent instance on web_search_enabled toggle!\n")


if __name__ == "__main__":
    unittest.main()

