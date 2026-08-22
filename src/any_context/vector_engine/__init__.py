"""
Vector Engine package for AnyContext.
Provides modular contextual enrichment, parallel ingestion, and parallel retrieval.
"""
from any_context.vector_engine.enricher import ContextualEnricher, SemanticEnvelope

__all__ = ["ContextualEnricher", "SemanticEnvelope"]
