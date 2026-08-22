"""
Vector Engine package for AnyContext (Fase 2).
Provides modular contextual enrichment, encapsulated LanceDB storage,
and high-throughput parallel ingestion and retrieval pipelines.
"""
from any_context.vector_engine.models import ScoredChunk, RetrievalConfig, IngestionConfig
from any_context.vector_engine.enricher import ContextualEnricher, SemanticEnvelope
from any_context.vector_engine.store import LanceDBStore
from any_context.vector_engine.filters import RelevanceFilter
from any_context.vector_engine.retriever import ParallelRetriever
from any_context.vector_engine.indexer import ParallelIndexer

__all__ = [
    "ScoredChunk",
    "RetrievalConfig",
    "IngestionConfig",
    "ContextualEnricher",
    "SemanticEnvelope",
    "LanceDBStore",
    "RelevanceFilter",
    "ParallelRetriever",
    "ParallelIndexer",
]
