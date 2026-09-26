use actx_agent::{Agent, AgentEvent};
use crate::commands::registry::{autocomplete_commands, SlashCommand};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppStatus {
    Idle,
    Thinking,
    Streaming,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone)]
pub struct ChatMessageItem {
    pub role: MessageRole,
    pub content: String,
    pub thinking: Option<String>,
    pub timestamp: String,
}

pub struct App {
    pub running: bool,
    pub active_workspace: String,
    pub active_model: String,
    pub status: AppStatus,

    // Chat history & Viewport
    pub chat_history: Vec<ChatMessageItem>,
    pub current_stream_buffer: String,
    pub current_thinking_buffer: String,
    pub scroll_offset: u16,

    // Input & Slash Command Palette
    pub input_buffer: String,
    pub cursor_idx: usize,
    pub accordion_open: bool,
    pub slash_palette_open: bool,
    pub slash_palette_idx: usize,
    pub slash_matches: Vec<&'static SlashCommand>,

    // Agent handles & channels
    pub agent: Option<Arc<Agent>>,
    pub is_generating: bool,
}

impl App {
    pub fn new(workspace: String, model: String, agent: Option<Agent>) -> Self {
        let initial_history = vec![ChatMessageItem {
            role: MessageRole::System,
            content: format!(
                "AnyContext (actx) Native Rust Engine ready.\nWorkspace: [{}] | Model: [{}]\nType /help to view available commands.",
                workspace, model
            ),
            thinking: None,
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        }];

        Self {
            running: true,
            active_workspace: workspace,
            active_model: model,
            status: AppStatus::Idle,
            chat_history: initial_history,
            current_stream_buffer: String::new(),
            current_thinking_buffer: String::new(),
            scroll_offset: 0,
            input_buffer: String::new(),
            cursor_idx: 0,
            accordion_open: true,
            slash_palette_open: false,
            slash_palette_idx: 0,
            slash_matches: Vec::new(),
            agent: agent.map(Arc::new),
            is_generating: false,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.input_buffer.insert(self.cursor_idx, c);
        self.cursor_idx += 1;
        self.update_slash_palette();
    }

    pub fn delete_backspace(&mut self) {
        if self.cursor_idx > 0 {
            self.cursor_idx -= 1;
            self.input_buffer.remove(self.cursor_idx);
            self.update_slash_palette();
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_idx > 0 {
            self.cursor_idx -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_idx < self.input_buffer.len() {
            self.cursor_idx += 1;
        }
    }

    pub fn toggle_accordion(&mut self) {
        self.accordion_open = !self.accordion_open;
    }

    pub fn update_slash_palette(&mut self) {
        let trimmed = self.input_buffer.trim();
        if trimmed.starts_with('/') {
            let prefix = trimmed.split_whitespace().next().unwrap_or("/");
            self.slash_matches = autocomplete_commands(prefix);
            self.slash_palette_open = !self.slash_matches.is_empty();
            if self.slash_palette_idx >= self.slash_matches.len() {
                self.slash_palette_idx = 0;
            }
        } else {
            self.slash_palette_open = false;
            self.slash_matches.clear();
            self.slash_palette_idx = 0;
        }
    }

    pub fn palette_up(&mut self) {
        if self.slash_palette_open && !self.slash_matches.is_empty() {
            if self.slash_palette_idx > 0 {
                self.slash_palette_idx -= 1;
            } else {
                self.slash_palette_idx = self.slash_matches.len() - 1;
            }
        } else if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn palette_down(&mut self) {
        if self.slash_palette_open && !self.slash_matches.is_empty() {
            if self.slash_palette_idx + 1 < self.slash_matches.len() {
                self.slash_palette_idx += 1;
            } else {
                self.slash_palette_idx = 0;
            }
        } else {
            self.scroll_offset += 1;
        }
    }

    pub fn complete_selected_slash(&mut self) {
        if self.slash_palette_open && !self.slash_matches.is_empty() {
            let selected = self.slash_matches[self.slash_palette_idx];
            self.input_buffer = format!("/{} ", selected.name);
            self.cursor_idx = self.input_buffer.len();
            self.slash_palette_open = false;
            self.slash_matches.clear();
        }
    }

    pub fn submit_input(&mut self, event_tx: mpsc::UnboundedSender<AgentEvent>) -> Option<String> {
        let text = std::mem::take(&mut self.input_buffer).trim().to_string();
        self.cursor_idx = 0;
        self.slash_palette_open = false;

        if text.is_empty() {
            return None;
        }

        // Handle Slash Commands
        if text.starts_with('/') {
            let mut parts = text.split_whitespace();
            let cmd_name = parts.next().unwrap_or("").trim_start_matches('/');
            let args: Vec<&str> = parts.collect();

            crate::commands::dispatch_slash_command(cmd_name, &args, self);
            return None;
        }

        // Push User Message
        self.chat_history.push(ChatMessageItem {
            role: MessageRole::User,
            content: text.clone(),
            thinking: None,
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        });

        self.current_stream_buffer.clear();
        self.current_thinking_buffer.clear();
        self.is_generating = true;
        self.status = AppStatus::Thinking;

        // Dispatch agent task if agent available
        if let Some(agent) = &self.agent {
            let agent = agent.clone();
            let query = text.clone();
            tokio::spawn(async move {
                let (mut rx, _handle) = agent.stream(query, None);
                while let Some(evt) = rx.recv().await {
                    let _ = event_tx.send(evt);
                }
            });
        } else {
            // Emulate response if agent is not attached
            let event_tx = event_tx.clone();
            tokio::spawn(async move {
                let _ = event_tx.send(AgentEvent::Thinking("Analyzing workspace and query semantics...".into()));
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                let _ = event_tx.send(AgentEvent::Delta("AnyContext native agent response for: ".into()));
                let _ = event_tx.send(AgentEvent::Delta(text));
                let _ = event_tx.send(AgentEvent::Done { total_turns: 1, total_tokens: Some(42) });
            });
        }

        None
    }

    pub fn handle_agent_event(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::Thinking(token) => {
                self.status = AppStatus::Thinking;
                self.current_thinking_buffer.push_str(&token);
            }
            AgentEvent::ToolStart { name, arguments, .. } => {
                self.current_thinking_buffer.push_str(&format!("\n🔧 [Tool Call: {} ({})]\n", name, arguments));
            }
            AgentEvent::ToolEnd { name, result, is_error, .. } => {
                if is_error {
                    self.current_thinking_buffer.push_str(&format!("❌ [Tool Error: {}]: {}\n", name, result));
                } else {
                    self.current_thinking_buffer.push_str(&format!("✔ [Tool Done: {}]\n", name));
                }
            }
            AgentEvent::Delta(token) => {
                self.status = AppStatus::Streaming;
                self.current_stream_buffer.push_str(&token);
            }
            AgentEvent::Done { .. } => {
                self.finalize_assistant_turn();
            }
            AgentEvent::Error(err) => {
                self.status = AppStatus::Error(err.clone());
                self.finalize_assistant_turn();
            }
            _ => {}
        }
    }

    pub fn finalize_assistant_turn(&mut self) {
        if !self.current_stream_buffer.is_empty() || !self.current_thinking_buffer.is_empty() {
            let thinking = if !self.current_thinking_buffer.is_empty() {
                Some(std::mem::take(&mut self.current_thinking_buffer))
            } else {
                None
            };

            let content = std::mem::take(&mut self.current_stream_buffer);

            self.chat_history.push(ChatMessageItem {
                role: MessageRole::Assistant,
                content,
                thinking,
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            });
        }

        self.is_generating = false;
        self.status = AppStatus::Idle;
    }
}
