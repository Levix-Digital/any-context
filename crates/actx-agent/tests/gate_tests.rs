//! Nothing-found and weak-evidence gate, and the Fast-to-Deep escalation (IMP-046).

use actx_agent::*;
use actx_lm::error::LmError;
use actx_lm::traits::{BoxedChunkStream, LmProvider};
use actx_lm::types::{ChatRequest, ChatResponse, FinishReason, StreamChunk};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

/// Answers by the kind of prompt and records the kind of every call.
#[derive(Clone, Default)]
struct KindProvider {
    calls: Arc<Mutex<Vec<&'static str>>>,
}

impl KindProvider {
    fn kinds(&self) -> Vec<&'static str> {
        self.calls.lock().unwrap().clone()
    }

    fn answer(&self, request: &ChatRequest) -> ChatResponse {
        let prompt = request.messages.last().map(|m| m.content.clone()).unwrap_or_default();
        let (kind, content) = if prompt.contains("break down the original question") {
            ("decompose", r#"["sub question"]"#)
        } else if prompt.contains("Determine whether additional search queries are needed") {
            ("reflect", "[]")
        } else if prompt.starts_with("fast prompt") {
            ("fast", "fast answer")
        } else {
            ("synthesis", "deep answer")
        };
        self.calls.lock().unwrap().push(kind);
        ChatResponse {
            id: "r".into(),
            model: "mock".into(),
            content: content.into(),
            thinking: None,
            tool_calls: vec![],
            finish_reason: Some(FinishReason::Stop),
            usage: None,
        }
    }
}

#[async_trait]
impl LmProvider for KindProvider {
    fn provider_id(&self) -> &'static str {
        "kind_mock"
    }

    async fn chat_complete(&self, request: ChatRequest) -> Result<ChatResponse, LmError> {
        Ok(self.answer(&request))
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<BoxedChunkStream, LmError> {
        let resp = self.answer(&request);
        let chunks = vec![
            Ok(StreamChunk::Token(resp.content)),
            Ok(StreamChunk::Completed { finish_reason: resp.finish_reason, usage: None }),
        ];
        Ok(Box::pin(futures::stream::iter(chunks)))
    }
}

/// What the Fast retriever finds.
#[derive(Clone, Copy, Debug)]
enum Found {
    Nothing,
    Weak,
    Sufficient,
}

struct FastRetriever(Found);

#[async_trait]
impl DeterministicRetriever for FastRetriever {
    async fn retrieve_and_format(&self, query: &str) -> Result<DeterministicRetrievalOutput, AgentError> {
        let ready = |evidence| DeterministicRetrievalOutput::ContextReady {
            formatted_prompt: format!("fast prompt for {query}"),
            chunk_count: 1,
            notice: None,
            evidence,
        };
        Ok(match self.0 {
            Found::Nothing => DeterministicRetrievalOutput::ZeroContext { message: "fast nothing found".into(), notice: None },
            Found::Weak => ready(Evidence::Weak),
            Found::Sufficient => ready(Evidence::Sufficient),
        })
    }
}

/// Deep retriever returning the same chunks on every batch.
struct DeepRetriever(Vec<RetrievedChunk>);

#[async_trait]
impl DeepSearchRetriever for DeepRetriever {
    async fn retrieve_batch(&self, _queries: &[String]) -> Result<Vec<RetrievedChunk>, AgentError> {
        Ok(self.0.clone())
    }

    fn nothing_found_message(&self, _query: &str) -> String {
        "deep nothing found".into()
    }
}

fn chunk(id: &str, weak: bool) -> RetrievedChunk {
    RetrievedChunk { id: id.into(), file_name: format!("{id}.md"), text: format!("text {id}"), weak, ..Default::default() }
}

fn build(mode: SearchMode, fast: Found, deep: Vec<RetrievedChunk>, store: Option<Arc<InMemorySessionStore>>) -> (Agent, KindProvider) {
    let provider = KindProvider::default();
    let mut builder = Agent::builder()
        .client(Arc::new(provider.clone()))
        .model("mock")
        .search_mode(mode)
        .execution_mode(if mode == SearchMode::Deep { AgentExecutionMode::DeepSearch } else { AgentExecutionMode::ReAct })
        .deterministic_retriever(Arc::new(FastRetriever(fast)))
        .deep_search_retriever(Arc::new(DeepRetriever(deep)));
    if let Some(store) = store {
        builder = builder.session_store(store);
    }
    (builder.build_sync().unwrap(), provider)
}

// A short question routes to Fast in automatic mode (keyword classifier, until IMP-038).
const QUESTION: &str = "where is the timeout set?";

async fn streamed(agent: &Agent) -> (Vec<String>, AgentResponse) {
    let (mut rx, handle) = agent.stream(QUESTION, Some("s".into()));
    let resp = handle.await.unwrap().unwrap();
    let mut out = Vec::new();
    while let Ok(evt) = rx.try_recv() {
        match evt {
            AgentEvent::Escalation { .. } => out.push("escalation".to_string()),
            AgentEvent::Caveat(_) => out.push("caveat".to_string()),
            AgentEvent::Delta(d) => out.push(format!("delta:{d}")),
            AgentEvent::Decomposition { .. } => out.push("deep".to_string()),
            AgentEvent::Done { .. } => out.push("done".to_string()),
            _ => {}
        }
    }
    (out, resp)
}

#[tokio::test]
async fn fast_pinned_nothing_found_makes_no_llm_call() {
    let (agent, provider) = build(SearchMode::Fast, Found::Nothing, vec![chunk("a", false)], None);
    let resp = agent.run(QUESTION, None).await.unwrap();
    assert_eq!(resp.content, "fast nothing found");
    let (events, _) = streamed(&agent).await;
    assert_eq!(events, ["delta:fast nothing found", "done"]);
    assert!(provider.kinds().is_empty());
}

#[tokio::test]
async fn fast_pinned_weak_evidence_puts_the_warning_first_and_keeps_it_out_of_history() {
    let store = Arc::new(InMemorySessionStore::new(50));
    let (agent, provider) = build(SearchMode::Fast, Found::Weak, vec![], Some(store.clone()));
    let (events, resp) = streamed(&agent).await;
    assert_eq!(events, ["caveat", "delta:fast answer", "done"]);
    assert_eq!(resp.content, format!("{WEAK_EVIDENCE_WARNING}\n\nfast answer"));
    assert_eq!(provider.kinds(), ["fast"]);

    let history = store.get_messages("s").await.unwrap();
    assert_eq!(history.last().unwrap().content, "fast answer");

    let resp = agent.run(QUESTION, None).await.unwrap();
    assert_eq!(resp.content, format!("{WEAK_EVIDENCE_WARNING}\n\nfast answer"));
}

#[tokio::test]
async fn sufficient_evidence_has_no_warning_and_no_escalation() {
    for mode in [SearchMode::Auto, SearchMode::Fast] {
        let (agent, provider) = build(mode, Found::Sufficient, vec![chunk("a", false)], None);
        let (events, resp) = streamed(&agent).await;
        assert_eq!(events, ["delta:fast answer", "done"], "{mode:?}");
        assert_eq!(resp.content, "fast answer");
        assert_eq!(provider.kinds(), ["fast"]);
    }
}

#[tokio::test]
async fn automatic_fast_escalates_to_deep_on_nothing_and_on_weak_evidence() {
    for found in [Found::Nothing, Found::Weak] {
        let (agent, provider) = build(SearchMode::Auto, found, vec![chunk("a", false)], None);
        let (events, resp) = streamed(&agent).await;
        assert_eq!(events.first().map(String::as_str), Some("escalation"), "{found:?}");
        assert!(events.contains(&"deep".to_string()), "{found:?}");
        assert_eq!(resp.content, "deep answer", "{found:?}");
        assert!(!provider.kinds().contains(&"fast"), "the Fast LLM call is not made: {found:?}");

        let (agent, provider) = build(SearchMode::Auto, found, vec![chunk("a", false)], None);
        assert_eq!(agent.run(QUESTION, None).await.unwrap().content, "deep answer", "{found:?}");
        assert!(provider.kinds().contains(&"synthesis"), "{found:?}");
    }
}

#[tokio::test]
async fn escalated_deep_that_finds_nothing_answers_without_synthesis() {
    let (agent, provider) = build(SearchMode::Auto, Found::Nothing, vec![], None);
    let (events, resp) = streamed(&agent).await;
    assert_eq!(events.first().map(String::as_str), Some("escalation"));
    assert!(events.contains(&"delta:deep nothing found".to_string()));
    assert_eq!(resp.content, "deep nothing found");
    assert!(!provider.kinds().contains(&"synthesis"));
    assert!(!provider.kinds().contains(&"fast"));
}

#[tokio::test]
async fn deep_with_only_weak_chunks_puts_the_warning_before_the_synthesis() {
    let store = Arc::new(InMemorySessionStore::new(50));
    let (agent, _) = build(SearchMode::Deep, Found::Sufficient, vec![chunk("a", true), chunk("b", true)], Some(store.clone()));
    let (events, resp) = streamed(&agent).await;
    let caveat = events.iter().position(|e| e == "caveat").expect("caveat sent");
    let first_delta = events.iter().position(|e| e.starts_with("delta:")).unwrap();
    assert!(caveat < first_delta);
    assert_eq!(resp.content, format!("{WEAK_EVIDENCE_WARNING}\n\ndeep answer"));
    assert_eq!(store.get_messages("s").await.unwrap().last().unwrap().content, "deep answer");
}

#[tokio::test]
async fn deep_with_one_strong_chunk_has_no_warning() {
    let (agent, _) = build(SearchMode::Deep, Found::Sufficient, vec![chunk("a", true), chunk("b", false)], None);
    let (events, resp) = streamed(&agent).await;
    assert!(!events.contains(&"caveat".to_string()));
    assert_eq!(resp.content, "deep answer");
}

#[tokio::test]
async fn deep_pinned_nothing_found_is_stored_and_makes_no_synthesis_call() {
    let store = Arc::new(InMemorySessionStore::new(50));
    let (agent, provider) = build(SearchMode::Deep, Found::Sufficient, vec![], Some(store.clone()));
    let resp = agent.run(QUESTION, Some("s")).await.unwrap();
    assert_eq!(resp.content, "deep nothing found");
    assert!(!provider.kinds().contains(&"synthesis"));
    let history = store.get_messages("s").await.unwrap();
    assert_eq!(history.last().unwrap().content, "deep nothing found");
}

#[test]
fn evidence_of_reads_the_weak_flags() {
    assert_eq!(evidence_of(&[]), None);
    assert_eq!(evidence_of(&[chunk("a", true)]), Some(Evidence::Weak));
    assert_eq!(evidence_of(&[chunk("a", true), chunk("b", false)]), Some(Evidence::Sufficient));
}
