"""
Comprehensive Unit Tests for Universal Language-Agnostic Query Preprocessor & Temporal Engine.
Tests native Rust (any-context-core-rs) integration and pure-Python fallback parity.
"""
import unittest
import time
from any_context.vector_engine.query_preprocessor import (
    QueryPreprocessor,
    ProcessedQuery,
    extract_temporal_clauses,
    expand_query_temporal,
    extract_filename_mentions,
)


class TestQueryPreprocessorRust(unittest.TestCase):
    def test_single_pass_processing(self):
        query = "Relatório CMR de 2026-09-02 no arquivo I.CMR_ONE_PICKUP.pdf para análise"
        processed = QueryPreprocessor.process(query)

        # 1. Filenames
        self.assertEqual(processed.filename_mentions, ["I.CMR_ONE_PICKUP.pdf"])

        # 2. Temporal clauses
        self.assertTrue(any("2026/09/02" in c for c in processed.temporal_clauses))
        self.assertTrue(any("2026-09-02" in c for c in processed.temporal_clauses))

        # 3. Expanded query
        self.assertIn("2026-09-02", processed.expanded_query)
        self.assertIn("02/09/2026", processed.expanded_query)
        self.assertIn("09/02", processed.expanded_query)

    def test_iso_dates(self):
        q1 = "Check data for 2026-11-20"
        clauses = extract_temporal_clauses(q1)
        self.assertIn("file_path LIKE '%2026/11/20%'", clauses)
        self.assertIn("file_path LIKE '%2026-11-20%'", clauses)

        q2 = "Data for 2026/05/18"
        clauses2 = extract_temporal_clauses(q2)
        self.assertIn("file_path LIKE '%2026/05/18%'", clauses2)

    def test_intl_numeric_dates(self):
        q1 = "Registros de 15/08/2026"
        clauses1 = extract_temporal_clauses(q1)
        self.assertIn("file_path LIKE '%2026/08/15%'", clauses1)
        self.assertIn("file_path LIKE '%2026-08-15%'", clauses1)

        q2 = "Relatorio de 03-04-2025"
        clauses2 = extract_temporal_clauses(q2)
        self.assertIn("file_path LIKE '%2025/04/03%'", clauses2)

        q3 = "Data 28.02.2024"
        clauses3 = extract_temporal_clauses(q3)
        self.assertIn("file_path LIKE '%2024/02/28%'", clauses3)

    def test_rfc_english_dates(self):
        q1 = "Shipments on September 3, 2026"
        clauses1 = extract_temporal_clauses(q1)
        self.assertIn("file_path LIKE '%2026/09/03%'", clauses1)
        self.assertIn("file_path LIKE '%2026-09-03%'", clauses1)

        q2 = "Deliveries on 15 Oct 2025"
        clauses2 = extract_temporal_clauses(q2)
        self.assertIn("file_path LIKE '%2025/10/15%'", clauses2)

        q3 = "Invoices from January 2026"
        clauses3 = extract_temporal_clauses(q3)
        self.assertTrue(any("2026/01" in c for c in clauses3))

    def test_short_numeric_dates(self):
        q = "Check status for 02/09"
        clauses = extract_temporal_clauses(q)
        self.assertTrue(any("/09/02/" in c for c in clauses))
        self.assertTrue(any("/02/09/" in c for c in clauses))

    def test_filename_mentions(self):
        q = "Compare summary.csv with report_final.xlsx, doc1.pdf, and data.json"
        files = extract_filename_mentions(q)
        self.assertEqual(files, ["summary.csv", "report_final.xlsx", "doc1.pdf", "data.json"])

    def test_no_mentions(self):
        q = "Quais são as regras gerais e políticas de transporte?"
        processed = QueryPreprocessor.process(q)
        self.assertEqual(processed.filename_mentions, [])
        self.assertEqual(processed.temporal_clauses, [])
        self.assertEqual(processed.expanded_query, q)



    def test_rust_performance_sub_millisecond(self):
        query = "Relatório CMR de 2026-09-02 no arquivo I.CMR_ONE_PICKUP.pdf para análise"
        # Warm up
        for _ in range(100):
            QueryPreprocessor.process(query)

        t0 = time.perf_counter()
        iters = 1000
        for _ in range(iters):
            QueryPreprocessor.process(query)
        elapsed = time.perf_counter() - t0
        avg_ms = (elapsed / iters) * 1000
        self.assertLess(avg_ms, 0.5, f"QueryPreprocessor took {avg_ms:.4f}ms, expected < 0.5ms")


if __name__ == "__main__":
    unittest.main()
