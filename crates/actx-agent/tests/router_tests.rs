//! Unit tests for ChatModelRouter and dynamic query complexity routing

use actx_agent::router::*;
use actx_agent::*;
use actx_lm::error::LmError;
use actx_lm::traits::{BoxedChunkStream, LmProvider};
use actx_lm::types::{ChatRequest, ChatResponse, FinishReason, StreamChunk, TokenUsage};
use async_trait::async_trait;
use std::time::Instant;

#[derive(Clone)]
struct SimpleMockProvider;

#[async_trait]
impl LmProvider for SimpleMockProvider {
    fn provider_id(&self) -> &'static str {
        "simple_mock"
    }

    async fn chat_complete(&self, _request: ChatRequest) -> Result<ChatResponse, LmError> {
        Ok(ChatResponse {
            id: "resp-mock".to_string(),
            model: "mock".to_string(),
            content: "Mock agent direct response".to_string(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: Some(TokenUsage::new(10, 10)),
        })
    }

    async fn chat_stream(&self, _request: ChatRequest) -> Result<BoxedChunkStream, LmError> {
        let resp = self.chat_complete(_request).await?;
        let chunks = vec![
            Ok(StreamChunk::Token(resp.content)),
            Ok(StreamChunk::Completed {
                finish_reason: resp.finish_reason,
                usage: resp.usage,
            }),
        ];
        Ok(Box::pin(futures::stream::iter(chunks)))
    }
}

#[test]
fn test_router_fast_queries_classification() {
    let router = ChatModelRouter::default();

    let fast_queries = vec![
        "onde fica o timeout da conexão?",
        "qual linha define a porta 8080?",
        "qual comando limpa o cache?",
        "qual a versão atual do actx?",
        "o que significa o status IDLE?",
        "qual o atalho para alternar de workspace?",
        "mostre a função calculate_sum",
    ];

    for q in fast_queries {
        let decision = router.evaluate(q, SearchMode::Auto);
        assert_eq!(
            decision.complexity,
            QueryComplexity::Fast,
            "Expected Fast complexity for query '{}', got decision: {:?}",
            q,
            decision
        );
        assert_eq!(
            decision.mode,
            AgentExecutionMode::ReAct,
            "Expected ReAct execution mode for fast query in Auto mode: '{}'",
            q
        );
        assert!(
            decision.confidence < 0.45,
            "Confidence score should be below threshold for fast query '{}', got: {}",
            q,
            decision.confidence
        );
    }
}

#[test]
fn test_router_deep_queries_classification() {
    let router = ChatModelRouter::default();

    let deep_queries = vec![
        "explique a arquitetura sistêmica de ponta a ponta do orquestrador de sincronização e seus trade-offs",
        "faça uma auditoria de segurança completa de todas as ocorrências de decodificação zlib e trate possíveis panics",
        "compare a abordagem de armazenamento com LanceDB versus Sqlite e aponte os prós e contras",
        "quais módulos precisam ser alterados para suportar o novo protocolo? e também como os testes e o CI são impactados?",
        "descreva o ciclo de vida completo de uma requisição desde a CLI até a síntese final com referências",
        "qual a melhor abordagem para refatoração do pipeline considerando impacto em performance e concorrência?",
    ];

    for q in deep_queries {
        let decision = router.evaluate(q, SearchMode::Auto);
        assert_eq!(
            decision.complexity,
            QueryComplexity::Deep,
            "Expected Deep complexity for query '{}', got decision: {:?}",
            q,
            decision
        );
        assert_eq!(
            decision.mode,
            AgentExecutionMode::DeepSearch,
            "Expected DeepSearch execution mode for complex query in Auto mode: '{}'",
            q
        );
        assert!(
            decision.confidence >= 0.45,
            "Confidence score should meet or exceed threshold for deep query '{}', got: {}",
            q,
            decision.confidence
        );
    }
}

#[test]
fn test_router_forced_modes() {
    let router = ChatModelRouter::default();

    // Fast query under forced Deep mode
    let fast_query = "what is the default port?";
    let decision_deep = router.evaluate(fast_query, SearchMode::Deep);
    assert_eq!(decision_deep.mode, AgentExecutionMode::DeepSearch);
    assert!(decision_deep.reason.contains("forced by user"));

    // Deep query under forced Fast mode
    let deep_query = "explain the system architecture end-to-end and its trade-offs";
    let decision_fast = router.evaluate(deep_query, SearchMode::Fast);
    assert_eq!(decision_fast.mode, AgentExecutionMode::ReAct);
    assert!(decision_fast.reason.contains("forced by user"));
}

#[test]
fn test_router_intent_detection() {
    let router = ChatModelRouter::default();

    // Code Intent
    let q_code = "como refatorar a função calculate_metrics com async fn e struct em Rust?";
    let d_code = router.evaluate(q_code, SearchMode::Auto);
    assert_eq!(d_code.intent, QueryIntent::Code);

    // Architecture Intent
    let q_arch = "desenhe o diagrama da arquitetura hexagonal e o fluxo de dados entre camadas";
    let d_arch = router.evaluate(q_arch, SearchMode::Auto);
    assert_eq!(d_arch.intent, QueryIntent::Architecture);

    // Document Intent
    let q_doc = "qual é a cláusula de confidencialidade do contrato e da fatura CMR?";
    let d_doc = router.evaluate(q_doc, SearchMode::Auto);
    assert_eq!(d_doc.intent, QueryIntent::Document);

    // General Intent
    let q_gen = "olá bom dia, como você está se sentindo hoje?";
    let d_gen = router.evaluate(q_gen, SearchMode::Auto);
    assert_eq!(d_gen.intent, QueryIntent::General);
}

#[test]
fn test_router_sub_microsecond_performance() {
    let router = ChatModelRouter::default();
    let samples = vec![
        "qual a porta padrão do servidor HTTP?",
        "explique a arquitetura sistêmica do orquestrador de sincronização e seus trade-offs",
        "onde fica o arquivo settings.db no Windows?",
        "compare a implementação com LanceDB versus Sqlite e aponte prós e contras",
        "como importar o trait LmProvider em crates/actx-lm?",
    ];

    let iterations = 5_000;
    let start = Instant::now();
    for i in 0..iterations {
        let q = samples[i % samples.len()];
        let _ = router.evaluate(q, SearchMode::Auto);
    }
    let elapsed = start.elapsed();
    let avg_micros = elapsed.as_micros() as f64 / iterations as f64;

    println!(
        "ChatModelRouter Performance: {} evaluations in {:?} (avg: {:.3} µs / evaluation)",
        iterations, elapsed, avg_micros
    );

    // SLA: Must be strictly under 50 microseconds per classification
    assert!(
        avg_micros < 50.0,
        "Classification took {:.3} µs, exceeding 50 µs SLA",
        avg_micros
    );
}

#[tokio::test]
async fn test_agent_stream_emits_routing_decision_event() {
    let provider = std::sync::Arc::new(SimpleMockProvider);

    let agent = Agent::builder()
        .client(provider)
        .model("test-model")
        .search_mode(SearchMode::Auto)
        .build_sync()
        .expect("build agent");

    let query = "explique a arquitetura sistêmica de ponta a ponta do orquestrador";
    let (mut rx, _handle) = agent.stream(query, Some("test_session".to_string()));

    // First event emitted must be RoutingDecision
    let first_event = rx.recv().await.expect("first event should exist");
    match first_event {
        AgentEvent::RoutingDecision {
            mode,
            complexity,
            intent,
            confidence,
            reason,
        } => {
            assert_eq!(complexity, "Deep");
            assert!(mode.contains("DeepSearch"));
            assert_eq!(intent, "architecture");
            assert!(confidence >= 0.45);
            assert!(!reason.is_empty());
        }
        other => panic!("Expected AgentEvent::RoutingDecision as first event, got {:?}", other),
    }
}
