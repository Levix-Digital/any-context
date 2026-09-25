"""
Universal Language-Agnostic Query Preprocessor & Temporal Engine for AnyContext.
Exclusively powered by native Rust (any-context-core-rs).
"""
try:
    import any_context_core_rs
except ImportError as err:
    raise ImportError(
        "any-context-core-rs native binary extension is required for AnyContext execution. "
        "Please build it with 'maturin develop' or install the native wheel."
    ) from err

QueryPreprocessor = any_context_core_rs.QueryPreprocessor
ProcessedQuery = any_context_core_rs.ProcessedQuery
extract_temporal_clauses = any_context_core_rs.extract_temporal_clauses
expand_query_temporal = any_context_core_rs.expand_query_temporal
extract_filename_mentions = any_context_core_rs.extract_filename_mentions

__all__ = [
    "QueryPreprocessor",
    "ProcessedQuery",
    "extract_temporal_clauses",
    "expand_query_temporal",
    "extract_filename_mentions",
]
