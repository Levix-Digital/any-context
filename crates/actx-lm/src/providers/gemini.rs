//! Google Gemini Developer REST API adapter with streaming SSE support

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::Deserialize;
use std::time::Duration;

use crate::error::LmError;
use crate::sse::decode_sse_stream;
use crate::traits::{BoxedChunkStream, LmProvider};
use crate::types::{
    ChatRequest, ChatResponse, FinishReason, Role, StreamChunk, TokenUsage,
};

/// Google Gemini REST API adapter
#[derive(Debug, Clone)]
pub struct GeminiProvider {
    api_key: String,
    base_url: String,
    http_client: reqwest::Client,
}

impl GeminiProvider {
    /// Create a new Gemini provider with Google AI Studio API key
    pub fn new(api_key: impl Into<String>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_default();

        Self {
            api_key: api_key.into(),
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            http_client,
        }
    }

    /// Override the base URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn serialize_request(&self, request: &ChatRequest) -> serde_json::Value {
        let mut contents = Vec::new();
        let mut system_text = String::new();

        for msg in &request.messages {
            match msg.role {
                Role::System => {
                    if !system_text.is_empty() {
                        system_text.push_str("\n\n");
                    }
                    system_text.push_str(&msg.content);
                }
                Role::User => {
                    contents.push(serde_json::json!({
                        "role": "user",
                        "parts": [{ "text": msg.content }]
                    }));
                }
                Role::Assistant => {
                    contents.push(serde_json::json!({
                        "role": "model",
                        "parts": [{ "text": msg.content }]
                    }));
                }
                Role::Tool => {
                    contents.push(serde_json::json!({
                        "role": "user",
                        "parts": [{ "text": format!("Tool Output: {}", msg.content) }]
                    }));
                }
            }
        }

        let mut body = serde_json::json!({
            "contents": contents,
        });

        if !system_text.is_empty() {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{ "text": system_text }]
            });
        }

        let mut gen_config = serde_json::Map::new();
        if let Some(t) = request.params.temperature {
            gen_config.insert("temperature".to_string(), serde_json::json!(t));
        }
        if let Some(p) = request.params.top_p {
            gen_config.insert("topP".to_string(), serde_json::json!(p));
        }
        if let Some(m) = request.params.max_tokens {
            gen_config.insert("maxOutputTokens".to_string(), serde_json::json!(m));
        }
        if let Some(ref s) = request.params.stop {
            gen_config.insert("stopSequences".to_string(), serde_json::json!(s));
        }

        if !gen_config.is_empty() {
            body["generationConfig"] = serde_json::Value::Object(gen_config);
        }

        body
    }
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsage>,
    error: Option<GeminiError>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContent>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct GeminiContent {
    parts: Option<Vec<GeminiPart>>,
}

#[derive(Deserialize)]
struct GeminiPart {
    text: Option<String>,
}

#[derive(Deserialize)]
struct GeminiUsage {
    #[serde(rename = "promptTokenCount")]
    prompt_tokens: Option<u32>,
    #[serde(rename = "candidatesTokenCount")]
    completion_tokens: Option<u32>,
    #[serde(rename = "totalTokenCount")]
    total_tokens: Option<u32>,
}

#[derive(Deserialize)]
struct GeminiError {
    message: String,
}

#[derive(Deserialize)]
struct GeminiBatchEmbedResponse {
    embeddings: Vec<GeminiEmbeddingContent>,
}

#[derive(Deserialize)]
struct GeminiEmbeddingContent {
    values: Vec<f32>,
}

#[async_trait]
impl LmProvider for GeminiProvider {
    fn provider_id(&self) -> &'static str {
        "gemini"
    }

    async fn chat_complete(&self, request: ChatRequest) -> Result<ChatResponse, LmError> {
        let model = if request.model.starts_with("models/") {
            request.model.clone()
        } else {
            format!("models/{}", request.model)
        };

        let url = format!("{}/{}:generateContent?key={}", self.base_url, model, self.api_key);
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let body = self.serialize_request(&request);

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
                    provider: "gemini".to_string(),
                    message: error_text,
                });
            } else if status.as_u16() == 429 {
                return Err(LmError::RateLimit {
                    provider: "gemini".to_string(),
                    message: error_text,
                    retry_after_secs: None,
                });
            } else {
                return Err(LmError::ProviderError {
                    provider: "gemini".to_string(),
                    status: status.as_u16(),
                    message: error_text,
                });
            }
        }

        let parsed: GeminiResponse = resp.json().await?;
        if let Some(err) = parsed.error {
            return Err(LmError::ProviderError {
                provider: "gemini".to_string(),
                status: status.as_u16(),
                message: err.message,
            });
        }

        let mut content = String::new();
        let mut finish_reason = None;

        if let Some(candidates) = parsed.candidates {
            if let Some(first) = candidates.into_iter().next() {
                if let Some(c) = first.content {
                    if let Some(parts) = c.parts {
                        for p in parts {
                            if let Some(t) = p.text {
                                content.push_str(&t);
                            }
                        }
                    }
                }
                finish_reason = match first.finish_reason.as_deref() {
                    Some("STOP") => Some(FinishReason::Stop),
                    Some("MAX_TOKENS") => Some(FinishReason::Length),
                    Some("SAFETY") => Some(FinishReason::ContentFilter),
                    Some(other) => Some(FinishReason::Other(other.to_string())),
                    None => None,
                };
            }
        }

        let usage = parsed.usage_metadata.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens.unwrap_or(0),
            completion_tokens: u.completion_tokens.unwrap_or(0),
            total_tokens: u.total_tokens.unwrap_or(0),
        });

        Ok(ChatResponse {
            id: format!("gemini-{}", uuid_timestamp()),
            model: request.model,
            content,
            thinking: None,
            tool_calls: Vec::new(),
            finish_reason,
            usage,
        })
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<BoxedChunkStream, LmError> {
        let model = if request.model.starts_with("models/") {
            request.model.clone()
        } else {
            format!("models/{}", request.model)
        };

        let url = format!(
            "{}/{}:streamGenerateContent?alt=sse&key={}",
            self.base_url, model, self.api_key
        );

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let body = self.serialize_request(&request);

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
                    provider: "gemini".to_string(),
                    message: error_text,
                });
            } else if status.as_u16() == 429 {
                return Err(LmError::RateLimit {
                    provider: "gemini".to_string(),
                    message: error_text,
                    retry_after_secs: None,
                });
            } else {
                return Err(LmError::ProviderError {
                    provider: "gemini".to_string(),
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
                    match serde_json::from_str::<GeminiResponse>(&event.data) {
                        Ok(chunk) => {
                            if let Some(candidates) = chunk.candidates {
                                if let Some(first) = candidates.into_iter().next() {
                                    if let Some(c) = first.content {
                                        if let Some(parts) = c.parts {
                                            for p in parts {
                                                if let Some(text) = p.text {
                                                    if !text.is_empty() {
                                                        return Some(Ok(StreamChunk::Token(text)));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if let Some(fr) = first.finish_reason {
                                        let finish = match fr.as_str() {
                                            "STOP" => FinishReason::Stop,
                                            "MAX_TOKENS" => FinishReason::Length,
                                            "SAFETY" => FinishReason::ContentFilter,
                                            other => FinishReason::Other(other.to_string()),
                                        };
                                        return Some(Ok(StreamChunk::Completed {
                                            finish_reason: Some(finish),
                                            usage: None,
                                        }));
                                    }
                                }
                            }

                            if let Some(u) = chunk.usage_metadata {
                                return Some(Ok(StreamChunk::Completed {
                                    finish_reason: None,
                                    usage: Some(TokenUsage {
                                        prompt_tokens: u.prompt_tokens.unwrap_or(0),
                                        completion_tokens: u.completion_tokens.unwrap_or(0),
                                        total_tokens: u.total_tokens.unwrap_or(0),
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

    async fn embed(&self, model: &str, texts: &[String]) -> Result<Vec<Vec<f32>>, LmError> {
        let model_id = if model.starts_with("models/") {
            model.to_string()
        } else {
            format!("models/{}", model)
        };

        let url = format!(
            "{}/{}:batchEmbedContents?key={}",
            self.base_url, model_id, self.api_key
        );

        let requests: Vec<serde_json::Value> = texts
            .iter()
            .map(|t| {
                serde_json::json!({
                    "model": model_id,
                    "content": {
                        "parts": [{ "text": t }]
                    }
                })
            })
            .collect();

        let body = serde_json::json!({ "requests": requests });

        let resp = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            return Err(LmError::ProviderError {
                provider: "gemini".to_string(),
                status: status.as_u16(),
                message: error_text,
            });
        }

        let parsed: GeminiBatchEmbedResponse = resp.json().await?;
        Ok(parsed.embeddings.into_iter().map(|e| e.values).collect())
    }
}

fn uuid_timestamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
