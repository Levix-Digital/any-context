//! Abstraction traits for language model providers (Strategy Pattern)

use std::pin::Pin;
use async_trait::async_trait;
use futures::Stream;
use crate::error::LmError;
use crate::types::{ChatRequest, ChatResponse, StreamChunk};

/// Result type emitted by individual stream chunks
pub type StreamResult = Result<StreamChunk, LmError>;

/// Type-erased boxed asynchronous stream of chunk events
pub type BoxedChunkStream = Pin<Box<dyn Stream<Item = StreamResult> + Send>>;

/// Strategy contract implemented by every model provider (LLM, SLM, local or cloud)
#[async_trait]
pub trait LmProvider: Send + Sync {
    /// Unique provider identifier (e.g. "openai", "anthropic", "gemini", "ollama", "groq", "mock")
    fn provider_id(&self) -> &'static str;

    /// Complete a chat session non-streamingly
    async fn chat_complete(&self, request: ChatRequest) -> Result<ChatResponse, LmError>;

    /// Stream a chat session token-by-token with Server-Sent Events (SSE)
    async fn chat_stream(&self, request: ChatRequest) -> Result<BoxedChunkStream, LmError>;

    /// Optional dense vector embeddings generation
    async fn embed(&self, model: &str, texts: &[String]) -> Result<Vec<Vec<f32>>, LmError> {
        let _ = (model, texts);
        Err(LmError::Unsupported(format!(
            "Provider '{}' does not implement text embedding",
            self.provider_id()
        )))
    }
}
