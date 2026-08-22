import os
import unittest
import tempfile
import shutil
from any_context.vector_engine.enricher import ContextualEnricher, SemanticEnvelope

class TestContextualEnricher(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.mkdtemp()
        self.db_file = os.path.join(self.test_dir, "test_enricher.db")
        # Reset singleton instance for isolated test
        ContextualEnricher._instance = None
        self.enricher = ContextualEnricher(db_path=self.db_file)

    def tearDown(self):
        ContextualEnricher._instance = None
        shutil.rmtree(self.test_dir, ignore_errors=True)

    def test_extract_rich_summary_and_keywords(self):
        text = """
        # Regras de Imigração e Viagem de Menores no Canadá
        Diretrizes oficiais para entrada e permanência de crianças menores desacompanhadas ou com um único dos pais no Canadá.
        Requisitos obrigatórios incluem carta de consentimento autenticada, documentos de custódia legal e comprovante de tutela.
        Autoridades de fronteira do IRCC realizam verificações rigorosas para garantir a segurança dos menores.
        Prazos e penalidades aplicam-se a declarações incompletas ou ausência de autorização formal.
        """
        envelope = self.enricher.extract_rich_summary_and_keywords(
            doc_text=text,
            file_name="Regras_Menores_Canada.md",
            file_path="/path/to/Regras_Menores_Canada.md"
        )

        self.assertIsInstance(envelope, SemanticEnvelope)
        self.assertIn("Regras de Imigração", envelope.summary)
        self.assertGreaterEqual(len(envelope.keywords), 3)
        self.assertTrue(any("menores" in k.lower() or "viagem" in k.lower() or "canadá" in k.lower() for k in envelope.keywords))

    def test_sha256_cache_hit(self):
        text = "Documento confidencial sobre auditoria de TI e seguranca da informacao."
        env1 = self.enricher.extract_rich_summary_and_keywords(
            doc_text=text,
            file_name="Auditoria_TI.pdf"
        )
        # Second call with exact same text should hit SQLite cache
        env2 = self.enricher.extract_rich_summary_and_keywords(
            doc_text=text,
            file_name="Auditoria_TI.pdf"
        )
        self.assertEqual(env1.content_hash, env2.content_hash)
        self.assertEqual(env1.summary, env2.summary)
        self.assertEqual(env1.keywords, env2.keywords)

    def test_apply_envelope_to_chunk(self):
        text = "Parágrafo sobre documentação de custódia."
        envelope = SemanticEnvelope(
            summary="Documento 'Regras_Menores.pdf': Regras de custódia para menores no Canadá.",
            keywords=["custódia", "menores", "canadá", "autorização"],
            content_hash="dummy_hash_123",
            file_name="Regras_Menores.pdf"
        )
        enveloped_chunk = self.enricher.apply_envelope_to_chunk(text, envelope)
        self.assertIn("[Context:", enveloped_chunk)
        self.assertIn("Keywords: custódia, menores, canadá, autorização", enveloped_chunk)
        self.assertIn("Parágrafo sobre documentação de custódia.", enveloped_chunk)

if __name__ == "__main__":
    unittest.main()
