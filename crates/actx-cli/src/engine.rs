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

/// Builds an active `Agent` configured for the specified workspace and model.
pub async fn build_agent(
    provider: Arc<dyn LmProvider>,
    model: &str,
    workspace: &str,
) -> Result<Agent, String> {
    let system_prompt = format!(
        "You are AnyContext (actx), an ultra-fast, local-first agentic context engine.\n\
         Active Workspace: {}\n\
         Help the user by retrieving relevant code context, diagnosing issues, and answering queries concisely.",
        workspace
    );

    Agent::builder()
        .client(provider)
        .model(model)
        .system_prompt(system_prompt)
        .execution_mode(AgentExecutionMode::ReAct)
        .search_mode(SearchMode::Auto)
        .max_turns(10)
        .build()
        .await
        .map_err(|e| format!("Agent construction failed: {}", e))
}
