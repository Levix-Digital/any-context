//! Routing tests (IMP-038): the route comes from the search mode, never from the question's words.

use actx_agent::router::*;
use actx_agent::*;
use actx_lm::error::LmError;
use actx_lm::traits::{BoxedChunkStream, LmProvider};
use actx_lm::types::{ChatRequest, ChatResponse, FinishReason, StreamChunk, TokenUsage};
use async_trait::async_trait;

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

#[tokio::test]
async fn test_agent_stream_emits_the_route_first() {
    let cases = [
        (SearchMode::Auto, "explique a arquitetura sistêmica de ponta a ponta do orquestrador", "fast", false),
        (SearchMode::Fast, "explique a arquitetura sistêmica de ponta a ponta do orquestrador", "fast", true),
        (SearchMode::Deep, "qual porta?", "deep", true),
    ];
    for (mode, query, expected_route, expected_pinned) in cases {
        let agent = Agent::builder()
            .client(std::sync::Arc::new(SimpleMockProvider))
            .model("test-model")
            .search_mode(mode)
            .build_sync()
            .expect("build agent");
        let (mut rx, _handle) = agent.stream(query, Some("test_session".to_string()));
        match rx.recv().await.expect("first event should exist") {
            AgentEvent::RoutingDecision { route, pinned, reason } => {
                assert_eq!(route, expected_route, "{mode:?}");
                assert_eq!(pinned, expected_pinned, "{mode:?}");
                assert!(!reason.is_empty());
            }
            other => panic!("Expected AgentEvent::RoutingDecision as first event, got {:?}", other),
        }
    }
}

#[test]
fn test_router_detects_status_questions_in_every_search_mode() {
    let status_queries = [
        "is everything indexed yet?",
        "what is the sync status?",
        "já está tudo indexado?",
        "como está o progresso da sincronização?",
        "quantos chunks tem o workspace?",
    ];
    for q in status_queries {
        for mode in [SearchMode::Auto, SearchMode::Fast, SearchMode::Deep] {
            let decision = route(q, mode);
            assert_eq!(decision.route, Route::Status, "query '{q}' in {mode:?}");
            assert_eq!(decision.reason, "live status question");
        }
    }
}

#[test]
fn test_router_keeps_how_to_questions_out_of_the_status_route() {
    for q in ["how do I sync a folder?", "como adicionar um site ao workspace?", "how do I add a website?"] {
        assert_eq!(route(q, SearchMode::Auto).route, Route::Fast, "query '{q}'");
    }
}

#[test]
fn test_questions_the_old_classifier_sent_to_deep_start_fast_in_automatic_mode() {
    for q in [
        "Compare the pros and cons of the two architectures, and also explain the end-to-end lifecycle?",
        "faça uma auditoria de segurança completa de todos os módulos e quais arquivos se relacionam",
        "Explain step by step the whole flow from the request to the response, including every module involved, the data stored at each step, the errors that can happen and how they are reported back to the user interface",
    ] {
        assert_eq!(route(q, SearchMode::Auto), RouteDecision { route: Route::Fast, pinned: false, reason: mode_route(SearchMode::Auto).reason });
    }
}
