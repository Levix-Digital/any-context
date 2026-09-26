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
                self.breadcrumbs = vec!["Menu Principal".to_string(), "Busca & Grounding".to_string()];
                self.items = build_grounding_menu();
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
        self.breadcrumbs = vec!["Menu Principal".to_string(), "Busca & Grounding".to_string()];
        self.items = build_grounding_menu();
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

    pub fn back(&mut self, active_workspace: &str, active_model: &str) -> bool {
        if let Some((prev_menu, prev_idx)) = self.menu_history.pop() {
            self.current_menu_id = prev_menu.clone();
            self.selected_idx = prev_idx;

            if prev_menu == "main" {
                self.breadcrumbs = vec!["Menu Principal".to_string()];
                self.items = build_main_menu(active_workspace, active_model);
            } else {
                self.breadcrumbs = vec!["Menu Principal".to_string(), prev_menu.clone()];
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
            title: "Modos de Busca & Grounding RAG".to_string(),
            description: "Configurar estratégia: Auto Grounding, Strict, Hybrid, Proactive, Fast ou Deep".to_string(),
            icon: "🔍".to_string(),
            badge: Some("[Auto]".to_string()),
            shortcut: Some("/mode".to_string()),
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
    ]
}

pub fn build_grounding_menu() -> Vec<MenuItem> {
    vec![
        MenuItem {
            id: "grounding_action:auto".to_string(),
            title: "Auto Grounding (Padrão Inteligente)".to_string(),
            description: "Alterna automaticamente entre busca rápida e reflexiva conforme a pergunta".to_string(),
            icon: "🎯".to_string(),
            badge: Some("[Padrão]".to_string()),
            shortcut: Some("/mode auto".to_string()),
            is_submenu: false,
        },
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
        MenuItem {
            id: "grounding_action:fast".to_string(),
            title: "Fast RAG (Single-Turn)".to_string(),
            description: "Busca vetorial e lexical direta de latência ultra-baixa (<50ms)".to_string(),
            icon: "⚡".to_string(),
            badge: None,
            shortcut: Some("/mode fast".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "grounding_action:deep".to_string(),
            title: "Deep Search (Reflexive ReAct)".to_string(),
            description: "Raciocínio multi-turn com decomposição de subtarefas e auto-correção".to_string(),
            icon: "🧠".to_string(),
            badge: None,
            shortcut: Some("/mode deep".to_string()),
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
    let check = |env_var: &str| -> (&'static str, Option<String>) {
        if std::env::var(env_var).map(|v| !v.trim().is_empty()).unwrap_or(false) {
            ("Configurada", Some("[Ativo]".to_string()))
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
            description: format!("Chave: GEMINI_API_KEY - Status: {}", check("GEMINI_API_KEY").0),
            icon: "🔑".to_string(),
            badge: check("GEMINI_API_KEY").1,
            shortcut: None,
            is_submenu: false,
        },
        MenuItem {
            id: "keys_info:openai".to_string(),
            title: "OpenAI (GPT-4o, o1, o3-mini)".to_string(),
            description: format!("Chave: OPENAI_API_KEY - Status: {}", check("OPENAI_API_KEY").0),
            icon: "🔑".to_string(),
            badge: check("OPENAI_API_KEY").1,
            shortcut: None,
            is_submenu: false,
        },
        MenuItem {
            id: "keys_info:anthropic".to_string(),
            title: "Anthropic (Claude 3.5 Sonnet, Haiku)".to_string(),
            description: format!("Chave: ANTHROPIC_API_KEY - Status: {}", check("ANTHROPIC_API_KEY").0),
            icon: "🔑".to_string(),
            badge: check("ANTHROPIC_API_KEY").1,
            shortcut: None,
            is_submenu: false,
        },
        MenuItem {
            id: "keys_info:deepseek".to_string(),
            title: "DeepSeek (DeepSeek V3, R1 Reasoner)".to_string(),
            description: format!("Chave: DEEPSEEK_API_KEY - Status: {}", check("DEEPSEEK_API_KEY").0),
            icon: "🔑".to_string(),
            badge: check("DEEPSEEK_API_KEY").1,
            shortcut: None,
            is_submenu: false,
        },
        MenuItem {
            id: "keys_info:groq".to_string(),
            title: "Groq Cloud (Llama 3.3 70B LPU)".to_string(),
            description: format!("Chave: GROQ_API_KEY - Status: {}", check("GROQ_API_KEY").0),
            icon: "🔑".to_string(),
            badge: check("GROQ_API_KEY").1,
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
        assert!(main.iter().any(|i| i.id == "exit"));

        let models = build_models_menu("mock");
        let mock_item = models.iter().find(|i| i.id == "model:mock");
        assert!(mock_item.is_some());
        assert_eq!(mock_item.unwrap().badge, Some("[Ativo]".to_string()));

        let sync = build_sync_menu();
        assert_eq!(sync.len(), 2);
        assert!(sync.iter().any(|i| i.id == "sync_action:force"));

        let grounding = build_grounding_menu();
        assert_eq!(grounding.len(), 6);
        assert!(grounding.iter().any(|i| i.id == "grounding_action:strict"));
        assert!(grounding.iter().any(|i| i.id == "grounding_action:hybrid"));

        let sources = build_sources_menu("Default");
        assert_eq!(sources.len(), 3);
        assert!(sources.iter().any(|i| i.id == "sources_action:all"));

        let keys = build_keys_menu();
        assert_eq!(keys.len(), 6);
    }
}
