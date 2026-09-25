//! Comprehensive unit tests for actx-agent

use actx_agent::*;
use actx_lm::error::LmError;
use actx_lm::traits::{BoxedChunkStream, LmProvider};
use actx_lm::types::{
    ChatRequest, ChatResponse, FinishReason, Role, StreamChunk, TokenUsage, ToolCall,
};
use async_trait::async_trait;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Dynamic test mock provider that responds sequentially according to programmed turns
#[derive(Clone)]
struct TestMockProvider {
    turn_counter: Arc<AtomicUsize>,
    responses: Arc<tokio::sync::Mutex<Vec<ChatResponse>>>,
}

impl TestMockProvider {
    fn new(responses: Vec<ChatResponse>) -> Self {
        Self {
            turn_counter: Arc::new(AtomicUsize::new(0)),
            responses: Arc::new(tokio::sync::Mutex::new(responses)),
        }
    }
}

#[async_trait]
impl LmProvider for TestMockProvider {
    fn provider_id(&self) -> &'static str {
        "test_mock"
    }

    async fn chat_complete(&self, _request: ChatRequest) -> Result<ChatResponse, LmError> {
        let idx = self.turn_counter.fetch_add(1, Ordering::SeqCst);
        let list = self.responses.lock().await;
        if idx < list.len() {
            Ok(list[idx].clone())
        } else {
            Ok(ChatResponse {
                id: format!("resp-{}", idx),
                model: "test-model".to_string(),
                content: format!("Fallback turn {}", idx),
                thinking: None,
                tool_calls: vec![],
                finish_reason: Some(FinishReason::Stop),
                usage: Some(TokenUsage::new(10, 10)),
            })
        }
    }

    async fn chat_stream(&self, _request: ChatRequest) -> Result<BoxedChunkStream, LmError> {
        let resp = self.chat_complete(_request).await?;
        let s = futures::stream::iter(vec![
            Ok(StreamChunk::Token(resp.content)),
            Ok(StreamChunk::Completed {
                finish_reason: Some(FinishReason::Stop),
                usage: resp.usage,
            }),
        ]);
        Ok(Box::pin(s))
    }
}

#[tokio::test]
async fn test_direct_response_without_tools() {
    let mock = Arc::new(TestMockProvider::new(vec![ChatResponse {
        id: "resp-1".to_string(),
        model: "mock".to_string(),
        content: "Olá! Como posso ajudar você hoje?".to_string(),
        thinking: None,
        tool_calls: vec![],
        finish_reason: Some(FinishReason::Stop),
        usage: Some(TokenUsage::new(15, 20)),
    }]));

    let agent = Agent::builder()
        .client(mock)
        .model("mock-model")
        .max_turns(5)
        .build()
        .await
        .unwrap();

    let resp = agent.run("Oi", None).await.unwrap();
    assert_eq!(resp.content, "Olá! Como posso ajudar você hoje?");
    assert_eq!(resp.total_turns, 1);
    assert_eq!(resp.tool_calls_count, 0);
}

#[tokio::test]
async fn test_single_tool_execution_react_loop() {
    // Turn 1: Model requests tool call "search_db"
    // Turn 2: Model receives tool observation and outputs final synthesis
    let mock = Arc::new(TestMockProvider::new(vec![
        ChatResponse {
            id: "call-1".to_string(),
            model: "mock".to_string(),
            content: "Vou buscar no banco de dados.".to_string(),
            thinking: Some("O usuário pediu detalhes da autenticação. Devo buscar 'jwt auth'.".to_string()),
            tool_calls: vec![ToolCall::new("tc_1", "search_db", r#"{"query":"jwt auth"}"#)],
            finish_reason: Some(FinishReason::ToolCalls),
            usage: Some(TokenUsage::new(20, 10)),
        },
        ChatResponse {
            id: "resp-2".to_string(),
            model: "mock".to_string(),
            content: "O sistema valida JWT no middleware src/auth/jwt.rs e expira em 15 minutos.".to_string(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: Some(TokenUsage::new(40, 25)),
        },
    ]));

    // Register native search_db tool
    let search_tool = Arc::new(NativeTool::new(
        "search_db",
        "Busca nos arquivos do workspace",
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" }
            },
            "required": ["query"]
        }),
        |args| async move {
            let q = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            Ok(format!("Documentos encontrados para '{}': src/auth/jwt.rs (expiração: 15min)", q))
        },
    ));

    let agent = Agent::builder()
        .client(mock)
        .model("mock")
        .tool(search_tool)
        .max_turns(5)
        .build()
        .await
        .unwrap();

    let resp = agent.run("Como funciona a expiração de JWT?", None).await.unwrap();
    assert!(resp.content.contains("src/auth/jwt.rs"));
    assert_eq!(resp.total_turns, 2);
    assert_eq!(resp.tool_calls_count, 1);
}

#[tokio::test]
async fn test_tool_defensive_self_healing_on_error() {
    // Turn 1: Model makes tool call with invalid query
    // Turn 2: Model receives error observation and gives graceful reply
    let mock = Arc::new(TestMockProvider::new(vec![
        ChatResponse {
            id: "call-1".to_string(),
            model: "mock".to_string(),
            content: "".to_string(),
            thinking: None,
            tool_calls: vec![ToolCall::new("tc_fail", "flaky_tool", r#"{"param":"fail"}"#)],
            finish_reason: Some(FinishReason::ToolCalls),
            usage: None,
        },
        ChatResponse {
            id: "resp-2".to_string(),
            model: "mock".to_string(),
            content: "A ferramenta reportou indisponibilidade temporária. Posso tentar outra abordagem.".to_string(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: None,
        },
    ]));

    let flaky_tool = Arc::new(NativeTool::new(
        "flaky_tool",
        "Ferramenta instável para teste",
        json!({}),
        |_| async move { Err(ToolError::new("Network connection refused")) },
    ));

    let agent = Agent::builder()
        .client(mock)
        .model("mock")
        .tool(flaky_tool)
        .max_turns(5)
        .build()
        .await
        .unwrap();

    let resp = agent.run("Executar flaky", None).await.unwrap();
    assert!(resp.content.contains("indisponibilidade temporária"));
    assert_eq!(resp.total_turns, 2);
    assert_eq!(resp.tool_calls_count, 1);
}

#[tokio::test]
async fn test_circuit_breaker_max_turns_termination() {
    // Model keeps endlessly generating tool calls
    let loop_response = ChatResponse {
        id: "call-loop".to_string(),
        model: "mock".to_string(),
        content: "".to_string(),
        thinking: None,
        tool_calls: vec![ToolCall::new("tc_loop", "dummy", "{}")],
        finish_reason: Some(FinishReason::ToolCalls),
        usage: None,
    };

    let final_forced_response = ChatResponse {
        id: "forced-final".to_string(),
        model: "mock".to_string(),
        content: "Síntese final forçada pelo corte de segurança.".to_string(),
        thinking: None,
        tool_calls: vec![],
        finish_reason: Some(FinishReason::Stop),
        usage: None,
    };

    let mock = Arc::new(TestMockProvider::new(vec![
        loop_response.clone(),
        loop_response.clone(),
        final_forced_response,
    ]));

    let dummy_tool = Arc::new(NativeTool::new("dummy", "dummy", json!({}), |_| async move {
        Ok("ok".to_string())
    }));

    let agent = Agent::builder()
        .client(mock)
        .model("mock")
        .tool(dummy_tool)
        .max_turns(3) // Strict 3 turns cap!
        .build()
        .await
        .unwrap();

    let resp = agent.run("Loop infinito", None).await.unwrap();
    assert_eq!(resp.total_turns, 3);
    assert_eq!(resp.content, "Síntese final forçada pelo corte de segurança.");
}

#[tokio::test]
async fn test_in_memory_session_store_accumulation() {
    let mock = Arc::new(TestMockProvider::new(vec![
        ChatResponse {
            id: "1".to_string(),
            model: "mock".to_string(),
            content: "Meu nome é AnyContext.".to_string(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: None,
        },
        ChatResponse {
            id: "2".to_string(),
            model: "mock".to_string(),
            content: "Você se chama Guilherme.".to_string(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: None,
        },
    ]));

    let store = Arc::new(InMemorySessionStore::new(10));
    let agent = Agent::builder()
        .client(mock)
        .model("mock")
        .session_store(store.clone())
        .build()
        .await
        .unwrap();

    let sid = "session-123";
    agent.run("Quem é você?", Some(sid)).await.unwrap();

    let messages = store.get_messages(sid).await.unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, Role::User);
    assert_eq!(messages[1].role, Role::Assistant);

    agent.run("Quem sou eu?", Some(sid)).await.unwrap();
    let messages2 = store.get_messages(sid).await.unwrap();
    assert_eq!(messages2.len(), 4);
}

#[tokio::test]
async fn test_sqlite_session_store_persistence() {
    let temp_db = std::env::temp_dir().join(format!("actx_test_{}.db", uuid::Uuid::new_v4()));
    let store = Arc::new(SqliteSessionStore::open(&temp_db, 20).unwrap());

    let sid = "sess-sqlite-1";
    let msg1 = actx_lm::types::ChatMessage::user("Primeira mensagem");
    let msg2 = actx_lm::types::ChatMessage::assistant("Resposta 1");

    store.append_messages(sid, &[msg1, msg2]).await.unwrap();

    let loaded = store.get_messages(sid).await.unwrap();
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].content, "Primeira mensagem");
    assert_eq!(loaded[1].content, "Resposta 1");

    store.clear_session(sid).await.unwrap();
    let empty = store.get_messages(sid).await.unwrap();
    assert!(empty.is_empty());

    let _ = std::fs::remove_file(temp_db);
}

#[tokio::test]
async fn test_event_streaming_pipeline() {
    let mock = Arc::new(TestMockProvider::new(vec![
        ChatResponse {
            id: "call-1".to_string(),
            model: "mock".to_string(),
            content: "".to_string(),
            thinking: Some("Raciocínio preliminar...".to_string()),
            tool_calls: vec![ToolCall::new("tc_stream", "search_db", r#"{"q":"teste"}"#)],
            finish_reason: Some(FinishReason::ToolCalls),
            usage: None,
        },
        ChatResponse {
            id: "resp-2".to_string(),
            model: "mock".to_string(),
            content: "Resposta final transmitida.".to_string(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: Some(TokenUsage::new(10, 15)),
        },
    ]));

    let tool = Arc::new(NativeTool::new("search_db", "search", json!({}), |_| async move {
        Ok("resultado".to_string())
    }));

    let agent = Agent::builder()
        .client(mock)
        .model("mock")
        .tool(tool)
        .build()
        .await
        .unwrap();

    let (mut rx, handle) = agent.stream("Buscar", None);

    let mut event_types = Vec::new();
    while let Some(evt) = rx.recv().await {
        match evt {
            AgentEvent::Thinking(_) => event_types.push("thinking"),
            AgentEvent::ToolStart { .. } => event_types.push("tool_start"),
            AgentEvent::ToolEnd { .. } => event_types.push("tool_end"),
            AgentEvent::Delta(_) => event_types.push("delta"),
            AgentEvent::Done { .. } => event_types.push("done"),
            _ => {}
        }
    }

    let final_res = handle.await.unwrap().unwrap();
    assert_eq!(final_res.content, "Resposta final transmitida.");
    assert!(event_types.contains(&"thinking"));
    assert!(event_types.contains(&"tool_start"));
    assert!(event_types.contains(&"tool_end"));
    assert!(event_types.contains(&"delta"));
    assert!(event_types.contains(&"done"));
}
