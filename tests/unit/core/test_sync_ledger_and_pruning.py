import os
import sys
import tempfile
import shutil
import unittest
from langchain_core.messages import HumanMessage, AIMessage

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.config.db_store import ConfigDBStore
from any_context.core.agent import _strip_historical_citation_footers, _prune_messages_for_llm
from any_context.core.utils import get_system_prompt
from any_context.server.rpc_bridge import StdioRPCServer
from tests.e2e_helpers import safe_stdout_write


class TestSyncLedgerAndPruning(unittest.TestCase):
    """
    Unit Test Suite: Validates the Workspace Sync Ledger and historical
    citation pruning architecture to prevent LLM hallucination/citation of deleted files.
    """

    def setUp(self):
        self.temp_dir = tempfile.mkdtemp(prefix="actx_test_ledger_")
        self.db_path = os.path.join(self.temp_dir, "test_settings.db")
        self._orig_env = os.environ.get("ACTX_SETTINGS_DB")
        self._orig_test_mode = os.environ.get("ACTX_TEST_MODE")
        os.environ["ACTX_SETTINGS_DB"] = self.db_path
        os.environ["ACTX_TEST_MODE"] = "1"
        self.store = ConfigDBStore(db_path=self.db_path)
        self.store.add_workspace("TestWS", paths=["/dummy/path"])

    def tearDown(self):
        if self._orig_env is not None:
            os.environ["ACTX_SETTINGS_DB"] = self._orig_env
        else:
            os.environ.pop("ACTX_SETTINGS_DB", None)
        if self._orig_test_mode is not None:
            os.environ["ACTX_TEST_MODE"] = self._orig_test_mode
        else:
            os.environ.pop("ACTX_TEST_MODE", None)
        if hasattr(self, "temp_dir") and os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_01_sync_ledger_crud(self):
        """Validates record, retrieve, and clear operations on the workspace_sync_ledger."""
        safe_stdout_write("\n>>> [SYNC LEDGER] Testing record, retrieve, and clear...\n")
        
        # Initial state should have empty lists
        ledger = self.store.get_sync_ledger("TestWS")
        self.assertEqual(ledger["deleted_sources"], [])
        self.assertEqual(ledger["added_sources"], [])
        self.assertEqual(ledger["modified_sources"], [])
        self.assertIsNone(ledger["last_sync_timestamp"])

        # Record a sync with deletions and additions
        deleted = ["/docs/report_old.pdf", "/docs/invoice_2025.xlsx"]
        added = ["/docs/report_2026.pdf"]
        modified = ["/docs/summary.md"]

        self.store.record_sync_ledger(
            workspace_name="TestWS",
            deleted_sources=deleted,
            added_sources=added,
            modified_sources=modified
        )

        ledger = self.store.get_sync_ledger("TestWS")
        self.assertIsNotNone(ledger)
        self.assertEqual(ledger["workspace"], "TestWS")
        self.assertEqual(ledger["deleted_sources"], deleted)
        self.assertEqual(ledger["added_sources"], added)
        self.assertEqual(ledger["modified_sources"], modified)
        self.assertIsNotNone(ledger["last_sync_timestamp"])

        # Clear ledger
        self.store.clear_sync_ledger("TestWS")
        ledger_after = self.store.get_sync_ledger("TestWS")
        self.assertEqual(ledger_after["deleted_sources"], [])
        self.assertIsNone(ledger_after["last_sync_timestamp"])

    def test_02_strip_historical_citation_footers(self):
        """Validates that _strip_historical_citation_footers accurately strips citation footers while preserving content."""
        safe_stdout_write("\n>>> [PRUNING] Testing citation footer stripping...\n")

        msg_with_footer = (
            "Aqui está o resumo da remessa 12345:\n"
            "- Peso total: 500kg\n"
            "- Data de coleta: 01/09/2026\n\n"
            "📄 Fontes Consultadas no Workspace:\n"
            "  - I.CMR_ONE_PICKUP.pdf (C:/path/I.CMR_ONE_PICKUP.pdf)\n"
            "  - Report.pdf (C:/path/Report.pdf)\n"
        )

        stripped = _strip_historical_citation_footers(msg_with_footer)
        self.assertIn("Aqui está o resumo da remessa 12345:", stripped)
        self.assertIn("- Peso total: 500kg", stripped)
        self.assertIn("- Data de coleta: 01/09/2026", stripped)
        self.assertNotIn("📄 Fontes Consultadas", stripped)
        self.assertNotIn("I.CMR_ONE_PICKUP.pdf", stripped)

        # Also test with markdown divider '---'
        msg_with_divider = (
            "Análise do contrato concluída com sucesso.\n\n"
            "---\n"
            "📄 **Fontes Consultadas (Arquivos Locais):**\n"
            "- contrato_2026.pdf"
        )
        stripped_divider = _strip_historical_citation_footers(msg_with_divider)
        self.assertIn("Análise do contrato concluída com sucesso.", stripped_divider)
        self.assertNotIn("contrato_2026.pdf", stripped_divider)
        self.assertNotIn("Fontes Consultadas", stripped_divider)

        # Message without footer should remain unchanged
        msg_without_footer = "Olá! Como posso ajudar você hoje com seus documentos?"
        self.assertEqual(_strip_historical_citation_footers(msg_without_footer), msg_without_footer)

    def test_03_prune_messages_for_llm_strips_prior_citations(self):
        """Validates that _prune_messages_for_llm sanitizes expurgated deleted files from citations while preserving active provenance."""
        safe_stdout_write("\n>>> [PRUNING] Testing message pruning pipeline...\n")

        # Record deleted file in TestWS ledger
        self.store.record_sync_ledger(
            workspace_name="TestWS",
            deleted_sources=["/path/to/deleted_doc.pdf"],
            added_sources=[],
            modified_sources=[]
        )

        hist_ai_text = (
            "A taxa de entrega foi confirmada no documento antigo.\n\n"
            "📄 Fontes Consultadas no Workspace:\n"
            "  - deleted_doc.pdf (/path/to/deleted_doc.pdf)\n"
        )
        messages = [
            HumanMessage(content="Qual era a taxa de entrega?"),
            AIMessage(content=hist_ai_text),
            HumanMessage(content="E qual é a taxa atualizada hoje?")
        ]

        pruned = _prune_messages_for_llm(messages, active_workspace="TestWS")
        self.assertEqual(len(pruned), 3)

        # The historical AIMessage should have its citation footer removed because deleted_doc.pdf was purged
        pruned_ai_content = pruned[1].content
        self.assertIn("A taxa de entrega foi confirmada no documento antigo.", pruned_ai_content)
        self.assertNotIn("📄 Fontes Consultadas", pruned_ai_content)
        self.assertNotIn("deleted_doc.pdf", pruned_ai_content)

        # Active file on immediate prior turn MUST be preserved so follow-up provenance works
        active_ai_text = (
            "A taxa de entrega foi confirmada no documento ativo.\n\n"
            "📄 Fontes Consultadas:\n"
            "- active_report.pdf (Última Modificação: 2026-09-01)"
        )
        messages_active = [
            HumanMessage(content="Qual era a taxa de entrega?"),
            AIMessage(content=active_ai_text),
            HumanMessage(content="Qual arquivo foi a fonte dessa informação?")
        ]
        pruned_active = _prune_messages_for_llm(messages_active, active_workspace="TestWS")
        self.assertIn("active_report.pdf", pruned_active[1].content)
        self.assertIn("📄 Fontes Consultadas", pruned_active[1].content)

    def test_04_system_prompt_includes_ledger_directive(self):
        """Validates that get_system_prompt injects deleted sources warning when ledger has deletions."""
        safe_stdout_write("\n>>> [SYSTEM PROMPT] Testing ledger directive injection...\n")

        # Record deleted file
        self.store.record_sync_ledger(
            workspace_name="TestWS",
            deleted_sources=["/path/to/I.CMR_ONE_PICKUP.pdf"],
            added_sources=["/path/to/new_manifest.pdf"],
            modified_sources=[]
        )

        prompt = get_system_prompt(active_workspace="TestWS", store=self.store)
        self.assertIn("WORKSPACE SYNC LEDGER (RECENT MUTATIONS)", prompt)
        self.assertIn("I.CMR_ONE_PICKUP.pdf", prompt)
        self.assertIn("CRITICAL FACTUAL CONSISTENCY ON DELETED FILES", prompt)
        self.assertIn("STRICTLY FORBIDDEN from citing them as active sources", prompt)
        self.assertIn("new_manifest.pdf", prompt)

    def test_05_rpc_bridge_detects_changes_on_switch(self):
        """Validates that StdioRPCServer checks workspace changes strictly on switch and displays yellow badge."""
        safe_stdout_write("\n>>> [RPC BRIDGE] Testing workspace switch change detection...\n")

        from unittest.mock import patch
        with patch("any_context.ingestion.orchestrator.check_workspace_changes") as mock_chk:
            mock_chk.return_value = {
                "has_changes": True,
                "deleted_files": ["/doc/deleted.pdf"],
                "new_files": [],
                "modified_files": []
            }
            server = StdioRPCServer(default_workspace="TestWS", store=self.store)
            state = server.get_state()
            self.assertIn("sync_info", state)
            self.assertEqual(state["sync_info"], "🟡 1 deleted • /sync")


if __name__ == "__main__":
    unittest.main()
