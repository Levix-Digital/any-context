use actx_agent::{Agent, AgentEvent};
use crate::commands::registry::{autocomplete_commands, SlashCommand};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::tui::menu::MenuState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppStatus {
    Idle,
    Thinking,
    Streaming,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub palette_navigated: bool,

    // Interactive Menu State
    pub menu_state: MenuState,

    // Agent handles & channels
    pub agent: Option<Arc<Agent>>,
    pub is_generating: bool,
}

impl App {
    pub fn new(workspace: String, model: String, agent: Option<Agent>) -> Self {
        let initial_history = vec![ChatMessageItem {
            role: MessageRole::System,
            content: format!(
                "AnyContext (actx) Native Rust Engine ready.\nWorkspace: [{}] | Model: [{}]\nType /menu (or press F1) for interactive menu, /help for commands.",
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
            palette_navigated: false,
            menu_state: MenuState::default(),
            agent: agent.map(Arc::new),
            is_generating: false,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.input_buffer.insert(self.cursor_idx, c);
        self.cursor_idx += 1;
        self.palette_navigated = false;
        self.update_slash_palette();
    }

    pub fn delete_backspace(&mut self) {
        if self.cursor_idx > 0 {
            self.cursor_idx -= 1;
            self.input_buffer.remove(self.cursor_idx);
            self.palette_navigated = false;
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
            self.palette_navigated = false;
        }
    }

    pub fn palette_up(&mut self) {
        if self.slash_palette_open && !self.slash_matches.is_empty() {
            self.palette_navigated = true;
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
            self.palette_navigated = true;
            if self.slash_palette_idx + 1 < self.slash_matches.len() {
                self.slash_palette_idx += 1;
            } else {
                self.slash_palette_idx = 0;
            }
        } else {
            self.scroll_offset += 1;
        }
    }

    pub fn complete_selected_slash(&mut self, execute_if_zero_args: bool) {
        if self.slash_palette_open && !self.slash_matches.is_empty() {
            let selected = self.slash_matches[self.slash_palette_idx];
            let name = selected.name;
            self.slash_palette_open = false;
            self.slash_matches.clear();
            self.palette_navigated = false;

            let zero_arg_commands = [
                "exit", "menu", "clear", "diagnostics", "status", "version",
                "sync", "sources", "models", "keys", "history", "billing",
                "reset-memory", "paste", "check-update", "ocr", "shared",
                "help", "info", "inspect", "onboarding"
            ];

            if execute_if_zero_args && zero_arg_commands.contains(&name) {
                self.input_buffer.clear();
                self.cursor_idx = 0;
                crate::commands::dispatch_slash_command(name, &[], self);
            } else {
                self.input_buffer = format!("/{} ", name);
                self.cursor_idx = self.input_buffer.len();
            }
        }
    }

    pub fn open_menu(&mut self) {
        self.slash_palette_open = false;
        self.menu_state.open_main(&self.active_workspace, &self.active_model);
    }

    pub fn close_menu(&mut self) {
        self.menu_state.is_open = false;
    }

    pub fn menu_up(&mut self) {
        self.menu_state.previous();
    }

    pub fn menu_down(&mut self) {
        self.menu_state.next();
    }

    pub fn menu_back(&mut self) {
        self.menu_state.back(&self.active_workspace, &self.active_model);
    }

    pub fn menu_select(&mut self) {
        if let Some(item) = self.menu_state.selected_item().cloned() {
            if item.is_submenu {
                self.menu_state.open_submenu(&item.id, &self.active_workspace, &self.active_model);
            } else if item.id.starts_with("switch:") {
                let target_ws = item.id.trim_start_matches("switch:");
                crate::commands::dispatch_slash_command("switch", &[target_ws], self);
                self.menu_state.is_open = false;
            } else if item.id.starts_with("model:") {
                let target_m = item.id.trim_start_matches("model:");
                crate::commands::dispatch_slash_command("model", &[target_m], self);
                self.menu_state.is_open = false;
            } else if item.id == "sync_action:incremental" {
                crate::commands::dispatch_slash_command("sync", &[], self);
                self.menu_state.is_open = false;
            } else if item.id == "sync_action:force" {
                crate::commands::dispatch_slash_command("sync", &["--force"], self);
                self.menu_state.is_open = false;
            } else if item.id.starts_with("grounding_action:") {
                let mode = item.id.trim_start_matches("grounding_action:");
                crate::commands::dispatch_slash_command("search", &[mode], self);
                self.menu_state.is_open = false;
            } else if item.id == "sources" {
                crate::commands::dispatch_slash_command("sources", &[], self);
                self.menu_state.is_open = false;
            } else if item.id == "keys" {
                crate::commands::dispatch_slash_command("keys", &[], self);
                self.menu_state.is_open = false;
            } else if item.id == "diagnostics" {
                crate::commands::dispatch_slash_command("diagnostics", &[], self);
                self.menu_state.is_open = false;
            } else if item.id == "history" {
                crate::commands::dispatch_slash_command("history", &[], self);
                self.menu_state.is_open = false;
            } else if item.id == "clear" {
                crate::commands::dispatch_slash_command("clear", &[], self);
                self.menu_state.is_open = false;
            } else if item.id == "exit" {
                self.running = false;
                self.menu_state.is_open = false;
            }
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
