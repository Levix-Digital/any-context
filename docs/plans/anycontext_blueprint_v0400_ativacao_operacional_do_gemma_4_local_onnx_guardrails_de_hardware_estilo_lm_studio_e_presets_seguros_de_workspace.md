---
plan_id: anycontext_blueprint_v0400_ativacao_operacional_do_gemma_4_local_onnx_guardrails_de_hardware_estilo_lm_studio_e_presets_seguros_de_workspace
project: any-context
title: 'Blueprint v0.40.0: Ativacao Operacional do Gemma 4 Local ONNX, Guardrails de Hardware Estilo LM Studio e Presets Seguros de Workspace'
version: v0.40.0
status: superseded
author: Levi Silveira
created_at: '2026-10-14T16:38:18'
updated_at: '2026-10-15T08:22:07'
session_id: 01fd77a2-4853-4967-b572-f0ce69495ea0
tags:
- gemma4
- onnx
- guardrails
- hardware
- lm_studio_style
- presets
- workspace
- dev_cycle
supersedes: anycontext_blueprint_v0400_ativacao_operacional_do_gemma_4_local_onnx_e_presets_estaticos_de_workspace
superseded_by: anycontext_blueprint_v0410_downloader_assistido_de_pesos_gemma_4_onnx_na_tui_e_cli_actx_download_tier
parent_plan_id: null
milestones:
- name: 'Marco 1: Guardrail de Hardware no Core'
  done: false
- name: 'Marco 2: Despacho do Gemma 4 no engine.rs'
  done: false
- name: 'Marco 3: Comando /preset Inteligente'
  done: false
- name: 'Marco 4: Paridade na TUI e Testes Automatizados'
  done: false
- name: 'Marco 5: Dual-Doc MANUAL_TESTS e Release v0.40.0'
  done: false
status_log:
- at: '2026-10-15T08:22:07'
  note: Superseded by anycontext_blueprint_v0410_downloader_assistido_de_pesos_gemma_4_onnx_na_tui_e_cli_actx_download_tier
- at: '2026-10-16T10:47:35'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# 📐 Blueprint de Engenharia: AnyContext v0.40.0
## Ativação Operacional do Gemma 4 Local ONNX, Guardrails de Hardware Estilo LM Studio & Presets Seguros de Workspace

---

### 📌 Metadados do Plano
- **Projeto**: AnyContext (`actx`)
- **Versão-Alvo**: `v0.40.0` (Sucessora da `v0.39.0`)
- **Autor**: Levi Silveira & Antigravity Assistant
- **Data**: 2026-10-14
- **Fase**: Fase 3 do Master Plan (Kit de Models, Soberania Local & Proteção de Hardware)
- **Status**: Proposta Técnica Consolidada (Gate 3 - `dev-cycle-protocol`)
- **Componentes**:
  - `crates/actx-cli`: Fiação de backend no `resolve_lm_provider` para despacho do Gemma 4 local ONNX, guardrails de hardware no `/model` e badges no menu da TUI.
  - `crates/any-context-core-rs`: Comando canônico `/preset` (alias `/kit`) inteligente via `HardwareProfiler` e `CommandEngine` com persistência no SQLite.
  - `crates/actx-lm`: Validação e integração do `GemmaOnnxProvider` aos fluxos de conversação reais com fallback gracioso.
  - `crates/actx-onnx`: Resolução do diretório canônico de modelos `%LOCALAPPDATA%/actx/models/`.
- **Dual-Doc**: `README.md` (UserDoc), `TECDOC.md` (TecDoc - ADR-125), `tests/MANUAL_TESTS.md` (Cenário 40).

---

## 🏛️ 1. Objetivo & Decisões Arquiteturais (ADR-125)

### Contexto e Alinhamento Estratégico
Na versão `v0.38.0`, construímos a infraestrutura neural do Gemma 4 (`OnnxGemmaEngine` em `actx-onnx` e `GemmaOnnxProvider` em `actx-lm`) e os menus da TUI.
Contudo, duas decisões de arquitetura e usabilidade foram consolidadas:

1. **Soberania do Usuário vs. Roteamento Dinâmico de Inferência**:
   - Ficar alternando dinamicamente o modelo de inferência (ex: SLM local no turno 1 e Claude/GPT-4o no turno 2) quebra a previsibilidade do usuário, gera riscos de falhas de autenticação em ambientes restritos e viola a escolha deliberada de modelo feita pelo operador.
   - **Decisão**: A escolha de modelo feita pelo usuário é **soberana e imutável** durante o chat. O roteamento dinâmico (`SearchMode::Auto`) opera **exclusivamente na profundidade do RAG** (`Fast RAG` vs `Deep Search`), preservando 100% de previsibilidade.

2. **Guardrails de Hardware Ativos nos Modelos Locais (Estilo LM Studio)**:
   - Em modelos de nuvem (Claude, GPT, Gemini, Groq, DeepSeek), o hardware da máquina é irrelevante (consome apenas ~15MB de RAM para streaming HTTP). O usuário tem liberdade total para alternar entre eles a qualquer momento, bastando ter a chave de API configurada.
   - Em modelos locais (Gemma 4 ONNX), tentar carregar um modelo maior que a RAM física da máquina (ex: Gemma 4 32B em máquina de 16GB) causa **swap thrashing catastrófico**, congelando o sistema operacional e derrubando o processo por OOM.
   - **Decisão**: O `HardwareProfiler` atua como **guardrail ativo**:
     - 🟢 **Verde (Compatível)**: Seleção instantânea sem alertas.
     - 🟡 **Amarelo (Risco de Swap)**: Alerta explicativo de memória alta, permitindo prosseguir.
     - 🔴 **Vermelho (Incompatível / Excede a RAM Total)**: **Bloqueio com aviso enfático de risco de travamento**. A execução só é liberada se o usuário passar a flag explícita `--force` (`/model gemma-4-32b --force`), ciente de que é por sua conta e risco.
     - **Auto-Detecção em Upgrade**: Se o usuário adicionar memória física à máquina, o `HardwareProfiler` detecta a nova capacidade instantaneamente ($<5\text{ms}$ via OS APIs), desbloqueando modelos superiores automaticamente.

3. **Presets Estáticos e Seguros de Workspace (`/preset` ou `/kit`)**:
   - Em vez de um modelo arbitrário fixo, o comando `/preset` consulta o `HardwareProfiler` e ajusta as configurações de forma atômica e segura:
     - `offline`: Seleciona **o maior modelo local seguro (🟢)** suportado pela máquina (Compact para 8GB, 12B para 16GB, 27B para 32GB, 32B para 64GB+), com `/mode strict` e `/web-search off`.
     - `balanced`: Seleciona modelo cloud custo-benefício (`gpt-4o-mini` ou `groq`), `/mode hybrid`, `/search auto` e `/web-search on`.
     - `frontier`: Seleciona modelo frontier (`claude-3-7-sonnet` ou `gpt-4o`), `/mode proactive` e `/search deep`.
     - `privacy`: Seleciona modelo local seguro (🟢), com `/mode strict` e `/web-search off`.

4. **A Fiação Operacional do Gemma 4 no CLI (`resolve_lm_provider`)**:
   - Adicionar o ramo para `gemma-4` e `gemma` em `crates/actx-cli/src/engine.rs`, localizando os arquivos `.onnx` no diretório canônico `%LOCALAPPDATA%/actx/models/`.
   - Garantia **ADR-110 (Zero-Breakage Guarantee)**: se o arquivo ONNX ainda não tiver sido baixado, emitir orientação clara (informando o comando `/profile <tier>` ou `actx-installer`) e executar fallback gracioso sem travar o terminal.

```mermaid
graph TD
    subgraph "1. Seleção do Usuário & Guardrail de Hardware"
        INPUT["Comando /model <nome> ou /preset offline"]
        PROFILER["HardwareProfiler (<5ms OS APIs)"]
        CHECK_TIER{"Modelo Local?<br/>Gemma 4"}
        CLOUD_OK["Nuvem: Checa API Key e Aplica Imediatamente"]
        SEMAPHORE{"Semáforo de RAM:<br/>🟢 Verde | 🟡 Amarelo | 🔴 Vermelho"}
        LOCAL_OK["🟢 Aplica Modelo Local no SQLite"]
        LOCAL_BLOCK["🔴 BLOQUEIA com Aviso Enfático de OOM<br/>(Exige --force para prosseguir)"]
    end

    subgraph "2. Despachante do CLI (crates/actx-cli/src/engine.rs)"
        RESOLVER{"resolve_lm_provider(model)"}
        CHECK_FILE{"Arquivo ONNX existe em disco?"}
        ONNX_ENGINE["GemmaOnnxProvider (actx-lm)"]
        FALLBACK_SAFE["Fallback Gracioso (Cloud / Mock) + Orientação Amigável"]
    end

    subgraph "3. Execução do Chat"
        CHAT_LOOP["ReActOrchestrator / DeepSearchOrchestrator"]
        DEPTH_ROUTING{"SearchMode::Auto?<br/>Fast RAG vs Deep Search"}
        INFERENCE["Inferência Soberana e Previsível"]
    end

    INPUT --> CHECK_TIER
    CHECK_TIER -->|Não (Cloud)| CLOUD_OK
    CHECK_TIER -->|Sim (Local)| PROFILER
    PROFILER --> SEMAPHORE
    SEMAPHORE -->|🟢 Verde| LOCAL_OK
    SEMAPHORE -->|🔴 Vermelho| LOCAL_BLOCK
    LOCAL_BLOCK -->|Com --force| LOCAL_OK
    LOCAL_OK --> RESOLVER
    CLOUD_OK --> RESOLVER
    RESOLVER --> CHECK_FILE
    CHECK_FILE -->|Sim| ONNX_ENGINE
    CHECK_FILE -->|Não (ADR-110)| FALLBACK_SAFE
    ONNX_ENGINE --> CHAT_LOOP
    FALLBACK_SAFE --> CHAT_LOOP
    CHAT_LOOP --> DEPTH_ROUTING
    DEPTH_ROUTING --> INFERENCE
```

---

## 🏛️ 2. Mapeamento de Paridade de Superfícies

| Superfície | Capacidade / Comportamento v0.40.0 |
| :--- | :--- |
| **CLI TUI Terminal** | Menu de Modelos `[F1]` com badges estilo LM Studio: `🟢 [Compatível]`, `🟡 [Risco de Swap]`, `🔴 [Hardware Insuficiente - Exige X GB]`. Modelos vermelhos demarcados com alerta. |
| **Comando `/model`** | Validação ativa de hardware. Bloqueia modelos locais 🔴 com aviso explicativo detalhando RAM total vs necessária, instruindo o uso de `--force` caso o usuário queira arriscar. |
| **Comando `/preset` (alias `/kit`)** | Aplica atalhos de workspace atômicos no SQLite (`offline`, `balanced`, `frontier`, `privacy`) resolvendo automaticamente o modelo local seguro ótimo. |
| **CLI One-Shot (Headless)** | Permite `actx "pergunta" --model gemma-4-12b` executando a inferência localmente via ONNX sem abrir a TUI. |
| **Slash Commands Help** | Adiciona formalmente `/profile` e `/preset` à tabela `DEFAULT_SLASH_COMMANDS` com autocomplete e ajuda contextual. |

---

## 📁 3. Arquivos e Linhas Impactadas

1. **`crates/any-context-core-rs/src/hardware/mod.rs`**:
   - Adicionar helper `validate_model_safety(model_name: &str, metrics: &HardwareMetrics) -> HardwareSafetyVerdict`:
     - Retorna `Safe` (🟢), `Tight` (🟡) ou `Unsafe { required_ram_gb: f32, total_ram_gb: f32 }` (🔴).
2. **`crates/actx-cli/src/engine.rs`**:
   - Implementar ramo `else if raw_model.starts_with("gemma-4") || raw_model.starts_with("gemma")` em `resolve_lm_provider`.
   - Adicionar helper `resolve_gemma_model_path(tier)` apontando para `%LOCALAPPDATA%/actx/models/<filename>`.
   - Implementar construção de `GemmaOnnxProvider` com fallback de segurança.
3. **`crates/any-context-core-rs/src/commands/engine.rs`**:
   - No comando `model`: validar guardrail de hardware. Se `Unsafe` e não houver flag `--force`, retornar erro amigável com métricas detalhadas.
   - Implementar comando `preset` (alias `kit`):
     - `offline`: seleciona o maior modelo local seguro (🟢), modo `strict`, web-search `off`.
     - `balanced`: modelo padrão cloud (`gpt-4o-mini` ou similar), modo `hybrid`, web-search `on`.
     - `frontier`: modelo frontier (`claude-3-7-sonnet` ou `gpt-4o`), modo `proactive`, search `deep`.
     - `privacy`: modelo local seguro (🟢), modo `strict`, web-search `off`.
4. **`crates/actx-cli/src/commands/registry.rs`**:
   - Incluir `/profile` e `/preset` (alias `/kit`) no slice `DEFAULT_SLASH_COMMANDS`.
5. **`crates/actx-cli/src/tui/app.rs` & `menu.rs`**:
   - Integrar badges coloridos de semáforo de hardware nos itens de modelo local na TUI.
6. **Crates Manifests (`Cargo.toml`)**:
   - Bump de versão para `0.40.0` nos 6 member crates e em `version.txt`.
7. **Dual-Doc & Testes Manuais**:
   - `TECDOC.md` (ADR-125), `README.md` (Guia Offline e Guardrails), `tests/MANUAL_TESTS.md` (Cenário 40).

---

## 📚 4. Plano de Atualização Dual-Doc (`UserDoc` + `TecDoc`)

- **`UserDoc` (`README.md`)**:
  - Nova seção no README: **"🧠 Soberania Local & Guardrails de Hardware (Estilo LM Studio)"**.
  - Documentação das flags de segurança e comandos `/model <nome> [--force]` e `/preset [offline|balanced|frontier|privacy]`.
- **`TecDoc` (`TECDOC.md`)**:
  - Registro do **ADR-125: Ativação Operacional do Gemma 4 Local ONNX no Motor CLI, Guardrails de Hardware e Presets Estáticos de Workspace**.
  - Diagrama Mermaid de ciclo de vida e resolução do `GemmaOnnxProvider`.
  - Tabela de mapeamento de requisitos de RAM e KV-cache dos modelos locais vs comportamento em falha (ADR-110).

---

## 🧪 5. Estratégia de Testes Automatizados & Proteção contra Regressão

1. **Testes Unitários em `crates/any-context-core-rs/src/hardware/mod.rs`**:
   - `test_validate_model_safety_verdicts`: testa veredictos 🟢, 🟡 e 🔴 para os 4 tiers do Gemma 4 em diferentes simulações de RAM (8GB, 16GB, 32GB, 64GB).
2. **Testes Unitários em `crates/actx-cli/src/engine.rs`**:
   - `test_resolve_lm_provider_gemma_family`: testa resolução de `gemma-4-compact`, `gemma-4-12b`, `gemma-4-27b`, `gemma-4-32b`.
   - `test_resolve_lm_provider_gemma_fallback_safe`: garante que sem os pesos baixados, o retorno é gracioso e não causa panic.
3. **Testes de Integração em `crates/any-context-core-rs/tests/command_tests.rs`**:
   - `test_model_command_hardware_guardrail_block_and_force`: valida que tentar setar um modelo 🔴 sem `--force` retorna erro informativo, e com `--force` aplica a configuração.
   - `test_preset_command_execution`: valida `/preset offline`, `/preset balanced`, `/preset frontier`, `/preset privacy` e confere a persistência atômica no SQLite (`NativeConfigDb`).
4. **Suíte Completa**:
   - Execução de `cargo test --workspace` garantindo zero regressões.

---

## 📝 6. Plano de Cenários de Teste Manual (`tests/MANUAL_TESTS.md`)

- **Cenário 40: Execução Operacional do Gemma 4 Local ONNX, Guardrails de Hardware e Presets de Workspace (v0.40.0)**
  - Adicionado de forma estritamente acumulativa (preservando intactos os cenários 1 a 39).
  - Passos detalhados:
    1. Testar guardrail de hardware: tentar `/model gemma-4-32b` em máquina com $<64\text{ GB}$ de RAM e verificar o bloqueio com aviso amigável.
    2. Testar override explícito: `/model gemma-4-32b --force` e verificar aplicação.
    3. Alternar para preset offline seguro: `/preset offline` e validar que o modelo escolhido é o 🟢 compatível com a máquina.
    4. Verificar configurações aplicadas via `/info` e `/status`.
    5. Enviar mensagem de chat e validar badge `[Local ONNX]` e resposta.
    6. Testar execução headless: `actx "Explique o que é RAG" --model gemma-4-compact`.

---

## 🔒 7. Higiene de Segredos, Migração de Banco e Plano de Rollback

- **Segredos**: Zero segredos ou tokens envolvidos.
- **Banco de Dados**: Reutiliza colunas já existentes na tabela `workspaces` (`model`, `grounding_mode`, `web_search_enabled`). Nenhuma migração destrutiva de schema.
- **Rollback**: Em caso de regressão no CI, `git checkout dev && git revert` restaura para a tag `v0.39.0` imediatamente.

---

## ⏱️ 8. Passo a Passo da Execução (Marcos 1 a 5)

| Marco | Descrição | Entregáveis |
| :--- | :--- | :--- |
| **Marco 1** | Guardrail de Hardware no Core | Função `validate_model_safety` no `HardwareProfiler` e checagem no comando `/model` |
| **Marco 2** | Despacho do Gemma 4 no `engine.rs` | Resolução canônica de caminhos ONNX e instanciação do `GemmaOnnxProvider` |
| **Marco 3** | Comando `/preset` Inteligente | Lógica do `/preset` com seleção do melhor modelo local 🟢 e registro nos slash commands |
| **Marco 4** | Paridade na TUI e Testes Automatizados | Badges na TUI, testes unitários de resolução e testes do `/preset` e `/model` |
| **Marco 5** | Dual-Doc, MANUAL_TESTS & Release | ADR-125 no `TECDOC.md`, `README.md`, Cenário 40, `cargo test --workspace` e release `v0.40.0` |
