---
plan_id: any_context_blueprint_v0327_paridade_estrita_de_grounding_no_rust_core_injeção_de_agentmd_lancedb_canônico_único_sem_legado_streaming_real_e_persistência_de_modos
project: any-context
title: 'Blueprint v0.32.7: Paridade Estrita de Grounding no Rust Core (Injeção de AGENT.md, LanceDB Canônico Único sem Legado, Streaming Real e Persistência de Modos)'
version: v0.32.7
status: superseded
author: Levi Silveira
created_at: '2026-10-02T15:53:47'
updated_at: '2026-10-06T09:18:38'
session_id: 0a07fe95-3c95-480f-904a-97aa13635137
tags:
- grounding
- strict-mode
- react-agent
- lancedb
- streaming
- query-flag
- sqlite-persistence
- parity
supersedes: any_context_blueprint_v0327_paridade_estrita_de_grounding_no_rust_core_agentmd_diretivas_antialucinação_busca_autônoma_e_resolução_canônica_do_lancedbbm25
superseded_by: any_context_blueprint_v03212_test_isolation_sandbox_universal_workspace_deletion_native_self_knowledge_user_guide
parent_plan_id: null
milestones:
- name: 'M1: LanceDB Caminho Unico Canonico'
  done: false
- name: 'M2: Persistencia de Grounding por Workspace'
  done: false
- name: 'M3: System Prompt e Grounding com AGENT.md'
  done: false
- name: 'M4: Streaming Real e Flag -q'
  done: false
- name: 'M5: Ferramenta search_db com BM25 e Chunks Reais'
  done: false
- name: 'M6: Testes Automatizados e Dual-Doc'
  done: false
- name: 'M7: Release v0.32.7 e CI Watch'
  done: false
status_log:
- at: '2026-10-06T09:18:38'
  note: Superseded by any_context_blueprint_v03212_test_isolation_sandbox_universal_workspace_deletion_native_self_knowledge_user_guide
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.32.7: Paridade Estrita de Grounding no Rust Core (Injeção de AGENT.md, LanceDB Canônico Único sem Legado, Streaming Real e Persistência de Modos)

## 1. 🎯 Objetivo & Decisões Arquiteturais (ADR-090)
### Contexto e Resolução de Todos os Problemas Reportados:
1. **LanceDB com Caminho Único Canônico (Zero Legado)**:
   - Esquece completamente qualquer caminho legado ou busca em diretórios antigos. O sistema terá um **único caminho canônico oficial**:
     - Windows: `%LOCALAPPDATA%\AnyContext\data\context_db\lancedb`
     - Linux: `~/.local/share/any-context/data/context_db/lancedb`
     - macOS: `~/Library/Application Support/AnyContext/data/context_db/lancedb`
     - (Com override opcional por variável de ambiente `ACTX_CONTEXT_DB`).
   - Sem heurísticas legadas, sem fallback para diretórios obsoletos. Uma única fonte da verdade onde os 34.124 chunks já residem.

2. **Paridade Estrita de Grounding & Injeção de AGENT.md**:
   - Incorporar `config/AGENT.md` nativamente compilado no binário Rust.
   - Em modo `STRICT`:
     - Mandato imperativo de busca obrigatória via `search_db` antes de responder qualquer questão factual.
     - Proibição categórica de uso de pesos paramétricos pré-treinados.
     - Proibição absoluta de perguntar passivamente *"Deseja que eu procure nos documentos?"*. O agente DEVE executar `search_db` autonomamente.
     - Se nenhum documento for encontrado, declarar categoricamente: `⚠️ Essa informação não consta nos documentos deste workspace.`
   - Em modo `HYBRID`: Priorizar workspace com separação visual de seções (`### 📂 Informações do Workspace` vs `### 💡 Conhecimento Geral do Modelo` devidamente rotulado).
   - Em modo `PROACTIVE`: Integração ampla com rotulagem rigorosa de fontes.
   - Rodapé de fontes obrigatório (`📄 Fontes Consultadas:`).

3. **Persistência e Isolamento do Grounding Mode no SQLite**:
   - Corrigir a gravação de `grounding_mode`: salvar diretamente na coluna `grounding_mode` da tabela `workspaces` no SQLite (`UPDATE workspaces SET grounding_mode = ? WHERE name = ?`), além de atualizar `settings`.
   - Implementar `get_workspace_grounding_mode(workspace)` e `set_workspace_grounding_mode(workspace, mode)` em `NativeConfigDb`.
   - Garantir que tanto no boot quanto na troca de workspace (`/switch`), o modo configurado é lido fielmente do banco.

4. **Streaming de Tokens em Tempo Real (Eliminação do "Texto Cuspido")**:
   - O `actx-agent` estava usando `chat_complete()` e cuspindo todo o bloco de uma vez em um único evento `Delta`.
   - Implementar streaming token-a-token utilizando `chat_stream()` de `actx-lm`, emitindo `AgentEvent::Delta(token)` em tempo real à medida que chegam da API/provedor.

5. **Janela ReAct & Reasoning `<think>` Ativa**:
   - Garantir que eventos de ferramentas (`ToolStart`, `ToolEnd`) e tokens de reasoning apareçam em tempo real no buffer do accordion.
   - Ao renderizar o accordion, inspecionar a última mensagem que contém `thinking` no histórico (`chat_history.iter().rev().find_map(...)`), em vez de apenas `chat_history.last()`.

6. **Padronização Global de Flag: `-q, --query` (com `-p, --prompt` como alias)**:
   - Padronizar `-q, --query` como a flag principal de pergunta one-shot global, mantendo `-p, --prompt` como alias compatível.
   - Destacar o suporte a queries posicionais diretas: `actx "sua pergunta aqui"`.

---

## 2. 🏛️ Mapeamento de Paridade de Superfícies
- **`any-context-core-rs`**:
  - `storage/lancedb.rs`: Função canônica única `get_default_lancedb_path() -> PathBuf` sem legado.
  - `storage/sqlite.rs`: Métodos `get_workspace_grounding_mode` e `set_workspace_grounding_mode`.
  - `commands/engine.rs`: Conectar comandos `/mode` e `/workspace` à persistência real na tabela `workspaces`.
- **`actx-lm` / `actx-agent`**:
  - `fsm.rs`: Streaming real token-a-token na resposta final via `chat_stream()`.
- **`actx-cli`**:
  - `src/prompt.rs`: Injeção de `AGENT.md` e regras rigorosas de Grounding.
  - `src/cli/args.rs`: Flag padrão `-q, --query` (com alias `-p, --prompt`).
  - `src/tui/app.rs` & `src/tui/ui.rs`: Persistência do modo e inspeção retroativa de histórico para o `<think>`.

---

## 3. 📁 Arquivos e Linhas Impactadas
1. `crates/any-context-core-rs/src/storage/lancedb.rs`: Caminho único canônico.
2. `crates/any-context-core-rs/src/storage/sqlite.rs`: Métodos de persistência de grounding por workspace.
3. `crates/any-context-core-rs/src/commands/engine.rs`: Persistência real em `execute_grounding_mode`.
4. `crates/actx-agent/src/fsm.rs`: Implementação de streaming token-a-token via `chat_stream()`.
5. `crates/actx-cli/src/prompt.rs` (Novo): Construção do prompt do sistema com `AGENT.md`.
6. `crates/actx-cli/src/engine.rs`: Conexão do novo prompt e `search_db` com caminho canônico e BM25/termos.
7. `crates/actx-cli/src/cli/args.rs`: Flag `-q, --query` prioritária.
8. `crates/actx-cli/src/tui/ui.rs`: Accordion `<think>` resiliente.
9. `TECDOC.md`: Registro do ADR-090.
10. `tests/MANUAL_TESTS.md`: Cenário 9 acumulativo.
11. `Cargo.toml` / `pyproject.toml`: Versão `v0.32.7`.

---

## 4. 📚 Plano Dual-Doc (`UserDoc` + `TecDoc`)
- **`UserDoc` (`README.md`)**:
  - Documentação da flag `-q, --query` (ou execução direta `actx "query"`).
  - Explicação do comportamento estrito de Grounding (busca obrigatória, sem alucinações).
- **`TecDoc` (`TECDOC.md`)**:
  - ADR-090: Arquitetura de Grounding Parity no Rust, Streaming Reativo e Caminho Canônico Único do LanceDB.

---

## 5. 🧪 Estratégia de Testes Automatizados & Proteção contra Regressão
- Testes unitários para persistência de `grounding_mode` no SQLite por workspace.
- Testes unitários para geração do prompt em `strict`, `hybrid` e `proactive`.
- Teste de resolução do caminho canônico único do LanceDB.
- Teste de parsing de flags CLI (`-q` e `-p` e posicional).
- Garantia de 100% de testes verdes (`cargo test --workspace`).

---

## 6. 📝 Plano de Cenários de Teste Manual (`tests/MANUAL_TESTS.md`)
- **Cenário 9: Validação Estrita de Grounding, Persistência de Modo, Streaming e Resolução Canônica**
  - Passo 1: Executar `actx -q "Quais programas de imigração para AI Engineer no Canadá?"`
  - Passo 2: Validar disparo autônomo do `search_db` e leitura dos chunks reais do LanceDB.
  - Passo 3: Abrir TUI, alterar `/mode proactive`, reiniciar e validar que o modo persiste.
  - Passo 4: Observar streaming token-a-token e atividade em `<think>`.

---

## 7. 🔒 Higiene de Segredos, Migração e Rollback
- Nenhuma credencial em código. Esquema SQLite compatível sem quebras. Rollback atômico em caso de falha no CI.

---

## 8. ⏱️ Passo a Passo da Execução
1. `M1`: Implementar caminho canônico único no `any-context-core-rs` (zero legado).
2. `M2`: Implementar persistência de `grounding_mode` por workspace no SQLite.
3. `M3`: Implementar módulo de prompt com `AGENT.md` e regras estritas de Grounding.
4. `M4`: Implementar streaming real token-a-token em `actx-agent` e padronizar `-q, --query`.
5. `M5`: Aprimorar `search_db` com caminho canônico e busca tokenizada/BM25.
6. `M6`: Testes automatizados (`cargo test --workspace`), Dual-Doc e Cenário 9 em `MANUAL_TESTS.md`.
7. `M7`: Version bump `v0.32.7`, commit, PR, CI Watch no GitHub Actions, Tag e publicação no repositório de releases.
