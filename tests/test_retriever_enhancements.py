"""
Tests for Query Expansion, Filename Grounding / Boost, and Atomic PDF Page Chunking.
Validates dev-cycle-protocol requirements for intelligent temporal & document retrieval.
"""
import pytest
from any_context.vector_engine.retriever import (
    expand_query_temporal,
    extract_filename_mentions,
    extract_temporal_clauses,
    ParallelRetriever,
)
from any_context.vector_engine.models import ScoredChunk, RetrievalConfig


def test_expand_query_temporal_pt_br():
    # 1. Full date in Portuguese
    q1 = "Quais são os agendamentos para 3 de Setembro de 2026?"
    exp1 = expand_query_temporal(q1)
    assert "2026-09-03" in exp1
    assert "2026/09/03" in exp1
    assert "03/09/2026" in exp1
    assert "09/03" in exp1

    # 2. Day and month without year
    q2 = "Verifique os registros do dia 01 de setembro."
    exp2 = expand_query_temporal(q2)
    assert "09-01" in exp2 or "09/01" in exp2

    # 3. Brazilian numeric format
    q3 = "O que aconteceu em 15/08/2026?"
    exp3 = expand_query_temporal(q3)
    assert "2026-08-15" in exp3
    assert "2026/08/15" in exp3
    assert "15/08/2026" in exp3


def test_expand_query_temporal_en():
    # 1. Full date in US English
    q1 = "What are the shipments scheduled for September 3, 2026?"
    exp1 = expand_query_temporal(q1)
    assert "2026-09-03" in exp1
    assert "2026/09/03" in exp1
    assert "03/09/2026" in exp1

    # 2. Month and day
    q2 = "Check report for August 15."
    exp2 = expand_query_temporal(q2)
    assert "08-15" in exp2 or "08/15" in exp2


def test_expand_query_no_dates():
    q = "Quais são as diretrizes gerais de segurança da empresa?"
    assert expand_query_temporal(q) == q


def test_extract_filename_mentions():
    # 1. Filename in parentheses
    q1 = "No documento CMR de 02/09/2026 (I.CMR_ONE_PICKUP.pdf), quem é o Consignee?"
    files1 = extract_filename_mentions(q1)
    assert files1 == ["I.CMR_ONE_PICKUP.pdf"]

    # 2. Multiple filenames with different extensions
    q2 = "Compare extraction_summary.csv com o arquivo 015-TSO-S10000527408.pdf e report.docx"
    files2 = extract_filename_mentions(q2)
    assert "extraction_summary.csv" in files2
    assert "015-TSO-S10000527408.pdf" in files2
    assert "report.docx" in files2

    # 3. No filenames
    q3 = "Liste todos os transportadores cadastrados."
    assert extract_filename_mentions(q3) == []


def test_extract_temporal_clauses_preserves_path_patterns():
    q = "Agendamentos para 2 de setembro de 2026"
    clauses = extract_temporal_clauses(q)
    assert any("2026/09/02" in c for c in clauses)
    assert any("2026-09-02" in c for c in clauses)


def test_parallel_retriever_filename_boost():
    class MockStore:
        def search_metadata(self, where_clause, limit, workspace=None, table_name=None):
            if "I.CMR_ONE_PICKUP.pdf" in where_clause:
                return [
                    ScoredChunk(
                        text="| IKEA CALGARY | BISON TRANSPORT INC. |",
                        file_name="I.CMR_ONE_PICKUP.pdf",
                        file_path="C:/docs/2026/09/02/I.CMR_ONE_PICKUP.pdf",
                        workspace="IKEAShipments",
                        score=0.5,
                        chunk_id="cmr_page_1"
                    )
                ]
            return []

        def search_vector(self, query_vector, limit, workspace=None, filter_expr=None, prefilter=None, table_name=None):
            return [
                ScoredChunk(
                    text="General checklist item 1",
                    file_name="checklist.pdf",
                    file_path="C:/docs/checklist.pdf",
                    workspace="IKEAShipments",
                    score=0.8,
                    chunk_id="chk_1"
                )
            ]

        def get_hybrid_engine(self, table_name=None):
            class MockHybridEngine:
                def fuse_and_diversify(self, dense_results, query, workspace=None, candidate_pool_k=100,
                                       target_top_k=20, max_per_source=3, max_density_chars=40000, rrf_k=60):
                    return [
                        {
                            "id": d["id"],
                            "score": d["score"],
                            "text": d["text"],
                            "file_name": d["file_name"],
                            "file_path": d["file_path"],
                            "workspace": d["workspace"],
                            "content_type": d["content_type"]
                        }
                        for d in dense_results
                    ]
            return MockHybridEngine()

    retriever = ParallelRetriever(store=MockStore())
    retriever._get_query_embedding = lambda q: [0.0] * 1536

    query = "No arquivo I.CMR_ONE_PICKUP.pdf de 02/09/2026, quem é o transportador?"
    results = retriever.search(query, workspace="IKEAShipments", config=RetrievalConfig.from_preset("balanced"))

    assert len(results) >= 1
    # First chunk must be the boosted target file chunk
    assert results[0].chunk_id == "cmr_page_1"
    assert results[0].score == 1.0
    assert "BISON TRANSPORT INC." in results[0].text
