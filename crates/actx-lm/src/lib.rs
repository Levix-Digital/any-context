//! # 🚀 actx-lm
//!
//! Universal, lightweight, and modular Language Model (LLM & SLM) façade and streaming provider engine for Rust.
//! Designed for zero-cost abstractions, thread safety, and unified streaming across both local SLMs (Ollama, LM Studio)
//! and major cloud LLM providers (OpenAI, Anthropic, Gemini, Groq, DeepSeek).

pub mod client;
pub mod error;
pub mod providers;
pub mod sse;
pub mod traits;
pub mod types;

pub use client::{LmClient, LmClientBuilder};
pub use error::LmError;
pub use providers::{
    AnthropicProvider, GeminiProvider, MockLmProvider, OpenAiCompatibleProvider, ProviderKind,
};
pub use traits::{BoxedChunkStream, LmProvider, StreamResult};
pub use types::{
    ChatMessage, ChatRequest, ChatResponse, FinishReason, Role, SamplingParams, StreamChunk,
    TokenUsage, ToolCall, ToolDefinition,
};

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_facade_mock_complete_and_stream() {
        let client = LmClient::builder()
            .provider_kind(ProviderKind::Mock)
            .default_model("mock-default")
            .build()
            .unwrap();

        assert_eq!(client.provider_id(), "mock");

        // Quick chat
        let resp = client.quick_chat("mock-default", "Ping").await.unwrap();
        assert!(resp.contains("Mock response to: Ping"));

        // Streaming chat
        let req = ChatRequest::new("mock-default", vec![ChatMessage::user("Stream me")]);
        let mut stream = client.chat_stream(req).await.unwrap();

        let mut collected = Vec::new();
        while let Some(chunk_res) = stream.next().await {
            let chunk = chunk_res.unwrap();
            match chunk {
                StreamChunk::Token(t) => collected.push(t),
                StreamChunk::Completed { finish_reason, usage } => {
                    assert_eq!(finish_reason, Some(FinishReason::Stop));
                    assert!(usage.is_some());
                }
                _ => {}
            }
        }
        assert!(!collected.is_empty());
    }

    #[tokio::test]
    async fn test_facade_builder_missing_provider_fails() {
        let result = LmClient::builder().build();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_facade_builder_openai_missing_key_fails() {
        let result = LmClient::builder().provider_kind(ProviderKind::OpenAi).build();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_facade_builder_ollama_local_succeeds_without_key() {
        let result = LmClient::builder()
            .provider_kind(ProviderKind::Ollama { base_url: None })
            .default_model("llama3.2:1b")
            .build();
        assert!(result.is_ok());
        let client = result.unwrap();
        assert_eq!(client.provider_id(), "ollama");
    }
}
