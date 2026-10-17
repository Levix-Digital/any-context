//! Chat model catalog: the models offered by `/preset`, `/models`, the models menu and
//! onboarding, and the successors of model ids that their providers have shut down.
//!
//! Model ids are the strings the user types and that are stored in `settings.db`; the
//! provider is chosen from the id prefix when the model is resolved (`claude*`, `gpt*`,
//! `gemini*`, `deepseek*`, `groq/`, `ollama/`).

/// Provider family of a catalog model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatProvider {
    /// Local Gemma 4 ONNX (generation planned; answers come from the fallback provider)
    LocalGemma,
    Gemini,
    OpenAi,
    Anthropic,
    DeepSeek,
    Groq,
    Ollama,
    Mock,
}

impl ChatProvider {
    /// Name shown to the user.
    pub fn label(&self) -> &'static str {
        match self {
            ChatProvider::LocalGemma => "Local Gemma 4",
            ChatProvider::Gemini => "Google",
            ChatProvider::OpenAi => "OpenAI",
            ChatProvider::Anthropic => "Anthropic",
            ChatProvider::DeepSeek => "DeepSeek",
            ChatProvider::Groq => "Groq",
            ChatProvider::Ollama => "Ollama",
            ChatProvider::Mock => "Mock",
        }
    }
}

/// A chat model offered to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatModel {
    pub id: &'static str,
    pub provider: ChatProvider,
    pub summary: &'static str,
}

/// Model used when nothing else is configured.
pub const DEFAULT_CHAT_MODEL: &str = "gpt-6-luna";
/// Model set by `/preset balanced`.
pub const BALANCED_PRESET_MODEL: &str = "gpt-6-luna";
/// Model set by `/preset frontier`.
pub const FRONTIER_PRESET_MODEL: &str = "claude-opus-5-5";

const CHAT_MODELS: &[ChatModel] = &[
    ChatModel { id: "gemma-4-compact", provider: ChatProvider::LocalGemma, summary: "Planned - Local Gemma 4 Compact generation (not available yet; uses the fallback provider)" },
    ChatModel { id: "gemma-4-12b", provider: ChatProvider::LocalGemma, summary: "Planned - Local Gemma 4 12B generation (not available yet; uses the fallback provider)" },
    ChatModel { id: "gemma-4-27b", provider: ChatProvider::LocalGemma, summary: "Planned - Local Gemma 4 27B generation (not available yet; uses the fallback provider)" },
    ChatModel { id: "gemma-4-32b", provider: ChatProvider::LocalGemma, summary: "Planned - Local Gemma 4 32B generation (not available yet; uses the fallback provider)" },
    ChatModel { id: "gemini-3.8-flash", provider: ChatProvider::Gemini, summary: "Low latency and multimodal" },
    ChatModel { id: "gemini-2.5-pro", provider: ChatProvider::Gemini, summary: "Long context and deeper reasoning" },
    ChatModel { id: "gpt-6-luna", provider: ChatProvider::OpenAi, summary: "Fast and cost-efficient (default)" },
    ChatModel { id: "gpt-6-sol", provider: ChatProvider::OpenAi, summary: "High-capability flagship" },
    ChatModel { id: "claude-opus-5-5", provider: ChatProvider::Anthropic, summary: "Deepest reasoning and analysis" },
    ChatModel { id: "claude-sonnet-5-5", provider: ChatProvider::Anthropic, summary: "Balanced speed and capability for coding" },
    ChatModel { id: "claude-haiku-5-5", provider: ChatProvider::Anthropic, summary: "Fastest and lowest cost" },
    ChatModel { id: "deepseek-flash", provider: ChatProvider::DeepSeek, summary: "High cost-benefit for coding" },
    ChatModel { id: "deepseek-v4-pro", provider: ChatProvider::DeepSeek, summary: "Step-by-step analytical reasoning" },
    ChatModel { id: "groq/openai/gpt-oss-120b", provider: ChatProvider::Groq, summary: "Fast inference on LPU hardware" },
    ChatModel { id: "ollama/qwen2.5-coder", provider: ChatProvider::Ollama, summary: "Offline model via the local Ollama endpoint" },
    ChatModel { id: "mock", provider: ChatProvider::Mock, summary: "Instant offline simulation for testing and demos" },
];

/// Retired model id (lowercase) and the current model that replaces it.
const RETIRED_MODELS: &[(&str, &str)] = &[
    // Anthropic: Claude Sonnet 3.7 and Haiku 3.5 retired 2026-02-19, Sonnet 3.5 on 2025-10-28.
    ("claude-3-7-sonnet", "claude-opus-5-5"),
    ("claude-3-7-sonnet-latest", "claude-opus-5-5"),
    ("claude-3-7-sonnet-20250219", "claude-opus-5-5"),
    ("claude-3-5-sonnet", "claude-sonnet-5-5"),
    ("claude-3-5-sonnet-latest", "claude-sonnet-5-5"),
    ("claude-3-5-sonnet-20241022", "claude-sonnet-5-5"),
    ("claude-3-5-sonnet-20240620", "claude-sonnet-5-5"),
    ("claude-3-5-haiku", "claude-haiku-5-5"),
    ("claude-3-5-haiku-latest", "claude-haiku-5-5"),
    ("claude-3-5-haiku-20241022", "claude-haiku-5-5"),
    // OpenAI: o1, o1-pro and o3-mini shut down 2026-10-23.
    ("o1", "gpt-6-sol"),
    ("o1-2024-12-17", "gpt-6-sol"),
    ("o1-pro", "gpt-6-sol"),
    ("o1-pro-2025-03-19", "gpt-6-sol"),
    ("o3-mini", "gpt-6-sol"),
    ("o3-mini-2025-01-31", "gpt-6-sol"),
    // Google: Gemini 1.5 Pro is no longer served.
    ("gemini-1.5-pro", "gemini-2.5-pro"),
    ("gemini-1.5-pro-latest", "gemini-2.5-pro"),
    // Groq: Llama 3.3 70B shut down 2026-08-16.
    ("llama-3.3-70b-versatile", "groq/openai/gpt-oss-120b"),
    ("groq/llama-3.3-70b", "groq/openai/gpt-oss-120b"),
    ("groq/llama-3.3-70b-versatile", "groq/openai/gpt-oss-120b"),
    // DeepSeek: the legacy names were discontinued 2026-07-24.
    ("deepseek-chat", "deepseek-flash"),
    ("deepseek-reasoner", "deepseek-v4-pro"),
];

/// Models offered by `/models` and the models menu, in display order.
pub fn chat_models() -> &'static [ChatModel] {
    CHAT_MODELS
}

/// Model saved when onboarding finishes with the given provider.
pub fn onboarding_default(provider: ChatProvider) -> &'static str {
    match provider {
        ChatProvider::Gemini => "gemini-2.5-flash",
        ChatProvider::OpenAi => DEFAULT_CHAT_MODEL,
        ChatProvider::Anthropic => "claude-sonnet-5-5",
        ChatProvider::Ollama => "ollama/llama3",
        ChatProvider::DeepSeek => "deepseek-flash",
        ChatProvider::Groq => "groq/openai/gpt-oss-120b",
        ChatProvider::LocalGemma => "gemma-4-12b",
        ChatProvider::Mock => "mock",
    }
}

/// The current model that replaces a retired model id, or `None` when the id is not retired.
/// Only exact ids match (ignoring case and surrounding spaces), so custom ids are never rewritten.
pub fn successor_of(model_id: &str) -> Option<&'static str> {
    let id = model_id.trim().to_ascii_lowercase();
    RETIRED_MODELS
        .iter()
        .find(|(retired, _)| *retired == id)
        .map(|(_, successor)| *successor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retired_ids_map_to_their_successors() {
        assert_eq!(successor_of("claude-3-7-sonnet"), Some("claude-opus-5-5"));
        assert_eq!(successor_of("claude-3-5-sonnet-20241022"), Some("claude-sonnet-5-5"));
        assert_eq!(successor_of("claude-3-5-haiku-20241022"), Some("claude-haiku-5-5"));
        assert_eq!(successor_of("o3-mini"), Some("gpt-6-sol"));
        assert_eq!(successor_of("gemini-1.5-pro"), Some("gemini-2.5-pro"));
        assert_eq!(successor_of("llama-3.3-70b-versatile"), Some("groq/openai/gpt-oss-120b"));
        assert_eq!(successor_of("deepseek-chat"), Some("deepseek-flash"));
        assert_eq!(successor_of("deepseek-reasoner"), Some("deepseek-v4-pro"));
    }

    #[test]
    fn successor_lookup_ignores_case_and_spaces() {
        assert_eq!(successor_of("  Claude-3-7-Sonnet "), Some("claude-opus-5-5"));
    }

    #[test]
    fn current_and_custom_ids_are_not_mapped() {
        assert_eq!(successor_of("claude-opus-5-5"), None);
        assert_eq!(successor_of("gpt-4o-mini"), None);
        assert_eq!(successor_of("claude-3-5-sonnet-custom"), None);
        assert_eq!(successor_of("ollama/llama3"), None);
    }

    #[test]
    fn offered_models_are_never_retired() {
        let mut offered: Vec<&str> = chat_models().iter().map(|m| m.id).collect();
        offered.extend([DEFAULT_CHAT_MODEL, BALANCED_PRESET_MODEL, FRONTIER_PRESET_MODEL]);
        for provider in [
            ChatProvider::LocalGemma, ChatProvider::Gemini, ChatProvider::OpenAi, ChatProvider::Anthropic,
            ChatProvider::DeepSeek, ChatProvider::Groq, ChatProvider::Ollama, ChatProvider::Mock,
        ] {
            offered.push(onboarding_default(provider));
        }
        for id in offered {
            assert_eq!(successor_of(id), None, "{id} is offered but listed as retired");
        }
    }

    #[test]
    fn every_successor_is_offered_or_a_default() {
        for (_, successor) in RETIRED_MODELS {
            assert!(
                chat_models().iter().any(|m| m.id == *successor),
                "successor {successor} is not in the catalog"
            );
        }
    }

    #[test]
    fn catalog_ids_are_unique() {
        let ids: Vec<&str> = chat_models().iter().map(|m| m.id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len());
    }
}
