use any_context_core_rs::storage::NativeConfigDb;

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub icon: String,
    pub badge: Option<String>,
    pub shortcut: Option<String>,
    pub is_submenu: bool,
}

#[derive(Debug, Clone)]
pub struct MenuState {
    pub is_open: bool,
    pub current_menu_id: String,
    pub menu_history: Vec<(String, usize)>, // (menu_id, previous_selected_index)
    pub selected_idx: usize,
    pub items: Vec<MenuItem>,
    pub breadcrumbs: Vec<String>,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            is_open: false,
            current_menu_id: "main".to_string(),
            menu_history: Vec::new(),
            selected_idx: 0,
            items: Vec::new(),
            breadcrumbs: vec!["Menu Principal".to_string()],
        }
    }
}

impl MenuState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_main(&mut self, active_workspace: &str, active_model: &str) {
        self.is_open = true;
        self.current_menu_id = "main".to_string();
        self.menu_history.clear();
        self.selected_idx = 0;
        self.breadcrumbs = vec!["Menu Principal".to_string()];
        self.items = build_main_menu(active_workspace, active_model);
    }

    pub fn open_submenu(&mut self, menu_id: &str, active_workspace: &str, active_model: &str) {
        self.menu_history.push((self.current_menu_id.clone(), self.selected_idx));
        self.current_menu_id = menu_id.to_string();
        self.selected_idx = 0;

        match menu_id {
            "workspaces" => {
                self.breadcrumbs = vec!["Main Menu".to_string(), "Workspaces".to_string()];
                self.items = build_workspaces_menu(active_workspace);
            }
            "models" => {
                self.breadcrumbs = vec!["Main Menu".to_string(), "AI Models".to_string()];
                self.items = build_models_menu(active_model);
            }
            "sync" => {
                self.breadcrumbs = vec!["Main Menu".to_string(), "Synchronization".to_string()];
                self.items = build_sync_menu();
            }
            "grounding" => {
                self.breadcrumbs = vec!["Main Menu".to_string(), "Grounding Strategy".to_string()];
                self.items = build_grounding_menu();
            }
            "search" => {
                self.breadcrumbs = vec!["Main Menu".to_string(), "Search Depth".to_string()];
                self.items = build_search_menu();
            }
            "sources" => {
                self.breadcrumbs = vec!["Main Menu".to_string(), "Sources & Documents".to_string()];
                self.items = build_sources_menu(active_workspace);
            }
            "keys" => {
                self.breadcrumbs = vec!["Main Menu".to_string(), "API Credentials".to_string()];
                self.items = build_keys_menu();
            }
            "document_ai" => {
                self.breadcrumbs = vec!["Main Menu".to_string(), "Document AI & Local Models".to_string()];
                self.items = build_document_ai_menu(None);
            }
            "doc_ai:ingestion" => {
                self.breadcrumbs = vec!["Main Menu".to_string(), "Document AI".to_string(), "Ingestion Classification".to_string()];
                self.items = build_doc_ai_ingestion_menu();
            }
            "doc_ai:scans" => {
                self.breadcrumbs = vec!["Main Menu".to_string(), "Document AI".to_string(), "Scan Sentinel".to_string()];
                self.items = build_doc_ai_scans_menu();
            }
            "doc_ai:queries" => {
                self.breadcrumbs = vec!["Main Menu".to_string(), "Document AI".to_string(), "Query Routing".to_string()];
                self.items = build_doc_ai_queries_menu();
            }
            "doc_ai:vision" => {
                self.breadcrumbs = vec!["Main Menu".to_string(), "Document AI".to_string(), "Document Vision".to_string()];
                self.items = build_doc_ai_vision_menu();
            }
            "doc_ai:store_status" => {
                self.breadcrumbs = vec!["Main Menu".to_string(), "Document AI".to_string(), "Storage Summary".to_string()];
                self.items = build_doc_ai_store_menu();
            }
            _ => {
                self.breadcrumbs = vec!["Main Menu".to_string()];
                self.items = build_main_menu(active_workspace, active_model);
            }
        }
    }

    pub fn open_workspaces(&mut self, active_workspace: &str) {
        self.is_open = true;
        self.current_menu_id = "workspaces".to_string();
        self.menu_history.clear();
        self.selected_idx = 0;
        self.breadcrumbs = vec!["Main Menu".to_string(), "Workspaces".to_string()];
        self.items = build_workspaces_menu(active_workspace);
    }

    pub fn open_models(&mut self, active_model: &str) {
        self.is_open = true;
        self.current_menu_id = "models".to_string();
        self.menu_history.clear();
        self.selected_idx = 0;
        self.breadcrumbs = vec!["Main Menu".to_string(), "AI Models".to_string()];
        self.items = build_models_menu(active_model);
    }

    pub fn open_sync(&mut self) {
        self.is_open = true;
        self.current_menu_id = "sync".to_string();
        self.menu_history.clear();
        self.selected_idx = 0;
        self.breadcrumbs = vec!["Main Menu".to_string(), "Synchronization".to_string()];
        self.items = build_sync_menu();
    }

    pub fn open_grounding(&mut self) {
        self.is_open = true;
        self.current_menu_id = "grounding".to_string();
        self.menu_history.clear();
        self.selected_idx = 0;
        self.breadcrumbs = vec!["Main Menu".to_string(), "Grounding Strategy".to_string()];
        self.items = build_grounding_menu();
    }

    pub fn open_search(&mut self) {
        self.is_open = true;
        self.current_menu_id = "search".to_string();
        self.menu_history.clear();
        self.selected_idx = 0;
        self.breadcrumbs = vec!["Main Menu".to_string(), "Search Depth".to_string()];
        self.items = build_search_menu();
    }

    pub fn open_sources(&mut self, active_workspace: &str) {
        self.is_open = true;
        self.current_menu_id = "sources".to_string();
        self.menu_history.clear();
        self.selected_idx = 0;
        self.breadcrumbs = vec!["Main Menu".to_string(), "Sources & Documents".to_string()];
        self.items = build_sources_menu(active_workspace);
    }

    pub fn open_keys(&mut self) {
        self.is_open = true;
        self.current_menu_id = "keys".to_string();
        self.menu_history.clear();
        self.selected_idx = 0;
        self.breadcrumbs = vec!["Main Menu".to_string(), "API Credentials".to_string()];
        self.items = build_keys_menu();
    }

    pub fn open_document_ai(&mut self, active_download_name: Option<&str>) {
        self.is_open = true;
        self.current_menu_id = "document_ai".to_string();
        self.menu_history.clear();
        self.selected_idx = 0;
        self.breadcrumbs = vec!["Main Menu".to_string(), "Document AI & Local Models".to_string()];
        self.items = build_document_ai_menu(active_download_name);
    }

    pub fn back(&mut self, active_workspace: &str, active_model: &str) -> bool {
        if let Some((prev_menu, prev_idx)) = self.menu_history.pop() {
            self.current_menu_id = prev_menu.clone();
            self.selected_idx = prev_idx;

            match prev_menu.as_str() {
                "main" => {
                    self.breadcrumbs = vec!["Main Menu".to_string()];
                    self.items = build_main_menu(active_workspace, active_model);
                }
                "document_ai" => {
                    self.breadcrumbs = vec!["Main Menu".to_string(), "Document AI & Local Models".to_string()];
                    self.items = build_document_ai_menu(None);
                }
                "workspaces" => {
                    self.breadcrumbs = vec!["Main Menu".to_string(), "Workspaces".to_string()];
                    self.items = build_workspaces_menu(active_workspace);
                }
                "models" => {
                    self.breadcrumbs = vec!["Main Menu".to_string(), "AI Models".to_string()];
                    self.items = build_models_menu(active_model);
                }
                "sync" => {
                    self.breadcrumbs = vec!["Main Menu".to_string(), "Synchronization".to_string()];
                    self.items = build_sync_menu();
                }
                "grounding" => {
                    self.breadcrumbs = vec!["Main Menu".to_string(), "Grounding Strategy".to_string()];
                    self.items = build_grounding_menu();
                }
                "search" => {
                    self.breadcrumbs = vec!["Main Menu".to_string(), "Search Depth".to_string()];
                    self.items = build_search_menu();
                }
                "sources" => {
                    self.breadcrumbs = vec!["Main Menu".to_string(), "Sources & Documents".to_string()];
                    self.items = build_sources_menu(active_workspace);
                }
                "keys" => {
                    self.breadcrumbs = vec!["Main Menu".to_string(), "API Credentials".to_string()];
                    self.items = build_keys_menu();
                }
                _ => {
                    self.breadcrumbs = vec!["Main Menu".to_string(), prev_menu];
                }
            }
            true
        } else {
            self.is_open = false;
            false
        }
    }

    pub fn next(&mut self) {
        if !self.items.is_empty() {
            self.selected_idx = (self.selected_idx + 1) % self.items.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.items.is_empty() {
            if self.selected_idx == 0 {
                self.selected_idx = self.items.len() - 1;
            } else {
                self.selected_idx -= 1;
            }
        }
    }

    pub fn selected_item(&self) -> Option<&MenuItem> {
        self.items.get(self.selected_idx)
    }
}

pub fn build_main_menu(active_workspace: &str, active_model: &str) -> Vec<MenuItem> {
    vec![
        MenuItem {
            id: "workspaces".to_string(),
            title: "Workspaces & Context Folders".to_string(),
            description: "List, switch, or select context workspaces".to_string(),
            icon: "📂".to_string(),
            badge: Some(format!("[{}]", active_workspace)),
            shortcut: Some("/switch".to_string()),
            is_submenu: true,
        },
        MenuItem {
            id: "sync".to_string(),
            title: "Synchronization & Reindexing".to_string(),
            description: "Run incremental or forced file synchronization".to_string(),
            icon: "🔄".to_string(),
            badge: Some("[SHA-256]".to_string()),
            shortcut: Some("/sync".to_string()),
            is_submenu: true,
        },
        MenuItem {
            id: "sources".to_string(),
            title: "Monitored Sources & Web Portals".to_string(),
            description: "List local directories and indexed documentation URLs".to_string(),
            icon: "📁".to_string(),
            badge: None,
            shortcut: Some("/sources".to_string()),
            is_submenu: true,
        },
        MenuItem {
            id: "models".to_string(),
            title: "AI Models & LLM Providers".to_string(),
            description: "Select active model (OpenAI, Anthropic, Gemini, DeepSeek, Groq, Ollama, Mock)".to_string(),
            icon: "🤖".to_string(),
            badge: Some(format!("[{}]", active_model)),
            shortcut: Some("/model".to_string()),
            is_submenu: true,
        },
        MenuItem {
            id: "grounding".to_string(),
            title: "Grounding Strategy".to_string(),
            description: "Configure grounding policy: Strict, Hybrid, or Proactive".to_string(),
            icon: "🛡️".to_string(),
            badge: None,
            shortcut: Some("/mode".to_string()),
            is_submenu: true,
        },
        MenuItem {
            id: "search".to_string(),
            title: "Search Depth (/search)".to_string(),
            description: "Configure workspace search mode: Auto, Fast, or Deep".to_string(),
            icon: "🎯".to_string(),
            badge: None,
            shortcut: Some("/search".to_string()),
            is_submenu: true,
        },
        MenuItem {
            id: "document_ai".to_string(),
            title: "Document AI & Local Models".to_string(),
            description: "Local classification <1µs, 2D spatial layout, and vision model downloads".to_string(),
            icon: "👁️".to_string(),
            badge: Some("[Zero-Overhead]".to_string()),
            shortcut: None,
            is_submenu: true,
        },
        MenuItem {
            id: "keys".to_string(),
            title: "Credentials & API Keys".to_string(),
            description: "Audit presence of configured provider API keys".to_string(),
            icon: "🔑".to_string(),
            badge: None,
            shortcut: Some("/keys".to_string()),
            is_submenu: true,
        },
        MenuItem {
            id: "diagnostics".to_string(),
            title: "Diagnostics & System Health".to_string(),
            description: "Inspect health of 100% Rust engine, SQLite, and LanceDB".to_string(),
            icon: "🩺".to_string(),
            badge: Some("[100% Rust]".to_string()),
            shortcut: Some("/diagnostics".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "history".to_string(),
            title: "Conversation History".to_string(),
            description: "Display turn count and session memory status".to_string(),
            icon: "📜".to_string(),
            badge: None,
            shortcut: Some("/history".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "clear".to_string(),
            title: "Clear Chat View".to_string(),
            description: "Clear visual chat history while preserving engine state".to_string(),
            icon: "🧹".to_string(),
            badge: None,
            shortcut: Some("/clear".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "exit".to_string(),
            title: "Exit AnyContext Session".to_string(),
            description: "Save state and exit application, restoring terminal".to_string(),
            icon: "🚪".to_string(),
            badge: None,
            shortcut: Some("/exit".to_string()),
            is_submenu: false,
        },
    ]
}

pub fn build_workspaces_menu(active_workspace: &str) -> Vec<MenuItem> {
    let mut items = Vec::new();

    let names = NativeConfigDb::open_default()
        .and_then(|db| db.list_workspace_names())
        .unwrap_or_else(|_| vec!["Default".to_string()]);

    for ws in names {
        let is_active = ws.eq_ignore_ascii_case(active_workspace);
        items.push(MenuItem {
            id: format!("switch:{}", ws),
            title: format!("Workspace: {}", ws),
            description: if is_active {
                "Currently active workspace".to_string()
            } else {
                format!("Switch to workspace '{}'", ws)
            },
            icon: "📂".to_string(),
            badge: if is_active {
                Some("[Active]".to_string())
            } else {
                None
            },
            shortcut: None,
            is_submenu: false,
        });
    }

    items.push(MenuItem {
        id: "workspace_tip".to_string(),
        title: "💡 Delete Workspace (Tip)".to_string(),
        description: "To delete a workspace, run: /switch --delete <name>".to_string(),
        icon: "🗑️".to_string(),
        badge: Some("[Tip]".to_string()),
        shortcut: None,
        is_submenu: false,
    });

    items
}

pub fn build_models_menu(active_model: &str) -> Vec<MenuItem> {
    let models = &[
        ("gemini-3.8-flash", "Google - Ultra-low latency and multimodality"),
        ("gpt-4o-mini", "OpenAI - Fast, cost-efficient, and intelligent"),
        ("gpt-4o", "OpenAI - High-capability multimodal flagship"),
        ("claude-3-5-sonnet-20241022", "Anthropic - Advanced reasoning and coding"),
        ("claude-3-5-haiku-20241022", "Anthropic - Ultra-fast and efficient"),
        ("gemini-1.5-pro", "Google - Extended context window"),
        ("deepseek-chat", "DeepSeek - High cost-benefit for coding"),
        ("deepseek-reasoner", "DeepSeek R1 - Step-by-step analytical reasoning"),
        ("llama-3.3-70b-versatile", "Groq - Instant inference on LPU hardware"),
        ("ollama/qwen2.5-coder", "Local - Offline model via local Ollama endpoint"),
        ("mock", "Mock - Instant offline simulation for testing and demos"),
    ];

    models
        .iter()
        .map(|(m, desc)| {
            let is_active = m.eq_ignore_ascii_case(active_model);
            MenuItem {
                id: format!("model:{}", m),
                title: m.to_string(),
                description: desc.to_string(),
                icon: "🤖".to_string(),
                badge: if is_active {
                    Some("[Active]".to_string())
                } else {
                    None
                },
                shortcut: None,
                is_submenu: false,
            }
        })
        .collect()
}

pub fn build_sync_menu() -> Vec<MenuItem> {
    vec![
        MenuItem {
            id: "sync_action:incremental".to_string(),
            title: "Incremental Synchronization (Default)".to_string(),
            description: "Only analyzes new and modified files via SHA-256 hash ($0.00)".to_string(),
            icon: "⚡".to_string(),
            badge: Some("[Fast]".to_string()),
            shortcut: Some("/sync".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "sync_action:force".to_string(),
            title: "Forced Synchronization (--force)".to_string(),
            description: "Recalculates all hashes and fully re-indexes the workspace".to_string(),
            icon: "🔄".to_string(),
            badge: Some("[Full]".to_string()),
            shortcut: Some("/sync --force".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "sync_action:cancel".to_string(),
            title: "Cancel In-Progress Sync".to_string(),
            description: "Stops indexing/crawler worker and resets sync status".to_string(),
            icon: "🛑".to_string(),
            badge: Some("[Stop]".to_string()),
            shortcut: Some("/sync cancel".to_string()),
            is_submenu: false,
        },
    ]
}

pub fn build_grounding_menu() -> Vec<MenuItem> {
    vec![
        MenuItem {
            id: "grounding_action:strict".to_string(),
            title: "Strict Mode (100% Verified Facts)".to_string(),
            description: "Responses strictly grounded in indexed documents, zero external speculation".to_string(),
            icon: "🔒".to_string(),
            badge: Some("[Audit]".to_string()),
            shortcut: Some("/mode strict".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "grounding_action:hybrid".to_string(),
            title: "Hybrid Mode (Dual-Layer)".to_string(),
            description: "Layer 1: workspace facts + Layer 2: identified external suggestions".to_string(),
            icon: "⚖️".to_string(),
            badge: Some("[Balanced]".to_string()),
            shortcut: Some("/mode hybrid".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "grounding_action:proactive".to_string(),
            title: "Proactive Mode (Synthesis & Recommendations)".to_string(),
            description: "Broad synthesis, research insights, and proactive source recommendations".to_string(),
            icon: "💡".to_string(),
            badge: Some("[Research]".to_string()),
            shortcut: Some("/mode proactive".to_string()),
            is_submenu: false,
        },
    ]
}

pub fn build_search_menu() -> Vec<MenuItem> {
    vec![
        MenuItem {
            id: "search_action:auto".to_string(),
            title: "Auto Search Depth (Smart Default)".to_string(),
            description: "Automatically switches between fast and reflexive search based on query complexity".to_string(),
            icon: "🎯".to_string(),
            badge: Some("[Default]".to_string()),
            shortcut: Some("/search auto".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "search_action:fast".to_string(),
            title: "Fast RAG (Single-Turn <50ms)".to_string(),
            description: "Direct ultra-low latency vector and lexical search".to_string(),
            icon: "⚡".to_string(),
            badge: Some("[Ultra Fast]".to_string()),
            shortcut: Some("/search fast".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "search_action:deep".to_string(),
            title: "Deep Search (Reflexive ReAct)".to_string(),
            description: "Multi-turn reasoning with subtask decomposition and self-correction".to_string(),
            icon: "🧠".to_string(),
            badge: Some("[Deep]".to_string()),
            shortcut: Some("/search deep".to_string()),
            is_submenu: false,
        },
    ]
}

pub fn build_sources_menu(active_workspace: &str) -> Vec<MenuItem> {
    let db = NativeConfigDb::open_default().ok();
    let folders = db.as_ref().and_then(|d| d.get_workspace_folders(active_workspace).ok()).unwrap_or_default();
    let urls = db.as_ref().and_then(|d| d.get_workspace_web_urls(active_workspace).ok()).unwrap_or_default();
    let total_active = folders.len() + urls.len();

    vec![
        MenuItem {
            id: "sources_action:active".to_string(),
            title: format!("Workspace Sources '{}' ({})", active_workspace, total_active),
            description: "Displays all folders and web portals indexed in the current workspace".to_string(),
            icon: "📂".to_string(),
            badge: Some(format!("[{} sources]", total_active)),
            shortcut: Some("/sources".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "sources_action:all".to_string(),
            title: "All Sources & Workspaces (--all)".to_string(),
            description: "Lists all folders and URLs configured across all workspaces".to_string(),
            icon: "🌐".to_string(),
            badge: Some("[Global]".to_string()),
            shortcut: Some("/sources --all".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "sources_action:inspect".to_string(),
            title: "Inspect LanceDB Vectors & Chunks (/inspect)".to_string(),
            description: "Audit integrity of vectors, chunks, and Apache Arrow indices".to_string(),
            icon: "🔎".to_string(),
            badge: Some("[LanceDB]".to_string()),
            shortcut: Some("/inspect".to_string()),
            is_submenu: false,
        },
    ]
}

pub fn build_keys_menu() -> Vec<MenuItem> {
    let db = NativeConfigDb::open_default().ok();
    let check = |provider: &str, env_var: &str| -> (&'static str, Option<String>) {
        if std::env::var(env_var).map(|v| !v.trim().is_empty()).unwrap_or(false) {
            ("Configured (Env)", Some("[Active]".to_string()))
        } else if let Some(ref d) = db {
            if let Ok(Some(k)) = d.get_api_key(provider) {
                if !k.trim().is_empty() {
                    return ("Configured (Vault)", Some("[Active]".to_string()));
                }
            }
            ("Not configured", Some("[Missing]".to_string()))
        } else {
            ("Not configured", Some("[Missing]".to_string()))
        }
    };

    vec![
        MenuItem {
            id: "keys_action:audit".to_string(),
            title: "Credential Audit Report".to_string(),
            description: "Outputs detailed report in chat with status of each provider".to_string(),
            icon: "📋".to_string(),
            badge: Some("[Audit]".to_string()),
            shortcut: Some("/keys".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "keys_info:gemini".to_string(),
            title: "Google Gemini (Gemini 3.8 Flash, 1.5 Pro)".to_string(),
            description: format!("Key: GEMINI_API_KEY - Status: {}", check("gemini", "GEMINI_API_KEY").0),
            icon: "🔑".to_string(),
            badge: check("gemini", "GEMINI_API_KEY").1,
            shortcut: None,
            is_submenu: false,
        },
        MenuItem {
            id: "keys_info:openai".to_string(),
            title: "OpenAI (GPT-4o, o1, o3-mini)".to_string(),
            description: format!("Key: OPENAI_API_KEY - Status: {}", check("openai", "OPENAI_API_KEY").0),
            icon: "🔑".to_string(),
            badge: check("openai", "OPENAI_API_KEY").1,
            shortcut: None,
            is_submenu: false,
        },
        MenuItem {
            id: "keys_info:anthropic".to_string(),
            title: "Anthropic (Claude 3.5 Sonnet, Haiku)".to_string(),
            description: format!("Key: ANTHROPIC_API_KEY - Status: {}", check("anthropic", "ANTHROPIC_API_KEY").0),
            icon: "🔑".to_string(),
            badge: check("anthropic", "ANTHROPIC_API_KEY").1,
            shortcut: None,
            is_submenu: false,
        },
        MenuItem {
            id: "keys_info:deepseek".to_string(),
            title: "DeepSeek (DeepSeek V3, R1 Reasoner)".to_string(),
            description: format!("Key: DEEPSEEK_API_KEY - Status: {}", check("deepseek", "DEEPSEEK_API_KEY").0),
            icon: "🔑".to_string(),
            badge: check("deepseek", "DEEPSEEK_API_KEY").1,
            shortcut: None,
            is_submenu: false,
        },
        MenuItem {
            id: "keys_info:groq".to_string(),
            title: "Groq Cloud (Llama 3.3 70B LPU)".to_string(),
            description: format!("Key: GROQ_API_KEY - Status: {}", check("groq", "GROQ_API_KEY").0),
            icon: "🔑".to_string(),
            badge: check("groq", "GROQ_API_KEY").1,
            shortcut: None,
            is_submenu: false,
        },
    ]
}

pub fn build_document_ai_menu(active_download_name: Option<&str>) -> Vec<MenuItem> {
    let mut items = Vec::new();

    if let Some(name) = active_download_name {
        items.push(MenuItem {
            id: "model_action:cancel_active".to_string(),
            title: format!("🛑 Cancel Download of {}", name),
            description: "Immediately aborts download and purges temporary files from disk".to_string(),
            icon: "🛑".to_string(),
            badge: Some("[In Progress]".to_string()),
            shortcut: Some("Esc".to_string()),
            is_submenu: false,
        });
    }

    items.push(MenuItem {
        id: "doc_ai:ingestion".to_string(),
        title: "📂 1. Document Classification (Ingestion)".to_string(),
        description: "File typology and structural categorization (invoices, reports, code) during indexing".to_string(),
        icon: "📂".to_string(),
        badge: Some("[Modular]".to_string()),
        shortcut: None,
        is_submenu: true,
    });

    items.push(MenuItem {
        id: "doc_ai:scans".to_string(),
        title: "👁️ 2. Scans & Rasterized PDF Sentinel".to_string(),
        description: "Visually detects whether a page is a scan/photo before extracting text or layout".to_string(),
        icon: "👁️".to_string(),
        badge: Some("[Modular]".to_string()),
        shortcut: None,
        is_submenu: true,
    });

    items.push(MenuItem {
        id: "doc_ai:queries".to_string(),
        title: "🔍 3. Query Routing (Fast vs Deep)".to_string(),
        description: "Classifies query complexity for direct lookup (<100ms) or multi-step reasoning".to_string(),
        icon: "🔍".to_string(),
        badge: Some("[Modular]".to_string()),
        shortcut: None,
        is_submenu: true,
    });

    items.push(MenuItem {
        id: "doc_ai:vision".to_string(),
        title: "🖼️ 4. Document Vision (Air-Gapped Visual OCR)".to_string(),
        description: "Visual inspection of charts, tables, and diagrams (Multimodal Cloud vs Local SLMs)".to_string(),
        icon: "🖼️".to_string(),
        badge: Some("[Modular]".to_string()),
        shortcut: None,
        is_submenu: true,
    });

    items.push(MenuItem {
        id: "doc_ai:store_status".to_string(),
        title: "🛡️ 5. Storage Summary & Fallbacks".to_string(),
        description: "Report on on-disk models, local folder, and zero-breakage guarantees (ADR-110)".to_string(),
        icon: "🛡️".to_string(),
        badge: Some("[Architecture]".to_string()),
        shortcut: None,
        is_submenu: true,
    });

    items
}

pub fn build_doc_ai_ingestion_menu() -> Vec<MenuItem> {
    let mut items = Vec::new();

    items.push(MenuItem {
        id: "doc_ai_info:heuristic_ingestion".to_string(),
        title: "Rust Structural Heuristic (<1µs / 0MB RAM)".to_string(),
        description: "Ultra-high speed active default. Zero downloads and sub-microsecond latency.".to_string(),
        icon: "⚡".to_string(),
        badge: Some("[Active / Default]".to_string()),
        shortcut: None,
        is_submenu: false,
    });

    for model_id in &["laya-int8", "layoutlmv3-int8"] {
        if let Some(spec) = any_context_core_rs::ingestion::OnnxModelManager::find_spec(model_id) {
            let is_installed = any_context_core_rs::ingestion::OnnxModelManager::is_installed(spec);
            let mb = spec.size_bytes as f64 / (1024.0 * 1024.0);
            let (id, title, badge, icon) = if is_installed {
                (
                    format!("model_action:toggle:{}", spec.id),
                    spec.name.to_string(),
                    Some("[Installed / Active]".to_string()),
                    "✔".to_string(),
                )
            } else {
                (
                    format!("model_action:download:{}", spec.id),
                    format!("Download / Attach {}", spec.name),
                    Some(format!("[Download (~{:.0} MB)]", mb)),
                    "📥".to_string(),
                )
            };
            items.push(MenuItem {
                id,
                title,
                description: format!("{} | Fallback: {}", spec.description, spec.fallback_description),
                icon,
                badge,
                shortcut: None,
                is_submenu: false,
            });
        }
    }

    items
}

pub fn build_doc_ai_scans_menu() -> Vec<MenuItem> {
    let mut items = Vec::new();

    items.push(MenuItem {
        id: "doc_ai_info:heuristic_spatial".to_string(),
        title: "Rust 2D Spatial Heuristic (<5MB RAM)".to_string(),
        description: "Native bounding box and geometric layout parser without external dependencies.".to_string(),
        icon: "📐".to_string(),
        badge: Some("[Active / Default]".to_string()),
        shortcut: None,
        is_submenu: false,
    });

    if let Some(spec) = any_context_core_rs::ingestion::OnnxModelManager::find_spec("mobilenetv4-rvl-cdip") {
        let is_installed = any_context_core_rs::ingestion::OnnxModelManager::is_installed(spec);
        let mb = spec.size_bytes as f64 / (1024.0 * 1024.0);
        let (id, title, badge, icon) = if is_installed {
            (
                format!("model_action:toggle:{}", spec.id),
                spec.name.to_string(),
                Some("[Installed / Active]".to_string()),
                "✔".to_string(),
            )
        } else {
            (
                format!("model_action:download:{}", spec.id),
                format!("Download / Attach {}", spec.name),
                Some(format!("[Download (~{:.0} MB)]", mb)),
                "📥".to_string(),
            )
        };
        items.push(MenuItem {
            id,
            title,
            description: format!("{} | Fallback: {}", spec.description, spec.fallback_description),
            icon,
            badge,
            shortcut: None,
            is_submenu: false,
        });
    }

    items
}

pub fn build_doc_ai_queries_menu() -> Vec<MenuItem> {
    let mut items = Vec::new();

    items.push(MenuItem {
        id: "doc_ai_info:deterministic_router".to_string(),
        title: "RFC-042 Deterministic Classifier (<1µs / 0MB RAM)".to_string(),
        description: "Routes queries between Fast RAG and Deep Search without cold start or memory overhead.".to_string(),
        icon: "⚡".to_string(),
        badge: Some("[Active / Default]".to_string()),
        shortcut: None,
        is_submenu: false,
    });

    if let Some(spec) = any_context_core_rs::ingestion::OnnxModelManager::find_spec("bge-small-onnx") {
        let is_installed = any_context_core_rs::ingestion::OnnxModelManager::is_installed(spec);
        let mb = spec.size_bytes as f64 / (1024.0 * 1024.0);
        let (id, title, badge, icon) = if is_installed {
            (
                format!("model_action:toggle:{}", spec.id),
                spec.name.to_string(),
                Some("[Installed / Active]".to_string()),
                "✔".to_string(),
            )
        } else {
            (
                format!("model_action:download:{}", spec.id),
                format!("Download / Attach {}", spec.name),
                Some(format!("[Download (~{:.0} MB)]", mb)),
                "📥".to_string(),
            )
        };
        items.push(MenuItem {
            id,
            title,
            description: format!("{} | Fallback: {}", spec.description, spec.fallback_description),
            icon,
            badge,
            shortcut: None,
            is_submenu: false,
        });
    }

    items
}

pub fn build_doc_ai_vision_menu() -> Vec<MenuItem> {
    let mut items = Vec::new();

    items.push(MenuItem {
        id: "doc_ai_action:vision_cloud".to_string(),
        title: "Vision via Multimodal Provider (Cloud / Active Provider)".to_string(),
        description: "Uses configured model (Gemini Flash, Claude Sonnet, or GPT-4o) for image inspection without downloads.".to_string(),
        icon: "☁️".to_string(),
        badge: Some("[Active Cloud]".to_string()),
        shortcut: Some("/model".to_string()),
        is_submenu: false,
    });

    for model_id in &["clip-vit-int8"] {
        if let Some(spec) = any_context_core_rs::ingestion::OnnxModelManager::find_spec(model_id) {
            let is_installed = any_context_core_rs::ingestion::OnnxModelManager::is_installed(spec);
            let mb = spec.size_bytes as f64 / (1024.0 * 1024.0);
            let (id, title, badge, icon) = if is_installed {
                (
                    format!("model_action:toggle:{}", spec.id),
                    spec.name.to_string(),
                    Some("[Installed / Active]".to_string()),
                    "✔".to_string(),
                )
            } else {
                (
                    format!("model_action:download:{}", spec.id),
                    format!("Download / Attach {}", spec.name),
                    Some(format!("[Download (~{:.0} MB)]", mb)),
                    "📥".to_string(),
                )
            };
            items.push(MenuItem {
                id,
                title,
                description: format!("{} | Fallback: {}", spec.description, spec.fallback_description),
                icon,
                badge,
                shortcut: None,
                is_submenu: false,
            });
        }
    }

    items
}

pub fn build_doc_ai_store_menu() -> Vec<MenuItem> {
    let summary = any_context_core_rs::ingestion::OnnxModelManager::inspect_store();
    let total_mb = summary.total_bytes_on_disk as f64 / (1024.0 * 1024.0);

    vec![
        MenuItem {
            id: "store_info:summary".to_string(),
            title: format!("Models on Disk: {} of {} installed ({:.1} MB)", summary.installed_count, summary.total_count, total_mb),
            description: format!("Local directory: {}", summary.models_dir.display()),
            icon: "📊".to_string(),
            badge: Some(format!("[{:.1} MB]", total_mb)),
            shortcut: None,
            is_submenu: false,
        },
        MenuItem {
            id: "store_info:dir".to_string(),
            title: "Local Storage Directory".to_string(),
            description: format!("{}", summary.models_dir.display()),
            icon: "📁".to_string(),
            badge: None,
            shortcut: None,
            is_submenu: false,
        },
        MenuItem {
            id: "store_info:resilience".to_string(),
            title: "Resilience & Zero Breakage (ADR-110)".to_string(),
            description: "AnyContext never fails due to missing models. Rust heuristics take 100% of the load.".to_string(),
            icon: "🛡️".to_string(),
            badge: Some("[Guarantee]".to_string()),
            shortcut: None,
            is_submenu: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_state_lifecycle() {
        let mut state = MenuState::new();
        assert!(!state.is_open);

        state.open_main("Default", "gpt-4o-mini");
        assert!(state.is_open);
        assert_eq!(state.current_menu_id, "main");
        assert!(!state.items.is_empty());
        assert_eq!(state.selected_idx, 0);

        // Navigation
        state.next();
        assert_eq!(state.selected_idx, 1);
        state.previous();
        assert_eq!(state.selected_idx, 0);
        state.previous(); // wrap-around
        assert_eq!(state.selected_idx, state.items.len() - 1);

        // Submenu
        state.open_submenu("workspaces", "Default", "gpt-4o-mini");
        assert_eq!(state.current_menu_id, "workspaces");
        assert_eq!(state.breadcrumbs.len(), 2);

        // Back
        let still_open = state.back("Default", "gpt-4o-mini");
        assert!(still_open);
        assert_eq!(state.current_menu_id, "main");

        // Back from main closes menu
        let still_open = state.back("Default", "gpt-4o-mini");
        assert!(!still_open);
        assert!(!state.is_open);
    }

    #[test]
    fn test_menu_builders() {
        let main = build_main_menu("TestWS", "claude-3-5-sonnet");
        assert!(main.iter().any(|i| i.id == "workspaces" && i.is_submenu));
        assert!(main.iter().any(|i| i.id == "sync" && i.is_submenu));
        assert!(main.iter().any(|i| i.id == "models" && i.is_submenu));
        assert!(main.iter().any(|i| i.id == "search" && i.is_submenu));
        assert!(main.iter().any(|i| i.id == "document_ai" && i.is_submenu));
        assert!(main.iter().any(|i| i.id == "exit"));

        let models = build_models_menu("mock");
        let mock_item = models.iter().find(|i| i.id == "model:mock");
        assert!(mock_item.is_some());
        assert_eq!(mock_item.unwrap().badge, Some("[Active]".to_string()));

        let sync = build_sync_menu();
        assert_eq!(sync.len(), 3);
        assert!(sync.iter().any(|i| i.id == "sync_action:force"));
        assert!(sync.iter().any(|i| i.id == "sync_action:cancel"));

        let grounding = build_grounding_menu();
        assert_eq!(grounding.len(), 3);
        assert!(grounding.iter().any(|i| i.id == "grounding_action:strict"));
        assert!(grounding.iter().any(|i| i.id == "grounding_action:hybrid"));
        assert!(grounding.iter().any(|i| i.id == "grounding_action:proactive"));

        let search = build_search_menu();
        assert_eq!(search.len(), 3);
        assert!(search.iter().any(|i| i.id == "search_action:auto"));
        assert!(search.iter().any(|i| i.id == "search_action:fast"));
        assert!(search.iter().any(|i| i.id == "search_action:deep"));

        let doc_ai = build_document_ai_menu(None);
        assert_eq!(doc_ai.len(), 5);
        assert!(doc_ai.iter().any(|i| i.id == "doc_ai:ingestion"));
        assert!(doc_ai.iter().any(|i| i.id == "doc_ai:scans"));
        assert!(doc_ai.iter().any(|i| i.id == "doc_ai:queries"));
        assert!(doc_ai.iter().any(|i| i.id == "doc_ai:vision"));

        let ingestion = build_doc_ai_ingestion_menu();
        assert!(ingestion.iter().any(|i| i.id.contains("laya-int8")));

        let scans = build_doc_ai_scans_menu();
        assert!(scans.iter().any(|i| i.id.contains("mobilenetv4")));

        let queries = build_doc_ai_queries_menu();
        assert!(queries.iter().any(|i| i.id.contains("bge-small")));

        let vision = build_doc_ai_vision_menu();
        assert!(vision.iter().any(|i| i.id == "doc_ai_action:vision_cloud"));

        let sources = build_sources_menu("Default");
        assert_eq!(sources.len(), 3);
        assert!(sources.iter().any(|i| i.id == "sources_action:all"));

        let keys = build_keys_menu();
        assert_eq!(keys.len(), 6);
    }
}
