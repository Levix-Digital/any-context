//! The LLM's not-enough signal: automatic Fast escalates to Deep when the answer is the signal (IMP-038).

use actx_agent::escalation::{NOT_ENOUGH_REASON, NOT_ENOUGH_SIGNAL};
use actx_agent::*;
use actx_lm::error::LmError;
use actx_lm::traits::{BoxedChunkStream, LmProvider};
use actx_lm::types::{ChatRequest, ChatResponse, FinishReason, StreamChunk};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

/// Answers the Fast prompt with `fast_answer` (streamed in 4-character tokens), Deep calls with
/// fixed text; records every call's kind and the Fast prompts.
#[derive(Clone)]
struct SignalProvider {
    fast_answer: String,
    calls: Arc<Mutex<Vec<&'static str>>>,
    fast_prompts: Arc<Mutex<Vec<String>>>,
}

impl SignalProvider {
    fn new(fast_answer: &str) -> Self {
        Self { fast_answer: fast_answer.into(), calls: Default::default(), fast_prompts: Default::default() }
    }

    fn kinds(&self) -> Vec<&'static str> {
        self.calls.lock().unwrap().clone()
    }

    fn fast_prompt(&self) -> String {
        self.fast_prompts.lock().unwrap().last().cloned().unwrap_or_default()
    }

    fn answer(&self, request: &ChatRequest) -> String {
        let prompt = request.messages.last().map(|m| m.content.clone()).unwrap_or_default();
        let (kind, content) = if prompt.contains("break down the original question") {
            ("decompose", r#"["sub question"]"#.to_string())
        } else if prompt.contains("Determine whether additional search queries are needed") {
            ("reflect", "[]".to_string())
        } else if prompt.starts_with("fast prompt") {
            self.fast_prompts.lock().unwrap().push(prompt.clone());
            ("fast", self.fast_answer.clone())
        } else {
            ("synthesis", "deep answer".to_string())
        };
        self.calls.lock().unwrap().push(kind);
        content
    }
}

#[async_trait]
impl LmProvider for SignalProvider {
    fn provider_id(&self) -> &'static str {
        "signal_mock"
    }

    async fn chat_complete(&self, request: ChatRequest) -> Result<ChatResponse, LmError> {
        Ok(ChatResponse {
            id: "r".into(),
            model: "mock".into(),
            content: self.answer(&request),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: None,
        })
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<BoxedChunkStream, LmError> {
        let content: Vec<char> = self.answer(&request).chars().collect();
        let mut chunks: Vec<Result<StreamChunk, LmError>> = content
            .chunks(4)
            .map(|c| Ok(StreamChunk::Token(c.iter().collect())))
            .collect();
        chunks.push(Ok(StreamChunk::Completed { finish_reason: Some(FinishReason::Stop), usage: None }));
        Ok(Box::pin(futures::stream::iter(chunks)))
    }
}

struct SufficientRetriever;

#[async_trait]
impl DeterministicRetriever for SufficientRetriever {
    async fn retrieve_and_format(&self, query: &str) -> Result<DeterministicRetrievalOutput, AgentError> {
        Ok(DeterministicRetrievalOutput::ContextReady {
            formatted_prompt: format!("fast prompt for {query}"),
            chunk_count: 1,
            notice: None,
            evidence: Evidence::Sufficient,
        })
    }
}

struct DeepRetriever;

#[async_trait]
impl DeepSearchRetriever for DeepRetriever {
    async fn retrieve_batch(&self, _queries: &[String]) -> Result<Vec<RetrievedChunk>, AgentError> {
        Ok(vec![RetrievedChunk { id: "a".into(), file_name: "a.md".into(), text: "text a".into(), ..Default::default() }])
    }
}

const QUESTION: &str = "what is the retry limit of the importer?";

fn build(mode: SearchMode, fast_answer: &str, deep: bool, store: Option<Arc<InMemorySessionStore>>) -> (Agent, SignalProvider) {
    let provider = SignalProvider::new(fast_answer);
    let mut builder = Agent::builder()
        .client(Arc::new(provider.clone()))
        .model("mock")
        .search_mode(mode)
        .deterministic_retriever(Arc::new(SufficientRetriever));
    if deep {
        builder = builder.deep_search_retriever(Arc::new(DeepRetriever));
    }
    if let Some(store) = store {
        builder = builder.session_store(store);
    }
    (builder.build_sync().unwrap(), provider)
}

async fn streamed(agent: &Agent) -> (Vec<String>, AgentResponse) {
    let (mut rx, handle) = agent.stream(QUESTION, Some("s".into()));
    let resp = handle.await.unwrap().unwrap();
    let mut out = Vec::new();
    while let Ok(evt) = rx.try_recv() {
        match evt {
            AgentEvent::Escalation { reason } => out.push(format!("escalation:{reason}")),
            AgentEvent::Delta(d) => out.push(format!("delta:{d}")),
            AgentEvent::Decomposition { .. } => out.push("deep".to_string()),
            _ => {}
        }
    }
    (out, resp)
}

fn shown(events: &[String]) -> String {
    events.iter().filter_map(|e| e.strip_prefix("delta:")).collect()
}

#[tokio::test]
async fn automatic_fast_escalates_when_the_answer_is_the_signal() {
    let store = Arc::new(InMemorySessionStore::new(50));
    let (agent, provider) = build(SearchMode::Auto, &format!("\n{NOT_ENOUGH_SIGNAL}"), true, Some(store.clone()));
    let (events, resp) = streamed(&agent).await;

    let escalation = events.iter().position(|e| e.starts_with("escalation:")).expect("escalation sent");
    assert_eq!(events[escalation], format!("escalation:{NOT_ENOUGH_REASON}"));
    let deep = events.iter().position(|e| e == "deep").expect("deep ran");
    assert!(escalation < deep);
    assert!(events[..escalation].iter().all(|e| !e.starts_with("delta:")), "no Fast token shown: {events:?}");
    assert!(!shown(&events).contains("ACTX"));
    assert_eq!(resp.content, "deep answer");
    assert_eq!(provider.kinds().first(), Some(&"fast"));
    assert!(provider.kinds().contains(&"synthesis"));
    assert!(provider.fast_prompt().contains(NOT_ENOUGH_SIGNAL), "the Fast prompt carries the rule");

    let history = store.get_messages("s").await.unwrap();
    assert!(history.iter().all(|m| !m.content.contains(NOT_ENOUGH_SIGNAL)));
    assert_eq!(history.last().unwrap().content, "deep answer");
    assert_eq!(history.len(), 2, "only the question and the Deep answer are stored");
}

#[tokio::test]
async fn automatic_fast_escalates_in_run_too() {
    let (agent, provider) = build(SearchMode::Auto, NOT_ENOUGH_SIGNAL, true, None);
    let resp = agent.run(QUESTION, None).await.unwrap();
    assert_eq!(resp.content, "deep answer");
    assert!(provider.kinds().contains(&"synthesis"));
}

#[tokio::test]
async fn a_normal_answer_streams_unchanged_with_one_call() {
    let (agent, provider) = build(SearchMode::Auto, "The retry limit is 5.", true, None);
    let (events, resp) = streamed(&agent).await;
    assert_eq!(shown(&events), "The retry limit is 5.");
    assert!(events.iter().all(|e| !e.starts_with("escalation:")));
    assert_eq!(resp.content, "The retry limit is 5.");
    assert_eq!(provider.kinds(), ["fast"]);
}

#[tokio::test]
async fn a_short_answer_that_looks_like_a_signal_prefix_is_shown_at_the_end() {
    let (agent, _) = build(SearchMode::Auto, "[[", true, None);
    let (events, resp) = streamed(&agent).await;
    assert_eq!(shown(&events), "[[");
    assert_eq!(resp.content, "[[");
}

#[tokio::test]
async fn fast_pinned_has_no_rule_and_never_escalates() {
    let (agent, provider) = build(SearchMode::Fast, &format!("Partial. {NOT_ENOUGH_SIGNAL}"), true, None);
    let (events, resp) = streamed(&agent).await;
    assert!(events.iter().all(|e| !e.starts_with("escalation:")));
    assert_eq!(resp.content, "Partial. ");
    assert_eq!(provider.kinds(), ["fast"]);
    assert!(!provider.fast_prompt().contains(NOT_ENOUGH_SIGNAL));

    let (agent, provider) = build(SearchMode::Fast, NOT_ENOUGH_SIGNAL, true, None);
    let resp = agent.run(QUESTION, None).await.unwrap();
    assert_eq!(resp.content, "", "the signal is stripped, not escalated");
    assert_eq!(provider.kinds(), ["fast"]);
}

#[tokio::test]
async fn deep_pinned_runs_deep_for_a_short_question() {
    let (agent, provider) = build(SearchMode::Deep, "unused", true, None);
    let (events, resp) = streamed(&agent).await;
    assert!(events.contains(&"deep".to_string()));
    assert_eq!(resp.content, "deep answer");
    assert!(!provider.kinds().contains(&"fast"));
}
