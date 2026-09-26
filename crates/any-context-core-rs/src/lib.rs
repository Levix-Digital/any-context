use pyo3::prelude::*;

pub mod models;
pub mod ingestion;
pub mod retrieval;
pub mod storage;
pub mod agent;

pub use models::ChunkPayload;
pub use ingestion::IngestionRouter;
pub use ingestion::WorkspaceScanner;
pub use retrieval::{HybridRetrieverEngine, QueryPreprocessor, ProcessedQuery};
pub use storage::{NativeConfigDb, NativeLanceStore, get_default_settings_db_path};
pub use agent::{PyAgentEngine, PyAgentResponse, PyAgentEvent};

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
    pub(crate) client: actx_lm::LmClient,
    pub(crate) runtime: std::sync::Arc<tokio::runtime::Runtime>,
}

impl PyLmClient {
    pub fn provider_arc(&self) -> std::sync::Arc<dyn actx_lm::traits::LmProvider> {
        self.client.provider().clone()
    }

    pub fn runtime_arc(&self) -> std::sync::Arc<tokio::runtime::Runtime> {
        self.runtime.clone()
    }
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

/// PyO3 container for hybrid search request parameters.
#[pyclass]
#[derive(Debug, Clone)]
pub struct PyHybridSearchRequest {
    #[pyo3(get, set)]
    pub query_text: String,
    #[pyo3(get, set)]
    pub sub_query_id: Option<String>,
    #[pyo3(get, set)]
    pub query_vector: Option<Vec<f32>>,
    #[pyo3(get, set)]
    pub workspace: Option<String>,
    #[pyo3(get, set)]
    pub target_workspaces: Vec<String>,
    #[pyo3(get, set)]
    pub linked_sources: Vec<String>,
    #[pyo3(get, set)]
    pub top_k: usize,
    #[pyo3(get, set)]
    pub candidate_pool_k: usize,
    #[pyo3(get, set)]
    pub min_score: f64,
    #[pyo3(get, set)]
    pub max_chunks_per_source: usize,
    #[pyo3(get, set)]
    pub max_density_chars: usize,
    #[pyo3(get, set)]
    pub table_name: String,
    #[pyo3(get, set)]
    pub rrf_k: usize,
}

#[pymethods]
impl PyHybridSearchRequest {
    #[new]
    #[pyo3(signature = (
        query_text,
        sub_query_id=None,
        query_vector=None,
        workspace=None,
        target_workspaces=None,
        linked_sources=None,
        top_k=20,
        candidate_pool_k=100,
        min_score=0.0,
        max_chunks_per_source=3,
        max_density_chars=40000,
        table_name="workspace_chunks".to_string(),
        rrf_k=60
    ))]
    pub fn new(
        query_text: String,
        sub_query_id: Option<String>,
        query_vector: Option<Vec<f32>>,
        workspace: Option<String>,
        target_workspaces: Option<Vec<String>>,
        linked_sources: Option<Vec<String>>,
        top_k: usize,
        candidate_pool_k: usize,
        min_score: f64,
        max_chunks_per_source: usize,
        max_density_chars: usize,
        table_name: String,
        rrf_k: usize,
    ) -> Self {
        Self {
            query_text,
            sub_query_id,
            query_vector,
            workspace,
            target_workspaces: target_workspaces.unwrap_or_default(),
            linked_sources: linked_sources.unwrap_or_default(),
            top_k,
            candidate_pool_k,
            min_score,
            max_chunks_per_source,
            max_density_chars,
            table_name,
            rrf_k,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "PyHybridSearchRequest(query_text={:?}, sub_query_id={:?}, workspace={:?}, top_k={})",
            self.query_text, self.sub_query_id, self.workspace, self.top_k
        )
    }
}

impl PyHybridSearchRequest {
    pub fn to_native(&self) -> retrieval::HybridSearchRequest {
        retrieval::HybridSearchRequest {
            sub_query_id: self.sub_query_id.clone(),
            query_text: self.query_text.clone(),
            query_vector: self.query_vector.clone(),
            workspace: self.workspace.clone(),
            target_workspaces: self.target_workspaces.clone(),
            linked_sources: self.linked_sources.clone(),
            top_k: self.top_k,
            candidate_pool_k: self.candidate_pool_k,
            min_score: self.min_score,
            max_chunks_per_source: self.max_chunks_per_source,
            max_density_chars: self.max_density_chars,
            table_name: self.table_name.clone(),
            rrf_k: self.rrf_k,
        }
    }
}

/// Scored chunk result container exposed to Python.
#[pyclass]
#[derive(Debug, Clone)]
pub struct PyHybridSearchResult {
    #[pyo3(get)]
    pub chunk_id: String,
    #[pyo3(get)]
    pub file_name: String,
    #[pyo3(get)]
    pub file_path: String,
    #[pyo3(get)]
    pub workspace: String,
    #[pyo3(get)]
    pub text: String,
    #[pyo3(get)]
    pub content_type: String,
    #[pyo3(get)]
    pub content_hash: String,
    #[pyo3(get)]
    pub score: f64,
    #[pyo3(get)]
    pub dense_score: Option<f32>,
    #[pyo3(get)]
    pub sparse_score: Option<f32>,
    #[pyo3(get)]
    pub token_count: usize,
    #[pyo3(get)]
    pub matched_subqueries: Vec<String>,
}

#[pymethods]
impl PyHybridSearchResult {
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = pyo3::types::PyDict::new_bound(py);
        dict.set_item("chunk_id", &self.chunk_id)?;
        dict.set_item("id", &self.chunk_id)?;
        dict.set_item("file_name", &self.file_name)?;
        dict.set_item("file_path", &self.file_path)?;
        dict.set_item("workspace", &self.workspace)?;
        dict.set_item("text", &self.text)?;
        dict.set_item("content_type", &self.content_type)?;
        dict.set_item("content_hash", &self.content_hash)?;
        dict.set_item("score", self.score)?;
        dict.set_item("dense_score", self.dense_score)?;
        dict.set_item("sparse_score", self.sparse_score)?;
        dict.set_item("token_count", self.token_count)?;
        dict.set_item("matched_subqueries", &self.matched_subqueries)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "PyHybridSearchResult(chunk_id={:?}, file_name={:?}, score={:.4}, matched_subqueries={:?})",
            self.chunk_id, self.file_name, self.score, self.matched_subqueries
        )
    }
}

impl PyHybridSearchResult {
    pub fn from_native(native: retrieval::HybridSearchResult) -> Self {
        Self {
            chunk_id: native.chunk_id,
            file_name: native.file_name,
            file_path: native.file_path,
            workspace: native.workspace,
            text: native.text,
            content_type: native.content_type,
            content_hash: native.content_hash,
            score: native.score,
            dense_score: native.dense_score,
            sparse_score: native.sparse_score,
            token_count: native.token_count,
            matched_subqueries: native.matched_subqueries,
        }
    }
}

/// High-performance native Rust Hybrid Pipeline exposed to Python with GIL release.
#[pyclass]
pub struct PyHybridPipeline {
    inner: std::sync::Arc<retrieval::NativeHybridPipeline>,
}

#[pymethods]
impl PyHybridPipeline {
    #[new]
    #[pyo3(signature = (db_path))]
    pub fn new(db_path: &str) -> PyResult<Self> {
        let pipeline = retrieval::NativeHybridPipeline::open(db_path)
            .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;
        Ok(Self {
            inner: std::sync::Arc::new(pipeline),
        })
    }

    #[pyo3(signature = (req))]
    pub fn search(
        &self,
        py: Python<'_>,
        req: &PyHybridSearchRequest,
    ) -> PyResult<Vec<PyHybridSearchResult>> {
        let native_req = req.to_native();
        let pipeline = self.inner.clone();
        let results = py
            .allow_threads(move || pipeline.search(native_req))
            .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;

        Ok(results
            .into_iter()
            .map(PyHybridSearchResult::from_native)
            .collect())
    }

    #[pyo3(signature = (requests))]
    pub fn retrieve_hybrid_batch(
        &self,
        py: Python<'_>,
        requests: Vec<PyHybridSearchRequest>,
    ) -> PyResult<Vec<PyHybridSearchResult>> {
        let native_reqs: Vec<retrieval::HybridSearchRequest> =
            requests.into_iter().map(|r| r.to_native()).collect();
        let pipeline = self.inner.clone();
        let results = py
            .allow_threads(move || pipeline.retrieve_hybrid_batch(native_reqs))
            .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;

        Ok(results
            .into_iter()
            .map(PyHybridSearchResult::from_native)
            .collect())
    }

    #[pyo3(signature = (table_name, id, text, file_name, file_path, workspace="Default".to_string(), content_type="Local Document".to_string()))]
    pub fn add_chunk_to_bm25(
        &self,
        table_name: &str,
        id: String,
        text: String,
        file_name: String,
        file_path: String,
        workspace: String,
        content_type: String,
    ) {
        self.inner.add_chunk_to_bm25(
            table_name,
            id,
            text,
            file_name,
            file_path,
            workspace,
            content_type,
        );
    }

    pub fn save_bm25(&self, table_name: &str) -> PyResult<()> {
        self.inner
            .save_bm25(table_name)
            .map_err(pyo3::exceptions::PyIOError::new_err)
    }

    pub fn load_bm25_from_file(&self, table_name: &str, file_path: &str) -> PyResult<()> {
        self.inner
            .load_bm25_from_file(table_name, file_path)
            .map_err(pyo3::exceptions::PyIOError::new_err)
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
    m.add_class::<PyAgentEngine>()?;
    m.add_class::<PyAgentResponse>()?;
    m.add_class::<PyAgentEvent>()?;
    m.add_class::<PyHybridSearchRequest>()?;
    m.add_class::<PyHybridSearchResult>()?;
    m.add_class::<PyHybridPipeline>()?;
    m.add_function(wrap_pyfunction!(extract_temporal_clauses, m)?)?;
    m.add_function(wrap_pyfunction!(expand_query_temporal, m)?)?;
    m.add_function(wrap_pyfunction!(extract_filename_mentions, m)?)?;
    m.add_function(wrap_pyfunction!(estimate_token_count, m)?)?;
    m.add_function(wrap_pyfunction!(truncate_to_token_ceiling, m)?)?;
    m.add_function(wrap_pyfunction!(get_embedding_token_limit, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

