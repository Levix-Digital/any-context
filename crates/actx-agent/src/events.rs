//! Event-driven streaming architecture for actx-agent
//! Emits fine-grained events for thinking blocks, tool calls, streaming text, and RFC-042 tree rendering.

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Granular event emitted during agent execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum AgentEvent {
    /// Internal reasoning token from model (e.g. DeepSeek-R1, Claude thinking)
    Thinking(String),

    /// Beginning of a tool execution turn
    ToolStart {
        id: String,
        name: String,
        arguments: String,
    },

    /// Completion of a tool execution turn
    ToolEnd {
        id: String,
        name: String,
        result: String,
        is_error: bool,
    },

    /// Streaming text delta for the final response
    Delta(String),

    /// RFC-042 Deep Search: Query decomposition into orthogonal sub-queries
    Decomposition {
        sub_queries: Vec<String>,
    },

    /// RFC-042 Deep Search: Starting a new reflection iteration
    IterationStart {
        iteration: usize,
        max_iterations: usize,
    },

    /// RFC-042 Deep Search: Information gap evaluation result
    GapAnalysis {
        is_sufficient: bool,
        missing_aspects: Vec<String>,
    },

    /// Execution completed successfully
    Done {
        total_turns: usize,
        total_tokens: Option<usize>,
    },

    /// Unrecoverable error occurred
    Error(String),
}

/// Asynchronous channel sender for agent events
pub type EventSender = mpsc::UnboundedSender<AgentEvent>;

/// Asynchronous channel receiver for agent events
pub type EventReceiver = mpsc::UnboundedReceiver<AgentEvent>;

/// Helper to create a new unbounded event channel
pub fn event_channel() -> (EventSender, EventReceiver) {
    mpsc::unbounded_channel()
}
