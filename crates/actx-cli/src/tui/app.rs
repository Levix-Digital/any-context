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

#[derive(Debug)]
pub struct ActiveModelDownload {
    pub model_id: String,
    pub model_name: String,
    pub total_bytes: u64,
    pub downloaded_bytes: std::sync::Arc<std::sync::atomic::AtomicU64>,
    pub is_completed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub is_failed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub error_msg: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    pub cancel_tx: tokio::sync::watch::Sender<bool>,
    pub start_time: std::time::Instant,
}

#[derive(Debug, Clone)]
pub struct OnboardingState {
    pub step: usize, // 0 = Provider & Key, 1 = Document AI Profile, 2 = Initial Folder
    pub selected_provider_idx: usize,
    pub api_key_input: String,
    pub selected_doc_ai_idx: usize, // 0 = Leve & Nuvem (0 MB), 1 = Local Air-Gapped (ONNX)
    pub folder_input: String,
    pub focus_input: bool,
}

impl OnboardingState {
    pub fn new() -> Self {
        Self {
            step: 0,
            selected_provider_idx: 0,
            api_key_input: String::new(),
            selected_doc_ai_idx: 0,
            folder_input: String::new(),
            focus_input: false,
        }
    }
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

    // ReAct & Reasoning Accordion Scrolling
    pub reasoning_scroll_offset: u16,
    pub reasoning_auto_scroll: bool,
    pub reasoning_max_scroll: u16,

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

    // Active Model Download Telemetry & Cancellation
    pub active_model_download: Option<ActiveModelDownload>,

    // Interactive Onboarding Wizard
    pub onboarding_state: Option<OnboardingState>,

    // Agent handles & channels
    pub agent: Option<Arc<Agent>>,
    pub is_generating: bool,

    // Background Synchronization Telemetry
    pub sync_status: Option<any_context_core_rs::storage::WorkspaceSyncStatus>,
    pub last_sync_poll: std::time::Instant,
    pub tick_count: u64,
    pub source_health_warning: Option<String>,
    pub changes_detected: bool,
    pub last_health_check: std::time::Instant,
}

impl App {
    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        self.poll_model_download();
    }

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
            .and_then(|d| d.get_workspace_search_mode(&workspace).ok())
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

        let initial_sync_status = db.as_ref()
            .and_then(|d| d.get_sync_status(&workspace).ok().flatten());

        let onboarding_completed = db.as_ref()
            .and_then(|d| d.get_setting("onboarding_completed").ok().flatten())
            .map(|v| v == "true")
            .unwrap_or(false);

        let onboarding_state = if !onboarding_completed && model == "mock" {
            Some(OnboardingState::new())
        } else {
            None
        };

        let app = Self {
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
            reasoning_scroll_offset: 0,
            reasoning_auto_scroll: true,
            reasoning_max_scroll: 0,
            input_buffer: String::new(),
            cursor_idx: 0,
            input_history: std::collections::HashMap::new(),
            history_index: None,
            current_draft: String::new(),
            accordion_open: false,
            slash_palette_open: false,
            slash_palette_idx: 0,
            slash_matches: Vec::new(),
            palette_navigated: false,
            menu_state: MenuState::default(),
            active_model_download: None,
            onboarding_state,
            agent: agent.map(Arc::new),
            is_generating: false,
            sync_status: initial_sync_status,
            last_sync_poll: std::time::Instant::now(),
            tick_count: 0,
            source_health_warning: None,
            changes_detected: false,
            last_health_check: std::time::Instant::now().checked_sub(std::time::Duration::from_secs(10)).unwrap_or_else(std::time::Instant::now),
        };

        app
    }

    pub fn poll_sync_status(&mut self) {
        if self.last_sync_poll.elapsed() < std::time::Duration::from_millis(400) {
            return;
        }
        self.last_sync_poll = std::time::Instant::now();
        if let Ok(db) = any_context_core_rs::storage::NativeConfigDb::open_default() {
            self.sync_status = db.get_sync_status(&self.active_workspace).unwrap_or(None);
        }
        self.check_source_health();
    }

    pub fn check_source_health(&mut self) {
        if self.last_health_check.elapsed() < std::time::Duration::from_secs(4) {
            return;
        }
        self.last_health_check = std::time::Instant::now();

        if let Ok(db) = any_context_core_rs::storage::NativeConfigDb::open_default() {
            let folders = db.get_workspace_folders(&self.active_workspace).unwrap_or_default();
            let mut missing_count = 0;
            for f in &folders {
                let p = std::path::Path::new(f);
                if !p.exists() && any_context_core_rs::storage::path_healer::try_heal_path(p).is_none() {
                    missing_count += 1;
                }
            }

            if missing_count > 0 {
                self.source_health_warning = Some(format!("⚠️ {} folder(s) unreachable", missing_count));
                self.changes_detected = false;
            } else {
                self.source_health_warning = None;

                let cached_files = db.get_workspace_files_stat_cache(&self.active_workspace).unwrap_or_default();
                let scanner = any_context_core_rs::ingestion::scanner::WorkspaceScanner::new();
                let diff = scanner.scan_and_diff_native(&folders, &cached_files);
                self.changes_detected = !diff.is_up_to_date && diff.missing_folders.is_empty();
            }
        }
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

        // 2. Set new active workspace and reload its persistent configuration
        self.active_workspace = target_ws.to_string();
        if let Ok(db) = any_context_core_rs::storage::NativeConfigDb::open_default() {
            if let Ok(s) = db.get_workspace_search_mode(target_ws) {
                self.search_mode = s;
            }
            if let Ok(m) = db.get_workspace_model(target_ws) {
                self.active_model = m;
            }
            if let Ok(g) = db.get_workspace_grounding_mode(target_ws) {
                self.grounding_mode = g;
            }
            if let Ok(w) = db.get_workspace_web_search(target_ws) {
                self.web_search_enabled = w;
            }
        }

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
            // Start clean with branded welcome message for this workspace
            self.chat_history = vec![Self::create_welcome_message(
                &self.active_workspace,
                &self.active_model,
                &self.grounding_mode,
                &self.search_mode,
                self.web_search_enabled,
            )];
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

    pub fn open_document_ai_menu(&mut self) {
        self.slash_palette_open = false;
        let active_name = self.active_model_download.as_ref().map(|d| d.model_name.as_str());
        self.menu_state.open_document_ai(active_name);
    }

    pub fn is_downloading_model(&self) -> bool {
        self.active_model_download.is_some()
    }

    pub fn cancel_model_download(&mut self) {
        if let Some(dl) = self.active_model_download.take() {
            let _ = dl.cancel_tx.send(true);
            let name = dl.model_name;
            self.chat_history.push(ChatMessageItem {
                role: MessageRole::System,
                content: format!(
                    "🛑 [Download Cancelled]\nDownload of **{}** was stopped at your request.\nNo temporary files remain on disk, and AnyContext continues operating via native heuristics.",
                    name
                ),
                thinking: None,
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            });
            self.scroll_to_bottom();
        }
    }

    pub fn start_model_download(&mut self, model_id: &str) {
        if let Some(spec) = any_context_core_rs::ingestion::OnnxModelManager::find_spec(model_id) {
            let spec_name = spec.name.to_string();
            let spec_mb = spec.size_bytes as f64 / (1024.0 * 1024.0);
            let spec_file = spec.file_name.to_string();
            let spec_clone = spec.clone();

            let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
            let downloaded_bytes = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
            let is_completed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let is_failed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let error_msg = std::sync::Arc::new(std::sync::Mutex::new(None));

            let dl_bytes_bg = downloaded_bytes.clone();
            let is_comp_bg = is_completed.clone();
            let is_fail_bg = is_failed.clone();
            let err_msg_bg = error_msg.clone();

            let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel::<(u64, u64)>();

            tokio::spawn(async move {
                let dl_writer = dl_bytes_bg.clone();
                let drain_handle = tokio::spawn(async move {
                    while let Some((downloaded, _total)) = progress_rx.recv().await {
                        dl_writer.store(downloaded, std::sync::atomic::Ordering::Relaxed);
                    }
                });

                match any_context_core_rs::ingestion::OnnxModelManager::download_model_with_progress(
                    &spec_clone,
                    Some(progress_tx),
                    Some(cancel_rx),
                ).await {
                    Ok(_) => {
                        let _ = drain_handle.await;
                        is_comp_bg.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    Err(e) => {
                        let _ = drain_handle.await;
                        *err_msg_bg.lock().unwrap() = Some(e);
                        is_fail_bg.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            });

            self.active_model_download = Some(ActiveModelDownload {
                model_id: model_id.to_string(),
                model_name: spec_name.clone(),
                total_bytes: spec.size_bytes,
                downloaded_bytes,
                is_completed,
                is_failed,
                error_msg,
                cancel_tx,
                start_time: std::time::Instant::now(),
            });

            self.chat_history.push(ChatMessageItem {
                role: MessageRole::System,
                content: format!(
                    "📥 [ONNX Model Download Started]\nStarting async download of **{}** (~{:.0} MB)...\nFile: `{}`\nDestination: `%LOCALAPPDATA%\\AnyContext\\models\\`\nReal-time progress is shown in the footer. Press [Esc] to cancel anytime.",
                    spec_name, spec_mb, spec_file
                ),
                thinking: None,
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            });
            self.scroll_to_bottom();
        }
    }

    pub fn poll_model_download(&mut self) {
        if let Some(ref dl) = self.active_model_download {
            use std::sync::atomic::Ordering;
            if dl.is_completed.load(Ordering::Relaxed) {
                let name = dl.model_name.clone();
                let file_name = any_context_core_rs::ingestion::OnnxModelManager::find_spec(&dl.model_id)
                    .map(|s| s.file_name)
                    .unwrap_or("model.onnx");
                self.chat_history.push(ChatMessageItem {
                    role: MessageRole::System,
                    content: format!(
                        "✔ [Model Successfully Attached]\n**{}** is installed on disk and active!\nFile: `%LOCALAPPDATA%\\AnyContext\\models\\{}`.\nNeural pipeline is now active for this task.",
                        name, file_name
                    ),
                    thinking: None,
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                });
                self.scroll_to_bottom();
                self.active_model_download = None;
            } else if dl.is_failed.load(Ordering::Relaxed) {
                let name = dl.model_name.clone();
                let err_text = dl.error_msg.lock().unwrap().clone().unwrap_or_else(|| "Unknown error".to_string());
                if err_text.contains("cancel") || err_text.contains("cancelado") {
                    self.chat_history.push(ChatMessageItem {
                        role: MessageRole::System,
                        content: format!(
                            "🛑 [Download Cancelled]\nDownload of **{}** was cancelled. AnyContext continues operating via native heuristics.",
                            name
                        ),
                        thinking: None,
                        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    });
                } else {
                    self.chat_history.push(ChatMessageItem {
                        role: MessageRole::System,
                        content: format!(
                            "❌ [Download Failed]\nCould not download **{}**: {}.\nAnyContext will continue operating via deterministic fallback without interruption.",
                            name, err_text
                        ),
                        thinking: None,
                        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    });
                }
                self.scroll_to_bottom();
                self.active_model_download = None;
            }
        }
    }

    pub fn open_onboarding(&mut self) {
        self.onboarding_state = Some(OnboardingState::new());
        self.menu_state.is_open = false;
    }

    pub fn finish_onboarding(&mut self) {
        if let Some(onboarding) = self.onboarding_state.take() {
            let db = any_context_core_rs::storage::NativeConfigDb::open_default().ok();
            let provider_names = ["gemini", "openai", "anthropic", "ollama", "mock"];
            let default_models = ["gemini-2.5-flash", "gpt-4o-mini", "claude-3-5-sonnet", "llama3", "mock"];
            let prov = provider_names.get(onboarding.selected_provider_idx).unwrap_or(&"gemini");
            let model = default_models.get(onboarding.selected_provider_idx).unwrap_or(&"gemini-2.5-flash");

            if let Some(ref d) = db {
                let _ = d.set_setting("default_provider", prov);
                let _ = d.set_setting("default_model", model);
                let _ = d.set_setting("onboarding_completed", "true");

                if !onboarding.api_key_input.trim().is_empty() {
                    let key_setting = match *prov {
                        "gemini" => "gemini_api_key",
                        "openai" => "openai_api_key",
                        "anthropic" => "anthropic_api_key",
                        _ => "api_key",
                    };
                    let _ = d.set_setting(key_setting, onboarding.api_key_input.trim());
                }

                if onboarding.selected_doc_ai_idx == 0 {
                    let _ = d.set_setting("enable_vision_llm", "true");
                } else {
                    let _ = d.set_setting("enable_vision_llm", "false");
                }

                if !onboarding.folder_input.trim().is_empty() {
                    let _ = d.add_workspace_folder(&self.active_workspace, onboarding.folder_input.trim());
                }
            }

            self.active_model = model.to_string();
            self.chat_history.push(ChatMessageItem {
                role: MessageRole::System,
                content: format!(
                    "✨ [AnyContext Successfully Configured]\nProvider: `{}` | Model: `{}` | Document AI Profile: `{}`.\nAll set! Type your question below or use /menu to explore features.",
                    prov, model, if onboarding.selected_doc_ai_idx == 0 { "Lightweight Cloud" } else { "Local Air-Gapped" }
                ),
                thinking: None,
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            });
            self.scroll_to_bottom();
        }
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
            if item.id == "model_action:cancel_active" {
                self.cancel_model_download();
                self.menu_state.is_open = false;
            } else if item.is_submenu {
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
                crate::commands::dispatch_slash_command("sync", &["--incremental"], self);
                self.menu_state.is_open = false;
            } else if item.id == "sync_action:force" {
                crate::commands::dispatch_slash_command("sync", &["--force"], self);
                self.menu_state.is_open = false;
            } else if item.id == "sync_action:cancel" {
                crate::commands::dispatch_slash_command("sync", &["cancel"], self);
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
            } else if let Some(_source_id) = item.id.strip_prefix("source_item:") {
                let name_part = item.title.split('.').nth(1).unwrap_or(&item.title);
                let clean_name = name_part.trim().trim_start_matches("📁").trim_start_matches("🌐").trim();
                self.input_buffer = format!("/sources rename \"{}\" ", clean_name);
                self.cursor_idx = self.input_buffer.len();
                self.chat_history.push(ChatMessageItem {
                    role: MessageRole::System,
                    content: format!("✏️ Rename Source: '{}'\nType the new display name and press [Enter] to save instantly.", clean_name),
                    thinking: None,
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                });
                self.scroll_to_bottom();
                self.menu_state.is_open = false;
            } else if item.id == "keys_action:audit" {
                crate::commands::dispatch_slash_command("keys", &["audit"], self);
                self.menu_state.is_open = false;
            } else if item.id == "doc_ai_action:vision_cloud" {
                self.chat_history.push(ChatMessageItem {
                    role: MessageRole::System,
                    content: "ℹ [Document AI: Cloud/VPC Vision]\nTo enable high-precision visual inspection via Cloud/VPC, select an active multimodal model using /model (e.g. gemini-1.5-pro, claude-3-5-sonnet, or gpt-4o). AnyContext sends diagrams, layouts, and images directly to the configured model.".to_string(),
                    thinking: None,
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                });
                self.scroll_to_bottom();
                self.menu_state.is_open = false;
            } else if let Some(model_id) = item.id.strip_prefix("model_action:download:") {
                self.start_model_download(model_id);
                self.menu_state.is_open = false;
            } else if let Some(model_id) = item.id.strip_prefix("model_action:toggle:") {
                if let Some(spec) = any_context_core_rs::ingestion::OnnxModelManager::find_spec(model_id) {
                    self.chat_history.push(ChatMessageItem {
                        role: MessageRole::System,
                        content: format!(
                            "✔ [ONNX Model Active]\n**{}** is installed on disk and active for {}.\nAutomatic resilient fallback: {}.",
                            spec.name, spec.category.as_str(), spec.fallback_description
                        ),
                        thinking: None,
                        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    });
                }
                self.scroll_to_bottom();
                self.menu_state.is_open = false;
            } else if item.id.starts_with("doc_ai_info:") {
                self.chat_history.push(ChatMessageItem {
                    role: MessageRole::System,
                    content: format!("ℹ [{}]\n{}", item.title, item.description),
                    thinking: None,
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                });
                self.scroll_to_bottom();
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
                    "document_ai" => self.open_document_ai_menu(),
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

            if cmd_name.eq_ignore_ascii_case("onboarding") || cmd_name.eq_ignore_ascii_case("setup") {
                self.open_onboarding();
                crate::commands::dispatch_slash_command(cmd_name, &args, self);
                return None;
            }

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
            AgentEvent::RoutingDecision { mode, complexity, intent, confidence, reason } => {
                let badge = if complexity == "Deep" || mode.contains("DeepSearch") {
                    "🧠 [ModelRouter: Deep Search]"
                } else {
                    "⚡ [ModelRouter: Fast RAG]"
                };
                self.current_thinking_buffer.push_str(&format!(
                    "{} (Intent: {}, Conf: {:.0}%)\n• Reason: {}\n\n",
                    badge, intent, confidence * 100.0, reason
                ));
            }
            AgentEvent::Decomposition { sub_queries } => {
                self.status = AppStatus::Thinking;
                self.current_thinking_buffer.push_str("\n🌲 [Deep Search: Decomposing Query into Sub-Queries]:\n");
                for (i, q) in sub_queries.iter().enumerate() {
                    self.current_thinking_buffer.push_str(&format!("  {}. {}\n", i + 1, q));
                }
                self.current_thinking_buffer.push('\n');
            }
            AgentEvent::IterationStart { iteration, max_iterations } => {
                self.status = AppStatus::Thinking;
                self.current_thinking_buffer.push_str(&format!("🔄 [Deep Search Iteration {}/{}]: Batch Retrieval & Reflection...\n", iteration, max_iterations));
            }
            AgentEvent::GapAnalysis { is_sufficient, missing_aspects } => {
                if is_sufficient {
                    self.current_thinking_buffer.push_str("✔ [Evidence Sufficient]: Synthesizing grounded response with citations.\n\n");
                } else {
                    self.current_thinking_buffer.push_str(&format!("🔎 [Gap Analysis - Missing Aspects]: {}\n\n", missing_aspects.join(", ")));
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
