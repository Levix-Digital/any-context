---
plan_id: any_context_blueprint_v0348_path_healing_and_temporal_recency_v2
project: any-context
title: 'Blueprint v0.34.8: Smart Path Healing, Regras Temporais Historicas & Deteccao Leve de Alteracoes'
version: v0.34.8
status: superseded
author: Levi Silveira
created_at: '2026-10-10T12:18:43'
updated_at: '2026-10-10T12:42:22'
session_id: a94494e0-08bb-4378-888d-a3bde8673537
tags:
- path-healing
- temporal-recency
- http-304
- metadata-cascade
- rag
- change-detection
supersedes: any_context_blueprint_v0348_path_healing_and_temporal_recency
superseded_by: anycontext_blueprint_v0349_harmonizacao_aurora_boreal_na_tui_auto_scroll_react_e_mensagens_profissionais_de_atualizacao
parent_plan_id: null
milestones:
- name: 'M1: Smart Path Healing'
  done: true
- name: 'M2: Cascata Temporal e HTTP 304 Web'
  done: true
- name: 'M3: Propagacao last_modified e Prompt'
  done: true
- name: 'M4: Deteccao de Mudancas e Saude de Fontes'
  done: true
- name: 'M5: Testes e Dual-Doc'
  done: true
status_log:
- at: '2026-10-10T12:37:58'
  note: 'Milestone ''M1: Smart Path Healing'': Cura fonetica e estrutural implementada e testada'
- at: '2026-10-10T12:38:00'
  note: 'Milestone ''M2: Cascata Temporal e HTTP 304 Web'': Cascata de 5 tiers e RFC 7232 HTTP 304 recuperados do historico Python'
- at: '2026-10-10T12:38:01'
  note: 'Milestone ''M3: Propagacao last_modified e Prompt'': last_modified propagado no BM25, Pipeline RAG e Prompt LLM'
- at: '2026-10-10T12:38:03'
  note: 'Milestone ''M4: Deteccao de Mudancas e Saude de Fontes'': missing_folders, alert status e mudancas ativas no TUI'
- at: '2026-10-10T12:38:04'
  note: 'Milestone ''M5: Testes e Dual-Doc'': 235 testes verdes, ADR-116 em TECDOC e Cenario 31 em MANUAL_TESTS'
- at: '2026-10-10T12:38:05'
  note: Release v0.34.8 finalizada e publicada com sucesso
- at: '2026-10-10T12:42:22'
  note: Superseded by anycontext_blueprint_v0349_harmonizacao_aurora_boreal_na_tui_auto_scroll_react_e_mensagens_profissionais_de_atualizacao
- at: '2026-10-16T10:47:35'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# 📐 Blueprint de Engenharia v0.34.8: Smart Path Healing, Recuperação das Regras Temporais Históricas & Detecção Leve de Alterações

## 📌 Contexto & Recuperação do Histórico de Desenvolvimento (Legado Python ➔ Rust)

Após investigação aprofundada no histórico do git (`git log -S "Last-Modified"`, commits `2c85f68` e `9d77227`), recuperamos com exatidão como as regras de data, hora e verificação de alterações funcionavam no AnyContext:

### 1. Como a Data e Hora Eram Coletadas em Pastas Locais:
- Coletadas diretamente dos metadados do sistema de arquivos (`fs::Metadata`):
  - `mtime` (`last_modified_date`): data e hora exata da última modificação (`fs::metadata(path)?.modified()`).
  - `ctime` (`creation_date`): data de criação do arquivo (`fs::metadata(path)?.created()`).
- No Rust atual: o `mtime` era convertido para RFC 3339, mas era **descartado** no meio do pipeline de busca (`HybridSearchResult`).

### 2. Como a Data e Hora Eram Coletadas em Sites Web (Sem Precisar Baixar para Ver):
- **Verificação Leve de Mudanças (Lightweight Header Check - RFC 7232)**:
  - Requisição HTTP `HEAD` ou `GET` condicional enviando os cabeçalhos:
    - `If-Modified-Since`: data do último crawl (`cached_last_modified`)
    - `If-None-Match`: `ETag` em cache
  - Se o servidor responder **`304 Not Modified`**: o AnyContext confirma em **< 50ms**, sem baixar o corpo HTML, que a página continua 100% idêntica!
  - Se o servidor responder `200 OK`, ou em requisição `HEAD`: o cabeçalho HTTP `Last-Modified` (ou `Date`) traz o carimbo oficial do servidor.
- **Cascata de Metadados de Frescor (Metadata Freshness Cascade)** quando a página era processada:
  1. **Tier 1 (Alta Confiança - Meta Tags HTML & Schema.org)**:
     - `<meta property="article:modified_time" content="...">`
     - `<meta name="dcterms.modified" content="...">`
     - `<meta name="dc.date.modified" content="...">`
     - `<meta property="article:published_time" content="...">`
     - `<meta name="dcterms.issued" content="...">`
     - JSON-LD Schema.org: `"dateModified": "..."` e `"datePublished": "..."`
     - `<time datetime="...">`
  2. **Tier 2 (Padrões de Texto no Rodapé)**:
     - `Date modified: YYYY-MM-DD`, `Last modified: YYYY-MM-DD`, `Last updated: YYYY-MM-DD`, `Page details YYYY-MM-DD`.
  3. **Tier 3 (Padrão de Data na URL)**:
     - `/YYYY/MM/DD/` ou `/YYYY/MM/`.
  4. **Tier 4 (XML Sitemap `lastmod`)**:
     - Campo `<lastmod>` do `sitemap.xml`.
  5. **Tier 5 (Cabeçalho HTTP)**:
     - `Last-Modified` do cabeçalho HTTP (RFC 2822 / 7231).
  6. **Tier 6 (Fallback)**:
     - Crawl timestamp atual (`Utc::now()`). *(No Rust estava fixo em `0.0` / 1970!)*

### 3. Como os Chunks Eram Formatados para o LLM Aplicar a Regra de Recência:
- No `search_tools.py` original:
  ```text
  --- [Document Chunk 1 | Source: file.docx | Workspace: Default | Last Modified: 2026-10-10 12:00:00 | Type: Local Document] ---
  Path: ...
  Content: ...
  ```
- No Rust atual (`search_db`), a string era gerada como:
  `• [{}] (Score: {:.2}, Source: {}, Type: {}):\n{}` — **sem nenhuma data ou hora**!
  Por isso o LLM não conseguia aplicar a regra de informação mais recente.

---

## 🏛️ Arquitetura Completa da Solução para a Release `v0.34.8`

```mermaid
flowchart TD
    subgraph PathHealing["1. Smart Path Healing"]
        A["Folder Configurado (SQLite)"] --> B{"Path::exists()?"}
        B -- Sim --> C["Caminho Válido"]
        B -- Não --> D["try_heal_path()"]
        D --> E["Remove/Adiciona 'Documentos/'"]
        E --> F["Normaliza Slashes & Drive Root"]
        F --> G{"Caminho Curado Existe?"}
        G -- Sim --> H["Auto-Update SQLite & Notifica TUI"]
        G -- Não --> I["Marca Fonte como Offline/Inacessível"]
    end

    subgraph TemporalExtraction["2. Extração & Cascata Temporal Nativas"]
        J["Arquivo Local"] --> K["fs::Metadata: mtime + ctime -> RFC 3339"]
        L["Site Web"] --> M["HTTP HEAD / Conditional GET 304"]
        M --> N{"HTTP 304 Not Modified?"}
        N -- Sim --> O["✔ Up to date sem baixar corpo!"]
        N -- Não --> P["Cascata: Meta Tags -> JSON-LD -> Rodapé -> URL -> Sitemap -> Header -> Utc::now()"]
    end

    subgraph RAGProvenance["3. RAG Provenance & Recency Rule"]
        K --> Q["LanceDB VectorRecord (last_modified)"]
        P --> Q
        Q --> R["HybridSearchResult (last_modified)"]
        R --> S["search_db Tool: Cabeçalho com | Modified: YYYY-MM-DDTHH:MM:SSZ"]
        S --> T["LLM aplica Universal Temporal Recency Rule com precisão absoluta"]
    end

    subgraph ChangeDetection["4. Detecção Ativa de Mudanças"]
        U["Heartbeat TUI / Workspace Switch / /sync"] --> V{"Pastas e URLs válidas?"}
        V -- Não --> W["Header/Footer: ⚠️ Folder/URL unreachable"]
        V -- Sim --> X["Diff Rápido: mtime local vs file_stat_cache & HTTP HEAD 304"]
        X -- Mudanças --> Y["Header/Footer: ⚡ Changes detected (run /sync)"]
        X -- Idêntico --> Z["Header/Footer: ✔ Up to date"]
        AA["Execução de /sync"] --> AB["Purga arquivos movidos/deletados do LanceDB e BM25"]
    end
```

---

## 📋 Marcos de Implementação (v0.34.8)

### Marco 1: Smart Path Healing (`crates/any-context-core-rs/src/storage/path_healer.rs`)
1. Implementar `try_heal_path(p: &Path) -> Option<PathBuf>`:
   - Testa variações: remoção/adição de `Documentos/`, `Documents/`, `OneDrive/`, normalização de barras `/` e `\`.
   - Testa existência da subpasta correspondente no volume.
2. Integrar em `NativeConfigDb::get_workspace_folders`:
   - Auto-cura e atualiza o registro na tabela `workspace_folders` se o caminho gravado estiver corrompido mas a pasta existir.
3. Integrar nos comandos `/folder` e `/sync`:
   - Avisa ao usuário: `🔄 Auto-resolved folder path to 'G:/My Drive/Levix Digital/AnyContext'`.

### Marco 2: Recuperação da Cascata Temporal e HTTP 304 para Web (`crates/any-context-core-rs/src/ingestion/crawler.rs`)
1. **Suporte a HTTP Conditional GET / HEAD (RFC 7232)**:
   - Na checagem de fontes web, passar `If-Modified-Since` e `If-None-Match`.
   - Se o servidor responder `304 Not Modified`, marca a página como inalterada instantaneamente sem baixar o HTML.
2. **Implementação da Cascata de Metadados de Frescor (Rust Nativo)**:
   - Extrai tags `<meta property="article:modified_time">`, `<meta name="dcterms.modified">`, `<meta name="dc.date.modified">`.
   - Extrai schema.org JSON-LD (`"dateModified": "..."`, `"datePublished": "..."`).
   - Extrai regex de rodapé (`Date modified:`, `Last modified:`, `Updated:`).
   - Extrai regex de URL (`/YYYY/MM/DD/`).
   - Extrai header HTTP `Last-Modified`.
   - Fallback para `Utc::now()` (nunca `0.0`!).
3. **No Orquestrador (`orchestrator.rs`)**:
   - Passar o timestamp real retornado pelo crawler para `embed_and_upsert_chunks`.

### Marco 3: Propagação de `last_modified` no Pipeline RAG e Injeção no Prompt do LLM
1. **BM25 (`crates/any-context-core-rs/src/retrieval/bm25.rs`)**:
   - Adicionar `pub last_modified: Option<String>` no `DocRecord`.
2. **Pipeline RAG (`crates/any-context-core-rs/src/retrieval/pipeline.rs`)**:
   - Adicionar `pub last_modified: Option<String>` em `HybridSearchResult`.
   - Propagar de `d.last_modified` (LanceDB) e `doc.last_modified` (BM25).
   - Aplicar recency boost suave no reranking para desempate a favor de versões mais novas.
3. **Injeção do Cabeçalho na Tool `search_db` (`crates/actx-cli/src/engine.rs`)**:
   - Formatar o cabeçalho entregue ao LLM com carimbo de tempo explícito:
     ```rust
     let mod_str = match &r.last_modified {
         Some(lm) if !lm.is_empty() => format!(" | Modified: {}", lm),
         _ => String::new(),
     };
     out.push(format!(
         "• [{}] (Score: {:.2}, Source: {}, Type: {}{}):\n{}",
         r.file_name, r.score, r.file_path, type_str, mod_str, r.text
     ));
     ```

### Marco 4: Detecção Ativa de Mudanças, Saúde de Fontes e Purga
1. **Diagnóstico de Saúde das Fontes (`WorkspaceHealth`)**:
   - Verifica se todas as pastas locais e portais web configurados estão acessíveis.
   - Se alguma fonte falhar: Header/Footer exibe `⚠️ [Sync: 1 source unreachable]`.
2. **Diff Rápido Local e Web**:
   - Pastas: compara `(mtime, size)` com `file_stat_cache`.
   - Web: checagem leve de `ETag` / `Last-Modified` via HTTP HEAD / 304.
   - Se houver alterações detectadas: exibe `⚡ [Sync: Changes detected - run /sync]`.
3. **Garantia de Purga em `/sync`**:
   - Purga arquivos movidos/deletados do LanceDB (`delete_by_file`) e BM25 (`remove_by_file`).
   - Log detalhado: `✔ Purged X moved/deleted file(s). Indexed Y new file(s). Workspace 100% synchronized.`

### Marco 5: Testes Unitários, E2E e Dual-Doc
1. Teste de `try_heal_path` com caminhos reais do Google Drive e separadores de barra.
2. Teste da cascata de metadados web e resposta HTTP 304.
3. Teste de propagação de `last_modified` no `search_db`.
4. Atualização do `TECDOC.md` (ADR-112) e `MANUAL_TESTS.md` (Cenário 27).
