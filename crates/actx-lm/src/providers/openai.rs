//! OpenAI native and universal OpenAI-compatible provider (Ollama, Groq, DeepSeek, OpenRouter, LM Studio)

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

use crate::error::LmError;
use crate::sse::decode_sse_stream;
use crate::traits::{BoxedChunkStream, LmProvider};
use crate::types::{
    ChatRequest, ChatResponse, FinishReason, StreamChunk, TokenUsage, ToolCall,
};

/// Universal provider implementing the OpenAI Chat Completions REST specification.
///
/// Can target official OpenAI, local SLMs (Ollama, LM Studio, vLLM, llama.cpp),
/// or cloud providers using the OpenAI API standard (Groq, DeepSeek, OpenRouter, Together AI).
#[derive(Debug, Clone)]
pub struct OpenAiCompatibleProvider {
    provider_name: String,
    base_url: String,
    api_key: Option<String>,
    extra_headers: HashMap<String, String>,
    http_client: reqwest::Client,
}

impl OpenAiCompatibleProvider {
    /// Construct a new OpenAI-compatible provider
    pub fn new(
        provider_name: impl Into<String>,
        base_url: impl Into<String>,
        api_key: Option<String>,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_default();

        let mut base = base_url.into();
        while base.ends_with('/') {
            base.pop();
        }

        Self {
            provider_name: provider_name.into(),
            base_url: base,
            api_key,
            extra_headers: HashMap::new(),
            http_client,
        }
    }

    /// Helper for standard OpenAI Cloud
    pub fn openai(api_key: impl Into<String>) -> Self {
        Self::new("openai", "https://api.openai.com/v1", Some(api_key.into()))
    }

    /// Helper for local Ollama SLMs
    pub fn ollama(base_url: Option<String>) -> Self {
        let url = base_url.unwrap_or_else(|| "http://localhost:11434/v1".to_string());
        Self::new("ollama", url, None)
    }

    /// Helper for Groq Cloud
    pub fn groq(api_key: impl Into<String>) -> Self {
        Self::new("groq", "https://api.groq.com/openai/v1", Some(api_key.into()))
    }

    /// Helper for OpenRouter
    pub fn openrouter(api_key: impl Into<String>) -> Self {
        let mut provider = Self::new("openrouter", "https://openrouter.ai/api/v1", Some(api_key.into()));
        provider.extra_headers.insert("HTTP-Referer".to_string(), "https://anycontext.ai".to_string());
        provider.extra_headers.insert("X-Title".to_string(), "AnyContext".to_string());
        provider
    }

    /// Helper for DeepSeek
    pub fn deepseek(api_key: impl Into<String>) -> Self {
        Self::new("deepseek", "https://api.deepseek.com/v1", Some(api_key.into()))
    }

    /// Add custom headers
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_headers.insert(key.into(), value.into());
        self
    }

    fn build_headers(&self) -> Result<HeaderMap, LmError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        if let Some(ref key) = self.api_key {
            let auth_val = format!("Bearer {}", key);
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&auth_val).map_err(|_| LmError::Auth {
                    provider: self.provider_name.clone(),
                    message: "Invalid characters in API key".to_string(),
                })?,
            );
        }

        for (k, v) in &self.extra_headers {
            if let (Ok(h_name), Ok(h_val)) = (
                reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                HeaderValue::from_str(v),
            ) {
                headers.insert(h_name, h_val);
            }
        }

        Ok(headers)
    }

    fn serialize_request(&self, request: &ChatRequest, stream: bool) -> serde_json::Value {
        let mut json = serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "stream": stream,
        });

        if let Some(t) = request.params.temperature {
            json["temperature"] = serde_json::json!(t);
        }
        if let Some(p) = request.params.top_p {
            json["top_p"] = serde_json::json!(p);
        }
        if let Some(m) = request.params.max_tokens {
            json["max_tokens"] = serde_json::json!(m);
        }
        if let Some(ref s) = request.params.stop {
            json["stop"] = serde_json::json!(s);
        }

        if !request.tools.is_empty() {
            let tools: Vec<serde_json::Value> = request
                .tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        }
                    })
                })
                .collect();
            json["tools"] = serde_json::json!(tools);
        }

        if stream {
            json["stream_options"] = serde_json::json!({
                "include_usage": true
            });
        }

        json
    }
}

#[derive(Deserialize)]
struct OpenAiChatCompletionResponse {
    id: Option<String>,
    model: Option<String>,
    choices: Option<Vec<OpenAiChoice>>,
    usage: Option<OpenAiUsage>,
    error: Option<OpenAiApiError>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: Option<OpenAiResponseMessage>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiResponseMessage {
    content: Option<String>,
    reasoning_content: Option<String>,
    tool_calls: Option<Vec<OpenAiToolCall>>,
}

#[derive(Deserialize)]
struct OpenAiToolCall {
    id: String,
    function: OpenAiFunctionCall,
}

#[derive(Deserialize)]
struct OpenAiFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize)]
struct OpenAiApiError {
    message: String,
}

#[derive(Deserialize)]
struct OpenAiStreamChunk {
    choices: Option<Vec<OpenAiStreamChoice>>,
    usage: Option<OpenAiUsage>,
}

#[derive(Deserialize)]
struct OpenAiStreamChoice {
    delta: Option<OpenAiDelta>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiDelta {
    content: Option<String>,
    reasoning_content: Option<String>,
    tool_calls: Option<Vec<OpenAiStreamToolCall>>,
}

#[derive(Deserialize)]
struct OpenAiStreamToolCall {
    index: usize,
    id: Option<String>,
    function: Option<OpenAiStreamFunctionCall>,
}

#[derive(Deserialize)]
struct OpenAiStreamFunctionCall {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiEmbeddingResponse {
    data: Vec<OpenAiEmbeddingData>,
}

#[derive(Deserialize)]
struct OpenAiEmbeddingData {
    embedding: Vec<f32>,
}

#[async_trait]
impl LmProvider for OpenAiCompatibleProvider {
    fn provider_id(&self) -> &'static str {
        // Leak string as static or return canonical name
        match self.provider_name.as_str() {
            "openai" => "openai",
            "ollama" => "ollama",
            "groq" => "groq",
            "deepseek" => "deepseek",
            "openrouter" => "openrouter",
            _ => "openai-compatible",
        }
    }

    async fn chat_complete(&self, request: ChatRequest) -> Result<ChatResponse, LmError> {
        let url = format!("{}/chat/completions", self.base_url);
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
                    provider: self.provider_name.clone(),
                    message: error_text,
                });
            } else if status.as_u16() == 429 {
                return Err(LmError::RateLimit {
                    provider: self.provider_name.clone(),
                    message: error_text,
                    retry_after_secs: None,
                });
            } else {
                return Err(LmError::ProviderError {
                    provider: self.provider_name.clone(),
                    status: status.as_u16(),
                    message: error_text,
                });
            }
        }

        let parsed: OpenAiChatCompletionResponse = resp.json().await?;
        if let Some(err) = parsed.error {
            return Err(LmError::ProviderError {
                provider: self.provider_name.clone(),
                status: status.as_u16(),
                message: err.message,
            });
        }

        let first_choice = parsed.choices.and_then(|mut c| c.drain(..).next());
        let (content, thinking, tool_calls, finish_reason) = if let Some(c) = first_choice {
            let finish = match c.finish_reason.as_deref() {
                Some("stop") => Some(FinishReason::Stop),
                Some("length") => Some(FinishReason::Length),
                Some("tool_calls") => Some(FinishReason::ToolCalls),
                Some("content_filter") => Some(FinishReason::ContentFilter),
                Some(other) => Some(FinishReason::Other(other.to_string())),
                None => None,
            };

            if let Some(msg) = c.message {
                let tc = msg
                    .tool_calls
                    .unwrap_or_default()
                    .into_iter()
                    .map(|t| ToolCall::new(t.id, t.function.name, t.function.arguments))
                    .collect();
                (msg.content.unwrap_or_default(), msg.reasoning_content, tc, finish)
            } else {
                (String::new(), None, Vec::new(), finish)
            }
        } else {
            (String::new(), None, Vec::new(), None)
        };

        let usage = parsed.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        Ok(ChatResponse {
            id: parsed.id.unwrap_or_else(|| "chatcmpl-unknown".to_string()),
            model: parsed.model.unwrap_or(request.model),
            content,
            thinking,
            tool_calls,
            finish_reason,
            usage,
        })
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<BoxedChunkStream, LmError> {
        let url = format!("{}/chat/completions", self.base_url);
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
                    provider: self.provider_name.clone(),
                    message: error_text,
                });
            } else if status.as_u16() == 429 {
                return Err(LmError::RateLimit {
                    provider: self.provider_name.clone(),
                    message: error_text,
                    retry_after_secs: None,
                });
            } else {
                return Err(LmError::ProviderError {
                    provider: self.provider_name.clone(),
                    status: status.as_u16(),
                    message: error_text,
                });
            }
        }

        let byte_stream = resp.bytes_stream();
        let sse_stream = decode_sse_stream(byte_stream);

        let provider_name = self.provider_name.clone();
        let mapped_stream = sse_stream.filter_map(move |event_res| {
            let provider_name = provider_name.clone();
            async move {
                match event_res {
                    Ok(event) => {
                        if event.data == "[DONE]" {
                            return Some(Ok(StreamChunk::Completed {
                                finish_reason: Some(FinishReason::Stop),
                                usage: None,
                            }));
                        }

                        match serde_json::from_str::<OpenAiStreamChunk>(&event.data) {
                            Ok(chunk) => {
                                if let Some(usage) = chunk.usage {
                                    return Some(Ok(StreamChunk::Completed {
                                        finish_reason: None,
                                        usage: Some(TokenUsage {
                                            prompt_tokens: usage.prompt_tokens,
                                            completion_tokens: usage.completion_tokens,
                                            total_tokens: usage.total_tokens,
                                        }),
                                    }));
                                }

                                if let Some(choices) = chunk.choices {
                                    if let Some(choice) = choices.into_iter().next() {
                                        if let Some(delta) = choice.delta {
                                            if let Some(reasoning) = delta.reasoning_content {
                                                if !reasoning.is_empty() {
                                                    return Some(Ok(StreamChunk::Reasoning(reasoning)));
                                                }
                                            }

                                            if let Some(text) = delta.content {
                                                if !text.is_empty() {
                                                    return Some(Ok(StreamChunk::Token(text)));
                                                }
                                            }

                                            if let Some(tool_calls) = delta.tool_calls {
                                                if let Some(tc) = tool_calls.into_iter().next() {
                                                    let args_delta = tc
                                                        .function
                                                        .as_ref()
                                                        .and_then(|f| f.arguments.clone())
                                                        .unwrap_or_default();
                                                    let fn_name = tc
                                                        .function
                                                        .as_ref()
                                                        .and_then(|f| f.name.clone());

                                                    return Some(Ok(StreamChunk::ToolCallDelta {
                                                        index: tc.index,
                                                        id: tc.id,
                                                        name: fn_name,
                                                        arguments_delta: args_delta,
                                                    }));
                                                }
                                            }
                                        }

                                        if let Some(fr) = choice.finish_reason {
                                            let finish = match fr.as_str() {
                                                "stop" => FinishReason::Stop,
                                                "length" => FinishReason::Length,
                                                "tool_calls" => FinishReason::ToolCalls,
                                                "content_filter" => FinishReason::ContentFilter,
                                                other => FinishReason::Other(other.to_string()),
                                            };
                                            return Some(Ok(StreamChunk::Completed {
                                                finish_reason: Some(finish),
                                                usage: None,
                                            }));
                                        }
                                    }
                                }

                                None
                            }
                            Err(e) => Some(Err(LmError::StreamError(format!(
                                "Failed to parse OpenAI SSE chunk for {}: {}",
                                provider_name, e
                            )))),
                        }
                    }
                    Err(e) => Some(Err(e)),
                }
            }
        });

        Ok(Box::pin(mapped_stream))
    }

    async fn embed(&self, model: &str, texts: &[String]) -> Result<Vec<Vec<f32>>, LmError> {
        let url = format!("{}/embeddings", self.base_url);
        let headers = self.build_headers()?;
        let body = serde_json::json!({
            "model": model,
            "input": texts,
        });

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
            return Err(LmError::ProviderError {
                provider: self.provider_name.clone(),
                status: status.as_u16(),
                message: error_text,
            });
        }

        let parsed: OpenAiEmbeddingResponse = resp.json().await?;
        Ok(parsed.data.into_iter().map(|d| d.embedding).collect())
    }
}
