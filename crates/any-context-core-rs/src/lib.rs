use pyo3::prelude::*;

pub mod models;
pub mod ingestion;

pub use models::ChunkPayload;
pub use ingestion::IngestionRouter;

/// AnyContext High-Performance Core Engine in Rust.
#[pymodule]
fn any_context_core_rs(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ChunkPayload>()?;
    m.add_class::<IngestionRouter>()?;
    m.add("__version__", "0.30.3")?;
    Ok(())
}
