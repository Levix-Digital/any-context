import unittest
from any_context.core.agent import _filter_citation_footer, _strip_historical_citation_footers
from any_context.vector_engine.retriever import extract_temporal_clauses
from any_context.core.grounding_strategies import StrictGroundingStrategy, HybridGroundingStrategy, ProactiveGroundingStrategy


class TestCitationProvenance(unittest.TestCase):
    def test_immediate_prior_turn_preserves_active_citations(self):
        msg = (
            "No dia 1 de setembro de 2026, tivemos quatro Shipments registrados.\n\n"
            "📄 Fontes Consultadas:\n"
            "- 015-TSO-26P1EC200774.pdf (Última Modificação: 2026-09-01)\n"
            "- 015-TSO-26P3EC200775.pdf (Última Modificação: 2026-09-01)"
        )
        filtered = _filter_citation_footer(msg, deleted_basenames=set(), is_immediate_prior=True)
        self.assertIn("📄 Fontes Consultadas:", filtered)
        self.assertIn("015-TSO-26P1EC200774.pdf", filtered)
        self.assertIn("015-TSO-26P3EC200775.pdf", filtered)

    def test_immediate_prior_turn_purges_deleted_file(self):
        msg = (
            "Foram localizados dois relatórios:\n\n"
            "📄 Fontes Consultadas:\n"
            "- active_report.pdf (Última Modificação: 2026-09-01)\n"
            "- deleted_report.pdf (Última Modificação: 2026-08-01)"
        )
        filtered = _filter_citation_footer(msg, deleted_basenames={"deleted_report.pdf"}, is_immediate_prior=True)
        self.assertIn("active_report.pdf", filtered)
        self.assertNotIn("deleted_report.pdf", filtered)

    def test_all_deleted_files_strips_footer(self):
        msg = (
            "Conteúdo anterior sintetizado.\n\n"
            "📄 Fontes Consultadas:\n"
            "- deleted_file.pdf (Última Modificação: 2026-08-01)"
        )
        filtered = _filter_citation_footer(msg, deleted_basenames={"deleted_file.pdf"}, is_immediate_prior=True)
        self.assertNotIn("📄 Fontes Consultadas:", filtered)
        self.assertNotIn("deleted_file.pdf", filtered)
        self.assertEqual(filtered.strip(), "Conteúdo anterior sintetizado.")

    def test_older_historical_turn_condenses_footer(self):
        msg = (
            "Resposta antiga do turno 1.\n\n"
            "📄 Fontes Consultadas:\n"
            "- docA.pdf (Última Modificação: 2026-05-01)\n"
            "- docB.pdf (Última Modificação: 2026-05-02)"
        )
        filtered = _filter_citation_footer(msg, deleted_basenames=set(), is_immediate_prior=False)
        self.assertIn("📄 Fontes Consultadas: docA.pdf, docB.pdf", filtered)


class TestTemporalExtraction(unittest.TestCase):
    def test_iso_date_extraction(self):
        clauses = extract_temporal_clauses("Qual o status em 2026-09-01?")
        self.assertIn("file_path LIKE '%2026/09/01%'", clauses)
        self.assertIn("file_path LIKE '%2026-09-01%'", clauses)

    def test_brazilian_date_extraction(self):
        clauses = extract_temporal_clauses("Checklist do dia 01/09/2026")
        self.assertIn("file_path LIKE '%2026/09/01%'", clauses)
        self.assertIn("file_path LIKE '%2026-09-01%'", clauses)

    def test_natural_language_portuguese(self):
        clauses = extract_temporal_clauses("No dia 1 de setembro de 2026, quantos Shipments tivemos registrados?")
        self.assertIn("file_path LIKE '%2026/09/01%'", clauses)
        self.assertIn("file_path LIKE '%2026-09-01%'", clauses)

    def test_natural_language_english(self):
        clauses = extract_temporal_clauses("Show shipments on September 1, 2026")
        self.assertIn("file_path LIKE '%2026/09/01%'", clauses)
        self.assertIn("file_path LIKE '%2026-09-01%'", clauses)

    def test_no_date_query_returns_empty(self):
        clauses = extract_temporal_clauses("Quais são as políticas gerais de frete da empresa?")
        self.assertEqual(clauses, [])


class TestGroundingStrategiesCitationDirective(unittest.TestCase):
    def test_strict_mode_mandates_citation_footer(self):
        strat = StrictGroundingStrategy()
        header_no_web = strat.format_turn_header("TestWS", web_search_enabled=False)
        self.assertIn("MANDATORY CITATION FOOTER", header_no_web)
        self.assertIn("📄 Fontes Consultadas:", header_no_web)

        header_web = strat.format_turn_header("TestWS", web_search_enabled=True)
        self.assertIn("MANDATORY CITATION FOOTER", header_web)

    def test_hybrid_mode_mandates_citation_footer(self):
        strat = HybridGroundingStrategy()
        header = strat.format_turn_header("TestWS", web_search_enabled=False)
        self.assertIn("MANDATORY CITATION FOOTER", header)

    def test_proactive_mode_mandates_citation_footer(self):
        strat = ProactiveGroundingStrategy()
        header = strat.format_turn_header("TestWS", web_search_enabled=False)
        self.assertIn("MANDATORY CITATION FOOTER", header)


if __name__ == "__main__":
    unittest.main()
