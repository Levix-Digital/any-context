//! Comprehensive unit tests for RFC-042 Deep Search Engine in actx-agent

use actx_agent::deep_search::*;
use actx_agent::*;
use actx_lm::error::LmError;
use actx_lm::traits::{BoxedChunkStream, LmProvider};
use actx_lm::types::{
    ChatRequest, ChatResponse, FinishReason, StreamChunk, TokenUsage,
};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Dynamic test mock provider that responds sequentially according to programmed turns
#[derive(Clone)]
struct MockDeepSearchProvider {
    turn_counter: Arc<AtomicUsize>,
    responses: Arc<tokio::sync::Mutex<Vec<ChatResponse>>>,
}

impl MockDeepSearchProvider {
    fn new(responses: Vec<ChatResponse>) -> Self {
        Self {
            turn_counter: Arc::new(AtomicUsize::new(0)),
            responses: Arc::new(tokio::sync::Mutex::new(responses)),
        }
    }
}

#[async_trait]
impl LmProvider for MockDeepSearchProvider {
    fn provider_id(&self) -> &'static str {
        "mock_deep_search"
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

    async fn chat_stream(&self, request: ChatRequest) -> Result<BoxedChunkStream, LmError> {
        let resp = self.chat_complete(request).await?;
        let mut chunks = Vec::new();

        if let Some(ref think) = resp.thinking {
            chunks.push(Ok(StreamChunk::Reasoning(think.clone())));
        }

        for word in resp.content.split_inclusive(' ') {
            chunks.push(Ok(StreamChunk::Token(word.to_string())));
        }

        chunks.push(Ok(StreamChunk::Completed {
            finish_reason: resp.finish_reason,
            usage: resp.usage,
        }));

        Ok(Box::pin(futures::stream::iter(chunks)))
    }
}

/// In-memory mock retriever returning canned chunks per query
#[derive(Clone, Default)]
struct MockRetriever {
    canned_chunks: Arc<tokio::sync::Mutex<Vec<RetrievedChunk>>>,
    recorded_queries: Arc<tokio::sync::Mutex<Vec<String>>>,
}

impl MockRetriever {
    fn with_chunks(chunks: Vec<RetrievedChunk>) -> Self {
        Self {
            canned_chunks: Arc::new(tokio::sync::Mutex::new(chunks)),
            recorded_queries: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl DeepSearchRetriever for MockRetriever {
    async fn retrieve_batch(&self, queries: &[String]) -> Result<Vec<RetrievedChunk>, AgentError> {
        let mut rec = self.recorded_queries.lock().await;
        for q in queries {
            rec.push(q.clone());
        }
        let chunks = self.canned_chunks.lock().await;
        Ok(chunks.clone())
    }
}

#[test]
fn test_parse_string_list_variants() {
    // 1. Clean JSON array
    let res1 = parse_string_list(r#"["What is BM25?", "How does LanceDB work?"]"#, "fallback");
    assert_eq!(res1, vec!["What is BM25?", "How does LanceDB work?"]);

    // 2. Markdown fenced JSON block
    let res2 = parse_string_list(
        "```json\n[\"Architecture overview\", \"Security protocols\"]\n```",
        "fallback",
    );
    assert_eq!(res2, vec!["Architecture overview", "Security protocols"]);

    // 3. Output prefixed with <think> reasoning tags
    let res3 = parse_string_list(
        "<think>Let me break this down into components.</think>\n[\"Subquery 1\", \"Subquery 2\"]",
        "fallback",
    );
    assert_eq!(res3, vec!["Subquery 1", "Subquery 2"]);

    // 4. Bulleted list fallback
    let res4 = parse_string_list(
        "1. First sub-query\n2. Second sub-query\n3. Third sub-query",
        "fallback",
    );
    assert_eq!(res4, vec!["First sub-query", "Second sub-query", "Third sub-query"]);

    // 5. Complete gibberish fallback
    let res5 = parse_string_list("Unparseable output with no brackets or numbers", "Original question");
    assert_eq!(res5, vec!["Original question"]);
}

#[test]
fn test_parse_rerank_decision() {
    assert!(parse_rerank_decision("YES"));
    assert!(parse_rerank_decision("  yes  "));
    assert!(parse_rerank_decision("<think>This chunk directly explains the algorithm</think> YES"));
    assert!(!parse_rerank_decision("NO"));
    assert!(!parse_rerank_decision("NO, this is irrelevant"));
    assert!(!parse_rerank_decision("YES and NO"));
}

#[tokio::test]
async fn test_deep_search_single_iteration_sufficient() {
    let mock_responses = vec![
        // 1. Decomposition response
        ChatResponse {
            id: "decomp".into(),
            model: "mock".into(),
            content: r#"["Subquery A: Ingestion", "Subquery B: Storage"]"#.into(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: None,
        },
        // 2. Reflection response (evidence is sufficient -> empty array)
        ChatResponse {
            id: "reflect".into(),
            model: "mock".into(),
            content: "[]".into(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: None,
        },
        // 3. Synthesis response
        ChatResponse {
            id: "synth".into(),
            model: "mock".into(),
            content: "AnyContext uses Tree-sitter for ingestion [src/router.rs:10-25] and LanceDB for storage [src/storage.rs:50-80].".into(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: None,
        },
    ];

    let client = Arc::new(MockDeepSearchProvider::new(mock_responses));
    let canned_chunks = vec![
        RetrievedChunk {
            id: "c1".into(),
            file_name: "router.rs".into(),
            file_path: "src/router.rs".into(),
            header_path: Some("fn parse".into()),
            start_line: Some(10),
            end_line: Some(25),
            text: "Tree-sitter parser implementation".into(),
            score: 0.95,
            matched_queries: vec!["Subquery A: Ingestion".into()],
        },
        RetrievedChunk {
            id: "c2".into(),
            file_name: "storage.rs".into(),
            file_path: "src/storage.rs".into(),
            header_path: Some("struct LanceStore".into()),
            start_line: Some(50),
            end_line: Some(80),
            text: "LanceDB native vector table".into(),
            score: 0.90,
            matched_queries: vec!["Subquery B: Storage".into()],
        },
    ];
    let retriever = Arc::new(MockRetriever::with_chunks(canned_chunks));

    let orchestrator = DeepSearchOrchestrator::new(
        client,
        retriever.clone(),
        DeepSearchConfig::default(),
        AgentConfig::default(),
        None,
    );

    let (tx, mut rx) = event_channel();
    let response = orchestrator
        .run("How does AnyContext ingest and store documents?", None, Some(tx))
        .await
        .expect("Deep Search failed");

    assert!(response.content.contains("Tree-sitter"));
    assert!(response.content.contains("[src/router.rs:10-25]"));
    assert_eq!(response.total_turns, 1);
    assert_eq!(response.tool_calls_count, 2);

    // Verify stream events
    let mut received_events = Vec::new();
    while let Ok(evt) = rx.try_recv() {
        received_events.push(evt);
    }

    assert!(received_events.iter().any(|e| matches!(e, AgentEvent::Decomposition { .. })));
    assert!(received_events.iter().any(|e| matches!(e, AgentEvent::IterationStart { iteration: 1, .. })));
    assert!(received_events.iter().any(|e| matches!(e, AgentEvent::GapAnalysis { is_sufficient: true, .. })));
    assert!(received_events.iter().any(|e| matches!(e, AgentEvent::Delta(_))));
    assert!(received_events.iter().any(|e| matches!(e, AgentEvent::Done { .. })));
}

#[tokio::test]
async fn test_deep_search_multi_iteration_gap_queries() {
    let mock_responses = vec![
        // 1. Decomposition response
        ChatResponse {
            id: "decomp".into(),
            model: "mock".into(),
            content: r#"["Initial overview"]"#.into(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: None,
        },
        // 2. Reflection 1: Not sufficient, gap query generated
        ChatResponse {
            id: "reflect_1".into(),
            model: "mock".into(),
            content: r#"["Deep dive into BM25 scoring details"]"#.into(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: None,
        },
        // 3. Reflection 2: Now sufficient!
        ChatResponse {
            id: "reflect_2".into(),
            model: "mock".into(),
            content: "[]".into(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: None,
        },
        // 4. Synthesis
        ChatResponse {
            id: "synth".into(),
            model: "mock".into(),
            content: "BM25 scoring combines term frequency and inverse document frequency [src/bm25.rs:1-50].".into(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: None,
        },
    ];

    let client = Arc::new(MockDeepSearchProvider::new(mock_responses));
    let canned_chunks = vec![
        RetrievedChunk {
            id: "c1".into(),
            file_name: "bm25.rs".into(),
            file_path: "src/bm25.rs".into(),
            header_path: None,
            start_line: Some(1),
            end_line: Some(50),
            text: "BM25 Okapi implementation".into(),
            score: 0.88,
            matched_queries: vec![],
        },
    ];
    let retriever = Arc::new(MockRetriever::with_chunks(canned_chunks));

    let orchestrator = DeepSearchOrchestrator::new(
        client,
        retriever.clone(),
        DeepSearchConfig {
            max_iterations: 3,
            ..Default::default()
        },
        AgentConfig::default(),
        None,
    );

    let (tx, mut rx) = event_channel();
    let response = orchestrator
        .run("Explain BM25 algorithm", None, Some(tx))
        .await
        .expect("Deep Search failed");

    assert_eq!(response.total_turns, 2);

    let mut iterations = Vec::new();
    let mut gap_analyses = Vec::new();
    while let Ok(evt) = rx.try_recv() {
        match evt {
            AgentEvent::IterationStart { iteration, .. } => iterations.push(iteration),
            AgentEvent::GapAnalysis { is_sufficient, .. } => gap_analyses.push(is_sufficient),
            _ => {}
        }
    }

    assert_eq!(iterations, vec![1, 2]);
    assert_eq!(gap_analyses, vec![false, true]);
}

#[tokio::test]
async fn test_deep_search_max_iterations_bounded() {
    let mock_responses = vec![
        // 1. Decomposition
        ChatResponse {
            id: "decomp".into(),
            model: "mock".into(),
            content: r#"["Topic 1"]"#.into(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: None,
        },
        // 2. Reflection 1 -> gap query
        ChatResponse {
            id: "r1".into(),
            model: "mock".into(),
            content: r#"["Gap 1"]"#.into(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: None,
        },
        // 3. Synthesis (max_iterations reached at iter 2)
        ChatResponse {
            id: "synth".into(),
            model: "mock".into(),
            content: "Synthesized bounded answer.".into(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: None,
        },
    ];

    let client = Arc::new(MockDeepSearchProvider::new(mock_responses));
    let retriever = Arc::new(MockRetriever::with_chunks(vec![]));

    let orchestrator = DeepSearchOrchestrator::new(
        client,
        retriever,
        DeepSearchConfig {
            max_iterations: 2,
            ..Default::default()
        },
        AgentConfig::default(),
        None,
    );

    let response = orchestrator
        .run("Continuous loop test", None, None)
        .await
        .expect("Should stop at max_iterations");

    assert_eq!(response.total_turns, 2);
    assert_eq!(response.content, "Synthesized bounded answer.");
}

#[tokio::test]
async fn test_agent_builder_deep_search_mode() {
    let mock_responses = vec![
        ChatResponse {
            id: "decomp".into(),
            model: "mock".into(),
            content: r#"["Single query"]"#.into(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: None,
        },
        ChatResponse {
            id: "reflect".into(),
            model: "mock".into(),
            content: "[]".into(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: None,
        },
        ChatResponse {
            id: "synth".into(),
            model: "mock".into(),
            content: "Grounded builder response.".into(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: None,
        },
    ];

    let client = Arc::new(MockDeepSearchProvider::new(mock_responses));
    let retriever = Arc::new(MockRetriever::with_chunks(vec![]));

    let agent = Agent::builder()
        .client(client)
        .execution_mode(AgentExecutionMode::DeepSearch)
        .deep_search_retriever(retriever)
        .build()
        .await
        .expect("Agent building failed");

    let response = agent.run("Test via Agent struct", None).await.expect("Execution failed");
    assert_eq!(response.content, "Grounded builder response.");
}
