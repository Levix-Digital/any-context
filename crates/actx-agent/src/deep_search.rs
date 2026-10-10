//! # 🧠 RFC-042 Deep Search Engine in Native Rust
//!
//! Autonomous, multi-phase reflective retrieval and synthesis engine inspired by
//! `zilliztech/deep-searcher` and RFC-042.
//!
//! Phases:
//! 1. **Decomposition**: Orthogonal breakdown of the user question into 1-4 targeted sub-queries.
//! 2. **Iterative Batch Retrieval & Reranking**: Concurrently searches vector and lexical indexes,
//!    deduplicating chunks and scoring relevance.
//! 3. **Gap Analysis & Reflection**: Evaluates whether collected evidence is sufficient or if missing
//!    aspects require focused gap queries (bounded by `max_iterations`).
//! 4. **Grounded Synthesis**: Produces a comprehensive final answer citing file provenance and line numbers.

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::events::{AgentEvent, EventSender};
use crate::fsm::AgentResponse;
use crate::session::SessionStore;
use actx_lm::traits::LmProvider;
use actx_lm::types::{ChatMessage, ChatRequest, FinishReason};
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

/// Normalized document chunk retrieved across lexical and vector sources
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievedChunk {
    pub id: String,
    pub file_name: String,
    pub file_path: String,
    pub header_path: Option<String>,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub text: String,
    pub score: f64,
    pub matched_queries: Vec<String>,
}

/// Abstract contract for asynchronous batch chunk retrieval
#[async_trait]
pub trait DeepSearchRetriever: Send + Sync {
    /// Retrieve relevant document chunks for a batch of sub-queries concurrently
    async fn retrieve_batch(&self, queries: &[String]) -> Result<Vec<RetrievedChunk>, AgentError>;
}

/// Runtime configuration for the Deep Search engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepSearchConfig {
    /// Maximum reflection/gap iterations (default: 3)
    pub max_iterations: usize,
    /// Maximum sub-queries generated during initial decomposition (default: 4)
    pub max_sub_queries: usize,
    /// Maximum gap queries generated per reflection loop (default: 3)
    pub max_gap_queries: usize,
    /// Whether to invoke the LLM boolean reranker for boundary chunks (default: false for speed)
    pub enable_reranker: bool,
    /// Candidate score threshold below which reranking is evaluated (default: 0.15)
    pub rerank_score_threshold: f64,
}

impl Default for DeepSearchConfig {
    fn default() -> Self {
        Self {
            max_iterations: 3,
            max_sub_queries: 4,
            max_gap_queries: 3,
            enable_reranker: false,
            rerank_score_threshold: 0.15,
        }
    }
}

// ============================================================================
// PROMPTS (Aligned with zilliztech/deep-searcher & RFC-042)
// ============================================================================

pub const SUB_QUERY_PROMPT: &str = r#"To answer this question more comprehensively, break down the original question into up to four distinct sub-questions.
If this is a simple question where decomposition is unnecessary, return only the original question in the list.

Original Question: {original_query}

Provide your response strictly as a JSON array of strings, for example:
["What is the core architecture?", "How does authentication integrate?"]

Return ONLY the JSON array without any markdown formatting or commentary."#;

pub const RERANK_PROMPT: &str = r#"Based on the query questions and the retrieved chunk, determine whether the chunk is helpful in answering any of the questions.
You can ONLY return "YES" or "NO", without any other information.

Query Questions:
{query_questions}

Retrieved Chunk:
<chunk>
{chunk_text}
</chunk>

Is the chunk helpful in answering any of the questions? Answer YES or NO:"#;

pub const REFLECT_PROMPT: &str = r#"Determine whether additional search queries are needed based on the original query, previous sub-queries, and all retrieved document chunks.
If further research is required to cover missing aspects, provide a JSON array of up to 3 search queries.
If no further research is required and the evidence is sufficient, return an empty array [].

Original Query: {original_query}

Previous Sub-Queries:
{previous_queries}

Retrieved Evidence Chunks:
{chunks_summary}

Respond EXCLUSIVELY with a valid JSON array of strings (e.g. ["query 1", "query 2"] or []):"#;

pub const SUMMARY_PROMPT: &str = r#"You are an expert AI system and deep technical research assistant.
Please provide a comprehensive, accurate, and direct conclusion answering the user's query based on the retrieved document chunks.

CRITICAL GROUNDING & LANGUAGE RULES:
1. STRICT LANGUAGE MATCHING: You MUST write the final response strictly in the EXACT same language as the user's Original Query (e.g., if the user wrote in Portuguese, answer 100% in Portuguese).
2. DIRECT CONCLUSION ONLY: Deliver ONLY the direct, grounded answer or conclusion. DO NOT include preambles, introductory filler (e.g., "Based on the evidence...", "Here is the response..."), meta-explanations of your research process, or repetition of explored sub-queries.
3. PRECISE PROVENANCE: Rely strictly on the provided evidence chunks. Whenever citing information or code, cite exact source provenance with file names and line ranges, e.g. `[path/to/file.rs:45-60]`.
4. If specific details are not found in the chunks, explicitly state the limitation.

Original Query: {original_query}

Retrieved Chunks:
{chunks_text}

Conclusion:"#;

// ============================================================================
// PARSER UTILITIES (Resilient JSON / Markdown / Python extraction)
// ============================================================================

/// Extracts a clean list of strings from LLM responses, stripping reasoning tokens,
/// markdown code fences, and Python/JSON brackets.
pub fn parse_string_list(raw_response: &str, fallback_query: &str) -> Vec<String> {
    let clean = clean_llm_response(raw_response);

    // 1. Try standard JSON array parsing
    if let Ok(list) = serde_json::from_str::<Vec<String>>(&clean) {
        let filtered: Vec<String> = list
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !filtered.is_empty() {
            return filtered;
        }
    }

    // 2. Try extracting content inside '[' and ']'
    if let (Some(start), Some(end)) = (clean.find('['), clean.rfind(']')) {
        if start < end {
            let slice = &clean[start..=end];
            if let Ok(list) = serde_json::from_str::<Vec<String>>(slice) {
                let filtered: Vec<String> = list
                    .into_iter()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !filtered.is_empty() {
                    return filtered;
                }
            }
        }
    }

    // 3. Line-by-line fallback (for bullet points like `1. query`, `- query`, `* query`)
    let mut line_queries = Vec::new();
    for line in clean.lines() {
        let trimmed = line.trim();
        let has_marker = trimmed.starts_with('-')
            || trimmed.starts_with('*')
            || trimmed.starts_with('•')
            || (trimmed.len() >= 3
                && trimmed.chars().next().map_or(false, |c| c.is_ascii_digit())
                && trimmed[1..].starts_with(". "));

        if has_marker {
            let cleaned_line = trimmed
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '-' || c == '*' || c == '•')
                .trim_matches(|c: char| c == '"' || c == '\'' || c == ',')
                .trim();
            if !cleaned_line.is_empty() {
                line_queries.push(cleaned_line.to_string());
            }
        }
    }

    if !line_queries.is_empty() {
        return line_queries;
    }

    // Fallback: return the original query if available
    if !fallback_query.trim().is_empty() {
        vec![fallback_query.trim().to_string()]
    } else {
        Vec::new()
    }
}

/// Evaluates boolean rerank response ("YES" vs "NO")
pub fn parse_rerank_decision(raw_response: &str) -> bool {
    let clean = clean_llm_response(raw_response).to_uppercase();
    clean.contains("YES") && !clean.contains("NO")
}

/// Strips <think>...</think> reasoning tags and markdown code blocks
pub fn clean_llm_response(raw: &str) -> String {
    let mut text = raw.to_string();

    // Strip <think>...</think> if present
    while let Some(start) = text.find("<think>") {
        if let Some(end) = text.find("</think>") {
            if end >= start {
                text.replace_range(start..end + 8, "");
            } else {
                text.replace_range(start.., "");
                break;
            }
        } else {
            text.replace_range(start.., "");
            break;
        }
    }

    let trimmed = text.trim();

    // Strip markdown code fences: ```json ... ``` or ```python ... ```
    if let Some(rest) = trimmed.strip_prefix("```") {
        let after_lang = if let Some(idx) = rest.find('\n') {
            &rest[idx + 1..]
        } else {
            rest
        };
        if let Some(code) = after_lang.strip_suffix("```") {
            return code.trim().to_string();
        }
    }

    trimmed.to_string()
}

// ============================================================================
// DEEP SEARCH ORCHESTRATOR
// ============================================================================

/// The Deep Search engine coordinating decomposition, iterative retrieval, reflection and synthesis.
pub struct DeepSearchOrchestrator {
    client: Arc<dyn LmProvider>,
    retriever: Arc<dyn DeepSearchRetriever>,
    config: DeepSearchConfig,
    agent_config: AgentConfig,
    session_store: Option<Arc<dyn SessionStore>>,
}

impl DeepSearchOrchestrator {
    pub fn new(
        client: Arc<dyn LmProvider>,
        retriever: Arc<dyn DeepSearchRetriever>,
        config: DeepSearchConfig,
        agent_config: AgentConfig,
        session_store: Option<Arc<dyn SessionStore>>,
    ) -> Self {
        Self {
            client,
            retriever,
            config,
            agent_config,
            session_store,
        }
    }

    /// Execute the full Deep Search workflow
    pub async fn run(
        &self,
        input: &str,
        session_id: Option<&str>,
        event_tx: Option<EventSender>,
    ) -> Result<AgentResponse, AgentError> {
        let start_time = Instant::now();
        let mut total_turns_executed = 0;
        let mut all_explored_queries = Vec::new();
        let mut all_accepted_chunks: Vec<RetrievedChunk> = Vec::new();
        let mut seen_chunk_keys: HashSet<String> = HashSet::new();

        // 1. Session History Preparation
        let mut working_history = Vec::new();
        if let (Some(sid), Some(ref store)) = (session_id, &self.session_store) {
            if let Ok(prior) = store.get_messages(sid).await {
                working_history.extend(prior);
            }
        }

        // --------------------------------------------------------------------
        // PHASE 1: Query Decomposition
        // --------------------------------------------------------------------
        let decomp_prompt = SUB_QUERY_PROMPT.replace("{original_query}", input);
        let decomp_req = ChatRequest::new(&self.agent_config.model, vec![ChatMessage::user(decomp_prompt)])
            .with_temperature(0.0);

        let decomp_res = self.client.chat_complete(decomp_req).await.map_err(AgentError::LmError)?;
        let mut sub_queries = parse_string_list(&decomp_res.content, input);
        if sub_queries.is_empty() {
            sub_queries.push(input.to_string());
        }
        sub_queries.truncate(self.config.max_sub_queries);

        if let Some(ref tx) = event_tx {
            let _ = tx.send(AgentEvent::Decomposition {
                sub_queries: sub_queries.clone(),
            });
        }
        all_explored_queries.extend(sub_queries.clone());

        let mut current_search_queries = sub_queries;

        // --------------------------------------------------------------------
        // PHASE 2: Iterative Retrieval & Reflection Loop
        // --------------------------------------------------------------------
        for iter in 1..=self.config.max_iterations {
            total_turns_executed = iter;

            if let Some(ref tx) = event_tx {
                let _ = tx.send(AgentEvent::IterationStart {
                    iteration: iter,
                    max_iterations: self.config.max_iterations,
                });
            }

            // A. Batch retrieve chunks for current sub-queries
            let retrieved = self
                .retriever
                .retrieve_batch(&current_search_queries)
                .await
                .unwrap_or_default();

            // B. Deduplicate and collect accepted chunks
            for chunk in retrieved {
                let key = if !chunk.id.is_empty() {
                    chunk.id.clone()
                } else {
                    format!("{}::{}:{}", chunk.file_path, chunk.start_line.unwrap_or(0), chunk.end_line.unwrap_or(0))
                };

                if seen_chunk_keys.insert(key) {
                    // Optional Reranker step for boundary scores
                    if self.config.enable_reranker && chunk.score < self.config.rerank_score_threshold {
                        let query_str = current_search_queries.join(", ");
                        let rerank_prompt = RERANK_PROMPT
                            .replace("{query_questions}", &query_str)
                            .replace("{chunk_text}", &chunk.text);
                        let rerank_req = ChatRequest::new(&self.agent_config.model, vec![ChatMessage::user(rerank_prompt)])
                            .with_temperature(0.0);

                        if let Ok(rerank_res) = self.client.chat_complete(rerank_req).await {
                            if parse_rerank_decision(&rerank_res.content) {
                                all_accepted_chunks.push(chunk);
                            }
                        }
                    } else {
                        all_accepted_chunks.push(chunk);
                    }
                }
            }

            // If reached maximum iterations, terminate search loop immediately
            if iter == self.config.max_iterations {
                if let Some(ref tx) = event_tx {
                    let _ = tx.send(AgentEvent::GapAnalysis {
                        is_sufficient: true,
                        missing_aspects: Vec::new(),
                    });
                }
                break;
            }

            // C. Reflection & Gap Analysis
            let chunks_summary = self.format_chunks_summary(&all_accepted_chunks);
            let prev_queries_str = all_explored_queries.join("\n- ");
            let reflect_prompt = REFLECT_PROMPT
                .replace("{original_query}", input)
                .replace("{previous_queries}", &format!("- {prev_queries_str}"))
                .replace("{chunks_summary}", &chunks_summary);

            let reflect_req = ChatRequest::new(&self.agent_config.model, vec![ChatMessage::user(reflect_prompt)])
                .with_temperature(0.0);

            let reflect_res = self.client.chat_complete(reflect_req).await.map_err(AgentError::LmError)?;
            let gap_queries = parse_string_list(&reflect_res.content, "");

            if gap_queries.is_empty() {
                // Evidence is deemed complete!
                if let Some(ref tx) = event_tx {
                    let _ = tx.send(AgentEvent::GapAnalysis {
                        is_sufficient: true,
                        missing_aspects: Vec::new(),
                    });
                }
                break;
            } else {
                let mut valid_gap_queries = Vec::new();
                for gq in gap_queries {
                    if !all_explored_queries.contains(&gq) {
                        all_explored_queries.push(gq.clone());
                        valid_gap_queries.push(gq);
                    }
                }

                if valid_gap_queries.is_empty() {
                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(AgentEvent::GapAnalysis {
                            is_sufficient: true,
                            missing_aspects: Vec::new(),
                        });
                    }
                    break;
                }

                valid_gap_queries.truncate(self.config.max_gap_queries);

                if let Some(ref tx) = event_tx {
                    let _ = tx.send(AgentEvent::GapAnalysis {
                        is_sufficient: false,
                        missing_aspects: valid_gap_queries.clone(),
                    });
                }

                current_search_queries = valid_gap_queries;
            }
        }

        // --------------------------------------------------------------------
        // PHASE 3: Grounded Synthesis with Provenance
        // --------------------------------------------------------------------
        let full_chunks_text = self.format_full_chunks(&all_accepted_chunks);

        let summary_prompt = SUMMARY_PROMPT
            .replace("{original_query}", input)
            .replace("{chunks_text}", &full_chunks_text);

        let mut synthesis_messages = working_history;
        synthesis_messages.push(ChatMessage::user(summary_prompt));

        let synth_req = ChatRequest::new(&self.agent_config.model, synthesis_messages)
            .with_temperature(self.agent_config.temperature.unwrap_or(0.0));

        let mut final_answer = String::new();
        let mut final_reason = None;

        if let Some(ref tx) = event_tx {
            if let Ok(mut stream) = self.client.chat_stream(synth_req.clone()).await {
                while let Some(chunk_res) = stream.next().await {
                    if let Ok(chunk) = chunk_res {
                        match chunk {
                            actx_lm::types::StreamChunk::Token(token) => {
                                final_answer.push_str(&token);
                                let _ = tx.send(AgentEvent::Delta(token));
                            }
                            actx_lm::types::StreamChunk::Reasoning(reasoning) => {
                                let _ = tx.send(AgentEvent::Thinking(reasoning));
                            }
                            actx_lm::types::StreamChunk::Completed { finish_reason, .. } => {
                                final_reason = finish_reason;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if final_answer.is_empty() {
            let comp = self.client.chat_complete(synth_req).await.map_err(AgentError::LmError)?;
            final_answer = comp.content;
            final_reason = comp.finish_reason;
            if let Some(ref tx) = event_tx {
                let _ = tx.send(AgentEvent::Delta(final_answer.clone()));
            }
        }

        let clean_final_answer = sanitize_direct_conclusion(&final_answer);

        // Record to session store if enabled
        if let (Some(sid), Some(ref store)) = (session_id, &self.session_store) {
            let new_msgs = vec![
                ChatMessage::user(input),
                ChatMessage::assistant(&clean_final_answer),
            ];
            let _ = store.append_messages(sid, &new_msgs).await;
        }

        if let Some(ref tx) = event_tx {
            let _ = tx.send(AgentEvent::Done {
                total_turns: total_turns_executed,
                total_tokens: None,
            });
        }

        Ok(AgentResponse {
            content: clean_final_answer,
            total_turns: total_turns_executed,
            tool_calls_count: all_accepted_chunks.len(),
            finish_reason: final_reason.or(Some(FinishReason::Stop)),
            execution_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    fn format_chunks_summary(&self, chunks: &[RetrievedChunk]) -> String {
        if chunks.is_empty() {
            return "NO RELATED CHUNKS FOUND YET.".to_string();
        }
        let mut out = String::new();
        for (i, c) in chunks.iter().enumerate() {
            let lines = match (c.start_line, c.end_line) {
                (Some(s), Some(e)) => format!(" [lines {s}-{e}]"),
                _ => String::new(),
            };
            let preview: String = c.text.chars().take(200).collect();
            out.push_str(&format!("{}. [{}{}] {}\n", i + 1, c.file_name, lines, preview.replace('\n', " ")));
        }
        out
    }

    fn format_full_chunks(&self, chunks: &[RetrievedChunk]) -> String {
        if chunks.is_empty() {
            return "No relevant documentation chunks found in the workspace.".to_string();
        }
        let mut out = String::new();
        for (i, c) in chunks.iter().enumerate() {
            let header = c.header_path.as_deref().unwrap_or(&c.file_name);
            let lines = match (c.start_line, c.end_line) {
                (Some(s), Some(e)) => format!(":{s}-{e}"),
                _ => String::new(),
            };
            out.push_str(&format!(
                "--- CHUNK {} [{}{}] (Context: {}) ---\n{}\n\n",
                i + 1,
                c.file_path,
                lines,
                header,
                c.text
            ));
        }
        out
    }
}

/// Sanitizes any remaining metadata or section headers, ensuring only the direct conclusion is returned.
pub fn sanitize_direct_conclusion(s: &str) -> String {
    let trimmed = s.trim();
    for prefix in &[
        "Conclusion:", "Conclusão:", "### Conclusion:", "### Conclusão:",
        "### Conclusion", "### Conclusão", "## Conclusion:", "## Conclusão:",
        "## Conclusion", "## Conclusão", "# Conclusion:", "# Conclusão:"
    ] {
        if let Some(stripped) = trimmed.strip_prefix(prefix) {
            return stripped.trim_start().to_string();
        }
    }
    trimmed.to_string()
}
