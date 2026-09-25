//! Configuration and execution modes for actx-agent

use serde::{Deserialize, Serialize};

/// High-level execution strategy of the agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentExecutionMode {
    /// Single-turn direct response without multi-step loop
    Direct,
    /// Standard ReAct (Thought -> Action -> Observation -> Final Answer)
    #[default]
    ReAct,
    /// RFC-042: Deep Search multi-phase reflective reasoning
    DeepSearch,
}

/// Search routing mode (RFC-042 alignment)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    #[default]
    Auto,
    Fast,
    Deep,
}

impl std::str::FromStr for SearchMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "auto" => Ok(SearchMode::Auto),
            "fast" => Ok(SearchMode::Fast),
            "deep" => Ok(SearchMode::Deep),
            other => Err(format!("Unknown search mode '{}'. Valid: auto, fast, deep", other)),
        }
    }
}

/// Runtime configuration for agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Identifier of the model to execute (e.g. "gpt-4o", "claude-3-5-sonnet", "qwen2.5-coder")
    pub model: String,

    /// Maximum ReAct turns allowed before forced termination
    pub max_turns: usize,

    /// Sampling temperature (0.0 for deterministic RAG)
    pub temperature: Option<f32>,

    /// Global system instruction prompt
    pub system_prompt: Option<String>,

    /// Active execution mode (ReAct vs DeepSearch)
    pub execution_mode: AgentExecutionMode,

    /// Search mode policy
    pub search_mode: SearchMode,

    /// Maximum turns retained in active conversation session history
    pub max_history_turns: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: "default".to_string(),
            max_turns: 10,
            temperature: Some(0.0),
            system_prompt: None,
            execution_mode: AgentExecutionMode::ReAct,
            search_mode: SearchMode::Auto,
            max_history_turns: 30,
        }
    }
}

impl AgentConfig {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Default::default()
        }
    }

    pub fn with_max_turns(mut self, turns: usize) -> Self {
        self.max_turns = turns;
        self
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn with_execution_mode(mut self, mode: AgentExecutionMode) -> Self {
        self.execution_mode = mode;
        self
    }

    pub fn with_search_mode(mut self, mode: SearchMode) -> Self {
        self.search_mode = mode;
        self
    }
}
