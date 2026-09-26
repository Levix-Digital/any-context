use std::sync::Arc;
use actx_agent::{Agent, AgentExecutionMode, SearchMode};
use actx_lm::providers::ProviderKind;
use actx_lm::traits::LmProvider;

/// Resolves an LmClient based on available environment credentials or model prefix.
pub fn resolve_lm_provider(model_override: Option<&str>) -> Result<(Arc<dyn LmProvider>, String), String> {
    let raw_model = model_override.map(|s| s.to_string()).unwrap_or_else(|| {
        std::env::var("ACTX_MODEL")
            .unwrap_or_else(|_| "gpt-4o-mini".to_string())
    });

    let (kind, default_m) = if raw_model == "mock" || raw_model.starts_with("mock") {
        (ProviderKind::Mock, "mock-model".to_string())
    } else if raw_model.starts_with("ollama/") || raw_model.starts_with("local/") {
        let actual_model = raw_model.trim_start_matches("ollama/").trim_start_matches("local/");
        (ProviderKind::Ollama { base_url: None }, actual_model.to_string())
    } else if raw_model.starts_with("claude") {
        (ProviderKind::Anthropic, raw_model.clone())
    } else if raw_model.starts_with("gemini") {
        (ProviderKind::Gemini, raw_model.clone())
    } else if raw_model.starts_with("deepseek") {
        (ProviderKind::DeepSeek, raw_model.clone())
    } else if raw_model.starts_with("groq") {
        (ProviderKind::Groq, raw_model.clone())
    } else if raw_model.starts_with("gpt") || raw_model.starts_with("o1") || raw_model.starts_with("o3") {
        (ProviderKind::OpenAi, raw_model.clone())
    } else if std::env::var("OPENAI_API_KEY").is_ok() {
        (ProviderKind::OpenAi, raw_model.clone())
    } else if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        (ProviderKind::Anthropic, "claude-3-5-sonnet-20241022".to_string())
    } else if std::env::var("GEMINI_API_KEY").is_ok() {
        (ProviderKind::Gemini, "gemini-2.0-flash".to_string())
    } else if std::env::var("DEEPSEEK_API_KEY").is_ok() {
        (ProviderKind::DeepSeek, "deepseek-chat".to_string())
    } else if std::env::var("GROQ_API_KEY").is_ok() {
        (ProviderKind::Groq, "llama-3.3-70b-versatile".to_string())
    } else {
        // Fallback to Mock provider for seamless testing / offline demo
        (ProviderKind::Mock, raw_model.clone())
    };

    let api_key = match &kind {
        ProviderKind::Anthropic => std::env::var("ANTHROPIC_API_KEY").ok(),
        ProviderKind::Gemini => std::env::var("GEMINI_API_KEY").ok(),
        ProviderKind::DeepSeek => std::env::var("DEEPSEEK_API_KEY").ok(),
        ProviderKind::Groq => std::env::var("GROQ_API_KEY").ok(),
        ProviderKind::OpenAi => std::env::var("OPENAI_API_KEY").ok(),
        _ => None,
    };

    let provider = kind.build(api_key, None).map_err(|e| format!("Failed to initialize LM provider: {}", e))?;
    Ok((provider, default_m))
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
