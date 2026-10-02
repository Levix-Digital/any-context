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

    let system_prompt = crate::prompt::build_system_prompt(workspace, grounding_mode, search_mode, web_search_enabled);

    let ws_for_tool = workspace.to_string();
    let provider_for_search = provider.clone();
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
            let tool_provider = provider_for_search.clone();
            async move {
                let query = args.get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();
                if query.is_empty() {
                    return Ok("Empty search query provided.".to_string());
                }

                let lance_path = any_context_core_rs::storage::get_default_lancedb_path();
                let lance_store = match any_context_core_rs::storage::NativeLanceStore::open(&lance_path) {
                    Ok(ls) => Arc::new(ls),
                    Err(e) => return Ok(format!("Search database unavailable: {e}")),
                };

                let pipeline = any_context_core_rs::retrieval::NativeHybridPipeline::new(
                    lance_store,
                    Some(tool_provider),
                );

                let req = any_context_core_rs::retrieval::HybridSearchRequest {
                    query_text: query.to_string(),
                    workspace: Some(ws.clone()),
                    top_k: 5,
                    candidate_pool_k: 30,
                    max_density_chars: 12_000,
                    min_score: 0.02,
                    table_name: "workspace_chunks".to_string(),
                    ..Default::default()
                };

                match pipeline.search_single_async(&req).await {
                    Ok(results) if !results.is_empty() => {
                        let mut out = Vec::new();
                        for r in results {
                            let type_str = if r.content_type.is_empty() { "Document" } else { &r.content_type };
                            out.push(format!(
                                "• [{}] (Score: {:.2}, Source: {}, Type: {}):\n{}",
                                r.file_name,
                                r.score,
                                r.file_path,
                                type_str,
                                r.text
                            ));
                        }
                        Ok(out.join("\n\n"))
                    }
                    Ok(_) => {
                        let db = any_context_core_rs::storage::NativeConfigDb::open_default().ok();
                        let folders = db.as_ref()
                            .and_then(|d| d.get_workspace_folders(&ws).ok())
                            .unwrap_or_default();
                        let web_urls = db.as_ref()
                            .and_then(|d| d.get_workspace_web_urls(&ws).ok())
                            .unwrap_or_default();

                        if !folders.is_empty() || !web_urls.is_empty() {
                            let mut sources = Vec::new();
                            if !folders.is_empty() {
                                sources.push(format!("folders: [{}]", folders.join(", ")));
                            }
                            if !web_urls.is_empty() {
                                sources.push(format!("web portals: [{}]", web_urls.join(", ")));
                            }
                            Ok(format!(
                                "Active workspace '{}' monitors {}. No indexed document chunks matched query '{}'.",
                                ws,
                                sources.join("; "),
                                query
                            ))
                        } else {
                            Ok(format!(
                                "No indexed document chunks found in workspace '{}' for query '{}'.",
                                ws, query
                            ))
                        }
                    }
                    Err(e) => {
                        Ok(format!("Search query returned 0 results: {e}"))
                    }
                }
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
