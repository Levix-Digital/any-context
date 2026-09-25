//! Integration tests validating the actx-lm public crate API

use actx_lm::{
    ChatMessage, ChatRequest, FinishReason, LmClient, MockLmProvider, ProviderKind, Role,
    StreamChunk, TokenUsage, ToolCall, ToolDefinition,
};
use futures::StreamExt;
use std::sync::Arc;

#[tokio::test]
async fn test_full_mock_generation_workflow() {
    let mock = MockLmProvider::new()
        .with_canned_thinking("Thinking about the query...")
        .with_canned_response("The answer is 42.");

    let client = LmClient::builder()
        .custom_provider(Arc::new(mock))
        .default_model("mock-model")
        .build()
        .expect("Client should build cleanly");

    // 1. Chat complete
    let req = ChatRequest::new("mock-model", vec![ChatMessage::user("What is the meaning?")]);
    let resp = client.chat_complete(req).await.unwrap();

    assert_eq!(resp.content, "The answer is 42.");
    assert_eq!(resp.thinking, Some("Thinking about the query...".to_string()));
    assert_eq!(resp.finish_reason, Some(FinishReason::Stop));
    assert!(resp.usage.unwrap().total_tokens > 0);

    // 2. Chat stream
    let req_stream = ChatRequest::new("mock-model", vec![ChatMessage::user("What is the meaning?")]);
    let mut stream = client.chat_stream(req_stream).await.unwrap();

    let mut thinking_tokens = Vec::new();
    let mut content_tokens = Vec::new();
    let mut finished = false;

    while let Some(chunk_res) = stream.next().await {
        let chunk = chunk_res.unwrap();
        match chunk {
            StreamChunk::Reasoning(t) => thinking_tokens.push(t),
            StreamChunk::Token(t) => content_tokens.push(t),
            StreamChunk::Completed { finish_reason, usage } => {
                assert_eq!(finish_reason, Some(FinishReason::Stop));
                assert!(usage.is_some());
                finished = true;
            }
            StreamChunk::ToolCallDelta { .. } => {}
        }
    }

    assert!(finished);
    assert_eq!(thinking_tokens.concat(), "Thinking about the query...");
    assert_eq!(content_tokens.concat(), "The answer is 42.");
}

#[tokio::test]
async fn test_provider_kind_instantiations() {
    // Ollama local without API key
    let ollama_client = LmClient::builder()
        .provider_kind(ProviderKind::Ollama {
            base_url: Some("http://127.0.0.1:11434/v1".to_string()),
        })
        .build()
        .unwrap();
    assert_eq!(ollama_client.provider_id(), "ollama");

    // OpenAI with API key
    let openai_client = LmClient::builder()
        .provider_kind(ProviderKind::OpenAi)
        .api_key("sk-test-key-12345")
        .build()
        .unwrap();
    assert_eq!(openai_client.provider_id(), "openai");

    // Anthropic with API key
    let anthropic_client = LmClient::builder()
        .provider_kind(ProviderKind::Anthropic)
        .api_key("sk-ant-test-key-12345")
        .build()
        .unwrap();
    assert_eq!(anthropic_client.provider_id(), "anthropic");

    // Gemini with API key
    let gemini_client = LmClient::builder()
        .provider_kind(ProviderKind::Gemini)
        .api_key("AIzaSy-test-key-12345")
        .build()
        .unwrap();
    assert_eq!(gemini_client.provider_id(), "gemini");

    // Groq with API key
    let groq_client = LmClient::builder()
        .provider_kind(ProviderKind::Groq)
        .api_key("gsk_test123")
        .build()
        .unwrap();
    assert_eq!(groq_client.provider_id(), "groq");

    // DeepSeek with API key
    let deepseek_client = LmClient::builder()
        .provider_kind(ProviderKind::DeepSeek)
        .api_key("sk-ds-test123")
        .build()
        .unwrap();
    assert_eq!(deepseek_client.provider_id(), "deepseek");

    // OpenRouter with API key
    let openrouter_client = LmClient::builder()
        .provider_kind(ProviderKind::OpenRouter)
        .api_key("sk-or-test123")
        .build()
        .unwrap();
    assert_eq!(openrouter_client.provider_id(), "openrouter");

    // Custom compatible (e.g. LM Studio)
    let custom_client = LmClient::builder()
        .provider_kind(ProviderKind::CustomCompatible {
            name: "lmstudio".to_string(),
            base_url: "http://localhost:1234/v1".to_string(),
        })
        .build()
        .unwrap();
    assert_eq!(custom_client.provider_id(), "openai-compatible");
}

#[test]
fn test_types_and_serialization() {
    let msg_sys = ChatMessage::system("System prompt");
    assert_eq!(msg_sys.role, Role::System);

    let msg_tool = ChatMessage::tool("{\"status\":\"ok\"}", "call_123");
    assert_eq!(msg_tool.role, Role::Tool);
    assert_eq!(msg_tool.tool_call_id, Some("call_123".to_string()));

    let tool_def = ToolDefinition::new(
        "calculator",
        "Calculates math expressions",
        serde_json::json!({
            "type": "object",
            "properties": {
                "expr": { "type": "string" }
            },
            "required": ["expr"]
        }),
    );
    assert_eq!(tool_def.name, "calculator");

    let tool_call = ToolCall::new("call_999", "calculator", "{\"expr\":\"2+2\"}");
    assert_eq!(tool_call.id, "call_999");
    assert_eq!(tool_call.arguments, "{\"expr\":\"2+2\"}");

    let mut req = ChatRequest::new("gpt-4o", vec![msg_sys, ChatMessage::user("Calculate 2+2")])
        .with_temperature(0.2)
        .with_max_tokens(1024)
        .with_tools(vec![tool_def]);

    req.params.stop = Some(vec!["STOP".to_string()]);

    assert_eq!(req.params.temperature, Some(0.2));
    assert_eq!(req.params.max_tokens, Some(1024));
    assert_eq!(req.tools.len(), 1);

    let usage = TokenUsage::new(50, 100);
    assert_eq!(usage.total_tokens, 150);
}
