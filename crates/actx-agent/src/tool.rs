//! Polymorphic tool calling infrastructure, schema definitions and execution registry

use crate::error::ToolError;
use actx_lm::types::ToolDefinition;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;

/// Universal contract for all tools executable by the agent
#[async_trait]
pub trait Tool: Send + Sync {
    /// Canonical identifier for the tool (e.g. "search_db", "web_search")
    fn name(&self) -> &str;

    /// Human- and LLM-readable description of what the tool accomplishes
    fn description(&self) -> &str;

    /// JSON schema describing the expected parameter types and constraints
    fn parameters_schema(&self) -> Value;

    /// Execute the tool logic with the provided parsed arguments
    async fn execute(&self, args: Value) -> Result<String, ToolError>;

    /// Convert into actx-lm compatible tool definition
    fn to_definition(&self) -> ToolDefinition {
        ToolDefinition::new(self.name(), self.description(), self.parameters_schema())
    }
}

pub type ToolFuture = Pin<Box<dyn Future<Output = Result<String, ToolError>> + Send>>;
pub type ToolFn = Box<dyn Fn(Value) -> ToolFuture + Send + Sync>;

/// Ergonomic wrapper for closure-based native Rust tools
pub struct NativeTool {
    name: String,
    description: String,
    schema: Value,
    handler: ToolFn,
}

impl NativeTool {
    pub fn new<F, Fut>(
        name: impl Into<String>,
        description: impl Into<String>,
        schema: Value,
        handler: F,
    ) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<String, ToolError>> + Send + 'static,
    {
        Self {
            name: name.into(),
            description: description.into(),
            schema,
            handler: Box::new(move |args| Box::pin(handler(args))),
        }
    }
}

#[async_trait]
impl Tool for NativeTool {
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
        (self.handler)(args).await
    }
}

/// Thread-safe registry of all active tools available to the agent
#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new tool in the registry synchronously
    pub fn register_sync(&self, tool: Arc<dyn Tool>) {
        if let Ok(mut map) = self.tools.write() {
            map.insert(tool.name().to_string(), tool);
        }
    }

    /// Register a new tool in the registry
    pub async fn register(&self, tool: Arc<dyn Tool>) {
        self.register_sync(tool);
    }

    /// Look up a tool by name synchronously
    pub fn get_sync(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.read().ok().and_then(|map| map.get(name).cloned())
    }

    /// Look up a tool by name
    pub async fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.get_sync(name)
    }

    /// Check if a tool with the given name is registered
    pub async fn contains(&self, name: &str) -> bool {
        self.tools.read().ok().map(|map| map.contains_key(name)).unwrap_or(false)
    }

    /// Retrieve all tool definitions formatted for actx-lm synchronously
    pub fn definitions_sync(&self) -> Vec<ToolDefinition> {
        self.tools
            .read()
            .ok()
            .map(|map| map.values().map(|t| t.to_definition()).collect())
            .unwrap_or_default()
    }

    /// Retrieve all tool definitions formatted for actx-lm
    pub async fn definitions(&self) -> Vec<ToolDefinition> {
        self.definitions_sync()
    }

    /// Execute a tool by name, handling raw JSON string argument parsing and defensive self-healing
    pub async fn execute(&self, name: &str, raw_args: &str) -> Result<String, ToolError> {
        let tool = match self.get_sync(name) {
            Some(t) => t,
            None => {
                return Err(ToolError::new(format!(
                    "Tool '{}' is not registered in the active agent toolset.",
                    name
                )))
            }
        };

        // Parse arguments defensively
        let parsed_args: Value = if raw_args.trim().is_empty() {
            serde_json::json!({})
        } else {
            match serde_json::from_str(raw_args) {
                Ok(v) => v,
                Err(e) => {
                    return Err(ToolError::new(format!(
                        "Invalid JSON arguments supplied for tool '{}': {}. Please correct the syntax.",
                        name, e
                    )));
                }
            }
        };

        tool.execute(parsed_args).await
    }
}
