use pyo3::prelude::*;

pub mod models;
pub mod ingestion;
pub mod retrieval;
pub mod storage;

pub use models::ChunkPayload;
pub use ingestion::IngestionRouter;
pub use ingestion::WorkspaceScanner;
pub use retrieval::{HybridRetrieverEngine, QueryPreprocessor, ProcessedQuery};
pub use storage::{NativeConfigDb, NativeLanceStore};

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

#[pyfunction]
fn estimate_token_count(text: &str) -> usize {
    retrieval::estimate_token_count(text)
}

#[pyfunction]
fn truncate_to_token_ceiling(text: &str, max_tokens: usize) -> (String, bool) {
    retrieval::truncate_to_token_ceiling(text, max_tokens)
}

#[pyfunction]
fn get_embedding_token_limit(model_name: &str) -> usize {
    retrieval::get_embedding_token_limit_rs(model_name)
}

#[pyclass]
pub struct PyLanceStore {
    inner: storage::NativeLanceStore,
}

#[pymethods]
impl PyLanceStore {
    #[new]
    fn new(db_path: &str) -> PyResult<Self> {
        let store = storage::NativeLanceStore::open(db_path)
            .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;
        Ok(Self { inner: store })
    }

    #[pyo3(signature = (workspace=None, table_name=None))]
    fn count_records(&self, workspace: Option<&str>, table_name: Option<&str>) -> PyResult<usize> {
        self.inner
            .count_records(workspace, table_name)
            .map_err(pyo3::exceptions::PyRuntimeError::new_err)
    }

    #[pyo3(signature = (workspace, table_name=None))]
    fn delete_by_workspace(&self, workspace: &str, table_name: Option<&str>) -> PyResult<()> {
        self.inner
            .delete_by_workspace(workspace, table_name)
            .map_err(pyo3::exceptions::PyRuntimeError::new_err)
    }

    #[pyo3(signature = (file_path, workspace=None, table_name=None))]
    fn delete_by_file(
        &self,
        file_path: &str,
        workspace: Option<&str>,
        table_name: Option<&str>,
    ) -> PyResult<()> {
        self.inner
            .delete_by_file(file_path, workspace, table_name)
            .map_err(pyo3::exceptions::PyRuntimeError::new_err)
    }

    #[pyo3(signature = (chunk_id, table_name=None))]
    fn delete_by_id(&self, chunk_id: &str, table_name: Option<&str>) -> PyResult<()> {
        self.inner
            .delete_by_id(chunk_id, table_name)
            .map_err(pyo3::exceptions::PyRuntimeError::new_err)
    }
}

#[pyclass]
pub struct PyConfigDb {
    inner: storage::NativeConfigDb,
}

#[pymethods]
impl PyConfigDb {
    #[new]
    fn new(db_path: &str) -> PyResult<Self> {
        let db = storage::NativeConfigDb::open(db_path)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { inner: db })
    }

    #[pyo3(signature = (name, description=None))]
    fn create_workspace(&self, name: &str, description: Option<&str>) -> PyResult<String> {
        self.inner
            .create_workspace(name, description)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn rename_workspace(&self, old_name: &str, new_name: &str) -> PyResult<bool> {
        self.inner
            .rename_workspace(old_name, new_name)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn delete_workspace(&self, name: &str) -> PyResult<bool> {
        self.inner
            .delete_workspace(name)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn get_setting(&self, key: &str) -> PyResult<Option<String>> {
        self.inner
            .get_setting(key)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn set_setting(&self, key: &str, value: &str) -> PyResult<()> {
        self.inner
            .set_setting(key, value)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn get_file_hash(&self, workspace: &str, file_path: &str) -> PyResult<Option<String>> {
        self.inner
            .get_file_hash(workspace, file_path)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn set_file_metadata(
        &self,
        workspace: &str,
        file_path: &str,
        hash: &str,
        last_mod: &str,
        size: i64,
        status: &str,
    ) -> PyResult<()> {
        self.inner
            .set_file_metadata(workspace, file_path, hash, last_mod, size, status)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn get_workspace_file_hashes(
        &self,
        workspace: &str,
    ) -> PyResult<std::collections::HashMap<String, String>> {
        self.inner
            .get_workspace_file_hashes(workspace)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn delete_file_metadata(&self, workspace: &str, file_path: &str) -> PyResult<bool> {
        self.inner
            .delete_file_metadata(workspace, file_path)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }
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
    m.add_class::<PyLanceStore>()?;
    m.add_class::<PyConfigDb>()?;
    m.add_function(wrap_pyfunction!(extract_temporal_clauses, m)?)?;
    m.add_function(wrap_pyfunction!(expand_query_temporal, m)?)?;
    m.add_function(wrap_pyfunction!(extract_filename_mentions, m)?)?;
    m.add_function(wrap_pyfunction!(estimate_token_count, m)?)?;
    m.add_function(wrap_pyfunction!(truncate_to_token_ceiling, m)?)?;
    m.add_function(wrap_pyfunction!(get_embedding_token_limit, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
