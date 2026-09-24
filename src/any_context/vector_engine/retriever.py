"""
Parallel Vector & BM25 Hybrid Retriever with Reciprocal Rank Fusion (Marco 6).
Provides concurrent multi-source dense vector search across LanceDB partitions,
sparse Okapi BM25 keyword search in native Rust, and unified RRF fusion with
source-fair diversification and density budgeting.
"""
import os
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import List, Optional, Dict, Any

from any_context.vector_engine.models import ScoredChunk, RetrievalConfig
from any_context.vector_engine.store import LanceDBStore
from any_context.vector_engine.query_preprocessor import (
    QueryPreprocessor,
    ProcessedQuery,
    extract_temporal_clauses,
    expand_query_temporal,
    extract_filename_mentions,
)





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
        guaranteed_file_cids: List[str] = []

        # 0A. Single-Pass Language-Agnostic Query Preprocessing (Rust any-context-core-rs)
        processed = QueryPreprocessor.process(query)
        filenames = processed.filename_mentions
        temporal_clauses = processed.temporal_clauses
        expanded_query = processed.expanded_query

        if filenames:
            for fn in filenames:
                fn_clean = fn.replace("'", "''")
                fn_clause = f"(file_name LIKE '%{fn_clean}%' OR file_path LIKE '%{fn_clean}%')"
                # If query also has temporal filter, combine with AND for precise intersection
                combined_where = f"({fn_clause}) AND ({' OR '.join(temporal_clauses)})" if temporal_clauses else fn_clause

                for ws_name, limit in targets:
                    try:
                        fn_matches = self._store.search_metadata(
                            where_clause=combined_where,
                            limit=limit,
                            workspace=ws_name if ws_name != "Default" else None,
                            table_name=table_name
                        )
                        # Fallback to filename alone if combined intersection yielded no chunks
                        if not fn_matches and temporal_clauses:
                            fn_matches = self._store.search_metadata(
                                where_clause=fn_clause,
                                limit=limit,
                                workspace=ws_name if ws_name != "Default" else None,
                                table_name=table_name
                            )

                        for sc in fn_matches:
                            sc.score = 1.0  # Max score boost for explicitly requested document
                            cid = sc.chunk_id or f"{sc.file_path}::{sc.text[:80]}"
                            raw_candidates_map[cid] = sc
                            if cid not in guaranteed_file_cids:
                                guaranteed_file_cids.append(cid)
                    except Exception:
                        pass

        # 0B. Temporal Hybrid Lexical Search: if temporal date signals exist, search LanceDB metadata
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
                        cid = sc.chunk_id or f"{sc.file_path}::{sc.text[:80]}"
                        if cid not in raw_candidates_map:
                            sc.score = 0.95
                            raw_candidates_map[cid] = sc
                        else:
                            raw_candidates_map[cid].score = max(raw_candidates_map[cid].score, 0.95)
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
        is_system_query = any(tok in query.lower() for tok in ["/link", "/sync", "/switch", "/menu", "/config", "/keys", "/api-keys", "shared sources", "shared source", "como funciona", "como usar", "ajuda", "help", "comando", "command"])
        if is_system_query and target_workspaces and "Global" in target_workspaces:
            target_ws = None


        effective_max_per_source = cfg.max_chunks_per_source
        if filenames:
            effective_max_per_source = max(cfg.max_chunks_per_source, 10)

        fused_raw = engine.fuse_and_diversify(
            dense_results=dense_dicts,
            query=expanded_query,
            workspace=target_ws,
            candidate_pool_k=cfg.candidate_pool_k,
            target_top_k=cfg.target_top_k,
            max_per_source=effective_max_per_source,
            max_density_chars=cfg.max_density_chars,
            rrf_k=getattr(cfg, "rrf_k", 60)
        )

        final_chunks: List[ScoredChunk] = []
        seen_cids = set()

        # Prioritize explicitly requested file chunks in the top slots
        if guaranteed_file_cids:
            for cid in guaranteed_file_cids:
                sc = raw_candidates_map.get(cid)
                if sc and cid not in seen_cids:
                    final_chunks.append(sc)
                    seen_cids.add(cid)

        allowed_workspaces = set(target_workspaces) if target_workspaces else ({workspace} if workspace else set())

        for r in fused_raw:
            cid = r["id"]
            if cid in seen_cids:
                continue
            r_ws = r.get("workspace", "Default")
            if allowed_workspaces and r_ws not in allowed_workspaces and r_ws != "Global":
                continue
            seen_cids.add(cid)
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

        return final_chunks[:cfg.target_top_k]
