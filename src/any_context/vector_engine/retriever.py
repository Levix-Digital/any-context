"""
Parallel Vector & BM25 Hybrid Retriever with Reciprocal Rank Fusion (Marco 6).
Provides concurrent multi-source dense vector search across LanceDB partitions,
sparse Okapi BM25 keyword search in native Rust, and unified RRF fusion with
source-fair diversification and density budgeting.
"""
import os
import re
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import List, Optional, Dict, Any

from any_context.vector_engine.models import ScoredChunk, RetrievalConfig
from any_context.vector_engine.store import LanceDBStore


MONTH_MAP = {
    "janeiro": "01", "january": "01", "jan": "01",
    "fevereiro": "02", "february": "02", "fev": "02", "feb": "02",
    "março": "03", "marco": "03", "march": "03", "mar": "03",
    "abril": "04", "april": "04", "abr": "04", "apr": "04",
    "maio": "05", "may": "05", "mai": "05",
    "junho": "06", "june": "06", "jun": "06",
    "julho": "07", "july": "07", "jul": "07",
    "agosto": "08", "august": "08", "ago": "08", "aug": "08",
    "setembro": "09", "september": "09", "set": "09", "sep": "09",
    "outubro": "10", "october": "10", "out": "10", "oct": "10",
    "novembro": "11", "november": "11", "nov": "11",
    "dezembro": "12", "december": "12", "dez": "12", "dec": "12"
}


def extract_temporal_clauses(query: str) -> List[str]:
    """
    Extracts deterministic date and directory path filters from conversational queries
    in both ISO, Brazilian/European (DD/MM/YYYY), and Portuguese/English natural language.
    Eliminates semantic dilution of dates in dense vector embeddings.
    """
    clauses = []
    q = query.lower()

    # 1. ISO format: YYYY-MM-DD or YYYY/MM/DD
    iso_matches = re.findall(r"\b(20\d{2})[-/](0[1-9]|1[0-2])[-/](0[1-9]|[12]\d|3[01])\b", q)
    for y, m, d in iso_matches:
        clauses.append(f"file_path LIKE '%{y}/{m}/{d}%'")
        clauses.append(f"file_path LIKE '%{y}-{m}-{d}%'")
        clauses.append(f"file_path LIKE '%/{m}/{d}/%'")
        clauses.append(f"file_path LIKE '%/{m}/{d}%'")

    # 2. Brazilian / European format: DD/MM/YYYY or DD-MM-YYYY
    br_matches = re.findall(r"\b(0[1-9]|[12]\d|3[01])[-/](0[1-9]|1[0-2])[-/](20\d{2})\b", q)
    for d, m, y in br_matches:
        clauses.append(f"file_path LIKE '%{y}/{m}/{d}%'")
        clauses.append(f"file_path LIKE '%{y}-{m}-{d}%'")
        clauses.append(f"file_path LIKE '%/{m}/{d}/%'")
        clauses.append(f"file_path LIKE '%/{m}/{d}%'")

    # Helper to add standardized path patterns
    def _add_clauses(yy_val: Optional[str], mm_val: str, dd_val: Optional[str]):
        if yy_val and dd_val:
            clauses.append(f"file_path LIKE '%{yy_val}/{mm_val}/{dd_val}%'")
            clauses.append(f"file_path LIKE '%{yy_val}-{mm_val}-{dd_val}%'")
            clauses.append(f"file_path LIKE '%/{mm_val}/{dd_val}/%'")
            clauses.append(f"file_path LIKE '%/{mm_val}/{dd_val}%'")
        elif dd_val:
            clauses.append(f"file_path LIKE '%/{mm_val}/{dd_val}/%'")
            clauses.append(f"file_path LIKE '%/{mm_val}/{dd_val}%'")
        elif yy_val:
            clauses.append(f"file_path LIKE '%/{yy_val}/{mm_val}/%'")
            clauses.append(f"file_path LIKE '%/{yy_val}/{mm_val}%'")

    # 3. Natural language (e.g., '1 de setembro de 2026', '01 de setembro', 'September 1, 2026')
    months_pattern = "|".join(sorted(MONTH_MAP.keys(), key=len, reverse=True))

    # Pattern A: Day-Month-Year (Portuguese/British, e.g. "1 de setembro de 2026", "1 sept 2026")
    nl_pattern_dmy = rf"\b(?:(\d{{1,2}})(?:st|nd|rd|th)?\s+(?:de\s+)?)?({months_pattern})(?:\s+(?:de\s+|,)?\s*(20\d{{2}}))?\b"
    for m_day, m_month_name, m_year in re.findall(nl_pattern_dmy, q):
        mm = MONTH_MAP.get(m_month_name)
        if not mm:
            continue
        dd = f"{int(m_day):02d}" if m_day else None
        yy = m_year if m_year else None
        _add_clauses(yy, mm, dd)

    # Pattern B: Month-Day-Year (US English, e.g. "September 1, 2026", "Sep 01 2026")
    nl_pattern_mdy = rf"\b({months_pattern})\s+(\d{{1,2}})(?:st|nd|rd|th)?(?:,?\s+(20\d{{2}}))?\b"
    for m_month_name, m_day, m_year in re.findall(nl_pattern_mdy, q):
        mm = MONTH_MAP.get(m_month_name)
        if not mm:
            continue
        dd = f"{int(m_day):02d}" if m_day else None
        yy = m_year if m_year else None
        _add_clauses(yy, mm, dd)

    return list(dict.fromkeys(clauses))


class ParallelRetriever:
    """
    High-performance parallel hybrid retriever powered 100% by native Rust (any-context-core-rs).
    Orchestrates concurrent LanceDB vector searches, native Rust BM25 lexical retrieval,
    and Reciprocal Rank Fusion (RRF) with Source-Fair Round-Robin and Density Budgeting.
    """

    def __init__(self, store: Optional[LanceDBStore] = None):
        self._store = store or LanceDBStore.get_instance()

    def _get_query_embedding(self, query_text: str) -> List[float]:
        """Generates query embedding vector using LlamaIndex / OpenAI configured model."""
        from llama_index.core.settings import Settings
        from any_context.tools.search_tools import configure_embedding_model

        if getattr(Settings, "_embed_model", None) is None:
            configure_embedding_model()

        return Settings.embed_model.get_query_embedding(query_text)

    def search(
        self,
        query: str,
        workspace: Optional[str] = None,
        target_workspaces: Optional[List[str]] = None,
        linked_sources: Optional[List[str]] = None,
        config: Optional[RetrievalConfig] = None,
        table_name: str = "workspace_chunks"
    ) -> List[ScoredChunk]:
        """
        Executes parallel multi-source vector retrieval and native Rust BM25 search,
        fusing candidates with Reciprocal Rank Fusion (RRF) and applying Source-Fair
        Round-Robin diversification and Density Budgeting in native Rust.
        """
        cfg = config or RetrievalConfig.from_preset("balanced")
        query_vector = self._get_query_embedding(query)

        # Build distinct workspace query targets
        targets = []
        if workspace:
            targets.append((workspace, cfg.candidate_pool_k))

        if target_workspaces:
            for ws in target_workspaces:
                if ws and ws != workspace and (ws, cfg.candidate_pool_k) not in targets:
                    targets.append((ws, max(10, cfg.candidate_pool_k // 2)))

        if not targets:
            targets.append(("Default", cfg.candidate_pool_k))

        raw_candidates_map: Dict[str, ScoredChunk] = {}

        # 0. Temporal Hybrid Lexical Search: if temporal date signals exist, search LanceDB metadata first
        temporal_clauses = extract_temporal_clauses(query)
        if temporal_clauses:
            where_or = " OR ".join(temporal_clauses)
            for ws_name, limit in targets:
                try:
                    temporal_matches = self._store.search_metadata(
                        where_clause=where_or,
                        limit=limit,
                        workspace=ws_name if ws_name != "Default" else None,
                        table_name=table_name
                    )
                    for sc in temporal_matches:
                        sc.score = 0.95
                        cid = sc.chunk_id or f"{sc.file_path}::{sc.text[:80]}"
                        raw_candidates_map[cid] = sc
                except Exception:
                    pass

        # 1. Concurrent dense vector searches across CPU threads in LanceDB
        max_workers = min(len(targets) + (1 if linked_sources else 0), os.cpu_count() or 4)
        with ThreadPoolExecutor(max_workers=max_workers) as executor:
            future_to_target = {
                executor.submit(self._store.search_vector, query_vector, limit, ws_name, None, None, table_name): ws_name
                for ws_name, limit in targets
            }

            # If linked shared sources are specified, search across all workspaces matching path
            if linked_sources:
                filter_or = " OR ".join([f"file_path LIKE '%{l_id}%'" for l_id in linked_sources])
                future_to_target[executor.submit(
                    self._store.search_vector,
                    query_vector,
                    cfg.candidate_pool_k * 2,
                    None,
                    None,
                    filter_or,
                    table_name
                )] = "SharedSources"

            for future in as_completed(future_to_target):
                try:
                    res = future.result()
                    if res:
                        for chunk in res:
                            cid = chunk.chunk_id or f"{chunk.file_path}::{chunk.text[:80]}"
                            if cid not in raw_candidates_map:
                                raw_candidates_map[cid] = chunk
                            else:
                                raw_candidates_map[cid].score = max(raw_candidates_map[cid].score, chunk.score)
                except Exception:
                    pass

        raw_dense_candidates = list(raw_candidates_map.values())
        raw_dense_candidates.sort(key=lambda c: c.score, reverse=True)

        # 2. Native Rust Hybrid Retrieval with Reciprocal Rank Fusion (RRF)
        engine = self._store.get_hybrid_engine(table_name=table_name)

        dense_dicts = [
            {
                "id": sc.chunk_id or f"{sc.file_path}::{sc.text[:60]}",
                "score": sc.score,
                "text": sc.text,
                "file_name": sc.file_name,
                "file_path": sc.file_path,
                "workspace": sc.workspace,
                "content_type": sc.content_type or "Local Document"
            }
            for sc in raw_dense_candidates
        ]

        target_ws = workspace if workspace != "Default" else None

        fused_raw = engine.fuse_and_diversify(
            dense_results=dense_dicts,
            query=query,
            workspace=target_ws,
            candidate_pool_k=cfg.candidate_pool_k,
            target_top_k=cfg.target_top_k,
            max_per_source=cfg.max_chunks_per_source,
            max_density_chars=cfg.max_density_chars,
            rrf_k=getattr(cfg, "rrf_k", 60)
        )

        final_chunks: List[ScoredChunk] = []
        for r in fused_raw:
            final_chunks.append(
                ScoredChunk(
                    text=r["text"],
                    file_name=r["file_name"],
                    file_path=r["file_path"],
                    workspace=r["workspace"],
                    score=r["score"],
                    content_type=r.get("content_type", "Local Document"),
                    chunk_id=r["id"],
                    metadata={
                        "file_name": r["file_name"],
                        "file_path": r["file_path"],
                        "workspace": r["workspace"],
                        "content_type": r.get("content_type", "Local Document")
                    }
                )
            )

        return final_chunks
