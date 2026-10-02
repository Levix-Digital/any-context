//! Finite State Machine (FSM) implementation for deterministic ReAct agent loops

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::events::{event_channel, AgentEvent, EventReceiver, EventSender};
use crate::session::SessionStore;
use crate::tool::ToolRegistry;
use actx_lm::traits::LmProvider;
use actx_lm::types::{ChatMessage, ChatRequest, FinishReason};
use futures::StreamExt;
use std::sync::Arc;
use std::time::Instant;

/// Final output resulting from an agent execution run
#[derive(Debug, Clone)]
pub struct AgentResponse {
    pub content: String,
    pub total_turns: usize,
    pub tool_calls_count: usize,
    pub finish_reason: Option<FinishReason>,
    pub execution_time_ms: u64,
}

/// Core orchestrator executing the ReAct FSM loop
pub struct ReActOrchestrator {
    client: Arc<dyn LmProvider>,
    tools: Arc<ToolRegistry>,
    session_store: Option<Arc<dyn SessionStore>>,
    config: AgentConfig,
}

impl ReActOrchestrator {
    pub fn new(
        client: Arc<dyn LmProvider>,
        tools: Arc<ToolRegistry>,
        session_store: Option<Arc<dyn SessionStore>>,
        config: AgentConfig,
    ) -> Self {
        Self {
            client,
            tools,
            session_store,
            config,
        }
    }

    pub fn tools(&self) -> &ToolRegistry {
        &self.tools
    }

    /// Synchronous/async execution of the agent loop to completion
    pub async fn run(
        &self,
        input: &str,
        session_id: Option<&str>,
        event_tx: Option<EventSender>,
    ) -> Result<AgentResponse, AgentError> {
        let start_time = Instant::now();
        let mut tool_calls_count = 0;

        // 1. Prepare conversation history from session store
        let mut working_messages = Vec::new();

        if let Some(ref sys_prompt) = self.config.system_prompt {
            working_messages.push(ChatMessage::system(sys_prompt));
        }

        let mut persisted_new_messages = Vec::new();

        if let (Some(sid), Some(ref store)) = (session_id, &self.session_store) {
            let prior = store.get_messages(sid).await.unwrap_or_default();
            working_messages.extend(prior);
        }

        let user_msg = ChatMessage::user(input);
        working_messages.push(user_msg.clone());
        persisted_new_messages.push(user_msg);

        // 2. Execute ReAct FSM Loop
        let mut final_content = String::new();
        let mut final_reason = None;
        let mut total_turns_executed = 0;

        for turn in 1..=self.config.max_turns {
            total_turns_executed = turn;
            let is_last_turn = turn == self.config.max_turns;

            // In the absolute last turn, cut off tools to force immediate synthesis
            let available_tools = if is_last_turn {
                working_messages.push(ChatMessage::system(
                    "Maximum reasoning turns reached. Please synthesize your final answer directly based on collected evidence.",
                ));
                Vec::new()
            } else {
                self.tools.definitions().await
            };

            let mut req = ChatRequest::new(&self.config.model, working_messages.clone())
                .with_tools(available_tools);

            if let Some(t) = self.config.temperature {
                req = req.with_temperature(t);
            }

            let mut response_content = String::new();
            let mut response_tool_calls = Vec::new();
            let mut finish_reason = None;
            let mut total_usage = None;
            let mut did_stream_tokens = false;

            if event_tx.is_some() {
                if let Ok(mut stream) = self.client.chat_stream(req.clone()).await {
                    let mut streamed_tools: std::collections::BTreeMap<usize, (Option<String>, Option<String>, String)> = std::collections::BTreeMap::new();

                    while let Some(chunk_res) = stream.next().await {
                        if let Ok(chunk) = chunk_res {
                            match chunk {
                                actx_lm::types::StreamChunk::Token(token) => {
                                    response_content.push_str(&token);
                                    if streamed_tools.is_empty() {
                                        did_stream_tokens = true;
                                        if let Some(ref tx) = event_tx {
                                            let _ = tx.send(AgentEvent::Delta(token));
                                        }
                                    }
                                }
                                actx_lm::types::StreamChunk::Reasoning(reasoning) => {
                                    if let Some(ref tx) = event_tx {
                                        let _ = tx.send(AgentEvent::Thinking(reasoning));
                                    }
                                }
                                actx_lm::types::StreamChunk::ToolCallDelta { index, id, name, arguments_delta } => {
                                    let entry = streamed_tools.entry(index).or_insert((None, None, String::new()));
                                    if let Some(i) = id { entry.0 = Some(i); }
                                    if let Some(n) = name { entry.1 = Some(n); }
                                    entry.2.push_str(&arguments_delta);
                                }
                                actx_lm::types::StreamChunk::Completed { finish_reason: fr, usage } => {
                                    finish_reason = fr;
                                    if usage.is_some() {
                                        total_usage = usage;
                                    }
                                }
                            }
                        }
                    }

                    if !streamed_tools.is_empty() {
                        response_tool_calls = streamed_tools.into_values().filter_map(|(id, name, args)| {
                            name.map(|n| actx_lm::types::ToolCall {
                                id: id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                                name: n,
                                arguments: args,
                            })
                        }).collect();
                    }
                }
            }

            // Fallback to chat_complete if streaming was not executed or produced no content and no tool calls
            if response_content.is_empty() && response_tool_calls.is_empty() {
                let comp_resp = self.client.chat_complete(req).await.map_err(AgentError::LmError)?;
                response_content = comp_resp.content;
                response_tool_calls = comp_resp.tool_calls;
                finish_reason = comp_resp.finish_reason;
                total_usage = comp_resp.usage;
                did_stream_tokens = false;

                if let Some(ref thinking) = comp_resp.thinking {
                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(AgentEvent::Thinking(thinking.clone()));
                    }
                }
            }

            // Check if model emitted tool calls and we're not forced to stop
            if !response_tool_calls.is_empty() && !is_last_turn {
                // Record assistant message with tool calls in working context
                let asst_msg = ChatMessage::assistant_with_tools(&response_content, response_tool_calls.clone());
                working_messages.push(asst_msg);

                for call in &response_tool_calls {
                    tool_calls_count += 1;

                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(AgentEvent::ToolStart {
                            id: call.id.clone(),
                            name: call.name.clone(),
                            arguments: call.arguments.clone(),
                        });
                    }

                    // Execute tool defensively with self-healing
                    let (tool_result_str, is_err) = match self.tools.execute(&call.name, &call.arguments).await {
                        Ok(res) => (res, false),
                        Err(err) => {
                            let msg = format!(
                                "Tool error in '{}': {}. Please correct your parameters and retry if necessary.",
                                call.name, err.message
                            );
                            (msg, true)
                        }
                    };

                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(AgentEvent::ToolEnd {
                            id: call.id.clone(),
                            name: call.name.clone(),
                            result: tool_result_str.clone(),
                            is_error: is_err,
                        });
                    }

                    let tool_msg = ChatMessage::tool(tool_result_str, call.id.clone());
                    working_messages.push(tool_msg);
                }

                // Continue to next turn to allow model to digest tool observation
                continue;
            } else {
                // Model delivered final answer in this turn!
                final_content = response_content;
                final_reason = finish_reason;

                if !did_stream_tokens {
                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(AgentEvent::Delta(final_content.clone()));
                    }
                }

                let final_asst_msg = ChatMessage::assistant(&final_content);
                persisted_new_messages.push(final_asst_msg);

                // Persist session if configured
                if let (Some(sid), Some(ref store)) = (session_id, &self.session_store) {
                    let _ = store.append_messages(sid, &persisted_new_messages).await;
                }

                if let Some(ref tx) = event_tx {
                    let total_tok = total_usage.map(|u| u.total_tokens as usize);
                    let _ = tx.send(AgentEvent::Done {
                        total_turns: total_turns_executed,
                        total_tokens: total_tok,
                    });
                }

                return Ok(AgentResponse {
                    content: final_content,
                    total_turns: total_turns_executed,
                    tool_calls_count,
                    finish_reason: final_reason,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                });
            }
        }

        // If loop finished by exhausting max_turns without returning
        if !final_content.is_empty() {
            persisted_new_messages.push(ChatMessage::assistant(&final_content));
        }
        if let (Some(sid), Some(ref store)) = (session_id, &self.session_store) {
            let _ = store.append_messages(sid, &persisted_new_messages).await;
        }

        Ok(AgentResponse {
            content: final_content,
            total_turns: total_turns_executed,
            tool_calls_count,
            finish_reason: final_reason,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    /// Stream events from agent execution asynchronously
    pub fn stream(
        self: Arc<Self>,
        input: String,
        session_id: Option<String>,
    ) -> (EventReceiver, tokio::task::JoinHandle<Result<AgentResponse, AgentError>>) {
        let (tx, rx) = event_channel();
        let handle = tokio::spawn(async move {
            let res = self.run(&input, session_id.as_deref(), Some(tx.clone())).await;
            if let Err(ref e) = res {
                let _ = tx.send(AgentEvent::Error(e.to_string()));
            }
            res
        });
        (rx, handle)
    }
}
