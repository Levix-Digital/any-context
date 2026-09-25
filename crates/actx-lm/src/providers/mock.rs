//! Mock provider for deterministic offline testing and CI/CD pipelines

use async_trait::async_trait;
use futures::stream;
use std::sync::{Arc, Mutex};
use crate::error::LmError;
use crate::traits::{BoxedChunkStream, LmProvider};
use crate::types::{
    ChatRequest, ChatResponse, FinishReason, Role, StreamChunk, TokenUsage,
};

/// Mock language model provider
#[derive(Debug, Clone)]
pub struct MockLmProvider {
    canned_response: Arc<Mutex<Option<String>>>,
    canned_thinking: Arc<Mutex<Option<String>>>,
}

impl MockLmProvider {
    /// Create a new mock provider
    pub fn new() -> Self {
        Self {
            canned_response: Arc::new(Mutex::new(None)),
            canned_thinking: Arc::new(Mutex::new(None)),
        }
    }

    /// Set a fixed response text to return on the next call
    pub fn with_canned_response(self, text: impl Into<String>) -> Self {
        *self.canned_response.lock().unwrap() = Some(text.into());
        self
    }

    /// Set reasoning/thinking content to return
    pub fn with_canned_thinking(self, thinking: impl Into<String>) -> Self {
        *self.canned_thinking.lock().unwrap() = Some(thinking.into());
        self
    }

    fn resolve_content(&self, request: &ChatRequest) -> String {
        if let Some(canned) = self.canned_response.lock().unwrap().as_ref() {
            return canned.clone();
        }

        // Echo the last user message as fallback
        let last_user_msg = request
            .messages
            .iter()
            .rev()
            .find(|m| m.role == Role::User)
            .map(|m| m.content.as_str())
            .unwrap_or("Hello from mock LM!");

        format!("Mock response to: {}", last_user_msg)
    }
}

impl Default for MockLmProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LmProvider for MockLmProvider {
    fn provider_id(&self) -> &'static str {
        "mock"
    }

    async fn chat_complete(&self, request: ChatRequest) -> Result<ChatResponse, LmError> {
        let content = self.resolve_content(&request);
        let thinking = self.canned_thinking.lock().unwrap().clone();

        let prompt_tokens = request.messages.iter().map(|m| m.content.len() as u32 / 4).sum();
        let completion_tokens = content.len() as u32 / 4;

        Ok(ChatResponse {
            id: format!("mock-resp-{}", uuid_or_timestamp()),
            model: request.model,
            content,
            thinking,
            tool_calls: Vec::new(),
            finish_reason: Some(FinishReason::Stop),
            usage: Some(TokenUsage::new(prompt_tokens, completion_tokens)),
        })
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<BoxedChunkStream, LmError> {
        let content = self.resolve_content(&request);
        let thinking = self.canned_thinking.lock().unwrap().clone();

        let prompt_tokens = request.messages.iter().map(|m| m.content.len() as u32 / 4).sum();
        let completion_tokens = content.len() as u32 / 4;

        let mut chunks: Vec<Result<StreamChunk, LmError>> = Vec::new();

        if let Some(think) = thinking {
            chunks.push(Ok(StreamChunk::Reasoning(think)));
        }

        // Split by words to simulate token streaming
        let words: Vec<&str> = content.split_inclusive(' ').collect();
        if words.is_empty() {
            chunks.push(Ok(StreamChunk::Token(content)));
        } else {
            for word in words {
                chunks.push(Ok(StreamChunk::Token(word.to_string())));
            }
        }

        chunks.push(Ok(StreamChunk::Completed {
            finish_reason: Some(FinishReason::Stop),
            usage: Some(TokenUsage::new(prompt_tokens, completion_tokens)),
        }));

        Ok(Box::pin(stream::iter(chunks)))
    }

    async fn embed(&self, _model: &str, texts: &[String]) -> Result<Vec<Vec<f32>>, LmError> {
        // Return 1536-dim mock vector of 0.1s
        Ok(vec![vec![0.1; 1536]; texts.len()])
    }
}

fn uuid_or_timestamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ChatMessage;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_mock_chat_complete() {
        let provider = MockLmProvider::new().with_canned_response("Test output");
        let req = ChatRequest::new("mock-model", vec![ChatMessage::user("Hi")]);
        let resp = provider.chat_complete(req).await.unwrap();

        assert_eq!(resp.content, "Test output");
        assert_eq!(resp.model, "mock-model");
        assert_eq!(resp.finish_reason, Some(FinishReason::Stop));
    }

    #[tokio::test]
    async fn test_mock_chat_stream() {
        let provider = MockLmProvider::new().with_canned_response("Hello brave world");
        let req = ChatRequest::new("mock-model", vec![ChatMessage::user("Hi")]);
        let mut stream = provider.chat_stream(req).await.unwrap();

        let mut tokens = Vec::new();
        while let Some(chunk) = stream.next().await {
            match chunk.unwrap() {
                StreamChunk::Token(t) => tokens.push(t),
                StreamChunk::Completed { finish_reason, .. } => {
                    assert_eq!(finish_reason, Some(FinishReason::Stop));
                }
                _ => {}
            }
        }

        assert_eq!(tokens.concat(), "Hello brave world");
    }
}
