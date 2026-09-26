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
            _ => {
                self.breadcrumbs = vec!["Menu Principal".to_string()];
                self.items = build_main_menu(active_workspace, active_model);
            }
        }
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
            is_submenu: false,
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
            description: "Configurar estratégia: Auto Grounding, Fast RAG ou Deep Search".to_string(),
            icon: "🔍".to_string(),
            badge: Some("[Auto]".to_string()),
            shortcut: Some("/search".to_string()),
            is_submenu: true,
        },
        MenuItem {
            id: "keys".to_string(),
            title: "Credenciais & Chaves de API".to_string(),
            description: "Auditar presença de chaves configuradas nos provedores".to_string(),
            icon: "🔑".to_string(),
            badge: None,
            shortcut: Some("/keys".to_string()),
            is_submenu: false,
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
        ("gpt-4o-mini", "OpenAI - Rápido, econômico e inteligente"),
        ("gpt-4o", "OpenAI - Flagship multimodal de alta capacidade"),
        ("claude-3-5-sonnet-20241022", "Anthropic - Raciocínio avançado e código"),
        ("claude-3-5-haiku-20241022", "Anthropic - Ultrarrápido e eficiente"),
        ("gemini-2.0-flash", "Google - Baixíssima latência e multimodalidade"),
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
            shortcut: Some("/search auto".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "grounding_action:fast".to_string(),
            title: "Fast RAG (Single-Turn)".to_string(),
            description: "Busca vetorial e lexical direta de latência ultra-baixa (<50ms)".to_string(),
            icon: "⚡".to_string(),
            badge: None,
            shortcut: Some("/fast".to_string()),
            is_submenu: false,
        },
        MenuItem {
            id: "grounding_action:deep".to_string(),
            title: "Deep Search (Reflexive ReAct)".to_string(),
            description: "Raciocínio multi-turn com decomposição de subtarefas e auto-correção".to_string(),
            icon: "🧠".to_string(),
            badge: None,
            shortcut: Some("/deep".to_string()),
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
        assert_eq!(grounding.len(), 3);
    }
}
