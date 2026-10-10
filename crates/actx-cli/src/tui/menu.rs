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
                self.breadcrumbs = vec!["Menu Principal".to_string(), "Workspaces".to_string()];
                self.items = build_workspaces_menu(active_workspace);
            }
            "models" => {
                self.breadcrumbs = vec!["Menu Principal".to_string(), "Modelos de IA".to_string()];
                self.items = build_models_menu(active_model);
            }
            "sync" => {
                self.breadcrumbs = vec!["Menu Principal".to_string(), "Sincronização".to_string()];
                self.items = build_sync_menu();
            }
            "grounding" => {
                self.breadcrumbs = vec!["Menu Principal".to_string(), "Grounding Strategy".to_string()];
                self.items = build_grounding_menu();
            }
            "search" => {
                self.breadcrumbs = vec!["Menu Principal".to_string(), "Profundidade de Busca".to_string()];
                self.items = build_search_menu();
            }
            "sources" => {
                self.breadcrumbs = vec!["Menu Principal".to_string(), "Fontes & Documentos".to_string()];
                self.items = build_sources_menu(active_workspace);
            }
            "keys" => {
                self.breadcrumbs = vec!["Menu Principal".to_string(), "Credenciais de API".to_string()];
                self.items = build_keys_menu();
            }
            "document_ai" => {
                self.breadcrumbs = vec!["Menu Principal".to_string(), "Document AI & Modelos Locais".to_string()];
                self.items = build_document_ai_menu(None);
            }
            "doc_ai:ingestion" => {
                self.breadcrumbs = vec!["Menu Principal".to_string(), "Document AI".to_string(), "Classificação de Ingestão".to_string()];
                self.items = build_doc_ai_ingestion_menu();
            }
            "doc_ai:scans" => {
                self.breadcrumbs = vec!["Menu Principal".to_string(), "Document AI".to_string(), "Sentinela de Scans".to_string()];
                self.items = build_doc_ai_scans_menu();
            }
            "doc_ai:queries" => {
                self.breadcrumbs = vec!["Menu Principal".to_string(), "Document AI".to_string(), "Roteamento de Perguntas".to_string()];
                self.items = build_doc_ai_queries_menu();
            }
            "doc_ai:vision" => {
                self.breadcrumbs = vec!["Menu Principal".to_string(), "Document AI".to_string(), "Visão de Documentos".to_string()];
                self.items = build_doc_ai_vision_menu();
            }
            "doc_ai:store_status" => {
                self.breadcrumbs = vec!["Menu Principal".to_string(), "Document AI".to_string(), "Resumo de Armazenamento".to_string()];
                self.items = build_doc_ai_store_menu();
            }
            _ => {
                self.breadcrumbs = vec!["Menu Principal".to_string()];
                self.items = build_main_menu(active_workspace, active_model);
            }
        }
    }

    pub fn open_workspaces(&mut self, active_workspace: &str) {
        self.is_open = true;
        self.current_menu_id = "workspaces".to_string();
        self.menu_history.clear();
        self.selected_idx = 0;
        self.breadcrumbs = vec!["Menu Principal".to_string(), "Workspaces".to_string()];
        self.items = build_workspaces_menu(active_workspace);
    }

    pub fn open_models(&mut self, active_model: &str) {
        self.is_open = true;
        self.current_menu_id = "models".to_string();
        self.menu_history.clear();
        self.selected_idx = 0;
        self.breadcrumbs = vec!["Menu Principal".to_string(), "Modelos de IA".to_string()];
        self.items = build_models_menu(active_model);
    }

    pub fn open_sync(&mut self) {
        self.is_open = true;
        self.current_menu_id = "sync".to_string();
        self.menu_history.clear();
        self.selected_idx = 0;
        self.breadcrumbs = vec!["Menu Principal".to_string(), "Sincronização".to_string()];
        self.items = build_sync_menu();
    }

    pub fn open_grounding(&mut self) {
        self.is_open = true;
        self.current_menu_id = "grounding".to_string();
        self.menu_history.clear();
        self.selected_idx = 0;
        self.breadcrumbs = vec!["Menu Principal".to_string(), "Grounding Strategy".to_string()];
        self.items = build_grounding_menu();
    }

    pub fn open_search(&mut self) {
        self.is_open = true;
        self.current_menu_id = "search".to_string();
        self.menu_history.clear();
        self.selected_idx = 0;
        self.breadcrumbs = vec!["Menu Principal".to_string(), "Profundidade de Busca".to_string()];
        self.items = build_search_menu();
    }

    pub fn open_sources(&mut self, active_workspace: &str) {
        self.is_open = true;
        self.current_menu_id = "sources".to_string();
        self.menu_history.clear();
        self.selected_idx = 0;
        self.breadcrumbs = vec!["Menu Principal".to_string(), "Fontes & Documentos".to_string()];
        self.items = build_sources_menu(active_workspace);
    }

    pub fn open_keys(&mut self) {
        self.is_open = true;
        self.current_menu_id = "keys".to_string();
        self.menu_history.clear();
        self.selected_idx = 0;
        self.breadcrumbs = vec!["Menu Principal".to_string(), "Credenciais de API".to_string()];
        self.items = build_keys_menu();
    }

    pub fn open_document_ai(&mut self, active_download_name: Option<&str>) {
        self.is_open = true;
        self.current_menu_id = "document_ai".to_string();
        self.menu_history.clear();
        self.selected_idx = 0;
        self.breadcrumbs = vec!["Menu Principal".to_string(), "Document AI & Modelos Locais".to_string()];
        self.items = build_document_ai_menu(active_download_name);
    }

    pub fn back(&mut self, active_workspace: &str, active_model: &str) -> bool {
        if let Some((prev_menu, prev_idx)) = self.menu_history.pop() {
            self.current_menu_id = prev_menu.clone();
            self.selected_idx = prev_idx;

            match prev_menu.as_str() {
                "main" => {
                    self.breadcrumbs = vec!["Menu Principal".to_string()];
                    self.items = build_main_menu(active_workspace, active_model);
                }
                "document_ai" => {
                    self.breadcrumbs = vec!["Menu Principal".to_string(), "Document AI & Modelos Locais".to_string()];
                    self.items = build_document_ai_menu(None);
                }
                "workspaces" => {
                    self.breadcrumbs = vec!["Menu Principal".to_string(), "Workspaces".to_string()];
                    self.items = build_workspaces_menu(active_workspace);
                }
                "models" => {
                    self.breadcrumbs = vec!["Menu Principal".to_string(), "Modelos de IA".to_string()];
                    self.items = build_models_menu(active_model);
                }
                "sync" => {
                    self.breadcrumbs = vec!["Menu Principal".to_string(), "Sincronização".to_string()];
                    self.items = build_sync_menu();
                }
                "grounding" => {
                    self.breadcrumbs = vec!["Menu Principal".to_string(), "Grounding Strategy".to_string()];
                    self.items = build_grounding_menu();
                }
                "search" => {
                    self.breadcrumbs = vec!["Menu Principal".to_string(), "Profundidade de Busca".to_string()];
                    self.items = build_search_menu();
                }
                "sources" => {
                    self.breadcrumbs = vec!["Menu Principal".to_string(), "Fontes & Documentos".to_string()];
                    self.items = build_sources_menu(active_workspace);
                }
                "keys" => {
                    self.breadcrumbs = vec!["Menu Principal".to_string(), "Credenciais de API".to_string()];
                    self.items = build_keys_menu();
                }
                _ => {
                    self.breadcrumbs = vec!["Menu Principal".to_string(), prev_menu];
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
            title: "Workspaces & Pastas de Contexto".to_string(),
            description: "Listar, alternar ou selecionar workspaces de contexto".to_string(),
            icon: "📂".to_string(),
            badge: Some(format!("[{}]", active_workspace)),
            shortcut: Some("/switch".to_string()),
            is_submenu: true,
        },
        MenuItem {
            id: "sync".to_string(),
            title: "Sincronização & Reindexação".to_string(),
            description: "Executar sincronização incremental ou forçada de arquivos".to_string(),
            icon: "🔄".to_string(),
            badge: Some("[SHA-256]".to_string()),
            shortcut: Some("/sync".to_string()),
            is_submenu: true,
        },
        MenuItem {
            id: "sources".to_string(),
            title: "Fontes Monitoradas & Portais Web".to_string(),
            description: "Listar diretórios locais e URLs de documentação cadastradas".to_string(),
            icon: "📁".to_string(),
            badge: None,
            shortcut: Some("/sources".to_string()),
            is_submenu: true,
        },
        MenuItem {
            id: "models".to_string(),
            title: "Modelos de IA & Provedores LLM".to_string(),
            description: "Selecionar modelo ativo (OpenAI, Anthropic, Gemini, DeepSeek, Groq, Ollama, Mock)".to_string(),
            icon: "🤖".to_string(),
            badge: Some(format!("[{}]", active_model)),
            shortcut: Some("/model".to_string()),
            is_submenu: true,
        },
        MenuItem {
            id: "grounding".to_string(),
            title: "Grounding Strategy (Ancoragem)".to_string(),
            description: "Configurar política de fidelidade: Strict, Hybrid ou Proactive".to_string(),
            icon: "🛡️".to_string(),
            badge: None,
            shortcut: Some("/mode".to_string()),
            is_submenu: true,
        },
        MenuItem {
            id: "search".to_string(),
            title: "Profundidade de Busca (/search)".to_string(),
            description: "Configurar modo de busca do workspace: Auto, Fast ou Deep".to_string(),
            icon: "🎯".to_string(),
            badge: None,
            shortcut: Some("/search".to_string()),
            is_submenu: true,
        },
        MenuItem {
            id: "document_ai".to_string(),
            title: "Document AI & Modelos Locais".to_string(),
            description: "Classificação local <1µs, layout 2D spatial e download de modelos de visão".to_string(),
            icon: "👁️".to_string(),
            badge: Some("[Zero-Overhead]".to_string()),
            shortcut: None,
            is_submenu: true,
        },
        MenuItem {
            id: "keys".to_string(),
            title: "Credenciais & Chaves de API".to_string(),
            description: "Auditar presença de chaves configuradas nos provedores".to_string(),
            icon: "🔑".to_string(),
            badge: None,
            shortcut: Some("/keys".to_string()),
            is_submenu: true,
        },
        MenuItem {
            id: "diagnostics".to_string(),
            title: "Diagnóstico & Saúde do Sistema".to_string(),
            description: "Inspecionar integridade da engine 100% Rust, SQLite e LanceDB".to_string(),
            icon: "🩺".to_string(),
            badge: Some("[100% Rust]".to_string()),
            shortcut: Some("/diagnostics".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "history".to_string(),
            title: "Histórico de Conversação".to_string(),
            description: "Exibir contagem de turnos e status de memória da sessão".to_string(),
            icon: "📜".to_string(),
            badge: None,
            shortcut: Some("/history".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "clear".to_string(),
            title: "Limpar Visualização do Chat".to_string(),
            description: "Limpar o histórico visual mantendo o estado do motor".to_string(),
            icon: "🧹".to_string(),
            badge: None,
            shortcut: Some("/clear".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "exit".to_string(),
            title: "Encerrar Sessão do AnyContext".to_string(),
            description: "Salvar estado e fechar aplicativo restaurando o terminal".to_string(),
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
                "Workspace ativo no momento".to_string()
            } else {
                format!("Alternar para o workspace '{}'", ws)
            },
            icon: "📂".to_string(),
            badge: if is_active {
                Some("[Ativo]".to_string())
            } else {
                None
            },
            shortcut: None,
            is_submenu: false,
        });
    }

    items.push(MenuItem {
        id: "workspace_tip".to_string(),
        title: "💡 Excluir Workspace (Dica)".to_string(),
        description: "Para excluir um workspace, use o comando: /switch --delete <nome>".to_string(),
        icon: "🗑️".to_string(),
        badge: Some("[Dica]".to_string()),
        shortcut: None,
        is_submenu: false,
    });

    items
}

pub fn build_models_menu(active_model: &str) -> Vec<MenuItem> {
    let models = &[
        ("gemini-3.8-flash", "Google - Baixíssima latência e multimodalidade"),
        ("gpt-4o-mini", "OpenAI - Rápido, econômico e inteligente"),
        ("gpt-4o", "OpenAI - Flagship multimodal de alta capacidade"),
        ("claude-3-5-sonnet-20241022", "Anthropic - Raciocínio avançado e código"),
        ("claude-3-5-haiku-20241022", "Anthropic - Ultrarrápido e eficiente"),
        ("gemini-1.5-pro", "Google - Janela de contexto estendida"),
        ("deepseek-chat", "DeepSeek - Excelente custo-benefício para código"),
        ("deepseek-reasoner", "DeepSeek R1 - Raciocínio analítico passo-a-passo"),
        ("llama-3.3-70b-versatile", "Groq - Inferência instantânea em hardware LPU"),
        ("ollama/qwen2.5-coder", "Local - Modelo offline via endpoint Ollama local"),
        ("mock", "Mock - Simulação instantânea offline para testes e demonstração"),
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
                    Some("[Ativo]".to_string())
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
            title: "Sincronização Incremental (Padrão)".to_string(),
            description: "Analisa apenas arquivos novos e modificados via hash SHA-256 ($0.00)".to_string(),
            icon: "⚡".to_string(),
            badge: Some("[Rápido]".to_string()),
            shortcut: Some("/sync".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "sync_action:force".to_string(),
            title: "Sincronização Forçada (--force)".to_string(),
            description: "Recalcula todos os hashes e reindexa integralmente o workspace".to_string(),
            icon: "🔄".to_string(),
            badge: Some("[Completo]".to_string()),
            shortcut: Some("/sync --force".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "sync_action:cancel".to_string(),
            title: "Cancelar Sincronização em Andamento".to_string(),
            description: "Interrompe o worker de indexação/crawler e reseta o status".to_string(),
            icon: "🛑".to_string(),
            badge: Some("[Parar]".to_string()),
            shortcut: Some("/sync cancel".to_string()),
            is_submenu: false,
        },
    ]
}

pub fn build_grounding_menu() -> Vec<MenuItem> {
    vec![
        MenuItem {
            id: "grounding_action:strict".to_string(),
            title: "Strict Mode (100% Fatos Verificados)".to_string(),
            description: "Respostas estritamente ancoradas nos documentos, zero especulação externa".to_string(),
            icon: "🔒".to_string(),
            badge: Some("[Auditoria]".to_string()),
            shortcut: Some("/mode strict".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "grounding_action:hybrid".to_string(),
            title: "Hybrid Mode (Dual-Layer)".to_string(),
            description: "Camada 1: fatos do workspace + Camada 2: sugestões externas identificadas".to_string(),
            icon: "⚖️".to_string(),
            badge: Some("[Equilibrado]".to_string()),
            shortcut: Some("/mode hybrid".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "grounding_action:proactive".to_string(),
            title: "Proactive Mode (Síntese & Recomendações)".to_string(),
            description: "Síntese ampla, insights de pesquisa e recomendações proativas de fontes".to_string(),
            icon: "💡".to_string(),
            badge: Some("[Pesquisa]".to_string()),
            shortcut: Some("/mode proactive".to_string()),
            is_submenu: false,
        },
    ]
}

pub fn build_search_menu() -> Vec<MenuItem> {
    vec![
        MenuItem {
            id: "search_action:auto".to_string(),
            title: "Auto Search Depth (Padrão Inteligente)".to_string(),
            description: "Alterna automaticamente entre busca rápida e reflexiva conforme a pergunta".to_string(),
            icon: "🎯".to_string(),
            badge: Some("[Padrão]".to_string()),
            shortcut: Some("/search auto".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "search_action:fast".to_string(),
            title: "Fast RAG (Single-Turn <50ms)".to_string(),
            description: "Busca vetorial e lexical direta de latência ultra-baixa".to_string(),
            icon: "⚡".to_string(),
            badge: Some("[Ultra Rápido]".to_string()),
            shortcut: Some("/search fast".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "search_action:deep".to_string(),
            title: "Deep Search (Reflexive ReAct)".to_string(),
            description: "Raciocínio multi-turn com decomposição de subtarefas e auto-correção".to_string(),
            icon: "🧠".to_string(),
            badge: Some("[Aprofundado]".to_string()),
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
            title: format!("Fontes do Workspace '{}' ({})", active_workspace, total_active),
            description: "Exibe todas as pastas e portais web indexados no workspace atual".to_string(),
            icon: "📂".to_string(),
            badge: Some(format!("[{} fontes]", total_active)),
            shortcut: Some("/sources".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "sources_action:all".to_string(),
            title: "Todas as Fontes & Workspaces (--all)".to_string(),
            description: "Lista todas as pastas e URLs configuradas em todos os workspaces".to_string(),
            icon: "🌐".to_string(),
            badge: Some("[Global]".to_string()),
            shortcut: Some("/sources --all".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "sources_action:inspect".to_string(),
            title: "Inspecionar Vetores LanceDB & Chunks (/inspect)".to_string(),
            description: "Auditar integridade dos vetores, chunks e índices Apache Arrow".to_string(),
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
            ("Configurada (Env)", Some("[Ativo]".to_string()))
        } else if let Some(ref d) = db {
            if let Ok(Some(k)) = d.get_api_key(provider) {
                if !k.trim().is_empty() {
                    return ("Configurada (Vault)", Some("[Ativo]".to_string()));
                }
            }
            ("Não configurada", Some("[Ausente]".to_string()))
        } else {
            ("Não configurada", Some("[Ausente]".to_string()))
        }
    };

    vec![
        MenuItem {
            id: "keys_action:audit".to_string(),
            title: "Relatório de Auditoria de Credenciais".to_string(),
            description: "Emite relatório detalhado no chat com status de cada provedor".to_string(),
            icon: "📋".to_string(),
            badge: Some("[Auditar]".to_string()),
            shortcut: Some("/keys".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "keys_info:gemini".to_string(),
            title: "Google Gemini (Gemini 3.8 Flash, 1.5 Pro)".to_string(),
            description: format!("Chave: GEMINI_API_KEY - Status: {}", check("gemini", "GEMINI_API_KEY").0),
            icon: "🔑".to_string(),
            badge: check("gemini", "GEMINI_API_KEY").1,
            shortcut: None,
            is_submenu: false,
        },
        MenuItem {
            id: "keys_info:openai".to_string(),
            title: "OpenAI (GPT-4o, o1, o3-mini)".to_string(),
            description: format!("Chave: OPENAI_API_KEY - Status: {}", check("openai", "OPENAI_API_KEY").0),
            icon: "🔑".to_string(),
            badge: check("openai", "OPENAI_API_KEY").1,
            shortcut: None,
            is_submenu: false,
        },
        MenuItem {
            id: "keys_info:anthropic".to_string(),
            title: "Anthropic (Claude 3.5 Sonnet, Haiku)".to_string(),
            description: format!("Chave: ANTHROPIC_API_KEY - Status: {}", check("anthropic", "ANTHROPIC_API_KEY").0),
            icon: "🔑".to_string(),
            badge: check("anthropic", "ANTHROPIC_API_KEY").1,
            shortcut: None,
            is_submenu: false,
        },
        MenuItem {
            id: "keys_info:deepseek".to_string(),
            title: "DeepSeek (DeepSeek V3, R1 Reasoner)".to_string(),
            description: format!("Chave: DEEPSEEK_API_KEY - Status: {}", check("deepseek", "DEEPSEEK_API_KEY").0),
            icon: "🔑".to_string(),
            badge: check("deepseek", "DEEPSEEK_API_KEY").1,
            shortcut: None,
            is_submenu: false,
        },
        MenuItem {
            id: "keys_info:groq".to_string(),
            title: "Groq Cloud (Llama 3.3 70B LPU)".to_string(),
            description: format!("Chave: GROQ_API_KEY - Status: {}", check("groq", "GROQ_API_KEY").0),
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
            title: format!("🛑 Cancelar Download de {}", name),
            description: "Interrompe imediatamente o download e remove arquivos temporários do disco".to_string(),
            icon: "🛑".to_string(),
            badge: Some("[Em Progresso]".to_string()),
            shortcut: Some("Esc".to_string()),
            is_submenu: false,
        });
    }

    items.push(MenuItem {
        id: "doc_ai:ingestion".to_string(),
        title: "📂 1. Classificação de Documentos (Ingestão)".to_string(),
        description: "Tipologia e categorização estrutural de arquivos (faturas, relatórios, código) na indexação".to_string(),
        icon: "📂".to_string(),
        badge: Some("[Modular]".to_string()),
        shortcut: None,
        is_submenu: true,
    });

    items.push(MenuItem {
        id: "doc_ai:scans".to_string(),
        title: "👁️ 2. Sentinela de Scans & PDFs Rasterizados".to_string(),
        description: "Detecta visualmente se uma página é foto/escaneamento antes de extrair texto ou layout".to_string(),
        icon: "👁️".to_string(),
        badge: Some("[Modular]".to_string()),
        shortcut: None,
        is_submenu: true,
    });

    items.push(MenuItem {
        id: "doc_ai:queries".to_string(),
        title: "🔍 3. Roteamento de Perguntas (Fast vs Deep)".to_string(),
        description: "Classifica complexidade da consulta para busca direta (<100ms) ou raciocínio multi-etapa".to_string(),
        icon: "🔍".to_string(),
        badge: Some("[Modular]".to_string()),
        shortcut: None,
        is_submenu: true,
    });

    items.push(MenuItem {
        id: "doc_ai:vision".to_string(),
        title: "🖼️ 4. Visão de Documentos (OCR Visual Air-Gapped)".to_string(),
        description: "Inspeção visual de gráficos, tabelas e diagramas (Nuvem Multimodal vs SLMs Locais)".to_string(),
        icon: "🖼️".to_string(),
        badge: Some("[Modular]".to_string()),
        shortcut: None,
        is_submenu: true,
    });

    items.push(MenuItem {
        id: "doc_ai:store_status".to_string(),
        title: "🛡️ 5. Resumo de Armazenamento & Fallbacks".to_string(),
        description: "Relatório de modelos em disco, pasta local e garantias de zero quebra (ADR-110)".to_string(),
        icon: "🛡️".to_string(),
        badge: Some("[Arquitetura]".to_string()),
        shortcut: None,
        is_submenu: true,
    });

    items
}

pub fn build_doc_ai_ingestion_menu() -> Vec<MenuItem> {
    let mut items = Vec::new();

    items.push(MenuItem {
        id: "doc_ai_info:heuristic_ingestion".to_string(),
        title: "Heurística Estrutural Rust (<1µs / 0MB RAM)".to_string(),
        description: "Padrão ativo de altíssima velocidade. Zero download e latência sub-microsegundo.".to_string(),
        icon: "⚡".to_string(),
        badge: Some("[Ativo / Padrão]".to_string()),
        shortcut: None,
        is_submenu: false,
    });

    if let Some(spec) = any_context_core_rs::ingestion::OnnxModelManager::find_spec("laya-int8") {
        let is_installed = any_context_core_rs::ingestion::OnnxModelManager::is_installed(spec);
        let mb = spec.size_bytes as f64 / (1024.0 * 1024.0);
        let (id, title, badge, icon) = if is_installed {
            (
                format!("model_action:toggle:{}", spec.id),
                spec.name.to_string(),
                Some("[Instalado / Ativo]".to_string()),
                "✔".to_string(),
            )
        } else {
            (
                format!("model_action:download:{}", spec.id),
                format!("Baixar / Acoplar {}", spec.name),
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

pub fn build_doc_ai_scans_menu() -> Vec<MenuItem> {
    let mut items = Vec::new();

    items.push(MenuItem {
        id: "doc_ai_info:heuristic_spatial".to_string(),
        title: "Heurística 2D Espacial Rust (<5MB RAM)".to_string(),
        description: "Parser nativo de bounding boxes e layout geométrico sem dependências externas.".to_string(),
        icon: "📐".to_string(),
        badge: Some("[Ativo / Padrão]".to_string()),
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
                Some("[Instalado / Ativo]".to_string()),
                "✔".to_string(),
            )
        } else {
            (
                format!("model_action:download:{}", spec.id),
                format!("Baixar / Acoplar {}", spec.name),
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
        title: "Classificador Determinístico RFC-042 (<1µs / 0MB RAM)".to_string(),
        description: "Roteia consultas entre Fast RAG e Deep Search sem cold start ou consumo de memória.".to_string(),
        icon: "⚡".to_string(),
        badge: Some("[Ativo / Padrão]".to_string()),
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
                Some("[Instalado / Ativo]".to_string()),
                "✔".to_string(),
            )
        } else {
            (
                format!("model_action:download:{}", spec.id),
                format!("Baixar / Acoplar {}", spec.name),
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
        title: "Visão via Provedor Multimodal (Nuvem / Provedor Ativo)".to_string(),
        description: "Usa o modelo configurado (Gemini Flash, Claude Sonnet ou GPT-4o) para inspeção de imagens sem downloads.".to_string(),
        icon: "☁️".to_string(),
        badge: Some("[Nuvem Ativa]".to_string()),
        shortcut: Some("/model".to_string()),
        is_submenu: false,
    });

    for model_id in &["smolvlm-500m", "moondream2-int4"] {
        if let Some(spec) = any_context_core_rs::ingestion::OnnxModelManager::find_spec(model_id) {
            let is_installed = any_context_core_rs::ingestion::OnnxModelManager::is_installed(spec);
            let mb = spec.size_bytes as f64 / (1024.0 * 1024.0);
            let (id, title, badge, icon) = if is_installed {
                (
                    format!("model_action:toggle:{}", spec.id),
                    spec.name.to_string(),
                    Some("[Instalado / Ativo]".to_string()),
                    "✔".to_string(),
                )
            } else {
                (
                    format!("model_action:download:{}", spec.id),
                    format!("Baixar / Acoplar {}", spec.name),
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
            title: format!("Modelos em Disco: {} de {} instalados ({:.1} MB)", summary.installed_count, summary.total_count, total_mb),
            description: format!("Diretório local: {}", summary.models_dir.display()),
            icon: "📊".to_string(),
            badge: Some(format!("[{:.1} MB]", total_mb)),
            shortcut: None,
            is_submenu: false,
        },
        MenuItem {
            id: "store_info:dir".to_string(),
            title: "Diretório de Armazenamento Local".to_string(),
            description: format!("{}", summary.models_dir.display()),
            icon: "📁".to_string(),
            badge: None,
            shortcut: None,
            is_submenu: false,
        },
        MenuItem {
            id: "store_info:resilience".to_string(),
            title: "Resiliência & Zero Quebra (ADR-110)".to_string(),
            description: "O AnyContext nunca quebra por modelos ausentes. Heurísticas Rust assumem 100% da carga.".to_string(),
            icon: "🛡️".to_string(),
            badge: Some("[Garantia]".to_string()),
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
        assert_eq!(mock_item.unwrap().badge, Some("[Ativo]".to_string()));

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
