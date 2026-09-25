//! Anthropic Messages API adapter with Claude thinking blocks and SSE streaming

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::Deserialize;
use std::time::Duration;

use crate::error::LmError;
use crate::sse::decode_sse_stream;
use crate::traits::{BoxedChunkStream, LmProvider};
use crate::types::{
    ChatRequest, ChatResponse, FinishReason, Role, StreamChunk, TokenUsage, ToolCall,
};

/// Adapter for Anthropic's Messages API (Claude 3.5 / 3.7 Sonnet, Haiku, Opus)
#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
    anthropic_version: String,
    http_client: reqwest::Client,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider with API key
    pub fn new(api_key: impl Into<String>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_default();

        Self {
            api_key: api_key.into(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            anthropic_version: "2023-06-01".to_string(),
            http_client,
        }
    }

    /// Override the base URL (for proxying or enterprise gateway)
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn build_headers(&self) -> Result<HeaderMap, LmError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self.api_key).map_err(|_| LmError::Auth {
                provider: "anthropic".to_string(),
                message: "Invalid characters in Anthropic API key".to_string(),
            })?,
        );
        headers.insert(
            "anthropic-version",
            HeaderValue::from_str(&self.anthropic_version).unwrap(),
        );

        Ok(headers)
    }

    fn serialize_request(&self, request: &ChatRequest, stream: bool) -> serde_json::Value {
        // Anthropic separates system prompt from conversation messages
        let mut system_prompts = Vec::new();
        let mut messages = Vec::new();

        for msg in &request.messages {
            match msg.role {
                Role::System => {
                    system_prompts.push(msg.content.clone());
                }
                Role::User => {
                    messages.push(serde_json::json!({
                        "role": "user",
                        "content": msg.content
                    }));
                }
                Role::Assistant => {
                    messages.push(serde_json::json!({
                        "role": "assistant",
                        "content": msg.content
                    }));
                }
                Role::Tool => {
                    // Anthropic represents tool results as a user turn with tool_result content block
                    messages.push(serde_json::json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": msg.tool_call_id.clone().unwrap_or_default(),
                            "content": msg.content
                        }]
                    }));
                }
            }
        }

        let max_tokens = request.params.max_tokens.unwrap_or(4096);
        let mut body = serde_json::json!({
            "model": request.model,
            "max_tokens": max_tokens,
            "messages": messages,
            "stream": stream,
        });

        if !system_prompts.is_empty() {
            body["system"] = serde_json::json!(system_prompts.join("\n\n"));
        }

        if let Some(t) = request.params.temperature {
            body["temperature"] = serde_json::json!(t);
        }
        if let Some(p) = request.params.top_p {
            body["top_p"] = serde_json::json!(p);
        }
        if let Some(ref s) = request.params.stop {
            body["stop_sequences"] = serde_json::json!(s);
        }

        if !request.tools.is_empty() {
            let tools: Vec<serde_json::Value> = request
                .tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.parameters,
                    })
                })
                .collect();
            body["tools"] = serde_json::json!(tools);
        }

        body
    }
}

#[derive(Deserialize)]
struct AnthropicResponse {
    id: Option<String>,
    model: Option<String>,
    content: Option<Vec<AnthropicContentBlock>>,
    stop_reason: Option<String>,
    usage: Option<AnthropicUsage>,
    error: Option<AnthropicError>,
}

#[derive(Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
    thinking: Option<String>,
    id: Option<String>,
    name: Option<String>,
    input: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
}

#[derive(Deserialize)]
struct AnthropicError {
    message: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: Option<String>,
    delta: Option<AnthropicDelta>,
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct AnthropicDelta {
    #[serde(rename = "type")]
    delta_type: Option<String>,
    text: Option<String>,
    thinking: Option<String>,
    partial_json: Option<String>,
    stop_reason: Option<String>,
}

#[async_trait]
impl LmProvider for AnthropicProvider {
    fn provider_id(&self) -> &'static str {
        "anthropic"
    }

    async fn chat_complete(&self, request: ChatRequest) -> Result<ChatResponse, LmError> {
        let url = format!("{}/messages", self.base_url);
        let headers = self.build_headers()?;
        let body = self.serialize_request(&request, false);

        let resp = self
            .http_client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            if status.as_u16() == 401 || status.as_u16() == 403 {
                return Err(LmError::Auth {
                    provider: "anthropic".to_string(),
                    message: error_text,
                });
            } else if status.as_u16() == 429 {
                return Err(LmError::RateLimit {
                    provider: "anthropic".to_string(),
                    message: error_text,
                    retry_after_secs: None,
                });
            } else {
                return Err(LmError::ProviderError {
                    provider: "anthropic".to_string(),
                    status: status.as_u16(),
                    message: error_text,
                });
            }
        }

        let parsed: AnthropicResponse = resp.json().await?;
        if let Some(err) = parsed.error {
            return Err(LmError::ProviderError {
                provider: "anthropic".to_string(),
                status: status.as_u16(),
                message: err.message,
            });
        }

        let mut content = String::new();
        let mut thinking = None;
        let mut tool_calls = Vec::new();

        if let Some(blocks) = parsed.content {
            for block in blocks {
                match block.block_type.as_str() {
                    "text" => {
                        if let Some(t) = block.text {
                            content.push_str(&t);
                        }
                    }
                    "thinking" => {
                        thinking = block.thinking;
                    }
                    "tool_use" => {
                        if let (Some(id), Some(name), Some(input)) = (block.id, block.name, block.input) {
                            tool_calls.push(ToolCall::new(id, name, input.to_string()));
                        }
                    }
                    _ => {}
                }
            }
        }

        let finish_reason = match parsed.stop_reason.as_deref() {
            Some("end_turn") | Some("stop_sequence") => Some(FinishReason::Stop),
            Some("max_tokens") => Some(FinishReason::Length),
            Some("tool_use") => Some(FinishReason::ToolCalls),
            Some(other) => Some(FinishReason::Other(other.to_string())),
            None => None,
        };

        let usage = parsed.usage.map(|u| TokenUsage {
            prompt_tokens: u.input_tokens.unwrap_or(0),
            completion_tokens: u.output_tokens.unwrap_or(0),
            total_tokens: u.input_tokens.unwrap_or(0) + u.output_tokens.unwrap_or(0),
        });

        Ok(ChatResponse {
            id: parsed.id.unwrap_or_else(|| "msg-unknown".to_string()),
            model: parsed.model.unwrap_or(request.model),
            content,
            thinking,
            tool_calls,
            finish_reason,
            usage,
        })
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<BoxedChunkStream, LmError> {
        let url = format!("{}/messages", self.base_url);
        let headers = self.build_headers()?;
        let body = self.serialize_request(&request, true);

        let resp = self
            .http_client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            if status.as_u16() == 401 || status.as_u16() == 403 {
                return Err(LmError::Auth {
                    provider: "anthropic".to_string(),
                    message: error_text,
                });
            } else if status.as_u16() == 429 {
                return Err(LmError::RateLimit {
                    provider: "anthropic".to_string(),
                    message: error_text,
                    retry_after_secs: None,
                });
            } else {
                return Err(LmError::ProviderError {
                    provider: "anthropic".to_string(),
                    status: status.as_u16(),
                    message: error_text,
                });
            }
        }

        let byte_stream = resp.bytes_stream();
        let sse_stream = decode_sse_stream(byte_stream);

        let mapped_stream = sse_stream.filter_map(|event_res| async move {
            match event_res {
                Ok(event) => {
                    match serde_json::from_str::<AnthropicStreamEvent>(&event.data) {
                        Ok(chunk) => {
                            if let Some(delta) = chunk.delta {
                                if let Some(text) = delta.text {
                                    if !text.is_empty() {
                                        return Some(Ok(StreamChunk::Token(text)));
                                    }
                                }
                                if let Some(thinking) = delta.thinking {
                                    if !thinking.is_empty() {
                                        return Some(Ok(StreamChunk::Reasoning(thinking)));
                                    }
                                }
                                if let Some(partial) = delta.partial_json {
                                    return Some(Ok(StreamChunk::ToolCallDelta {
                                        index: 0,
                                        id: None,
                                        name: None,
                                        arguments_delta: partial,
                                    }));
                                }
                                if let Some(sr) = delta.stop_reason {
                                    let finish = match sr.as_str() {
                                        "end_turn" | "stop_sequence" => FinishReason::Stop,
                                        "max_tokens" => FinishReason::Length,
                                        "tool_use" => FinishReason::ToolCalls,
                                        other => FinishReason::Other(other.to_string()),
                                    };
                                    return Some(Ok(StreamChunk::Completed {
                                        finish_reason: Some(finish),
                                        usage: None,
                                    }));
                                }
                            }

                            if let Some(usage) = chunk.usage {
                                return Some(Ok(StreamChunk::Completed {
                                    finish_reason: None,
                                    usage: Some(TokenUsage {
                                        prompt_tokens: usage.input_tokens.unwrap_or(0),
                                        completion_tokens: usage.output_tokens.unwrap_or(0),
                                        total_tokens: usage.input_tokens.unwrap_or(0)
                                            + usage.output_tokens.unwrap_or(0),
                                    }),
                                }));
                            }

                            None
                        }
                        Err(_) => None,
                    }
                }
                Err(e) => Some(Err(e)),
            }
        });

        Ok(Box::pin(mapped_stream))
    }
}
