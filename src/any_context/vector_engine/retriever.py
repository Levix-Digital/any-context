"""
Parallel Vector Retriever (Fase 2).
Provides concurrent multi-source vector retrieval across active workspace,
Global, and Shared Sources, strictly decoupled via Dependency Injection of RetrievalConfig.
"""
import os
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import List, Optional

from any_context.vector_engine.models import ScoredChunk, RetrievalConfig
from any_context.vector_engine.store import LanceDBStore
from any_context.vector_engine.filters import RelevanceFilter


class ParallelRetriever:
    """
    High-performance parallel vector search engine.
    Encapsulates LanceDB multi-partition scanning and returns calibrated ScoredChunk contracts.
    """

    def __init__(self, store: Optional[LanceDBStore] = None):
        self._store = store or LanceDBStore.get_instance()

    def _get_query_embedding(self, query_text: str) -> List[float]:
        """Generates query embedding vector using LlamaIndex / OpenAI configured model."""
        from llama_index.core.settings import Settings
        from any_context.tools.search_tools import configure_embedding_model

        if Settings.embed_model is None:
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
        Executes parallel multi-source vector retrieval and applies decoupled RelevanceFilter.
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

        raw_candidates: List[ScoredChunk] = []

        # Execute parallel searches across CPU threads in Rust
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
                        raw_candidates.extend(res)
                except Exception:
                    pass

        # Sort raw candidates by score descending
        raw_candidates.sort(key=lambda c: c.score, reverse=True)

        # Apply decoupled RelevanceFilter (Thresholding -> Round-Robin -> Density Budgeting)
        return RelevanceFilter.filter_and_balance(raw_candidates, config=cfg)
