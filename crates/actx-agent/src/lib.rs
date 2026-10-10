//! # 🤖 actx-agent
//!
//! Deterministic ReAct and Deep Search agent orchestrator with finite state machine (FSM),
//! polymorphic tool registry, and session persistence for Rust.
//! Designed for zero-cost abstractions, thread safety, and modular AI workflows.

pub mod config;
pub mod deep_search;
pub mod error;
pub mod events;
pub mod fsm;
pub mod session;
pub mod tool;

pub use config::{AgentConfig, AgentExecutionMode, SearchMode};
pub use deep_search::{
    DeepSearchConfig, DeepSearchOrchestrator, DeepSearchRetriever, RetrievedChunk,
    clean_llm_response, parse_rerank_decision, parse_string_list,
};
pub use error::{AgentError, ToolError};
pub use events::{event_channel, AgentEvent, EventReceiver, EventSender};
pub use fsm::{AgentResponse, ReActOrchestrator};
pub use session::{InMemorySessionStore, SessionStore, SqliteSessionStore};
pub use tool::{NativeTool, Tool, ToolRegistry};

pub use actx_lm::traits::LmProvider;
pub use async_trait::async_trait;
use std::sync::Arc;

/// Default retriever delegating to a tool in the ToolRegistry (e.g. "search_db")
pub struct ToolBasedRetriever {
    tools: Arc<ToolRegistry>,
    tool_name: String,
}

impl ToolBasedRetriever {
    pub fn new(tools: Arc<ToolRegistry>, tool_name: impl Into<String>) -> Self {
        Self {
            tools,
            tool_name: tool_name.into(),
        }
    }
}

#[async_trait]
impl DeepSearchRetriever for ToolBasedRetriever {
    async fn retrieve_batch(&self, queries: &[String]) -> Result<Vec<RetrievedChunk>, AgentError> {
        let mut chunks = Vec::new();
        for q in queries {
            let args_json = serde_json::json!({ "query": q }).to_string();
            if let Ok(res_str) = self.tools.execute(&self.tool_name, &args_json).await {
                if !res_str.trim().is_empty() {
                    chunks.push(RetrievedChunk {
                        id: format!("tool_{}", uuid::Uuid::new_v4()),
                        file_name: "Knowledge Source".to_string(),
                        file_path: "workspace".to_string(),
                        header_path: None,
                        start_line: None,
                        end_line: None,
                        text: res_str,
                        score: 1.0,
                        matched_queries: vec![q.clone()],
                    });
                }
            }
        }
        Ok(chunks)
    }
}

/// High-level unified agent interface supporting both ReAct and DeepSearch
#[derive(Clone)]
pub struct Agent {
    orchestrator: Arc<ReActOrchestrator>,
    deep_search_orchestrator: Option<Arc<DeepSearchOrchestrator>>,
    execution_mode: AgentExecutionMode,
}

impl Agent {
    /// Create a fluent builder to configure an agent
    pub fn builder() -> AgentBuilder {
        AgentBuilder::default()
    }

    /// Execute the agent synchronously/asynchronously to completion
    pub async fn run(&self, input: &str, session_id: Option<&str>) -> Result<AgentResponse, AgentError> {
        if self.execution_mode == AgentExecutionMode::DeepSearch {
            if let Some(ref dso) = self.deep_search_orchestrator {
                return dso.run(input, session_id, None).await;
            }
        }
        self.orchestrator.run(input, session_id, None).await
    }

    /// Stream fine-grained events during agent execution
    pub fn stream(
        &self,
        input: impl Into<String>,
        session_id: Option<String>,
    ) -> (EventReceiver, tokio::task::JoinHandle<Result<AgentResponse, AgentError>>) {
        let inp = input.into();
        if self.execution_mode == AgentExecutionMode::DeepSearch {
            if let Some(ref dso) = self.deep_search_orchestrator {
                let (tx, rx) = event_channel();
                let dso_clone = dso.clone();
                let handle = tokio::spawn(async move {
                    let res = dso_clone.run(&inp, session_id.as_deref(), Some(tx.clone())).await;
                    if let Err(ref e) = res {
                        let _ = tx.send(AgentEvent::Error(e.to_string()));
                    }
                    res
                });
                return (rx, handle);
            }
        }
        let orch = self.orchestrator.clone();
        orch.stream(inp, session_id)
    }

    /// Access the underlying tool registry
    pub fn tools(&self) -> &ToolRegistry {
        self.orchestrator.tools()
    }

    /// Register an additional tool dynamically on this agent instance
    pub async fn register_tool(&self, tool: Arc<dyn Tool>) {
        self.orchestrator.tools().register(tool).await;
    }

    /// Register an additional tool dynamically on this agent instance synchronously
    pub fn register_tool_sync(&self, tool: Arc<dyn Tool>) {
        self.orchestrator.tools().register_sync(tool);
    }
}

/// Fluent builder for constructing an `Agent`
#[derive(Default)]
pub struct AgentBuilder {
    client: Option<Arc<dyn LmProvider>>,
    tools: ToolRegistry,
    registered_tools: Vec<Arc<dyn Tool>>,
    session_store: Option<Arc<dyn SessionStore>>,
    config: AgentConfig,
    deep_search_config: Option<DeepSearchConfig>,
    deep_search_retriever: Option<Arc<dyn DeepSearchRetriever>>,
}

impl AgentBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn client(mut self, client: Arc<dyn LmProvider>) -> Self {
        self.client = Some(client);
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.config.model = model.into();
        self
    }

    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.config.system_prompt = Some(prompt.into());
        self
    }

    pub fn max_turns(mut self, turns: usize) -> Self {
        self.config.max_turns = turns;
        self
    }

    pub fn temperature(mut self, temp: f32) -> Self {
        self.config.temperature = Some(temp);
        self
    }

    pub fn tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.registered_tools.push(tool);
        self
    }

    pub fn session_store(mut self, store: Arc<dyn SessionStore>) -> Self {
        self.session_store = Some(store);
        self
    }

    pub fn execution_mode(mut self, mode: AgentExecutionMode) -> Self {
        self.config.execution_mode = mode;
        self
    }

    pub fn search_mode(mut self, mode: SearchMode) -> Self {
        self.config.search_mode = mode;
        self
    }

    pub fn deep_search_config(mut self, config: DeepSearchConfig) -> Self {
        self.deep_search_config = Some(config);
        self
    }

    pub fn deep_search_retriever(mut self, retriever: Arc<dyn DeepSearchRetriever>) -> Self {
        self.deep_search_retriever = Some(retriever);
        self
    }

    /// Build the configured agent synchronously
    pub fn build_sync(self) -> Result<Agent, AgentError> {
        let client = self
            .client
            .ok_or_else(|| AgentError::ConfigError("Missing LmProvider client in AgentBuilder".to_string()))?;

        for t in self.registered_tools {
            self.tools.register_sync(t);
        }

        let tools_arc = Arc::new(self.tools);

        let orchestrator = Arc::new(ReActOrchestrator::new(
            client.clone(),
            tools_arc.clone(),
            self.session_store.clone(),
            self.config.clone(),
        ));

        let deep_search_orchestrator = if self.config.execution_mode == AgentExecutionMode::DeepSearch
            || self.deep_search_retriever.is_some()
        {
            let ds_cfg = self.deep_search_config.unwrap_or_default();
            let retriever = self.deep_search_retriever.unwrap_or_else(|| {
                Arc::new(ToolBasedRetriever::new(tools_arc.clone(), "search_db"))
            });

            Some(Arc::new(DeepSearchOrchestrator::new(
                client,
                retriever,
                ds_cfg,
                self.config.clone(),
                self.session_store,
            )))
        } else {
            None
        };

        Ok(Agent {
            orchestrator,
            deep_search_orchestrator,
            execution_mode: self.config.execution_mode,
        })
    }

    /// Build the configured agent asynchronously
    pub async fn build(self) -> Result<Agent, AgentError> {
        self.build_sync()
    }
}
