//! # 🤖 actx-agent
//!
//! Deterministic ReAct and Deep Search agent orchestrator with finite state machine (FSM),
//! polymorphic tool registry, and session persistence for Rust.
//! Designed for zero-cost abstractions, thread safety, and modular AI workflows.

pub mod config;
pub mod error;
pub mod events;
pub mod fsm;
pub mod session;
pub mod tool;

pub use config::{AgentConfig, AgentExecutionMode, SearchMode};
pub use error::{AgentError, ToolError};
pub use events::{event_channel, AgentEvent, EventReceiver, EventSender};
pub use fsm::{AgentResponse, ReActOrchestrator};
pub use session::{InMemorySessionStore, SessionStore, SqliteSessionStore};
pub use tool::{NativeTool, Tool, ToolRegistry};

use actx_lm::traits::LmProvider;
use std::sync::Arc;

/// High-level unified agent interface
#[derive(Clone)]
pub struct Agent {
    orchestrator: Arc<ReActOrchestrator>,
}

impl Agent {
    /// Create a fluent builder to configure an agent
    pub fn builder() -> AgentBuilder {
        AgentBuilder::default()
    }

    /// Execute the agent synchronously/asynchronously to completion
    pub async fn run(&self, input: &str, session_id: Option<&str>) -> Result<AgentResponse, AgentError> {
        self.orchestrator.run(input, session_id, None).await
    }

    /// Stream fine-grained events during agent execution
    pub fn stream(
        &self,
        input: impl Into<String>,
        session_id: Option<String>,
    ) -> (EventReceiver, tokio::task::JoinHandle<Result<AgentResponse, AgentError>>) {
        let orch = self.orchestrator.clone();
        orch.stream(input.into(), session_id)
    }

    /// Access the underlying tool registry
    pub fn tools(&self) -> &ToolRegistry {
        self.orchestrator.tools()
    }

    /// Register an additional tool dynamically on this agent instance
    pub async fn register_tool(&self, tool: Arc<dyn Tool>) {
        self.orchestrator.tools().register(tool).await;
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

    pub async fn build(self) -> Result<Agent, AgentError> {
        let client = self
            .client
            .ok_or_else(|| AgentError::ConfigError("Missing LmProvider client in AgentBuilder".to_string()))?;

        for t in self.registered_tools {
            self.tools.register(t).await;
        }

        let orchestrator = Arc::new(ReActOrchestrator::new(
            client,
            Arc::new(self.tools),
            self.session_store,
            self.config,
        ));

        Ok(Agent { orchestrator })
    }
}
