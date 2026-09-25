//! PyO3 bindings for actx-agent native Rust ReAct / Deep Search orchestrator

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;
use serde_json::Value;
use async_trait::async_trait;

use actx_agent::{
    Agent, AgentEvent, AgentExecutionMode,
    InMemorySessionStore, SearchMode, SessionStore,
    SqliteSessionStore, Tool, ToolError,
};
use actx_lm::traits::LmProvider;
use actx_lm::{LmClient, ProviderKind};
use crate::PyLmClient;

/// Bridge tool allowing Python callables to be registered as native tools in actx-agent
struct PyBridgeTool {
    name: String,
    description: String,
    schema: Value,
    callback: Arc<PyObject>,
}

#[async_trait]
impl Tool for PyBridgeTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> Value {
        self.schema.clone()
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let callback = self.callback.clone();
        let args_str = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string());

        let res = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<String, String> {
                let cb = callback.bind(py);
                let py_res = cb.call1((args_str.as_str(),))
                    .map_err(|e| format!("Python tool execution error: {}", e))?;
                
                let res_str: String = py_res.extract()
                    .or_else(|_| py_res.str().map(|s| s.to_string()))
                    .unwrap_or_else(|_| "null".to_string());
                Ok(res_str)
            })
        })
        .await
        .map_err(|e| ToolError::new(format!("Task execution error: {}", e)))?
        .map_err(ToolError::new)?;

        Ok(res)
    }
}

/// Final output resulting from a native agent execution run
#[pyclass]
#[derive(Clone)]
pub struct PyAgentResponse {
    #[pyo3(get)]
    pub content: String,
    #[pyo3(get)]
    pub total_turns: usize,
    #[pyo3(get)]
    pub tool_calls_count: usize,
    #[pyo3(get)]
    pub finish_reason: Option<String>,
    #[pyo3(get)]
    pub execution_time_ms: u64,
}

#[pymethods]
impl PyAgentResponse {
    fn __repr__(&self) -> String {
        format!(
            "AgentResponse(turns={}, tools_called={}, time_ms={}, finish_reason={:?})",
            self.total_turns, self.tool_calls_count, self.execution_time_ms, self.finish_reason
        )
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("content", &self.content)?;
        dict.set_item("total_turns", self.total_turns)?;
        dict.set_item("tool_calls_count", self.tool_calls_count)?;
        dict.set_item("finish_reason", &self.finish_reason)?;
        dict.set_item("execution_time_ms", self.execution_time_ms)?;
        Ok(dict.into())
    }
}

/// Fine-grained event emitted during native agent execution
#[pyclass]
#[derive(Clone)]
pub struct PyAgentEvent {
    #[pyo3(get)]
    pub event_type: String,
    #[pyo3(get)]
    pub payload_json: String,
}

#[pymethods]
impl PyAgentEvent {
    fn __repr__(&self) -> String {
        format!("AgentEvent(type='{}', payload={})", self.event_type, self.payload_json)
    }

    fn is_thinking(&self) -> bool { self.event_type == "Thinking" }
    fn is_tool_start(&self) -> bool { self.event_type == "ToolStart" }
    fn is_tool_end(&self) -> bool { self.event_type == "ToolEnd" }
    fn is_delta(&self) -> bool { self.event_type == "Delta" }
    fn is_decomposition(&self) -> bool { self.event_type == "Decomposition" }
    fn is_iteration_start(&self) -> bool { self.event_type == "IterationStart" }
    fn is_gap_analysis(&self) -> bool { self.event_type == "GapAnalysis" }
    fn is_done(&self) -> bool { self.event_type == "Done" }
    fn is_error(&self) -> bool { self.event_type == "Error" }

    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("type", &self.event_type)?;
        let json_module = py.import_bound("json")?;
        let parsed = json_module.call_method1("loads", (&self.payload_json,))
            .unwrap_or_else(|_| self.payload_json.clone().into_py(py).into_bound(py));
        dict.set_item("payload", parsed)?;
        Ok(dict.into())
    }
}

/// High-performance native Rust Agent Engine exposed to Python
#[pyclass]
pub struct PyAgentEngine {
    agent: Agent,
    runtime: Arc<tokio::runtime::Runtime>,
}

#[pymethods]
impl PyAgentEngine {
    #[new]
    #[pyo3(signature = (
        client=None,
        provider=None,
        api_key=None,
        base_url=None,
        default_model=None,
        system_prompt=None,
        max_turns=None,
        temperature=None,
        execution_mode=None,
        search_mode=None,
        db_path=None
    ))]
    fn new(
        client: Option<&PyLmClient>,
        provider: Option<String>,
        api_key: Option<String>,
        base_url: Option<String>,
        default_model: Option<String>,
        system_prompt: Option<String>,
        max_turns: Option<usize>,
        temperature: Option<f32>,
        execution_mode: Option<String>,
        search_mode: Option<String>,
        db_path: Option<String>,
    ) -> PyResult<Self> {
        let (lm_provider, runtime): (Arc<dyn LmProvider>, Arc<tokio::runtime::Runtime>) = if let Some(c) = client {
            (c.provider_arc(), c.runtime_arc())
        } else {
            let p_str = provider.unwrap_or_else(|| "mock".to_string());
            let kind = match p_str.to_lowercase().as_str() {
                "openai" => ProviderKind::OpenAi,
                "anthropic" => ProviderKind::Anthropic,
                "gemini" => ProviderKind::Gemini,
                "ollama" => ProviderKind::Ollama { base_url },
                "groq" => ProviderKind::Groq,
                "deepseek" => ProviderKind::DeepSeek,
                "openrouter" => ProviderKind::OpenRouter,
                "mock" => ProviderKind::Mock,
                other => ProviderKind::CustomCompatible {
                    name: other.to_string(),
                    base_url: base_url.unwrap_or_else(|| "http://localhost:8000/v1".to_string()),
                },
            };

            let mut builder = LmClient::builder().provider_kind(kind);
            if let Some(key) = api_key {
                builder = builder.api_key(key);
            }
            if let Some(ref model) = default_model {
                builder = builder.default_model(model.clone());
            }

            let lm = builder
                .build()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

            (lm.provider().clone(), Arc::new(rt))
        };

        let mut builder = Agent::builder().client(lm_provider);

        if let Some(ref m) = default_model {
            builder = builder.model(m.clone());
        }
        if let Some(ref sp) = system_prompt {
            builder = builder.system_prompt(sp.clone());
        }
        if let Some(turns) = max_turns {
            builder = builder.max_turns(turns);
        }
        if let Some(temp) = temperature {
            builder = builder.temperature(temp);
        }
        if let Some(ref em) = execution_mode {
            match em.to_lowercase().as_str() {
                "direct" => builder = builder.execution_mode(AgentExecutionMode::Direct),
                "react" => builder = builder.execution_mode(AgentExecutionMode::ReAct),
                "deep_search" | "deep" => builder = builder.execution_mode(AgentExecutionMode::DeepSearch),
                _ => {}
            }
        }
        if let Some(ref sm) = search_mode {
            match sm.to_lowercase().as_str() {
                "auto" => builder = builder.search_mode(SearchMode::Auto),
                "fast" => builder = builder.search_mode(SearchMode::Fast),
                "deep" => builder = builder.search_mode(SearchMode::Deep),
                _ => {}
            }
        }

        let store: Arc<dyn SessionStore> = if let Some(path) = db_path {
            Arc::new(SqliteSessionStore::open(path, 30)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?)
        } else {
            Arc::new(InMemorySessionStore::new(30))
        };
        builder = builder.session_store(store);

        let agent = runtime.block_on(async { builder.build().await })
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(Self { agent, runtime })
    }

    /// Register a Python callback as an executable tool in the agent
    fn register_tool(
        &self,
        name: String,
        description: String,
        schema_json: String,
        callback: PyObject,
    ) -> PyResult<()> {
        let schema: Value = if schema_json.trim().is_empty() {
            serde_json::json!({
                "type": "object",
                "properties": {}
            })
        } else {
            serde_json::from_str(&schema_json)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid JSON schema: {}", e)))?
        };

        let tool = Arc::new(PyBridgeTool {
            name,
            description,
            schema,
            callback: Arc::new(callback),
        });

        self.runtime.block_on(async {
            self.agent.register_tool(tool).await;
        });

        Ok(())
    }

    /// Return whether a tool with the given name is registered
    fn contains_tool(&self, name: &str) -> bool {
        self.runtime.block_on(self.agent.tools().contains(name))
    }

    /// Return the count of registered tools
    fn tools_count(&self) -> usize {
        self.runtime.block_on(async {
            self.agent.tools().definitions().await.len()
        })
    }

    /// Execute the ReAct loop to completion releasing GIL during asynchronous I/O
    #[pyo3(signature = (input, session_id=None))]
    fn run(&self, py: Python<'_>, input: &str, session_id: Option<&str>) -> PyResult<PyAgentResponse> {
        let agent = self.agent.clone();
        let input_owned = input.to_string();
        let session_id_owned = session_id.map(|s| s.to_string());
        let runtime = self.runtime.clone();

        let resp = py.allow_threads(move || {
            runtime.block_on(async move {
                agent.run(&input_owned, session_id_owned.as_deref()).await
            })
        }).map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(PyAgentResponse {
            content: resp.content,
            total_turns: resp.total_turns,
            tool_calls_count: resp.tool_calls_count,
            finish_reason: resp.finish_reason.map(|r| format!("{:?}", r)),
            execution_time_ms: resp.execution_time_ms,
        })
    }

    /// Stream execution events calling `callback(event)` for each event emitted
    #[pyo3(signature = (input, callback, session_id=None))]
    fn run_with_callback(
        &self,
        py: Python<'_>,
        input: &str,
        callback: PyObject,
        session_id: Option<&str>,
    ) -> PyResult<PyAgentResponse> {
        let runtime = self.runtime.clone();
        let (mut rx, handle) = {
            let _guard = runtime.enter();
            self.agent.stream(input, session_id.map(|s| s.to_string()))
        };

        loop {
            let next_event = py.allow_threads(|| {
                runtime.block_on(rx.recv())
            });

            match next_event {
                Some(ev) => {
                    let (event_type, payload_json) = match &ev {
                        AgentEvent::Thinking(t) => ("Thinking".to_string(), serde_json::to_string(t).unwrap_or_default()),
                        AgentEvent::ToolStart { id, name, arguments } => (
                            "ToolStart".to_string(),
                            serde_json::json!({ "id": id, "name": name, "arguments": arguments }).to_string(),
                        ),
                        AgentEvent::ToolEnd { id, name, result, is_error } => (
                            "ToolEnd".to_string(),
                            serde_json::json!({ "id": id, "name": name, "result": result, "is_error": is_error }).to_string(),
                        ),
                        AgentEvent::Delta(d) => ("Delta".to_string(), serde_json::to_string(d).unwrap_or_default()),
                        AgentEvent::Decomposition { sub_queries } => (
                            "Decomposition".to_string(),
                            serde_json::json!({ "sub_queries": sub_queries }).to_string(),
                        ),
                        AgentEvent::IterationStart { iteration, max_iterations } => (
                            "IterationStart".to_string(),
                            serde_json::json!({ "iteration": iteration, "max_iterations": max_iterations }).to_string(),
                        ),
                        AgentEvent::GapAnalysis { is_sufficient, missing_aspects } => (
                            "GapAnalysis".to_string(),
                            serde_json::json!({ "is_sufficient": is_sufficient, "missing_aspects": missing_aspects }).to_string(),
                        ),
                        AgentEvent::Done { total_turns, total_tokens } => (
                            "Done".to_string(),
                            serde_json::json!({ "total_turns": total_turns, "total_tokens": total_tokens }).to_string(),
                        ),
                        AgentEvent::Error(err) => ("Error".to_string(), serde_json::to_string(err).unwrap_or_default()),
                    };

                    let py_ev = PyAgentEvent { event_type, payload_json };
                    if let Err(e) = callback.call1(py, (py_ev,)) {
                        eprintln!("Warning in agent event callback: {}", e);
                    }
                }
                None => break,
            }
        }

        let resp = py.allow_threads(move || {
            runtime.block_on(handle)
        }).map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?
          .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(PyAgentResponse {
            content: resp.content,
            total_turns: resp.total_turns,
            tool_calls_count: resp.tool_calls_count,
            finish_reason: resp.finish_reason.map(|r| format!("{:?}", r)),
            execution_time_ms: resp.execution_time_ms,
        })
    }
}
