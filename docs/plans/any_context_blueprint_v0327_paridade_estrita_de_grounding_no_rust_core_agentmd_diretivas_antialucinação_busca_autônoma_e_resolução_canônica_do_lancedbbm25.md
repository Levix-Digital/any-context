---
plan_id: any_context_blueprint_v0327_paridade_estrita_de_grounding_no_rust_core_agentmd_diretivas_antialucinação_busca_autônoma_e_resolução_canônica_do_lancedbbm25
project: any-context
title: 'Blueprint v0.32.7: Paridade Estrita de Grounding no Rust Core (AGENT.md, Diretivas Antialucinação, Busca Autônoma e Resolução Canônica do LanceDB/BM25)'
version: v0.32.7
status: superseded
author: Levi Silveira
created_at: '2026-10-02T15:43:59'
updated_at: '2026-10-02T15:53:47'
session_id: 0a07fe95-3c95-480f-904a-97aa13635137
tags:
- grounding
- strict-mode
- react-agent
- lancedb
- bm25
- system-prompt
- citations
- parity
supersedes: any_context_blueprint_v0324_desacoplamento_universal_do_motor_de_comandos_e_separacao_grounding_mode_vs_search_mode
superseded_by: any_context_blueprint_v0327_paridade_estrita_de_grounding_no_rust_core_injeção_de_agentmd_lancedb_canônico_único_sem_legado_streaming_real_e_persistência_de_modos
parent_plan_id: null
milestones:
- name: 'M1: Resolução Canônica do LanceDB e BM25 Index'
  done: false
- name: 'M2: Motor de System Prompt e Grounding Directives com AGENT.md'
  done: false
- name: 'M3: Ferramenta search_db Resiliente com Tokenização e BM25'
  done: false
- name: 'M4: Testes Automatizados de Paridade e Isolamento'
  done: false
- name: 'M5: Dual-Doc (README.md TECDOC.md com ADR-090)'
  done: false
- name: 'M6: Testes Manuais Acumulativos (MANUAL_TESTS.md Cenário 9)'
  done: false
- name: 'M7: Fechamento Release v0.32.7 e CI Watch'
  done: false
status_log:
- at: '2026-10-02T15:53:47'
  note: Superseded by any_context_blueprint_v0327_paridade_estrita_de_grounding_no_rust_core_injeção_de_agentmd_lancedb_canônico_único_sem_legado_streaming_real_e_persistência_de_modos
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.32.7: Paridade Estrita de Grounding no Rust Core (AGENT.md, Diretivas Antialucinação, Busca Autônoma e Resolução Canônica do LanceDB/BM25)

## 1. 🎯 Objetivo & Decisões Arquiteturais (ADR-090)
### Contexto e Diagnóstico da Regressão Crítica:
Após a refatoração e migração da interface CLI/TUI para Rust (`actx`), o usuário constatou uma regressão funcional crítica no comportamento do agente em modo `STRICT`:
1. Ao perguntar sobre programas de imigração para AI Engineer no Canadá (`Quais programas de imigração onde posso imigrar de Trabalhador Temporário para Residente Permanente como AI Engineer?`), o modelo respondeu longos parágrafos de conhecimento pré-treinado geral (Express Entry, PNP, Alberta, etc.) **sem invocar a ferramenta `search_db`**.
2. Ao ser questionado (*"De onde veio essa informação?"*), o modelo admitiu que utilizou memória paramétrica genérica e perguntou passivamente: *"As informações que forneci são baseadas em conhecimentos gerais... Se você precisar... posso procurar em documentos... Deseja que eu faça isso?"*.
3. **Causas Raízes Identificadas**:
   - **System Prompt Esvaziado**: Em `crates/actx-cli/src/engine.rs`, o prompt do sistema foi reduzido a um bloco simplificado de 10 linhas que apenas mencionava `- Grounding Strategy: {grounding_mode}`, sem injetar o conteúdo de `config/AGENT.md`, sem proibir memória paramétrica, sem impor busca obrigatória prévia via `search_db` e sem diretivas de fidelidade.
   - **Desconexão de Caminho do LanceDB**: O Rust buscava LanceDB em `%LOCALAPPDATA%\AnyContext\lancedb` (pasta vazia com 0 registros), enquanto toda a base vetorial do usuário (34.124 chunks) reside canonicamente em `%LOCALAPPDATA%\AnyContext\data\context_db\lancedb`.
   - **Degradação de Busca em `search_db`**: A ferramenta em Rust realizava apenas uma busca literal SQL `text LIKE '%query%'` com a frase inteira do usuário. Frases naturais ou perguntas em português sobre documentos em inglês retornavam 0 resultados, ignorando o índice `BM25Index` já existente (`bm25_index.bin`) de 12 MB na pasta do usuário.

### Decisões Arquiteturais (ADR-090):
1. **Injeção Obrigatória de AGENT.md e Diretivas por Modo**:
   - `crates/actx-cli` incorporará o `AGENT.md` canônico (compilado via `include_str!` com fallback para leitura em runtime) e montará um prompt do sistema estrito com paridade 100% com o Python.
   - No modo `STRICT`: proibição categórica de respostas paramétricas, mandato de busca autônoma obrigatória via `search_db` antes de responder, proibição absoluta de perguntar *"Deseja que eu procure nos documentos?"*, e declaração formal de ausência (`⚠️ Essa informação não consta nos documentos deste workspace.`) caso nada seja encontrado.
   - No modo `HYBRID`: prioridade aos documentos locais com separação explícita de seções (`### 📂 Informações do Workspace` vs `### 💡 Conhecimento Geral do Modelo` com rótulo declaratório).
   - No modo `PROACTIVE`: integração fluida com citação explícita de fontes.
2. **Resolução Canônica de Armazenamento LanceDB e BM25**:
   - Criar `any_context_core_rs::storage::get_default_lancedb_path() -> PathBuf` que inspeciona sequencialmente os caminhos padrão do OS (`data/context_db/lancedb` antes de `lancedb`), detectando onde os arquivos `.lance` realmente existem.
3. **Ferramenta `search_db` Inteligente (BM25 + LanceDB Metadata)**:
   - A ferramenta `search_db` em Rust utilizará `bm25_index.bin` quando disponível para pontuação lexical rápida (Okapi BM25) e complementará com busca em metadados no LanceDB com tokenização de termos e keywords, formatando os chunks com título de arquivo, score e trecho relevante.

---

## 2. 🏛️ Mapeamento de Paridade de Superfícies
- **Core Domain & Storage (`crates/any-context-core-rs`)**:
  - `storage/lancedb.rs`: Exposição de `get_default_lancedb_path()`.
  - `commands/engine.rs`: Atualização de todas as chamadas de path para a função canônica.
- **Engine & System Prompt (`crates/actx-cli`)**:
  - `src/prompt.rs`: Novo módulo dedicado à construção estruturada do system prompt (AGENT.md + Active Workspace + Grounding Directives + Temporal Recency + Citations).
  - `src/engine.rs`: Substituição do prompt estático pelo `build_system_prompt()`; aprimoramento da ferramenta `search_db` para usar o caminho canônico e busca por termos/BM25.
- **CLI One-Shot & TUI Interactive**:
  - Ambos consom `build_agent_sync()` e recebem imediatamente as mesmas diretivas rígidas de Grounding e o mesmo motor de busca canônico.

---

## 3. 📁 Arquivos e Linhas Impactadas
1. `crates/any-context-core-rs/src/storage/lancedb.rs`:
   - Implementação de `pub fn get_default_lancedb_path() -> PathBuf`.
2. `crates/any-context-core-rs/src/storage/mod.rs` & `crates/any-context-core-rs/src/lib.rs`:
   - Re-export de `get_default_lancedb_path`.
3. `crates/any-context-core-rs/src/commands/engine.rs`:
   - Uso de `crate::storage::get_default_lancedb_path()` no lugar da função local obsoleta.
4. `crates/actx-cli/src/prompt.rs` (Novo):
   - Módulo puro para geração de system prompt canônico com `AGENT.md` embutido e matriz de Grounding.
5. `crates/actx-cli/src/engine.rs`:
   - Integração de `prompt::build_system_prompt()`.
   - Atualização de `search_db` com resolução canônica de caminho e busca tokenizada.
6. `crates/actx-cli/src/lib.rs` / `main.rs`:
   - Declaração de `pub mod prompt;`.
7. `crates/actx-cli/tests/`:
   - Testes unitários para geração do prompt e diretivas nos três modos (`strict`, `hybrid`, `proactive`).
8. `TECDOC.md`:
   - Registro do ADR-090 e diagramas Mermaid de fluxo de Grounding.
9. `tests/MANUAL_TESTS.md`:
   - Adição do Cenário 9 (Validação Estrita de Grounding sem Alucinação Paramétrica).
10. `Cargo.toml` / `pyproject.toml`:
    - Bump para `v0.32.7`.

---

## 4. 📚 Plano de Atualização Dual-Doc (`UserDoc` + `TecDoc`)
- **`UserDoc` (`README.md`)**:
  - Detalhar a garantia de fidelidade do modo `STRICT`: nunca responde com dados externos quando o documento não existe, exigindo busca autônoma local.
- **`TecDoc` (`TECDOC.md`)**:
  - ADR-090: Arquitetura de Grounding Parity no Rust Core e Resolução Canônica Multi-OS de LanceDB/BM25.
  - Diagrama Mermaid do pipeline de injeção de prompt e ciclo ReAct do agente.

---

## 5. 🧪 Estratégia de Testes Automatizados & Proteção contra Regressão
- **Testes Unitários em Rust**:
  - `test_system_prompt_strict_mode`: Valida que o prompt gerado em modo `strict` contém mandatos explícitos: ausência de alucinação paramétrica, mandato autônomo de busca via `search_db`, proibição de perguntar permissão para buscar documentos locais, e declaração de ausência formal.
  - `test_system_prompt_hybrid_mode`: Valida separação em seções de workspace e conhecimento geral rotulado.
  - `test_canonical_lancedb_path_resolution`: Valida que `get_default_lancedb_path()` prioriza pastas existentes contendo `workspace_chunks.lance`.
  - Execução completa: `cargo test --workspace` (149+ testes verdes garantidos).

---

## 6. 📝 Plano de Cenários de Teste Manual (`tests/MANUAL_TESTS.md`)
- **Cenário 9: Validação Estrita de Grounding sem Alucinação Paramétrica e Busca Canônica**
  - Configuração: Grounding STRICT, Web Search OFF.
  - Pergunta sobre tema ausente ou específico.
  - Verificação: Agente dispara `search_db` autonomamente na primeira rodada. Não alucina dados pré-treinados gerais. Se ausente, emite declaração de ausência factual e não pergunta *"Deseja que eu procure em documentos?"*.

---

## 7. 🔒 Higiene de Segredos, Migração de Banco e Plano de Rollback
- **Higiene de Segredos**: Nenhuma chave ou token em código.
- **Migração de Banco**: Compatível e aditiva; utiliza o esquema existente sem alterações destrutivas.
- **Plano de Rollback**: Reverter commits no git caso haja qualquer quebra de build no CI.

---

## 8. ⏱️ Passo a Passo da Execução
1. M1: Implementar `get_default_lancedb_path()` canônico em `any-context-core-rs`.
2. M2: Criar módulo `actx-cli/src/prompt.rs` com injeção de `AGENT.md` e regras de Grounding Strict/Hybrid/Proactive.
3. M3: Atualizar `actx-cli/src/engine.rs` para utilizar `build_system_prompt()` e aprimorar `search_db` com busca inteligente/BM25.
4. M4: Adicionar testes automatizados e rodar `cargo test --workspace`.
5. M5: Atualizar `TECDOC.md` (ADR-090) e `README.md`.
6. M6: Registrar Cenário 9 em `tests/MANUAL_TESTS.md` sem compactação.
7. M7: Bump para `0.32.7`, commit, PR `dev` -> `main`, monitoramento do CI, criação da tag `v0.32.7` e validação do release.
