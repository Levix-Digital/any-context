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

#[pyclass]
pub struct PyLmClient {
    client: actx_lm::LmClient,
    runtime: std::sync::Arc<tokio::runtime::Runtime>,
}

#[pymethods]
impl PyLmClient {
    #[new]
    #[pyo3(signature = (provider, api_key=None, base_url=None, default_model=None))]
    fn new(
        provider: &str,
        api_key: Option<String>,
        base_url: Option<String>,
        default_model: Option<String>,
    ) -> PyResult<Self> {
        let kind = match provider.to_lowercase().as_str() {
            "openai" => actx_lm::ProviderKind::OpenAi,
            "anthropic" => actx_lm::ProviderKind::Anthropic,
            "gemini" => actx_lm::ProviderKind::Gemini,
            "ollama" => actx_lm::ProviderKind::Ollama { base_url },
            "groq" => actx_lm::ProviderKind::Groq,
            "deepseek" => actx_lm::ProviderKind::DeepSeek,
            "openrouter" => actx_lm::ProviderKind::OpenRouter,
            "mock" => actx_lm::ProviderKind::Mock,
            other => actx_lm::ProviderKind::CustomCompatible {
                name: other.to_string(),
                base_url: base_url.unwrap_or_else(|| "http://localhost:8000/v1".to_string()),
            },
        };

        let mut builder = actx_lm::LmClient::builder().provider_kind(kind);
        if let Some(key) = api_key {
            builder = builder.api_key(key);
        }
        if let Some(model) = default_model {
            builder = builder.default_model(model);
        }

        let client = builder
            .build()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(Self {
            client,
            runtime: std::sync::Arc::new(runtime),
        })
    }

    fn provider_id(&self) -> String {
        self.client.provider_id().to_string()
    }

    fn quick_chat(&self, model: &str, prompt: &str) -> PyResult<String> {
        self.runtime
            .block_on(self.client.quick_chat(model, prompt))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    #[pyo3(signature = (model, messages_json, temperature=None, max_tokens=None))]
    fn chat_complete(
        &self,
        model: &str,
        messages_json: &str,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> PyResult<String> {
        let messages: Vec<actx_lm::ChatMessage> = serde_json::from_str(messages_json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        let mut req = actx_lm::ChatRequest::new(model, messages);
        if let Some(t) = temperature {
            req = req.with_temperature(t);
        }
        if let Some(m) = max_tokens {
            req = req.with_max_tokens(m);
        }

        let resp = self
            .runtime
            .block_on(self.client.chat_complete(req))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        serde_json::to_string(&resp)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn chat_stream_collect(&self, model: &str, messages_json: &str) -> PyResult<Vec<String>> {
        use futures::StreamExt;
        let messages: Vec<actx_lm::ChatMessage> = serde_json::from_str(messages_json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        let req = actx_lm::ChatRequest::new(model, messages);
        let mut stream = self
            .runtime
            .block_on(self.client.chat_stream(req))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let mut tokens = Vec::new();
        self.runtime.block_on(async {
            while let Some(chunk_res) = stream.next().await {
                if let Ok(chunk) = chunk_res {
                    match chunk {
                        actx_lm::StreamChunk::Token(t) => tokens.push(t),
                        actx_lm::StreamChunk::Reasoning(r) => tokens.push(format!("[THINK] {}", r)),
                        _ => {}
                    }
                }
            }
        });

        Ok(tokens)
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
    m.add_class::<PyLmClient>()?;
    m.add_function(wrap_pyfunction!(extract_temporal_clauses, m)?)?;
    m.add_function(wrap_pyfunction!(expand_query_temporal, m)?)?;
    m.add_function(wrap_pyfunction!(extract_filename_mentions, m)?)?;
    m.add_function(wrap_pyfunction!(estimate_token_count, m)?)?;
    m.add_function(wrap_pyfunction!(truncate_to_token_ceiling, m)?)?;
    m.add_function(wrap_pyfunction!(get_embedding_token_limit, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
