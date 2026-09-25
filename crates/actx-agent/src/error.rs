//! Error types for actx-agent

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Language model provider error: {0}")]
    LmError(#[from] actx_lm::LmError),

    #[error("Tool execution failed for '{name}': {message}")]
    ToolExecutionFailed {
        name: String,
        message: String,
        recoverable: bool,
    },

    #[error("Tool '{0}' not found in registry")]
    ToolNotFound(String),

    #[error("Invalid tool arguments for '{name}': {details}")]
    InvalidToolArguments { name: String, details: String },

    #[error("Agent reached maximum execution turns ({0})")]
    RecursionLimitReached(usize),

    #[error("Session persistence error: {0}")]
    SessionStoreError(String),

    #[error("Agent configuration error: {0}")]
    ConfigError(String),

    #[error("Internal agent error: {0}")]
    Internal(String),
}

#[derive(Error, Debug, Clone)]
#[error("{message}")]
pub struct ToolError {
    pub message: String,
    pub recoverable: bool,
}

impl ToolError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            recoverable: true,
        }
    }

    pub fn fatal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            recoverable: false,
        }
    }
}
