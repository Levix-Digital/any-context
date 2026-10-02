use std::sync::Arc;
use actx_agent::{Agent, AgentExecutionMode, SearchMode};
use actx_lm::providers::ProviderKind;
use actx_lm::traits::LmProvider;
use any_context_core_rs::storage::NativeConfigDb;

/// Resolves an LmClient based on persistent SQLite database configuration,
/// environment variables, or explicit overrides. Interface-agnostic.
pub fn resolve_lm_provider(
    model_override: Option<&str>,
    workspace: Option<&str>,
) -> Result<(Arc<dyn LmProvider>, String), String> {
    let db = NativeConfigDb::open_default().ok();

    // 1. Resolve effective model:
    // model_override > ACTX_MODEL env > workspace model from DB > default model from DB > "gpt-4o-mini"
    let effective_model = model_override
        .map(|s| s.to_string())
        .or_else(|| std::env::var("ACTX_MODEL").ok())
        .or_else(|| {
            if let Some(ws) = workspace {
                db.as_ref().and_then(|d| d.get_workspace_model(ws).ok())
            } else {
                None
            }
        })
        .or_else(|| db.as_ref().and_then(|d| d.get_default_model().ok()))
        .unwrap_or_else(|| "gpt-4o-mini".to_string());

    let raw_model = effective_model.trim().to_string();

    let get_key = |provider: &str| -> Option<String> {
        db.as_ref().and_then(|d| d.get_api_key(provider).ok().flatten())
    };

    let (kind, model_name, api_key) = if raw_model == "mock" || raw_model.starts_with("mock") {
        (ProviderKind::Mock, "mock-model".to_string(), None)
    } else if raw_model.starts_with("ollama/") || raw_model.starts_with("local/") {
        let actual_model = raw_model.trim_start_matches("ollama/").trim_start_matches("local/");
        (ProviderKind::Ollama { base_url: None }, actual_model.to_string(), None)
    } else if raw_model.starts_with("claude") {
        let key = get_key("anthropic");
        (ProviderKind::Anthropic, raw_model, key)
    } else if raw_model.starts_with("gemini") {
        let key = get_key("gemini");
        (ProviderKind::Gemini, raw_model, key)
    } else if raw_model.starts_with("deepseek") {
        let key = get_key("deepseek");
        (ProviderKind::DeepSeek, raw_model, key)
    } else if raw_model.starts_with("groq") {
        let key = get_key("groq");
        (ProviderKind::Groq, raw_model, key)
    } else if raw_model.starts_with("gpt") || raw_model.starts_with("o1") || raw_model.starts_with("o3") {
        let key = get_key("openai");
        (ProviderKind::OpenAi, raw_model, key)
    } else {
        // Fallback: check which provider has credentials configured in env or SQLite database
        if let Some(key) = get_key("openai") {
            (ProviderKind::OpenAi, raw_model, Some(key))
        } else if let Some(key) = get_key("anthropic") {
            (ProviderKind::Anthropic, raw_model, Some(key))
        } else if let Some(key) = get_key("gemini") {
            (ProviderKind::Gemini, raw_model, Some(key))
        } else if let Some(key) = get_key("deepseek") {
            (ProviderKind::DeepSeek, raw_model, Some(key))
        } else if let Some(key) = get_key("groq") {
            (ProviderKind::Groq, raw_model, Some(key))
        } else {
            (ProviderKind::Mock, raw_model, None)
        }
    };

    let provider = match kind.build(api_key, None) {
        Ok(p) => p,
        Err(_) => {
            // Resilient fallback to mock provider so actx never crashes
            Arc::new(actx_lm::providers::MockLmProvider::new())
        }
    };

    Ok((provider, model_name))
}

/// Builds an active `Agent` configured for the specified workspace and model synchronously.
pub fn build_agent_sync(
    provider: Arc<dyn LmProvider>,
    model: &str,
    workspace: &str,
    grounding_mode: &str,
    search_mode: &str,
    web_search_enabled: bool,
) -> Result<Agent, String> {
    let db_path = any_context_core_rs::storage::get_default_settings_db_path();
    let session_store = actx_agent::SqliteSessionStore::open(&db_path, 50).ok()
        .map(|s| Arc::new(s) as Arc<dyn actx_agent::SessionStore>);

    let web_status_str = if web_search_enabled { "ENABLED (Active)" } else { "DISABLED (Offline)" };
    let system_prompt = format!(
        "You are AnyContext (actx), an ultra-fast, local-first agentic context engine and AI assistant developed by Levix Digital.\n\
         \n\
         ### 🎯 ACTIVE WORKSPACE & OPERATIONAL ENVIRONMENT:\n\
         - Active Workspace: '{workspace}'\n\
         - Grounding Strategy: {grounding_mode}\n\
         - Search Retrieval Depth: {search_mode}\n\
         - Real-Time Web Search: {web_status_str}\n\
         \n\
         ### 🧠 PERSISTENT LONG-TERM CONVERSATION MEMORY:\n\
         - You maintain persistent conversation memory across turns and sessions stored locally in SQLite for each workspace.\n\
         - You remember prior conversations, past context, decisions, and instructions given in earlier turns of this workspace.\n\
         - When the user asks about previous topics, past conversations, or asks if you have long-term memory, ALWAYS recognize and reference your persistent memory and conversation history.\n\
         - NEVER claim that you lack long-term memory or that interactions are independent. You are AnyContext and you retain workspace memory.\n"
    );

    let ws_for_tool = workspace.to_string();
    let search_tool = actx_agent::NativeTool::new(
        "search_db",
        "Searches for relevant documents, source code, and knowledge context in the active workspace.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query, topic, or keyword to look up"
                }
            },
            "required": ["query"]
        }),
        move |args: serde_json::Value| {
            let ws = ws_for_tool.clone();
            async move {
                let query = args.get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();
                if query.is_empty() {
                    return Ok("Empty search query provided.".to_string());
                }

                let db = any_context_core_rs::storage::NativeConfigDb::open_default().ok();
                let folders = db.as_ref()
                    .and_then(|d| d.get_workspace_folders(&ws).ok())
                    .unwrap_or_default();

                let lance_path = if let Ok(local) = std::env::var("LOCALAPPDATA") {
                    std::path::PathBuf::from(local).join("AnyContext").join("lancedb")
                } else if let Some(d) = dirs::data_local_dir() {
                    d.join("AnyContext").join("lancedb")
                } else {
                    std::path::PathBuf::from("./lancedb")
                };

                let mut results = Vec::new();
                if let Ok(lance) = any_context_core_rs::storage::NativeLanceStore::open(&lance_path) {
                    let sanitized = query.replace('\'', "''");
                    if let Ok(hits) = lance.search_metadata(
                        &format!("text LIKE '%{}%'", sanitized),
                        5,
                        Some(&ws),
                        None,
                    ) {
                        for hit in hits {
                            results.push(format!("• [{}] (Score: {:.2}):\n{}", hit.file_name, hit.score, hit.text));
                        }
                    }
                }

                if results.is_empty() && !folders.is_empty() {
                    results.push(format!("Active workspace '{}' monitors folders: [{}]. No indexed vector chunks matched '{}'.", ws, folders.join(", "), query));
                } else if results.is_empty() {
                    results.push(format!("No indexed document chunks found in workspace '{}' for query '{}'.", ws, query));
                }

                Ok(results.join("\n\n"))
            }
        }
    );

    let search_policy = match search_mode.to_lowercase().as_str() {
        "fast" => SearchMode::Fast,
        "deep" => SearchMode::Deep,
        _ => SearchMode::Auto,
    };

    let mut builder = Agent::builder()
        .client(provider)
        .model(model)
        .system_prompt(system_prompt)
        .execution_mode(AgentExecutionMode::ReAct)
        .search_mode(search_policy)
        .max_turns(10)
        .tool(Arc::new(search_tool));

    if let Some(store) = session_store {
        builder = builder.session_store(store);
    }

    builder
        .build_sync()
        .map_err(|e| format!("Agent construction failed: {}", e))
}

/// Builds an active `Agent` configured for the specified workspace and model.
pub async fn build_agent(
    provider: Arc<dyn LmProvider>,
    model: &str,
    workspace: &str,
    grounding_mode: &str,
    search_mode: &str,
    web_search_enabled: bool,
) -> Result<Agent, String> {
    build_agent_sync(provider, model, workspace, grounding_mode, search_mode, web_search_enabled)
}
