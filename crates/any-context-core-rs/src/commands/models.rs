//! Domain models and result contracts for the Universal Command Engine.
//! Decoupled from any presentation layer (TUI, GUI, REST API, RPC, MCP).

use serde::{Deserialize, Serialize};

/// High-level presentation actions requested by command execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CommandAction {
    /// No special UI action required; display message only
    #[default]
    None,
    /// Terminate presentation session
    Exit,
    /// Clear conversation / presentation viewport
    ClearChat,
    /// Request UI to open an interactive modal/menu with specified identifier (e.g. "workspaces", "sync", "sources", "main")
    OpenMenu(String),
    /// Notify UI that active workspace changed
    SwitchWorkspace(String),
    /// Request UI to rebuild its background agent orchestrator
    RebuildAgent,
}

/// Reactive state updates emitted by command execution to be reflected by presentation adapters.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandStateUpdates {
    /// New active workspace name, if changed
    pub active_workspace: Option<String>,
    /// New active model name, if changed
    pub active_model: Option<String>,
    /// New Grounding Strategy mode ("strict", "hybrid", "proactive"), if changed
    pub grounding_mode: Option<String>,
    /// New Search Depth mode ("auto", "fast", "deep"), if changed
    pub search_mode: Option<String>,
    /// Real-time web search status toggle, if changed
    pub web_search_enabled: Option<bool>,
}

/// Standardized domain result returned by `CommandEngine::execute`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandResult {
    /// Whether the command executed successfully
    pub success: bool,
    /// Human-readable response / feedback message formatted in Markdown
    pub message: String,
    /// Presentation action requested by the command
    pub action: CommandAction,
    /// State updates that the UI should apply
    pub state_updates: CommandStateUpdates,
    /// Optional error details if execution failed
    pub error: Option<String>,
}

impl CommandResult {
    /// Create a successful command result with a message
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            action: CommandAction::None,
            state_updates: CommandStateUpdates::default(),
            error: None,
        }
    }

    /// Create an error command result with a message
    pub fn error(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self {
            success: false,
            message: msg.clone(),
            action: CommandAction::None,
            state_updates: CommandStateUpdates::default(),
            error: Some(msg),
        }
    }

    /// Attach a presentation action to the result
    pub fn with_action(mut self, action: CommandAction) -> Self {
        self.action = action;
        self
    }

    /// Attach state updates to the result
    pub fn with_state_updates(mut self, updates: CommandStateUpdates) -> Self {
        self.state_updates = updates;
        self
    }
}

/// UI-agnostic execution context supplied by presentation adapters to `CommandEngine`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Active workspace identifier
    pub active_workspace: String,
    /// Active language model identifier
    pub active_model: String,
    /// Active grounding strategy ("strict", "hybrid", "proactive")
    pub grounding_mode: String,
    /// Active search depth policy ("auto", "fast", "deep")
    pub search_mode: String,
    /// Whether real-time web search is active
    pub web_search_enabled: bool,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            active_workspace: "Default".to_string(),
            active_model: "gpt-4o-mini".to_string(),
            grounding_mode: "strict".to_string(),
            search_mode: "auto".to_string(),
            web_search_enabled: false,
        }
    }
}

/// Canonical Grounding Strategy Modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GroundingMode {
    #[default]
    Strict,
    Hybrid,
    Proactive,
}

impl GroundingMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            GroundingMode::Strict => "strict",
            GroundingMode::Hybrid => "hybrid",
            GroundingMode::Proactive => "proactive",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().trim_start_matches('-').to_lowercase().as_str() {
            "strict" | "s" => Some(GroundingMode::Strict),
            "hybrid" | "h" => Some(GroundingMode::Hybrid),
            "proactive" | "p" => Some(GroundingMode::Proactive),
            _ => None,
        }
    }
}

/// Canonical Search Depth Modes (RFC-042 alignment)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SearchDepthMode {
    #[default]
    Auto,
    Fast,
    Deep,
}

impl SearchDepthMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            SearchDepthMode::Auto => "auto",
            SearchDepthMode::Fast => "fast",
            SearchDepthMode::Deep => "deep",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().trim_start_matches('-').to_lowercase().as_str() {
            "auto" | "a" => Some(SearchDepthMode::Auto),
            "fast" | "f" => Some(SearchDepthMode::Fast),
            "deep" | "d" => Some(SearchDepthMode::Deep),
            _ => None,
        }
    }
}
