"""
Vector Engine Core Models and Dependency Injection Contracts.
"""
from dataclasses import dataclass, field, asdict
from typing import List, Dict, Any, Optional

@dataclass
class ScoredChunk:
    """Represents a single retrieved chunk with similarity score and domain metadata."""
    text: str
    file_name: str
    file_path: str
    workspace: str
    score: float
    last_modified: Optional[str] = None
    content_type: Optional[str] = "Local Document"
    document_summary: Optional[str] = None
    keywords: Optional[str] = None
    chunk_id: Optional[str] = None
    metadata: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class RetrievalConfig:
    """
    Immutable Configuration Object injected into ParallelRetriever and RelevanceFilter.
    Encapsulates all RAG presets, threshold boundaries, and density budgets.
    """
    candidate_pool_k: int = 100
    target_top_k: int = 20
    min_similarity_score: float = 0.50
    max_chunks_per_source: int = 3
    max_density_chars: int = 40000

    @classmethod
    def from_preset(cls, preset_name: Optional[str] = "balanced") -> "RetrievalConfig":
        """Factory mapping user preset profiles to strict mathematical RAG boundaries."""
        p = (preset_name or "balanced").lower().strip()
        if p == "turbo":
            return cls(
                candidate_pool_k=50,
                target_top_k=10,
                min_similarity_score=0.55,
                max_chunks_per_source=2,
                max_density_chars=20000
            )
        elif p in ["deep", "deep_research", "research"]:
            return cls(
                candidate_pool_k=150,
                target_top_k=40,
                min_similarity_score=0.45,
                max_chunks_per_source=5,
                max_density_chars=60000
            )
        else:  # balanced (default)
            return cls(
                candidate_pool_k=100,
                target_top_k=20,
                min_similarity_score=0.50,
                max_chunks_per_source=3,
                max_density_chars=40000
            )


@dataclass(frozen=True)
class IngestionConfig:
    """
    Immutable Configuration Object injected into ParallelIndexer.
    Controls chunking parameters, embedding batch sizes, and worker pool concurrency.
    """
    chunk_size: int = 1024
    chunk_overlap: int = 200
    batch_embed_size: int = 50
    max_workers: int = 8
