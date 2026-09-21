import os
import unittest
from unittest.mock import patch

from any_context.core.utils import get_api_key
from any_context.help.bootstrap import ensure_system_knowledge_indexed
from any_context.tools.search_tools import _execute_search_context
from any_context.vector_engine.store import LanceDBStore
from any_context.vector_engine.retriever import ParallelRetriever
from any_context.vector_engine.models import RetrievalConfig
import any_context_core_rs


class TestGlobalHelpCrossWorkspaceRetrieval(unittest.TestCase):
    """
    Validates cross-workspace Global help knowledge retrieval, runtime credential export,
    and native Rust BM25 tolerance for universal system knowledge.
    """

    def test_01_get_api_key_runtime_environ_export(self):
        """Tests that get_api_key exports resolved key to os.environ for third-party libraries."""
        old_env = os.environ.get("OPENAI_API_KEY")
        try:
            if "OPENAI_API_KEY" in os.environ:
                del os.environ["OPENAI_API_KEY"]

            resolved = get_api_key("openai")
            if resolved and resolved not in ["lm-studio", "placeholder"]:
                self.assertIn("OPENAI_API_KEY", os.environ)
                self.assertEqual(os.environ["OPENAI_API_KEY"], resolved)
        finally:
            if old_env is not None:
                os.environ["OPENAI_API_KEY"] = old_env

    def test_02_rust_bm25_allows_global_workspace_across_project_workspaces(self):
        """
        Tests that any_context_core_rs BM25 search does NOT filter out documents
        with workspace='Global' when workspace='IKEAShipments' is requested.
        """
        engine = any_context_core_rs.HybridRetrieverEngine()
        chunks = [
            {
                "id": "ikea_1",
                "text": "Checklist de transporte e remessas de mercadorias",
                "file_name": "checklist.pdf",
                "file_path": "/docs/checklist.pdf",
                "workspace": "IKEAShipments",
                "content_type": "PDF Document"
            },
            {
                "id": "global_link_1",
                "text": "O comando /link vincula repositórios de Shared Sources a um workspace ativo",
                "file_name": "help_registry",
                "file_path": "system://help_registry",
                "workspace": "Global",
                "content_type": "Plain Text Document"
            },
            {
                "id": "other_ws_1",
                "text": "Declaração de imposto de renda e deduções fiscais",
                "file_name": "irpf.pdf",
                "file_path": "/docs/irpf.pdf",
                "workspace": "TaxReturn",
                "content_type": "PDF Document"
            }
        ]
        engine.add_chunks_batch(chunks)

        # Search with workspace='IKEAShipments'
        matches = engine.bm25_search("/link comando Shared Sources", 10, "IKEAShipments") if hasattr(engine, "bm25_search") else None
        
        # Or search via fuse_and_diversify
        fused = engine.fuse_and_diversify(
            dense_results=[],
            query="/link comando Shared Sources",
            workspace="IKEAShipments",
            candidate_pool_k=10,
            target_top_k=5,
            max_per_source=2,
            max_density_chars=10000,
            rrf_k=60
        )
        fused_workspaces = [c["workspace"] for c in fused]
        self.assertIn("Global", fused_workspaces)
        self.assertNotIn("TaxReturn", fused_workspaces)

    def test_03_search_context_retrieves_global_from_project_workspace(self):
        """
        Tests that _execute_search_context from inside a project workspace
        retrieves Global documentation chunks for system command questions.
        """
        res = _execute_search_context(
            prompt_text="Como funciona o comando /link e para que serve o Shared Sources?",
            workspace="IKEAShipments"
        )
        self.assertTrue(len(res) > 100)
        self.assertTrue("Workspace: Global" in res or "help_registry" in res or "readme" in res)


if __name__ == "__main__":
    unittest.main()
