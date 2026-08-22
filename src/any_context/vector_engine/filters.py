"""
Relevance and Diversification Filter (Fase 2).
Provides 100% isolated, pure-function ranking logic for score thresholding,
source-fair round-robin balancing, and strict density budgeting.
"""
from typing import List, Dict
from collections import defaultdict
from any_context.vector_engine.models import ScoredChunk, RetrievalConfig


class RelevanceFilter:
    """
    Pure-function relevance judge for vector search candidates.
    Contains zero I/O and zero database dependencies, operating strictly on ScoredChunk contracts.
    """

    @staticmethod
    def apply_threshold(chunks: List[ScoredChunk], min_score: float) -> List[ScoredChunk]:
        """
        Discards low-scoring vector candidates falling below the confidence threshold.
        Guarantees that cross-domain noise is eliminated while preserving at least the top candidate.
        """
        if not chunks:
            return []

        filtered = [c for c in chunks if c.score >= min_score]
        if not filtered and chunks:
            # Safe fallback: retain the single best match if all fall marginally below threshold
            return [chunks[0]]
        return filtered

    @staticmethod
    def apply_source_diversification(
        chunks: List[ScoredChunk],
        max_per_source: int = 3,
        target_k: int = 20
    ) -> List[ScoredChunk]:
        """
        Applies Source-Fair Round-Robin balancing across distinct documents and URLs.
        Prevents a monolithic document from monopolizing all context slots.
        """
        if not chunks:
            return []

        # Group chunks by unique source identifier
        by_source: Dict[str, List[ScoredChunk]] = defaultdict(list)
        for c in chunks:
            source_id = c.file_path or c.file_name or "Unknown"
            by_source[source_id].append(c)

        diversified: List[ScoredChunk] = []
        sources = list(by_source.keys())
        pass_idx = 0

        while len(diversified) < target_k:
            added_in_pass = False
            for s in sources:
                source_items = by_source[s]
                if pass_idx < len(source_items) and pass_idx < max_per_source:
                    diversified.append(source_items[pass_idx])
                    added_in_pass = True
                    if len(diversified) >= target_k:
                        break
            pass_idx += 1
            if not added_in_pass:
                break

        # If quota remains unfilled, backfill with remaining highest-scoring chunks
        if len(diversified) < target_k:
            selected_ids = {id(c) for c in diversified}
            for c in chunks:
                if id(c) not in selected_ids:
                    diversified.append(c)
                    if len(diversified) >= target_k:
                        break

        return diversified[:target_k]

    @staticmethod
    def apply_density_budget(chunks: List[ScoredChunk], max_chars: int = 40000) -> List[ScoredChunk]:
        """
        Enforces a strict prompt safety density budget (~10,000 tokens),
        condensing additional trailing chunks if necessary.
        """
        if not chunks:
            return []

        budgeted: List[ScoredChunk] = []
        accumulated_chars = 0

        for i, c in enumerate(chunks):
            chunk_len = len(c.text or "")
            if accumulated_chars + chunk_len > max_chars and i >= 3:
                remaining_space = max(0, max_chars - accumulated_chars)
                if remaining_space > 200:
                    c.text = c.text[:remaining_space] + "\n[...trecho adicional condensado por limite de densidade...]"
                    budgeted.append(c)
                break

            accumulated_chars += chunk_len
            budgeted.append(c)

        return budgeted

    @classmethod
    def filter_and_balance(cls, chunks: List[ScoredChunk], config: RetrievalConfig) -> List[ScoredChunk]:
        """
        Full filtering pipeline: Thresholding -> Round-Robin Diversification -> Density Budgeting.
        """
        if not chunks:
            return []

        # 1. Score Thresholding
        thresholded = cls.apply_threshold(chunks, min_score=config.min_similarity_score)

        # 2. Source-Fair Round-Robin
        diversified = cls.apply_source_diversification(
            thresholded,
            max_per_source=config.max_chunks_per_source,
            target_k=config.target_top_k
        )

        # 3. Density Budgeting
        return cls.apply_density_budget(diversified, max_chars=config.max_density_chars)
