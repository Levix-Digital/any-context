use pyo3::prelude::*;

pub mod models;
pub mod ingestion;
pub mod retrieval;

pub use models::ChunkPayload;
pub use ingestion::IngestionRouter;
pub use ingestion::WorkspaceScanner;
pub use retrieval::HybridRetrieverEngine;

/// AnyContext High-Performance Core Engine in Rust.
#[pymodule]
fn any_context_core_rs(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ChunkPayload>()?;
    m.add_class::<IngestionRouter>()?;
    m.add_class::<WorkspaceScanner>()?;
    m.add_class::<HybridRetrieverEngine>()?;
    m.add("__version__", "0.30.30")?;
    Ok(())
}
