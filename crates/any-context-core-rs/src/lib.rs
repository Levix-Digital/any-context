use pyo3::prelude::*;

pub mod models;
pub mod ingestion;
pub mod retrieval;

pub use models::ChunkPayload;
pub use ingestion::IngestionRouter;
pub use ingestion::WorkspaceScanner;
pub use retrieval::{HybridRetrieverEngine, QueryPreprocessor, ProcessedQuery};

#[pyfunction]
fn extract_temporal_clauses(query: &str) -> Vec<String> {
    retrieval::extract_temporal_clauses_native(query)
}

#[pyfunction]
fn expand_query_temporal(query: &str) -> String {
    retrieval::expand_query_temporal_native(query)
}

#[pyfunction]
fn extract_filename_mentions(query: &str) -> Vec<String> {
    retrieval::extract_filename_mentions_native(query)
}

/// AnyContext High-Performance Core Engine in Rust.
#[pymodule]
fn any_context_core_rs(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ChunkPayload>()?;
    m.add_class::<IngestionRouter>()?;
    m.add_class::<WorkspaceScanner>()?;
    m.add_class::<HybridRetrieverEngine>()?;
    m.add_class::<QueryPreprocessor>()?;
    m.add_class::<ProcessedQuery>()?;
    m.add_function(wrap_pyfunction!(extract_temporal_clauses, m)?)?;
    m.add_function(wrap_pyfunction!(expand_query_temporal, m)?)?;
    m.add_function(wrap_pyfunction!(extract_filename_mentions, m)?)?;
    m.add("__version__", "0.30.31")?;
    Ok(())
}

