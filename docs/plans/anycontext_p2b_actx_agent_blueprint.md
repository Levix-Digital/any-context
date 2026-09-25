# 📋 Blueprint Técnico - Prioridade 2B: Orquestrador ReAct Nativo em Rust (`actx-agent`)

## 1. 🎯 Objetivo & Decisões Arquiteturais (ADR)
- **Objetivo**: Substituir o grafo ReAct em Python construído sobre LangGraph por uma Máquina de Estados Finitos (FSM) nativa em Rust na nova crate independente [`crates/actx-agent`](file:///C:/Users/guilh/source/repos/any-context/crates/actx-agent), consumindo [`actx-lm`](file:///C:/Users/guilh/source/repos/any-context/crates/actx-lm) e gerenciando um catálogo polimórfico de ferramentas (`ToolRegistry`), preparando o motor para a pesquisa agêntica profunda (RFC-042 Deep Search).
- **Decisões de Design**:
  1. **Nova Crate Desacoplada (`crates/actx-agent`)**: Crate autônoma no workspace Cargo, sem dependências de frameworks pesados, reutilizável em qualquer aplicação Rust.
  2. **FSM ReAct Determinística**: Estados finitos: `Idle` $\rightarrow$ `CallModel` $\rightarrow$ `ExecuteTools` $\rightarrow$ `CheckBudget` $\rightarrow$ `Finished`.
  3. **Proteção Anti-Recursão Absoluta**: Limite estrito configurável (`max_turns: 10`, default) com corte forçado e síntese do contexto acumulado, eliminando permanentemente erros de recursão infinita.
  4. **Catálogo Polimórfico Híbrido (`ToolRegistry`)**:
     - `NativeTool`: Execução direta no processo Rust (ex: busca vetorial/léxica no `NativeLanceStore` e SQLite sem cruzar a ponte PyO3).
     - `DynamicPyTool`: Callbacks assíncronos via PyO3 para ferramentas legadas ou scripts externos.
     - Suporte nativo ao protocolo **MCP** (*Model Context Protocol*) via adaptadores Stdio/SSE.
  5. **Auto-Correção e Execução Defensiva de Tools**: Erros de validação de schema JSON ou falhas de execução são capturados e devolvidos como observação amigável para a LLM se auto-corrigir no próximo turno.
  6. **Streaming de Eventos Granulares (`AgentEvent`)**: Emissão em tempo real via canal assíncrono Tokio (`Thinking`, `ToolStart`, `ToolEnd`, `Delta`, `Done`, `Error`).
  7. **Preparação para RFC-042 (Deep Search)**: Modelagem de `SearchMode` (`Auto`, `Fast`, `Deep`), estruturas `BatchQueryItem` e `BatchRetrievalResult`, e eventos de árvore para a futura `<DeepSearchTree />`.
  8. **SessionStore em SQLite Nativo**: Histórico limpo gravado em JSON sem zlib, com rolling window determinística de 15 turnos.

---

## 2. 🏛️ Mapeamento de Paridade de Superfícies
- **Terminal CLI (`src/any_context/cli/`)**: Mantém comandos, prompts e histórico consumindo o novo `AgentEngine` nativo via wrapper Python transparente.
- **OpenTUI Desktop (`src/any_context/tui/`)**: Recebe chunks e eventos de streaming diretamente através do Stdio RPC Bridge (`chat_stream`).
- **REST Web API (`src/any_context/server/api.py`)**: Endpoint `/v1/chat/completions` despacha chamadas para o motor nativo.
- **MCP Protocol Server (`src/any_context/server/mcp.py`)**: As tools e prompts do servidor MCP interagem com o mesmo `SessionStore` canônico.

---

## 3. 📁 Arquivos e Linhas Impactadas
- **Novos Arquivos**:
  - `crates/actx-agent/Cargo.toml`: Definição da crate, dependências (`actx-lm`, `tokio`, `serde`, `serde_json`, `async-trait`, `rusqlite`, `thiserror`).
  - `crates/actx-agent/src/lib.rs`: Façade público do agente (`Agent`, `AgentBuilder`, `AgentConfig`).
  - `crates/actx-agent/src/fsm.rs`: Máquina de estados ReAct assíncrona.
  - `crates/actx-agent/src/tool.rs`: Trait `Tool`, `ToolRegistry`, `NativeTool`, `ToolResult`, validação JSON.
  - `crates/actx-agent/src/session.rs`: `SessionStore` baseado em SQLite e memória.
  - `crates/actx-agent/src/events.rs`: Enum `AgentEvent` e receptor assíncrono.
  - `crates/actx-agent/tests/agent_tests.rs`: Testes unitários Rust (FSM, tools, circuit breaker, streaming).
  - `src/any_context/core/deep_search_models.py`: Modelos Pydantic da RFC-042 (`SearchMode`, `SubQuery`, `QueryPlan`, `GapAnalysisResult`, etc.).
  - `tests/unit/core/test_native_agent.py`: Testes de integração PyO3 em Python.
- **Arquivos Modificados**:
  - `Cargo.toml`: Adicionar `crates/actx-agent` aos members do workspace.
  - `crates/any-context-core-rs/Cargo.toml`: Adicionar dependência `actx-agent = { path = "../actx-agent" }`.
  - `crates/any-context-core-rs/src/lib.rs`: Exposição de classes PyO3 (`PyAgent`, `PyAgentEngine`, `PyAgentEvent`).
  - `src/any_context/core/agent.py`: Redirecionar `create_anycontext_agent` e `LazyAgentProxy` para o motor nativo Rust com fallback seguro.

---

## 4. 📚 Plano de Atualização Dual-Doc (`UserDoc` + `TecDoc`)
- **`UserDoc` (`README.md`)**:
  - Atualização do diagrama de arquitetura incluindo `crates/actx-agent`.
  - Explicação sobre o motor ReAct nativo em Rust (sem dependência de LangChain/LangGraph, cold boot acelerado).
  - Documentação dos modos `/fast` e `/deep` da RFC-042.
- **`TecDoc` (`TECDOC.md`)**:
  - **Seção 75**: Deep-Dive na Arquitetura do `actx-agent`.
  - Diagrama de estados da FSM ReAct em Mermaid.
  - Tabela de complexidade algorítmica e consumo de memória.
  - Especificação dos eventos de streaming e catálogo de ferramentas.
  - ADR detalhando a purga do LangGraph e integração com `actx-lm`.

---

## 5. 🧪 Estratégia de Testes Automatizados & Proteção contra Regressão
- **Testes Unitários em Rust (`crates/actx-agent/tests/`)**:
  - `test_direct_response_without_tools`: 1 turno único sem tool calls.
  - `test_single_tool_execution`: Chamada de tool, observação e resposta final.
  - `test_tool_argument_validation_recovery`: Auto-correção após erro de argumentos.
  - `test_recursion_limit_enforcement`: Interrupção garantida ao bater `max_turns: 10`.
  - `test_streaming_events_emission`: Emissão ordenada de `ToolStart`, `ToolEnd`, `Delta`.
- **Testes de Integração PyO3 (`tests/unit/core/test_native_agent.py`)**:
  - Instanciação de `PyAgent` a partir de Python.
  - Execução com tools nativas e callbacks Python.
- **Runner Master E2E**:
  - Execução de `python tests/run_all_e2e.py` garantindo que os 362+ testes continuam 100% verdes.

---

## 6. 📝 Plano de Cenários de Teste Manual (`tests/MANUAL_TESTS.md`)
- Adição de novo cenário no topo de `tests/MANUAL_TESTS.md` sem truncamento dos anteriores:
  - **Cenário 1: Execução do Agente Nativo ReAct com actx-agent**:
    - Pré-requisitos: AnyContext compilado com a nova crate `actx-agent`.
    - Passo 1: Executar query simples ("Olá, quem é você?") $\rightarrow$ Verificar resposta direta sem tool calls.
    - Passo 2: Executar query que exige busca no workspace ("Como o sistema de autenticação trata expiração de token?") $\rightarrow$ Observar evento de tool call `search_db` no terminal e resposta sintetizada com citações.
    - Passo 3: Forçar query complexa com teto de turnos para validar corte seguro.

---

## 7. 🔒 Higiene de Segredos, Migração de Banco e Plano de Rollback
- **Higiene de Segredos**: Validação executada via `verify_gate.py --gate 3` (aprovado, sem credenciais expostas).
- **Migração de Banco de Dados**:
  - Adição não-destrutiva da coluna `search_mode TEXT DEFAULT 'auto'` na tabela `workspaces` do SQLite (`PRAGMA table_info` antes de `ALTER TABLE`).
- **Plano de Rollback Atômico**:
  - Em `src/any_context/core/agent.py`, a implementação mantém uma chave de feature flag (`ACTX_NATIVE_AGENT=1`). Caso desabilitada ou se ocorrer falha na carga do binário, há fallback seguro transparente.

---

## 8. ⏱️ Passo a Passo da Execução
1. **Passo 1**: Criar a nova crate `crates/actx-agent` com código fonte modular (`fsm.rs`, `tool.rs`, `session.rs`, `events.rs`, `lib.rs`).
2. **Passo 2**: Implementar testes unitários em Rust e validar compilação MSVC x64 via `cargo test -p actx-agent`.
3. **Passo 3**: Integrar `actx-agent` em `crates/any-context-core-rs` e expor classes PyO3 (`PyAgentEngine`, `PyAgentEvent`).
4. **Passo 4**: Criar `src/any_context/core/deep_search_models.py` (modelos RFC-042) e atualizar `src/any_context/core/agent.py`.
5. **Passo 5**: Rodar suíte completa de testes automatizados (`cargo test`, `pytest`, `run_all_e2e.py`).
6. **Passo 6**: Atualizar documentação Dual-Doc (`README.md` e Seção 75 do `TECDOC.md`).
7. **Passo 7**: Registrar o cenário de teste manual em `tests/MANUAL_TESTS.md`.
8. **Passo 8**: Validar Gate 8 com `verify_gate.py --gate 8` e preparar fechamento e release.
