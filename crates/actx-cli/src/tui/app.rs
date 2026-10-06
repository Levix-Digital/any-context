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
    pub grounding_mode: String,
    pub search_mode: String,
    pub web_search_enabled: bool,
    pub status: AppStatus,

    // Chat history & Viewport
    pub chat_history: Vec<ChatMessageItem>,
    pub workspace_chat_buffers: std::collections::HashMap<String, Vec<ChatMessageItem>>,
    pub current_stream_buffer: String,
    pub current_thinking_buffer: String,
    pub scroll_offset: u16,
    pub auto_scroll: bool,
    pub max_scroll: u16,

    // Input & Prompt History
    pub input_buffer: String,
    pub cursor_idx: usize,
    pub input_history: std::collections::HashMap<String, Vec<String>>,
    pub history_index: Option<usize>,
    pub current_draft: String,

    // Slash Palette & Accordion
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
    pub fn create_welcome_message(
        workspace: &str,
        model: &str,
        grounding: &str,
        search: &str,
        web: bool,
    ) -> ChatMessageItem {
        ChatMessageItem {
            role: MessageRole::System,
            content: format!(
                "AnyContext (actx) Native Rust Engine ready.\nWorkspace: [{}] | Model: [{}] | Grounding: [{}] | Search: [{}] | Web: [{}]\nType /menu (or press F1) for interactive menu, /help for commands.",
                workspace, model, grounding.to_uppercase(), search.to_uppercase(), if web { "ON" } else { "OFF" }
            ),
            thinking: None,
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        }
    }

    pub fn new(workspace: String, model: String, agent: Option<Agent>) -> Self {
        let db = any_context_core_rs::storage::NativeConfigDb::open_default().ok();
        let grounding_mode = db.as_ref()
            .and_then(|d| d.get_workspace_grounding_mode(&workspace).ok())
            .unwrap_or_else(|| "strict".to_string());
        let search_mode = db.as_ref()
            .and_then(|d| d.get_setting("search_mode").ok().flatten())
            .unwrap_or_else(|| "auto".to_string());
        let web_search_enabled = db.as_ref()
            .and_then(|d| d.get_workspace_web_search(&workspace).ok())
            .unwrap_or(false);

        let initial_history = vec![Self::create_welcome_message(
            &workspace,
            &model,
            &grounding_mode,
            &search_mode,
            web_search_enabled,
        )];

        let mut app = Self {
            running: true,
            active_workspace: workspace,
            active_model: model,
            grounding_mode,
            search_mode,
            web_search_enabled,
            status: AppStatus::Idle,
            chat_history: initial_history,
            workspace_chat_buffers: std::collections::HashMap::new(),
            current_stream_buffer: String::new(),
            current_thinking_buffer: String::new(),
            scroll_offset: 0,
            auto_scroll: true,
            max_scroll: 0,
            input_buffer: String::new(),
            cursor_idx: 0,
            input_history: std::collections::HashMap::new(),
            history_index: None,
            current_draft: String::new(),
            accordion_open: true,
            slash_palette_open: false,
            slash_palette_idx: 0,
            slash_matches: Vec::new(),
            palette_navigated: false,
            menu_state: MenuState::default(),
            agent: agent.map(Arc::new),
            is_generating: false,
        };

        app.load_session_history_for_workspace();
        app
    }

    pub fn insert_char(&mut self, c: char) {
        if self.cursor_idx > self.input_buffer.len() {
            self.cursor_idx = self.input_buffer.len();
        } else if !self.input_buffer.is_char_boundary(self.cursor_idx) {
            while self.cursor_idx > 0 && !self.input_buffer.is_char_boundary(self.cursor_idx) {
                self.cursor_idx -= 1;
            }
        }
        self.input_buffer.insert(self.cursor_idx, c);
        self.cursor_idx += c.len_utf8();
        self.palette_navigated = false;
        self.update_slash_palette();
    }

    pub fn delete_backspace(&mut self) {
        if self.cursor_idx > 0 {
            let prev_idx = self.input_buffer[..self.cursor_idx]
                .char_indices()
                .last()
                .map(|(idx, _)| idx);
            if let Some(idx) = prev_idx {
                self.input_buffer.remove(idx);
                self.cursor_idx = idx;
                self.palette_navigated = false;
                self.update_slash_palette();
            }
        }
    }

    pub fn delete_forward(&mut self) {
        if self.cursor_idx < self.input_buffer.len() {
            self.input_buffer.remove(self.cursor_idx);
            self.palette_navigated = false;
            self.update_slash_palette();
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_idx > 0 {
            if let Some((prev_idx, _)) = self.input_buffer[..self.cursor_idx].char_indices().last() {
                self.cursor_idx = prev_idx;
            } else {
                self.cursor_idx = 0;
            }
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_idx < self.input_buffer.len() {
            if let Some((next_offset, _)) = self.input_buffer[self.cursor_idx..].char_indices().nth(1) {
                self.cursor_idx += next_offset;
            } else {
                self.cursor_idx = self.input_buffer.len();
            }
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

    pub fn scroll_up(&mut self, lines: u16) {
        self.auto_scroll = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    pub fn scroll_down(&mut self, lines: u16) {
        self.scroll_offset = (self.scroll_offset + lines).min(self.max_scroll);
        if self.scroll_offset >= self.max_scroll {
            self.auto_scroll = true;
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.auto_scroll = false;
        self.scroll_offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.auto_scroll = true;
        self.scroll_offset = self.max_scroll;
    }

    pub fn history_up(&mut self) {
        let history = self.input_history.entry(self.active_workspace.clone()).or_default();
        if history.is_empty() {
            return;
        }

        match self.history_index {
            None => {
                self.current_draft = self.input_buffer.clone();
                let last_idx = history.len() - 1;
                self.history_index = Some(last_idx);
                self.input_buffer = history[last_idx].clone();
                self.cursor_idx = self.input_buffer.len();
            }
            Some(idx) => {
                if idx > 0 {
                    let prev_idx = idx - 1;
                    self.history_index = Some(prev_idx);
                    self.input_buffer = history[prev_idx].clone();
                    self.cursor_idx = self.input_buffer.len();
                }
            }
        }
    }

    pub fn history_down(&mut self) {
        let history = self.input_history.entry(self.active_workspace.clone()).or_default();
        if let Some(idx) = self.history_index {
            if idx + 1 < history.len() {
                let next_idx = idx + 1;
                self.history_index = Some(next_idx);
                self.input_buffer = history[next_idx].clone();
                self.cursor_idx = self.input_buffer.len();
            } else {
                self.history_index = None;
                self.input_buffer = std::mem::take(&mut self.current_draft);
                self.cursor_idx = self.input_buffer.len();
            }
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
        } else {
            self.history_up();
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
            self.history_down();
        }
    }

    pub fn rebuild_agent(&mut self) {
        if let Ok((provider, _)) = crate::engine::resolve_lm_provider(Some(&self.active_model), Some(&self.active_workspace)) {
            let ws = self.active_workspace.clone();
            let model = self.active_model.clone();
            let mode = self.grounding_mode.clone();
            let search = self.search_mode.clone();
            let web = self.web_search_enabled;

            if let Ok(new_agent) = crate::engine::build_agent_sync(provider, &model, &ws, &mode, &search, web) {
                self.agent = Some(Arc::new(new_agent));
            }
        }
    }

    pub fn load_session_history_for_workspace(&mut self) {
        let db_path = any_context_core_rs::storage::get_default_settings_db_path();
        if let Ok(store) = actx_agent::SqliteSessionStore::open(&db_path, 50) {
            let session_id = format!("ws_{}", self.active_workspace);
            let msgs = store.get_messages_sync(&session_id).unwrap_or_default();

            if !msgs.is_empty() {
                for m in msgs {
                    let role = match m.role {
                        actx_lm::types::Role::User => MessageRole::User,
                        actx_lm::types::Role::Assistant => MessageRole::Assistant,
                        _ => MessageRole::System,
                    };
                    if role == MessageRole::System && m.content.starts_with("You are AnyContext") {
                        continue;
                    }
                    self.chat_history.push(ChatMessageItem {
                        role,
                        content: m.content,
                        thinking: None,
                        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    });
                }
                self.scroll_to_bottom();
            }
        }
    }

    pub fn switch_to_workspace(&mut self, target_ws: &str) {
        if self.active_workspace == target_ws {
            return;
        }

        // 1. Save current active workspace's visual chat buffer
        let current_history = std::mem::take(&mut self.chat_history);
        self.workspace_chat_buffers.insert(self.active_workspace.clone(), current_history);

        // 2. Set new active workspace
        self.active_workspace = target_ws.to_string();

        // 3. Reset ephemeral stream/thinking buffers and scroll
        self.current_stream_buffer.clear();
        self.current_thinking_buffer.clear();
        self.scroll_offset = 0;
        self.max_scroll = 0;
        self.auto_scroll = true;
        self.status = AppStatus::Idle;
        self.history_index = None;
        self.current_draft.clear();

        // 4. Restore existing view buffer if already in memory
        if let Some(buffered) = self.workspace_chat_buffers.remove(target_ws) {
            self.chat_history = buffered;
        } else {
            // First time accessing target_ws in this session:
            // Load messages from SQLite session store or initialize with clean welcome message
            let mut msgs_to_display = Vec::new();
            let db_path = any_context_core_rs::storage::get_default_settings_db_path();
            if let Ok(store) = actx_agent::SqliteSessionStore::open(&db_path, 50) {
                let session_id = format!("ws_{}", self.active_workspace);
                if let Ok(msgs) = store.get_messages_sync(&session_id) {
                    for m in msgs {
                        let role = match m.role {
                            actx_lm::types::Role::User => MessageRole::User,
                            actx_lm::types::Role::Assistant => MessageRole::Assistant,
                            _ => MessageRole::System,
                        };
                        if role == MessageRole::System && m.content.starts_with("You are AnyContext") {
                            continue;
                        }
                        msgs_to_display.push(ChatMessageItem {
                            role,
                            content: m.content,
                            thinking: None,
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                        });
                    }
                }
            }

            if msgs_to_display.is_empty() {
                self.chat_history = vec![Self::create_welcome_message(
                    &self.active_workspace,
                    &self.active_model,
                    &self.grounding_mode,
                    &self.search_mode,
                    self.web_search_enabled,
                )];
            } else {
                self.chat_history = msgs_to_display;
            }
        }

        self.scroll_to_bottom();
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
                "help", "info", "inspect", "onboarding", "switch", "model",
                "mode", "source", "workspace", "search"
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

    pub fn open_workspaces_menu(&mut self) {
        self.slash_palette_open = false;
        self.menu_state.open_workspaces(&self.active_workspace);
    }

    pub fn open_models_menu(&mut self) {
        self.slash_palette_open = false;
        self.menu_state.open_models(&self.active_model);
    }

    pub fn open_sync_menu(&mut self) {
        self.slash_palette_open = false;
        self.menu_state.open_sync();
    }

    pub fn open_grounding_menu(&mut self) {
        self.slash_palette_open = false;
        self.menu_state.open_grounding();
    }

    pub fn open_search_menu(&mut self) {
        self.slash_palette_open = false;
        self.menu_state.open_search();
    }

    pub fn open_sources_menu(&mut self) {
        self.slash_palette_open = false;
        self.menu_state.open_sources(&self.active_workspace);
    }

    pub fn open_keys_menu(&mut self) {
        self.slash_palette_open = false;
        self.menu_state.open_keys();
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
            } else if item.id == "workspace_tip" {
                self.input_buffer = "/switch --delete ".to_string();
                self.cursor_idx = self.input_buffer.len();
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
                crate::commands::dispatch_slash_command("mode", &[mode], self);
                self.menu_state.is_open = false;
            } else if item.id.starts_with("search_action:") {
                let mode = item.id.trim_start_matches("search_action:");
                crate::commands::dispatch_slash_command("search", &[mode], self);
                self.menu_state.is_open = false;
            } else if item.id == "sources_action:active" {
                crate::commands::dispatch_slash_command("sources", &["active"], self);
                self.menu_state.is_open = false;
            } else if item.id == "sources_action:all" {
                crate::commands::dispatch_slash_command("sources", &["--all"], self);
                self.menu_state.is_open = false;
            } else if item.id == "sources_action:inspect" {
                crate::commands::dispatch_slash_command("inspect", &[], self);
                self.menu_state.is_open = false;
            } else if item.id == "keys_action:audit" {
                crate::commands::dispatch_slash_command("keys", &["audit"], self);
                self.menu_state.is_open = false;
            } else if item.id == "sources" {
                self.open_sources_menu();
            } else if item.id == "keys" {
                self.open_keys_menu();
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

    pub fn to_execution_context(&self) -> any_context_core_rs::commands::ExecutionContext {
        any_context_core_rs::commands::ExecutionContext {
            active_workspace: self.active_workspace.clone(),
            active_model: self.active_model.clone(),
            grounding_mode: self.grounding_mode.clone(),
            search_mode: self.search_mode.clone(),
            web_search_enabled: self.web_search_enabled,
        }
    }

    pub fn apply_command_result(&mut self, res: any_context_core_rs::commands::CommandResult) {
        if let Some(m) = res.state_updates.active_model {
            self.active_model = m;
        }
        if let Some(g) = res.state_updates.grounding_mode {
            self.grounding_mode = g;
        }
        if let Some(s) = res.state_updates.search_mode {
            self.search_mode = s;
        }
        if let Some(w) = res.state_updates.web_search_enabled {
            self.web_search_enabled = w;
        }

        match res.action {
            any_context_core_rs::commands::CommandAction::Exit => {
                self.running = false;
            }
            any_context_core_rs::commands::CommandAction::ClearChat => {
                self.chat_history.clear();
                self.workspace_chat_buffers.remove(&self.active_workspace);
                self.current_stream_buffer.clear();
                self.current_thinking_buffer.clear();
                self.scroll_offset = 0;
                self.max_scroll = 0;
                self.auto_scroll = true;
                self.status = AppStatus::Idle;
            }
            any_context_core_rs::commands::CommandAction::OpenMenu(ref menu_name) => {
                match menu_name.as_str() {
                    "workspaces" => self.open_workspaces_menu(),
                    "models" => self.open_models_menu(),
                    "sync" => self.open_sync_menu(),
                    "grounding" => self.open_grounding_menu(),
                    "search" => self.open_search_menu(),
                    "sources" => self.open_sources_menu(),
                    "keys" => self.open_keys_menu(),
                    _ => self.open_menu(),
                }
            }
            any_context_core_rs::commands::CommandAction::SwitchWorkspace(ref ws) => {
                self.switch_to_workspace(ws);
                self.rebuild_agent();
            }
            any_context_core_rs::commands::CommandAction::RebuildAgent => {
                self.rebuild_agent();
            }
            any_context_core_rs::commands::CommandAction::None => {
                if let Some(ref ws) = res.state_updates.active_workspace {
                    if ws != &self.active_workspace {
                        self.switch_to_workspace(ws);
                        self.rebuild_agent();
                    }
                }
            }
        }

        if !res.message.is_empty() {
            self.chat_history.push(ChatMessageItem {
                role: MessageRole::System,
                content: res.message,
                thinking: None,
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            });
            self.scroll_to_bottom();
        }
    }

    pub fn submit_input(&mut self, event_tx: mpsc::UnboundedSender<AgentEvent>) -> Option<String> {
        let text = std::mem::take(&mut self.input_buffer).trim().to_string();
        self.cursor_idx = 0;
        self.slash_palette_open = false;

        if text.is_empty() {
            return None;
        }

        // Record in prompt history for this workspace
        let history = self.input_history.entry(self.active_workspace.clone()).or_default();
        if history.last() != Some(&text) {
            history.push(text.clone());
        }
        self.history_index = None;
        self.current_draft.clear();

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
        self.scroll_to_bottom();

        self.current_stream_buffer.clear();
        self.current_thinking_buffer.clear();
        self.is_generating = true;
        self.status = AppStatus::Thinking;

        // Dispatch agent task if agent available
        if let Some(agent) = &self.agent {
            let agent = agent.clone();
            let query = text.clone();
            let session_id = format!("ws_{}", self.active_workspace);
            tokio::spawn(async move {
                let (mut rx, _handle) = agent.stream(query, Some(session_id));
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

                self.chat_history.push(ChatMessageItem {
                    role: MessageRole::System,
                    content: format!("❌ Error: {}", err),
                    thinking: None,
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                });
                self.scroll_to_bottom();
                self.is_generating = false;
                self.status = AppStatus::Error(err);
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
