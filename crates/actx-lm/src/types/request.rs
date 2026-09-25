//! Request payloads and generation sampling parameters

use super::message::ChatMessage;
use super::tool::ToolDefinition;
use serde::{Deserialize, Serialize};

/// Hyperparameters controlling generation randomness, token budgets, and stopping criteria
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SamplingParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
}

impl Default for SamplingParams {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            top_p: Some(1.0),
            max_tokens: Some(4096),
            stop: None,
        }
    }
}

/// Normalized chat request across all LLMs and SLMs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub params: SamplingParams,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tools: Vec<ToolDefinition>,
    #[serde(default)]
    pub stream: bool,
}

impl ChatRequest {
    /// Create a new chat request for a specific model and conversation history
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            params: SamplingParams::default(),
            tools: Vec::new(),
            stream: false,
        }
    }

    /// Set sampling temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.params.temperature = Some(temperature);
        self
    }

    /// Set maximum tokens limit
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.params.max_tokens = Some(max_tokens);
        self
    }

    /// Set top-p nucleus sampling
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.params.top_p = Some(top_p);
        self
    }

    /// Attach tool definitions
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = tools;
        self
    }

    /// Enable or disable streaming
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }
}
