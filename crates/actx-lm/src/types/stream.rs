//! Streaming event chunks emitted during real-time generation

use super::response::{FinishReason, TokenUsage};
use serde::{Deserialize, Serialize};

/// Atomic stream event chunk emitted by a provider during token-by-token generation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum StreamChunk {
    /// Incremental natural language text token
    Token(String),
    /// Extended reasoning / thinking token (e.g. DeepSeek R1, Claude thinking, OpenAI o1/o3)
    Reasoning(String),
    /// Incremental tool call fragment
    ToolCallDelta {
        index: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        arguments_delta: String,
    },
    /// Stream completion signal with final finish reason and token metrics
    Completed {
        #[serde(skip_serializing_if = "Option::is_none")]
        finish_reason: Option<FinishReason>,
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<TokenUsage>,
    },
}
