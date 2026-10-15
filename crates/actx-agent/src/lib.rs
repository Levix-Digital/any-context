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
pub mod router;
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
pub use router::{
    ChatModelRouter, ChatRouterConfig, ComplexityClassifier, QueryComplexity, QueryIntent,
    RoutingDecision,
};
pub use session::{InMemorySessionStore, SessionStore, SqliteSessionStore};
pub use tool::{NativeTool, Tool, ToolRegistry};

pub use actx_lm::traits::LmProvider;
pub use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;
use futures::StreamExt;
use actx_lm::types::{ChatMessage, ChatRequest, FinishReason};

/// Abstract contract for asynchronous single-query deterministic RAG retrieval.
#[async_trait]
pub trait DeterministicRetriever: Send + Sync {
    /// Retrieves relevant chunks and formats them into a complete prompt or calibrated negative response.
    async fn retrieve_and_format(&self, query: &str) -> Result<DeterministicRetrievalOutput, AgentError>;
}

/// Output from a deterministic retrieval evaluation.
#[derive(Debug, Clone)]
pub enum DeterministicRetrievalOutput {
    /// Context was found, reordered in U, and enveloped in XML tags, ready for 1-hop execution.
    ContextReady {
        formatted_prompt: String,
        chunk_count: usize,
    },
    /// Zero chunks or below threshold; provides an honest negative response without LLM calls ($0.00 / 0 turns).
    ZeroContext {
        message: String,
    },
}

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

/// High-level unified agent interface supporting ReAct, DeepSearch, and dynamic ChatModelRouter
#[derive(Clone)]
pub struct Agent {
    orchestrator: Arc<ReActOrchestrator>,
    deep_search_orchestrator: Option<Arc<DeepSearchOrchestrator>>,
    deterministic_retriever: Option<Arc<dyn DeterministicRetriever>>,
    execution_mode: AgentExecutionMode,
    search_mode: SearchMode,
    router: Arc<ChatModelRouter>,
}

impl Agent {
    /// Create a fluent builder to configure an agent
    pub fn builder() -> AgentBuilder {
        AgentBuilder::default()
    }

    /// Access the active ChatModelRouter
    pub fn router(&self) -> &ChatModelRouter {
        &self.router
    }

    /// Access the active SearchMode policy
    pub fn search_mode(&self) -> SearchMode {
        self.search_mode
    }

    /// Access the base configured execution mode
    pub fn execution_mode(&self) -> AgentExecutionMode {
        self.execution_mode
    }

    /// Execute the agent synchronously/asynchronously to completion with dynamic routing
    pub async fn run(&self, input: &str, session_id: Option<&str>) -> Result<AgentResponse, AgentError> {
        let decision = self.router.evaluate(input, self.search_mode);
        if decision.mode == AgentExecutionMode::DeepSearch {
            if let Some(ref dso) = self.deep_search_orchestrator {
                return dso.run(input, session_id, None).await;
            }
        } else if self.deterministic_retriever.is_some()
            && (decision.mode == AgentExecutionMode::Direct
                || self.search_mode == SearchMode::Fast
                || decision.complexity == router::QueryComplexity::Fast)
        {
            let det_retriever = self.deterministic_retriever.clone().unwrap();
            let start_time = Instant::now();
            match det_retriever.retrieve_and_format(input).await? {
                DeterministicRetrievalOutput::ZeroContext { message } => {
                    if let (Some(sid), Some(store)) = (session_id, self.orchestrator.session_store()) {
                        let _ = store.append_messages(sid, &[
                            ChatMessage::user(input),
                            ChatMessage::assistant(&message),
                        ]).await;
                    }
                    return Ok(AgentResponse {
                        content: message,
                        tool_calls_count: 0,
                        total_turns: 0,
                        finish_reason: Some(FinishReason::Stop),
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                    });
                }
                DeterministicRetrievalOutput::ContextReady { formatted_prompt, chunk_count: _ } => {
                    let mut messages = Vec::new();
                    if let (Some(sid), Some(store)) = (session_id, self.orchestrator.session_store()) {
                        let prior = store.get_messages(sid).await.unwrap_or_default();
                        messages.extend(prior);
                    }
                    messages.push(ChatMessage::user(&formatted_prompt));

                    let mut req = ChatRequest::new(&self.orchestrator.config().model, messages);
                    if let Some(t) = self.orchestrator.config().temperature {
                        req = req.with_temperature(t);
                    }

                    let res = self.orchestrator.client().chat_complete(req).await
                        .map_err(AgentError::LmError)?;
                    let content = res.content;

                    if let (Some(sid), Some(store)) = (session_id, self.orchestrator.session_store()) {
                        let _ = store.append_messages(sid, &[
                            ChatMessage::user(input),
                            ChatMessage::assistant(&content),
                        ]).await;
                    }

                    return Ok(AgentResponse {
                        content,
                        tool_calls_count: 0,
                        total_turns: 1,
                        finish_reason: res.finish_reason,
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                    });
                }
            }
        }
        self.orchestrator.run(input, session_id, None).await
    }

    /// Stream fine-grained events during agent execution with dynamic routing telemetry
    pub fn stream(
        &self,
        input: impl Into<String>,
        session_id: Option<String>,
    ) -> (EventReceiver, tokio::task::JoinHandle<Result<AgentResponse, AgentError>>) {
        let inp = input.into();
        let decision = self.router.evaluate(&inp, self.search_mode);
        let (tx, rx) = event_channel();

        // Emit RoutingDecision event immediately so UI/CLI can render badges
        let _ = tx.send(AgentEvent::RoutingDecision {
            mode: format!("{:?}", decision.mode),
            complexity: format!("{:?}", decision.complexity),
            intent: decision.intent.as_str().to_string(),
            confidence: decision.confidence,
            reason: decision.reason,
        });

        if decision.mode == AgentExecutionMode::DeepSearch && self.deep_search_orchestrator.is_some() {
            let dso_clone = self.deep_search_orchestrator.clone().unwrap();
            let handle = tokio::spawn(async move {
                let res = dso_clone.run(&inp, session_id.as_deref(), Some(tx.clone())).await;
                if let Err(ref e) = res {
                    let _ = tx.send(AgentEvent::Error(e.to_string()));
                }
                res
            });
            return (rx, handle);
        }

        if self.deterministic_retriever.is_some()
            && (decision.mode == AgentExecutionMode::Direct
                || self.search_mode == SearchMode::Fast
                || decision.complexity == router::QueryComplexity::Fast)
        {
            let det_retriever = self.deterministic_retriever.clone().unwrap();
            let client = self.orchestrator.client().clone();
            let session_store = self.orchestrator.session_store().cloned();
            let config = self.orchestrator.config().clone();
            let tx_clone = tx.clone();
            let inp_clone = inp.clone();

            let handle = tokio::spawn(async move {
                let start_time = Instant::now();
                match det_retriever.retrieve_and_format(&inp_clone).await {
                    Ok(DeterministicRetrievalOutput::ZeroContext { message }) => {
                        let _ = tx_clone.send(AgentEvent::Delta(message.clone()));
                        let _ = tx_clone.send(AgentEvent::Done {
                            total_turns: 0,
                            total_tokens: Some(0),
                        });
                        if let (Some(sid), Some(ref store)) = (session_id.as_deref(), session_store.as_ref()) {
                            let _ = store.append_messages(sid, &[
                                ChatMessage::user(&inp_clone),
                                ChatMessage::assistant(&message),
                            ]).await;
                        }
                        Ok(AgentResponse {
                            content: message,
                            tool_calls_count: 0,
                            total_turns: 0,
                            finish_reason: Some(FinishReason::Stop),
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                        })
                    }
                    Ok(DeterministicRetrievalOutput::ContextReady { formatted_prompt, chunk_count: _ }) => {
                        let mut messages = Vec::new();
                        if let (Some(sid), Some(ref store)) = (session_id.as_deref(), session_store.as_ref()) {
                            let prior = store.get_messages(sid).await.unwrap_or_default();
                            messages.extend(prior);
                        }
                        messages.push(ChatMessage::user(&formatted_prompt));

                        let mut req = ChatRequest::new(&config.model, messages);
                        if let Some(t) = config.temperature {
                            req = req.with_temperature(t);
                        }

                        let mut response_content = String::new();
                        let mut finish_reason = None;

                        match client.chat_stream(req).await {
                            Ok(mut stream) => {
                                while let Some(chunk_res) = stream.next().await {
                                    if let Ok(chunk) = chunk_res {
                                        match chunk {
                                            actx_lm::types::StreamChunk::Token(token) => {
                                                response_content.push_str(&token);
                                                let _ = tx_clone.send(AgentEvent::Delta(token));
                                            }
                                            actx_lm::types::StreamChunk::Reasoning(reasoning) => {
                                                let _ = tx_clone.send(AgentEvent::Thinking(reasoning));
                                            }
                                            actx_lm::types::StreamChunk::Completed { finish_reason: fr, .. } => {
                                                finish_reason = fr;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                let _ = tx_clone.send(AgentEvent::Done {
                                    total_turns: 1,
                                    total_tokens: Some(response_content.split_whitespace().count()),
                                });

                                if let (Some(sid), Some(ref store)) = (session_id.as_deref(), session_store.as_ref()) {
                                    let _ = store.append_messages(sid, &[
                                        ChatMessage::user(&inp_clone),
                                        ChatMessage::assistant(&response_content),
                                    ]).await;
                                }

                                Ok(AgentResponse {
                                    content: response_content,
                                    tool_calls_count: 0,
                                    total_turns: 1,
                                    finish_reason,
                                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                                })
                            }
                            Err(e) => {
                                let err_msg = format!("LLM stream error: {e}");
                                let _ = tx_clone.send(AgentEvent::Error(err_msg.clone()));
                                Err(AgentError::LmError(e))
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx_clone.send(AgentEvent::Error(e.to_string()));
                        Err(e)
                    }
                }
            });
            return (rx, handle);
        }

        let orch = self.orchestrator.clone();
        let (mut inner_rx, handle) = orch.stream(inp, session_id);
        tokio::spawn(async move {
            while let Some(evt) = inner_rx.recv().await {
                let _ = tx.send(evt);
            }
        });
        (rx, handle)
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
    deterministic_retriever: Option<Arc<dyn DeterministicRetriever>>,
    router: Option<Arc<ChatModelRouter>>,
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

    pub fn router(mut self, router: Arc<ChatModelRouter>) -> Self {
        self.router = Some(router);
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

    pub fn deterministic_retriever(mut self, retriever: Arc<dyn DeterministicRetriever>) -> Self {
        self.deterministic_retriever = Some(retriever);
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

        let search_mode = if self.config.execution_mode == AgentExecutionMode::DeepSearch {
            SearchMode::Deep
        } else {
            self.config.search_mode
        };
        let deep_search_orchestrator = if self.config.execution_mode == AgentExecutionMode::DeepSearch
            || search_mode == SearchMode::Auto
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

        let router = self.router.unwrap_or_else(|| Arc::new(ChatModelRouter::default()));

        Ok(Agent {
            orchestrator,
            deep_search_orchestrator,
            deterministic_retriever: self.deterministic_retriever,
            execution_mode: self.config.execution_mode,
            search_mode,
            router,
        })
    }

    /// Build the configured agent asynchronously
    pub async fn build(self) -> Result<Agent, AgentError> {
        self.build_sync()
    }
}
