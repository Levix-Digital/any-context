use std::sync::Arc;
use actx_agent::{Agent, AgentExecutionMode, SearchMode};
use actx_lm::providers::ProviderKind;
use actx_lm::traits::LmProvider;

/// Resolves an LmClient based on available environment credentials or model prefix.
pub fn resolve_lm_provider(model_override: Option<&str>) -> Result<(Arc<dyn LmProvider>, String), String> {
    let explicit_model = model_override
        .map(|s| s.to_string())
        .or_else(|| std::env::var("ACTX_MODEL").ok());

    let (kind, model_name, api_key) = if let Some(raw_model) = explicit_model {
        if raw_model == "mock" || raw_model.starts_with("mock") {
            (ProviderKind::Mock, "mock-model".to_string(), None)
        } else if raw_model.starts_with("ollama/") || raw_model.starts_with("local/") {
            let actual_model = raw_model.trim_start_matches("ollama/").trim_start_matches("local/");
            (ProviderKind::Ollama { base_url: None }, actual_model.to_string(), None)
        } else if raw_model.starts_with("claude") {
            let key = std::env::var("ANTHROPIC_API_KEY").ok();
            (ProviderKind::Anthropic, raw_model, key)
        } else if raw_model.starts_with("gemini") {
            let key = std::env::var("GEMINI_API_KEY").ok();
            (ProviderKind::Gemini, raw_model, key)
        } else if raw_model.starts_with("deepseek") {
            let key = std::env::var("DEEPSEEK_API_KEY").ok();
            (ProviderKind::DeepSeek, raw_model, key)
        } else if raw_model.starts_with("groq") {
            let key = std::env::var("GROQ_API_KEY").ok();
            (ProviderKind::Groq, raw_model, key)
        } else if raw_model.starts_with("gpt") || raw_model.starts_with("o1") || raw_model.starts_with("o3") {
            let key = std::env::var("OPENAI_API_KEY").ok();
            (ProviderKind::OpenAi, raw_model, key)
        } else {
            // Check available keys
            if let Ok(key) = std::env::var("OPENAI_API_KEY") {
                (ProviderKind::OpenAi, raw_model, Some(key))
            } else if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
                (ProviderKind::Anthropic, raw_model, Some(key))
            } else if let Ok(key) = std::env::var("GEMINI_API_KEY") {
                (ProviderKind::Gemini, raw_model, Some(key))
            } else if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
                (ProviderKind::DeepSeek, raw_model, Some(key))
            } else if let Ok(key) = std::env::var("GROQ_API_KEY") {
                (ProviderKind::Groq, raw_model, Some(key))
            } else {
                (ProviderKind::Mock, raw_model, None)
            }
        }
    } else {
        // No explicit model requested -> detect based on available credentials
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            (ProviderKind::OpenAi, "gpt-4o-mini".to_string(), Some(key))
        } else if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            (ProviderKind::Anthropic, "claude-3-5-sonnet-20241022".to_string(), Some(key))
        } else if let Ok(key) = std::env::var("GEMINI_API_KEY") {
            (ProviderKind::Gemini, "gemini-3.8-flash".to_string(), Some(key))
        } else if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
            (ProviderKind::DeepSeek, "deepseek-chat".to_string(), Some(key))
        } else if let Ok(key) = std::env::var("GROQ_API_KEY") {
            (ProviderKind::Groq, "llama-3.3-70b-versatile".to_string(), Some(key))
        } else {
            // Local-first zero credential fallback
            (ProviderKind::Mock, "mock-model".to_string(), None)
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
