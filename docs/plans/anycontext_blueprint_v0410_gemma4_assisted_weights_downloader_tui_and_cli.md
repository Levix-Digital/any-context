# 📐 Blueprint de Engenharia: AnyContext v0.41.0
## Downloader Assistido de Pesos Gemma 4 ONNX na TUI e CLI (`actx download <tier>`)

---

### 📌 Metadados do Plano
- **Projeto**: AnyContext (`actx`)
- **Versão-Alvo**: `v0.41.0` (Sucessora da `v0.40.0`)
- **Autor**: Levi Silveira & Antigravity Assistant
- **Data**: 2026-10-15
- **Localização / Sede**: Calgary, Alberta, Canadá (100% Global Product)
- **Fase**: Fase 3 do Master Plan (Kit de Models, Soberania Local & Downloader Assistido)
- **Status**: Proposta Técnica Consolidada (Gate 3 - `dev-cycle-protocol`)
- **Componentes Impactados**:
  - `crates/any-context-core-rs`: Expansão do catálogo `ONNX_CATALOG` com a família Gemma 4, categoria `LanguageModel`, pre-flight check de espaço em disco e guardrail de RAM.
  - `crates/actx-cli`: Subcomando CLI headless `actx download <tier> [--force] [--check]`, barra de progresso no terminal, slash command `/download` no chat da TUI, ativação direta por clique no menu `[F1]` e cancelamento gracioso via `Esc`.
  - `crates/actx-onnx`: Resolução de caminhos, metadados e tamanhos nominais dos arquivos `.onnx` no diretório canônico `%LOCALAPPDATA%/actx/models/`.
- **Dual-Doc**: `README.md` (UserDoc), `TECDOC.md` (TecDoc - ADR-126), `tests/MANUAL_TESTS.md` (Cenário 41).

---

## 🏛️ 1. Objetivo & Decisões Arquiteturais (ADR-126)

### Contexto & Problema
Na versão `v0.40.0`, ativamos o despacho operacional do Gemma 4 Local ONNX (`resolve_lm_provider`) com guardrails de hardware estilo LM Studio (`validate_model_safety`), garantindo que o AnyContext não trave caso o usuário selecione modelos locais sem os pesos baixados (fallback offline gracioso ADR-110).

Entretanto, para obter os pesos `.onnx`, o usuário atualmente precisava baixar os arquivos externamente ou utilizar o instalador genérico, sem feedback visual direto na TUI ou facilidade no terminal CLI.

### Decisões Arquiteturais Fundamentais (ADR-126)

1. **Paridade Hexagonal de Download (CLI + TUI + Menu)**:
   - Toda a lógica de catalogação, URLs de CDN/Hugging Face, pré-requisitos de disco e canais de streaming com cancelamento reside no Core (`OnnxModelManager` em `any-context-core-rs`).
   - O usuário pode disparar o download de três formas idênticas:
     - **CLI Headless**: `actx download <tier> [--force]` (ideal para automações e setups em terminal puro).
     - **TUI Chat**: Slash command `/download <tier> [--force]` (com telemetria em tempo real no rodapé e cancelamento com `Esc`).
     - **TUI Menu `[F1]`**: Ao navegar até um modelo com status `(Instalar)` e teclar `[Enter]`, o download é iniciado imediatamente com feedback no chat.

2. **Pre-Flight Checks Ativos (Espaço em Disco & Guardrails de Hardware)**:
   - **Verificação de Disco**: Antes de iniciar o download de arquivos pesados (1.8GB a 20GB), o sistema verifica o espaço livre na partição do diretório de modelos (`%LOCALAPPDATA%/actx/models/`). Se `espaço_livre < tamanho_modelo + 500MB`, o download é abortado com mensagem explicativa de falta de espaço em disco.
   - **Verificação de RAM**: O `HardwareProfiler` afere a RAM física da máquina. Se o modelo for 🔴 `Unsafe` (ex: tentar baixar Gemma 4 32B em máquina de 16GB), o sistema bloqueia o download com diagnóstico enfático de OOM, exigindo a flag explícita `--force` (`actx download 32b --force` ou `/download 32b --force`).

3. **Escrita Atômica e Cancelamento Limpo**:
   - O download ocorre em streaming chunked HTTP diretamente para um arquivo temporário (`<filename>.download.tmp`).
   - O arquivo só é renomeado para `<filename>` após a conclusão de 100% dos bytes com flush de disco.
   - Pressionar `Esc` na TUI ou `Ctrl+C` no CLI aborta imediatamente a requisição HTTP e apaga o arquivo temporário `.download.tmp`, garantindo **zero arquivos corrompidos ou órfãos no disco**.

4. **Resolução Flexível de Nomes de Tier**:
   - Aceita tanto nomes completos (`gemma-4-compact`, `gemma-4-12b`, `gemma-4-27b`, `gemma-4-32b`) quanto aliases intuitivos (`compact`, `12b`, `27b`, `32b`, `all`).

```mermaid
flowchart TD
    subgraph "1. Disparo do Download"
        CMD_CLI["CLI: actx download <tier> [--force]"]
        CMD_CHAT["Chat TUI: /download <tier> [--force]"]
        CMD_MENU["Menu [F1]: Seleção de modelo '(Instalar)'"]
    end

    subgraph "2. Pre-Flight Checks (any-context-core-rs)"
        RESOLVE["OnnxModelManager::resolve_spec(tier)"]
        DISK_CHECK{"Espaço em Disco Suficiente?<br/>(Livre >= Tamanho + 500MB)"}
        RAM_CHECK{"HardwareProfiler::validate_model_safety<br/>🟢 Safe | 🟡 Tight | 🔴 Unsafe"}
        FORCE_CHECK{"Flag --force presente?"}
        BLOCK_DISK["❌ Erro: Espaço insuficiente em disco"]
        BLOCK_RAM["❌ Bloqueio Defensivo: Excede RAM física<br/>(Exige --force)"]
    end

    subgraph "3. Pipeline de Streaming Atômico"
        TMP_FILE["Criação de <model>.download.tmp"]
        STREAM["Streaming HTTP Chunked (Reqwest)"]
        PROGRESS["Telemetria: MBs, %, MB/s, ETA, Spinner"]
        CANCEL{"Usuário pressionou [Esc] ou Ctrl+C?"}
        CLEANUP["🛑 Aborta requisição e remove .download.tmp"]
        FINALIZE["Renomeia .download.tmp -> .onnx atômico"]
        READY["🎉 Modelo Pronto para Inferência Local (v0.40.0)"]
    end

    CMD_CLI --> RESOLVE
    CMD_CHAT --> RESOLVE
    CMD_MENU --> RESOLVE
    RESOLVE --> DISK_CHECK
    DISK_CHECK -->|Não| BLOCK_DISK
    DISK_CHECK -->|Sim| RAM_CHECK
    RAM_CHECK -->|🟢 Safe / 🟡 Tight| TMP_FILE
    RAM_CHECK -->|🔴 Unsafe| FORCE_CHECK
    FORCE_CHECK -->|Não| BLOCK_RAM
    FORCE_CHECK -->|Sim| TMP_FILE

    TMP_FILE --> STREAM
    STREAM --> PROGRESS
    STREAM --> CANCEL
    CANCEL -->|Sim| CLEANUP
    CANCEL -->|Não (100%)| FINALIZE
    FINALIZE --> READY
```

---

## 🏛️ 2. Mapeamento de Paridade de Superfícies

| Superfície | Capacidade / Comportamento v0.41.0 |
| :--- | :--- |
| **CLI Headless (`actx download`)** | Executa download no terminal stdout com barra de progresso gráfica, MB/s e ETA. Suporta flags `--force` e `--check` (para inspecionar tabela de modelos instalados). |
| **Chat TUI (`/download`)** | Slash command interativo com barra de progresso em tempo real no rodapé, cancelamento instantâneo via `[Esc]` e notificação no histórico de chat. |
| **Menu TUI `[F1]`** | Modelos não instalados disparam automaticamente o download ao teclar `[Enter]` na lista de modelos. |
| **Hardware Guardrails** | Bloqueia modelos 🔴 contra falta de disco ou excesso de RAM em todas as superfícies com diagnósticos claros. |

---

## 📁 3. Arquivos e Linhas Impactadas

1. **`crates/any-context-core-rs/src/ingestion/model_manager.rs`**:
   - Adicionar variante `OnnxModelCategory::LanguageModel` ao enum `OnnxModelCategory`.
   - Adicionar os 4 modelos Gemma 4 (`gemma-4-compact`, `gemma-4-12b`, `gemma-4-27b`, `gemma-4-32b`) à constante `ONNX_CATALOG`.
   - Implementar método `OnnxModelManager::resolve_spec(name: &str) -> Option<&'static OnnxModelSpec>`.
   - Implementar método `OnnxModelManager::check_download_safety(spec: &OnnxModelSpec, force: bool) -> Result<(), String>`.
2. **`crates/actx-cli/src/cli/args.rs`**:
   - Adicionar variante `CliCommand::Download { tier: Option<String>, force: bool, check: bool }`.
3. **`crates/actx-cli/src/cli/oneshot.rs`**:
   - Implementar a função `handle_download(tier_opt: Option<&str>, force: bool, check: bool)` com barra de progresso no terminal, telemetria de velocidade e suporte a Ctrl+C.
4. **`crates/actx-cli/src/commands/registry.rs`**:
   - Registrar o comando `/download` (aliases `dl`, `get-model`) em `DEFAULT_SLASH_COMMANDS`.
5. **`crates/actx-cli/src/tui/app.rs`**:
   - Adicionar tratamento para `/download [tier] [--force]` no despachante de comandos de chat.
   - Integrar `check_download_safety` antes de instanciar `start_model_download`.
   - Atualizar a notificação de término para sugerir `/model <tier>` e já marcar o modelo como pronto para uso.
6. **`crates/actx-cli/src/tui/menu.rs`**:
   - No `handle_menu_enter`: quando um modelo local com status `(Instalar)` for selecionado, disparar `start_model_download` para ele com feedback acolhedor.
7. **Bump de Versão**:
   - Bump para `v0.41.0` em `version.txt`, `crates/actx-cli/Cargo.toml`, `crates/any-context-core-rs/Cargo.toml`, etc.

---

## 📚 4. Plano de Atualização Dual-Doc (`UserDoc` + `TecDoc`)

- **`UserDoc` (`README.md`)**:
  - Nova subseção: **"📥 Download e Gerenciamento Assistido de Pesos Locais"**.
  - Documentação dos comandos `actx download <tier>`, `actx download --check` e `/download <tier>`.
- **`TecDoc` (`TECDOC.md`)**:
  - Registro formal do **ADR-126: Downloader Assistido de Pesos Gemma 4 ONNX no CLI e TUI com Pre-Flight Checks e Telemetria em Tempo Real**.
  - Diagrama de sequência de download assíncrono com cancelamento gracioso via canais Tokio `watch` e `mpsc`.

---

## 🧪 5. Estratégia de Testes Automatizados & Proteção contra Regressão

1. **Testes Unitários em `crates/any-context-core-rs/src/ingestion/model_manager.rs`**:
   - `test_resolve_spec_aliases`: valida que `"compact"`, `"12b"`, `"gemma-4-27b"` e `"32b"` resolvem para as especificações corretas.
   - `test_check_download_safety_disk_and_ram`: valida veredito de segurança com diferentes simulações de disco e RAM.
2. **Testes Unitários em `crates/actx-cli/src/cli/args.rs`**:
   - `test_cli_args_download_command_parsing`: valida parsing de `actx download 12b`, `actx download --check` e `actx download 32b --force`.
3. **Testes de Integração em `crates/actx-cli/tests/`**:
   - `test_download_cancellation_cleans_temporary_file`: garante que o arquivo temporário `.download.tmp` é excluído imediatamente em caso de cancelamento.
4. **Suíte Completa**:
   - Execução de `cargo test --workspace` garantindo 100% de testes verdes.

---

## 📝 6. Plano de Cenários de Teste Manual (`tests/MANUAL_TESTS.md`)

- **Cenário 41: Downloader Assistido de Modelos Locais Gemma 4 ONNX, Validação de Disco e Telemetria de Download (v0.41.0)**
  - Adicionado de forma estritamente acumulativa (preservando intactos os cenários 1 a 40).
  - Passos detalhados:
    1. **Inspeção de Modelos via CLI (`actx download --check`)**:
       - Executar `actx download --check` e validar tabela formatada listando os 4 tiers do Gemma 4, tamanho nominal, requisitos de RAM e status no disco.
    2. **Validação de Pre-Flight Guardrail no Download**:
       - Tentar `actx download 32b` em máquina com $<44\text{ GB}$ de RAM sem `--force` e verificar o bloqueio de segurança.
       - Tentar `actx download 32b --force` e verificar o início do download.
    3. **Teste de Barra de Progresso no CLI Headless**:
       - Executar `actx download compact` e validar barra de progresso visual, porcentagem e MB/s.
    4. **Teste de Download no Chat da TUI com Cancelamento com `Esc`**:
       - Abrir a TUI (`actx`) e executar `/download 12b`.
       - Verificar telemetria no rodapé: spinner, nome do modelo, MBs transferidos e porcentagem.
       - Pressionar `Esc` durante o download e comprovar mensagem no chat: `🛑 [Download Cancelled]` e remoção do arquivo temporário `.tmp`.
    5. **Teste de Download via Menu `[F1]`**:
       - Abrir menu `[F1]` -> `AI Models`, navegar até um modelo com status `(Instalar)` e teclar `[Enter]`.
       - Verificar início automático do download com notificação no chat.

---

## 🔒 7. Higiene de Segredos, Migração de Banco e Plano de Rollback

- **Segredos**: Zero tokens ou segredos envolvidos (downloads públicos com endpoints HTTPS).
- **Banco de Dados**: Nenhuma alteração estrutural no SQLite. Os arquivos são gerenciados diretamente no filesystem em `%LOCALAPPDATA%/actx/models/`.
- **Rollback**: Caso ocorra regressão, o commit na branch `dev` pode ser revertido imediatamente sem impacto na versão `v0.40.0`.

---

## ⏱️ 8. Passo a Passo da Execução (Marcos 1 a 5)

| Marco | Descrição | Entregáveis |
| :--- | :--- | :--- |
| **Marco 1** | Catálogo Gemma 4 & Pre-Flight Checks no Core | Expansão de `ONNX_CATALOG`, enum `LanguageModel`, `resolve_spec` e `check_download_safety` |
| **Marco 2** | Subcomando CLI `actx download` & Barra de Progresso | `CliCommand::Download` em `args.rs` e `handle_download` em `oneshot.rs` |
| **Marco 3** | Slash Command `/download` na TUI & Atalho no Menu `[F1]` | Registro em `registry.rs`, despachante no `app.rs` e ação no `menu.rs` |
| **Marco 4** | Testes Automatizados & Validação da Suíte | Testes unitários de catálogo, parsing de args e testes de cancelamento |
| **Marco 5** | Dual-Doc, MANUAL_TESTS (Cenário 41) & Release v0.41.0 | Atualização de `README.md`, ADR-126 em `TECDOC.md`, `tests/MANUAL_TESTS.md` e release |
