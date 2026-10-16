---
plan_id: anycontext_blueprint_v0400_ativacao_operacional_do_gemma_4_local_onnx_e_presets_estaticos_de_workspace
project: any-context
title: 'Blueprint v0.40.0: Ativacao Operacional do Gemma 4 Local ONNX e Presets Estaticos de Workspace'
version: v0.40.0
status: superseded
author: Levi Silveira
created_at: '2026-10-14T16:25:55'
updated_at: '2026-10-14T16:38:18'
session_id: 01fd77a2-4853-4967-b572-f0ce69495ea0
tags:
- gemma4
- onnx
- presets
- workspace
- dev_cycle
supersedes: anycontext_blueprint_v0390_ingestao_web_com_jina_reader_rjinaai_rag_fundamentado_puro_e_matriz_de_7_dominios_semanticos
superseded_by: anycontext_blueprint_v0400_ativacao_operacional_do_gemma_4_local_onnx_guardrails_de_hardware_estilo_lm_studio_e_presets_seguros_de_workspace
parent_plan_id: null
milestones:
- name: 'Marco 1: Despacho do Gemma 4 no engine.rs'
  done: false
- name: 'Marco 2: Comando /preset no Core'
  done: false
- name: 'Marco 3: Paridade na TUI e Headless'
  done: false
- name: 'Marco 4: Testes Automatizados'
  done: false
- name: 'Marco 5: Dual-Doc MANUAL_TESTS e Release v0.40.0'
  done: false
status_log:
- at: '2026-10-14T16:38:18'
  note: Superseded by anycontext_blueprint_v0400_ativacao_operacional_do_gemma_4_local_onnx_guardrails_de_hardware_estilo_lm_studio_e_presets_seguros_de_workspace
- at: '2026-10-16T10:47:35'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# 📐 Blueprint de Engenharia: AnyContext v0.40.0
## Ativação Operacional do Gemma 4 Local ONNX & Presets Estáticos de Workspace

---

### 📌 Metadados do Plano
- **Projeto**: AnyContext (`actx`)
- **Versão-Alvo**: `v0.40.0` (Sucessora da `v0.39.0`)
- **Autor**: Levi Silveira & Antigravity Assistant
- **Data**: 2026-10-14
- **Fase**: Fase 3 do Master Plan (Kit de Models & Soberania Local)
- **Status**: Proposta Técnica para Aprovação (Gate 3 - `dev-cycle-protocol`)
- **Componentes**:
  - `crates/actx-cli`: Fiação de backend no `resolve_lm_provider` para despacho do Gemma 4 local ONNX, badges de prontidão na TUI e modo headless.
  - `crates/any-context-core-rs`: Comando canônico `/preset` (alias `/kit`) no `CommandEngine` com aplicação atômica de configurações ao workspace ativo.
  - `crates/actx-lm`: Validação e integração do `GemmaOnnxProvider` aos fluxos de conversação reais.
  - `crates/actx-onnx`: Resolução do diretório canônico de modelos `%LOCALAPPDATA%/actx/models/`.
- **Dual-Doc**: `README.md` (UserDoc), `TECDOC.md` (TecDoc - ADR-125), `tests/MANUAL_TESTS.md` (Cenário 40).

---

## 🏛️ 1. Objetivo & Decisões Arquiteturais (ADR-125)

### Contexto e Alinhamento Estratégico
Na versão `v0.38.0`, construímos a infraestrutura neural do Gemma 4 (`OnnxGemmaEngine` em `actx-onnx` e `GemmaOnnxProvider` em `actx-lm`) e os menus da TUI.
Contudo, identificamos duas oportunidades cruciais de simplificação e consolidação operacional:
1. **Soberania do Usuário vs. Roteamento Dinâmico de Inferência**:
   - Ficar alternando dinamicamente o modelo de inferência (ex: SLM local no turno 1 e Claude/GPT-4o no turno 2) quebra a previsibilidade do usuário, gera riscos de falhas de autenticação em ambientes restritos e viola a escolha deliberada de modelo feita pelo operador.
   - **Decisão**: A escolha de modelo feita pelo usuário é **soberana e imutável** durante o chat. O roteamento dinâmico (`SearchMode::Auto`) opera **exclusivamente na profundidade do RAG** (`Fast RAG` vs `Deep Search`), preservando 100% de previsibilidade.
2. **A Fiação Operacional do Gemma 4 no CLI (`resolve_lm_provider`)**:
   - Atualmente, a função `resolve_lm_provider` em `crates/actx-cli/src/engine.rs` possui desvios para `claude`, `gpt`, `gemini`, `deepseek`, `groq`, `ollama`, mas **não possui a ramificação para `gemma-4`**.
   - Se o usuário selecionar `gemma-4-12b` e enviar uma mensagem, o sistema cai no fallback da OpenAI (erro 404) ou no Mock, sem acionar o `GemmaOnnxProvider`.
   - **Decisão**: Conectar o despacho do Gemma 4 no `engine.rs`, localizando os arquivos `.onnx` no diretório canônico `%LOCALAPPDATA%/actx/models/` com garantia rigorosa do **ADR-110 (Zero-Breakage Guarantee)**: se os pesos ainda não estiverem no disco, emitir orientação amigável e executar fallback gracioso sem travar o terminal.
3. **Presets Estáticos de Configuração (`/preset` ou `/kit`)**:
   - Em vez de alternância mágica oculta durante o chat, o comando `/preset` atua como um **atalho determinístico** de configuração de workspace, aplicando um conjunto coeso de flags (`model`, `grounding_mode`, `search_mode`, `web_search_enabled`).

```mermaid
graph TD
    subgraph "1. Seleção Soberana do Usuário"
        USER_INPUT["Comando /model gemma-4-12b ou /preset offline"]
        DB["SQLite (workspaces.model = 'gemma-4-12b')"]
    end

    subgraph "2. Despachante do CLI (crates/actx-cli/src/engine.rs)"
        RESOLVER{"resolve_lm_provider(model)"}
        CHECK_WEIGHTS{"Arquivo ONNX existe em<br/>%LOCALAPPDATA%/actx/models/?"}
        ONNX_PROVIDER["GemmaOnnxProvider (actx-lm & actx-onnx)"]
        FALLBACK_PROVIDER["Fallback Gracioso (Cloud / Mock) + Orientação Amigável"]
    end

    subgraph "3. Execução do Chat"
        CHAT_LOOP["ReActOrchestrator / DeepSearchOrchestrator"]
        DEPTH_ROUTING{"SearchMode::Auto?<br/>Fast RAG vs Deep Search"}
        INFERENCE["Geração Local Soberana via ONNX Runtime"]
    end

    USER_INPUT --> DB
    DB --> RESOLVER
    RESOLVER --> CHECK_WEIGHTS
    CHECK_WEIGHTS -->|Sim (Pesos Presentes)| ONNX_PROVIDER
    CHECK_WEIGHTS -->|Não (Sem Pesos - ADR-110)| FALLBACK_PROVIDER
    ONNX_PROVIDER --> CHAT_LOOP
    FALLBACK_PROVIDER --> CHAT_LOOP
    CHAT_LOOP --> DEPTH_ROUTING
    DEPTH_ROUTING --> INFERENCE
```

---

## 🏛️ 2. Mapeamento de Paridade de Superfícies

| Superfície | Capacidade / Comportamento v0.40.0 |
| :--- | :--- |
| **CLI TUI Terminal** | Exibe badge `[Local ONNX: Gemma 4 ...]` no header e status bar. Suporte completo a `/preset` com abertura de submenu visual se chamado sem argumentos. |
| **CLI One-Shot (Headless)** | Permite `actx "pergunta" --model gemma-4-12b` executando a inferência localmente via ONNX sem abrir a TUI. |
| **Comando `/model`** | Lista modelos Gemma 4 e ao selecionar, verifica se o arquivo ONNX existe no disco, informando status imediato. |
| **Comando `/preset` (alias `/kit`)** | Aplica profiles pré-definidos (`offline`, `balanced`, `frontier`, `privacy`) de forma transparente e atômica no SQLite. |
| **Slash Commands Help** | Adiciona formalmente `/profile` e `/preset` à tabela `DEFAULT_SLASH_COMMANDS` com autocomplete e ajuda contextual. |

---

## 📁 3. Arquivos e Linhas Impactadas

1. **`crates/actx-cli/src/engine.rs`**:
   - Implementar ramo `else if raw_model.starts_with("gemma-4") || raw_model.starts_with("gemma")` em `resolve_lm_provider`.
   - Adicionar helper `resolve_gemma_model_path(tier)` apontando para `%LOCALAPPDATA%/actx/models/<filename>`.
   - Implementar construção de `GemmaOnnxProvider` com fallback de segurança.
2. **`crates/any-context-core-rs/src/commands/engine.rs`**:
   - Implementar comando `preset` (alias `kit`):
     - `offline`: modelo `gemma-4-12b`, modo `strict`, web-search `off`.
     - `balanced`: modelo padrão cloud (`gpt-4o-mini` ou similar), modo `hybrid`, web-search `on`.
     - `frontier`: modelo frontier (`claude-3-7-sonnet` ou `gpt-4o`), modo `proactive`, search `deep`.
     - `privacy`: modelo `gemma-4-12b`, modo `strict`, web-search `off`.
3. **`crates/actx-cli/src/commands/registry.rs`**:
   - Incluir `/profile` e `/preset` (alias `/kit`) no slice `DEFAULT_SLASH_COMMANDS`.
4. **`crates/actx-cli/src/tui/app.rs` & `menu.rs`**:
   - Ajustar badges na TUI para refletir `Local ONNX (Active)` quando o modelo ativo for Gemma 4.
5. **Crates Manifests (`Cargo.toml`)**:
   - Bump de versão para `0.40.0` nos 6 member crates e em `version.txt`.
6. **Dual-Doc & Testes Manuais**:
   - `TECDOC.md` (ADR-125), `README.md` (Guia Offline), `tests/MANUAL_TESTS.md` (Cenário 40).

---

## 📚 4. Plano de Atualização Dual-Doc (`UserDoc` + `TecDoc`)

- **`UserDoc` (`README.md`)**:
  - Nova seção no README: **"🧠 Soberania Total: Execução 100% Offline com Gemma 4 Local ONNX"**.
  - Documentação do comando `/preset [offline|balanced|frontier|privacy]` com tabela de impactos no workspace.
- **`TecDoc` (`TECDOC.md`)**:
  - Registro do **ADR-125: Ativação Operacional do Gemma 4 Local ONNX no Motor CLI e Presets Estáticos de Workspace**.
  - Diagrama Mermaid de ciclo de vida e resolução do `GemmaOnnxProvider`.
  - Matriz de comportamento em falha (ADR-110).

---

## 🧪 5. Estratégia de Testes Automatizados & Proteção contra Regressão

1. **Testes Unitários em `crates/actx-cli/src/engine.rs`**:
   - `test_resolve_lm_provider_gemma_family`: testa resolução de `gemma-4-compact`, `gemma-4-12b`, `gemma-4-27b`, `gemma-4-32b`.
   - `test_resolve_lm_provider_gemma_fallback_safe`: garante que sem os pesos baixados, o retorno é gracioso e não causa panic.
2. **Testes de Integração em `crates/any-context-core-rs/tests/command_tests.rs`**:
   - `test_preset_command_execution`: valida `/preset offline`, `/preset balanced`, `/preset frontier`, `/preset privacy` e confere a persistência atômica no SQLite (`NativeConfigDb`).
3. **Suíte Completa**:
   - Execução de `cargo test --workspace` garantindo zero regressões.

---

## 📝 6. Plano de Cenários de Teste Manual (`tests/MANUAL_TESTS.md`)

- **Cenário 40: Execução Operacional do Gemma 4 Local ONNX e Presets de Workspace (v0.40.0)**
  - Adicionado de forma estritamente acumulativa (preservando intactos os cenários 1 a 39).
  - Passos detalhados:
    1. Alternar para preset offline: `/preset offline`.
    2. Verificar configurações aplicadas via `/info` e `/status`.
    3. Selecionar modelo específico via `/model gemma-4-12b`.
    4. Enviar mensagem de chat e validar badge `[Local ONNX]` e resposta.
    5. Testar execução headless: `actx "Explique o que é RAG" --model gemma-4-compact`.

---

## 🔒 7. Higiene de Segredos, Migração de Banco e Plano de Rollback

- **Segredos**: Zero segredos ou tokens envolvidos.
- **Banco de Dados**: Reutiliza colunas já existentes na tabela `workspaces` (`model`, `grounding_mode`, `web_search_enabled`). Nenhuma migração destrutiva de schema.
- **Rollback**: Em caso de regressão no CI, `git checkout dev && git revert` restaura para a tag `v0.39.0` imediatamente.

---

## ⏱️ 8. Passo a Passo da Execução (Marcos 1 a 5)

| Marco | Descrição | Entregáveis |
| :--- | :--- | :--- |
| **Marco 1** | Despacho do Gemma 4 no `engine.rs` | Resolução canônica de caminhos ONNX e instanciação do `GemmaOnnxProvider` |
| **Marco 2** | Comando `/preset` no Core | Lógica do `/preset` e registro de `/profile` e `/preset` nos slash commands |
| **Marco 3** | Paridade na TUI e Headless | Badges e tratamento de fallback na TUI e CLI One-Shot |
| **Marco 4** | Testes Automatizados | Testes unitários de resolução e testes do `/preset` em `command_tests.rs` |
| **Marco 5** | Dual-Doc, MANUAL_TESTS & Release | ADR-125 no `TECDOC.md`, `README.md`, Cenário 40, `cargo test --workspace` e release `v0.40.0` |
