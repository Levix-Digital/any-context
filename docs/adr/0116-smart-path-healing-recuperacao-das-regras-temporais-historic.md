---
id: 116
title: Smart Path Healing, Recuperação das Regras Temporais Históricas (Metadata Freshness Cascade + HTTP 304) e Detecção Ativa de Alterações de Fontes
status: accepted
date: '2026-10-10'
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0116: Smart Path Healing, Recuperação das Regras Temporais Históricas (Metadata Freshness Cascade + HTTP 304) e Detecção Ativa de Alterações de Fontes

- **Status**: Aprovado & Implementado (`v0.34.8`).
- **Contexto**:
  1. Caminhos de diretórios remotos e locais no Google Drive (`G:/`) ou pastas locais sofriam com falhas de resolução quando havia pequenas discrepâncias de montagem (por exemplo, inclusão indevida de `/Documentos/` quando a pasta está diretamente na raiz do Drive).
  2. A "Universal Temporal Recency Rule" declarada no prompt não conseguia ser executada com fidelidade pelo LLM porque os chunks entregues pela ferramenta `search_db` descartavam a data e hora nos cabeçalhos (`Modified: ...`), embora o LanceDB persistisse `last_modified`.
  3. No crawler web, páginas eram indexadas com `mtime: 0.0` (época UNIX 1970) e sem suporte a requisições leves de checagem ou extração de metadados de páginas HTML e HTTP.
  4. A TUI exibia fixamente `✔ Up to date` mesmo quando pastas monitoradas estavam inacessíveis ou quando arquivos haviam sido movidos, criados ou deletados.
- **Decisões**:
  1. **Smart Path Healing Engine (`any_context_core_rs::storage::path_healer`)**:
     - Implementação da função pura `try_heal_path(p: &Path) -> Option<PathBuf>` que testa heurísticas de resolução:
       * Remoção e adição de `/Documentos/` e `/Documents/`.
       * Remoção de `/OneDrive/`.
       * Normalização de barras `/` e `\`.
       * Busca pelo diretório folha na raiz ou árvore de pastas do volume montado.
     - Auto-cura transparente no `NativeConfigDb::get_workspace_folders`: se o caminho gravado não existir mas a pasta for curada, atualiza automaticamente a tabela `workspace_folders` no SQLite.
     - Validação e auto-resolução no comando `/folder` com notificação clara ao usuário.
  2. **Recuperação da Cascata de Metadados Temporais para Web (`extract_web_metadata`)**:
     - Restauração integral da lógica histórica em Rust nativo:
       * Tier 1: HTML Meta tags (`article:modified_time`, `dcterms.modified`, `dc.date.modified`, `article:published_time`, `dcterms.issued`), JSON-LD schema.org (`dateModified`, `datePublished`) e `<time datetime="...">`.
       * Tier 2: Padrões de rodapé (`Date modified:`, `Last modified:`, `Last updated:`, `Page details`).
       * Tier 3: Padrões de data na URL (`/YYYY/MM/DD/`).
       * Tier 4: Campo `<lastmod>` de XML Sitemaps.
       * Tier 5: Cabeçalho HTTP `Last-Modified` (RFC 2822 / 7231).
       * Tier 6: Timestamp de crawl corrente `Utc::now()` (nunca hardcoded `0.0`).
  3. **Verificação Leve de Fontes Web sem Download do Corpo (RFC 7232)**:
     - Implementação de `check_url_modified` com HTTP `HEAD` e cabeçalhos condicionais `If-None-Match` (`ETag`) e `If-Modified-Since` (`Last-Modified`).
     - Resposta `304 Not Modified` avaliada em < 50ms para confirmar `Up to date` sem tráfego de dados.
  4. **Propagação de `last_modified` Ponta-a-Ponta no RAG e Formatação para o LLM**:
     - Adicionado `pub last_modified: Option<String>` em `DocRecord` (BM25) e `HybridSearchResult`.
     - Injeção explícita de `| Modified: <timestamp>` no cabeçalho de cada chunk retornado pela tool `search_db`:
       `• [{file_name}] (Score: {score:.2}, Source: {file_path}, Type: {content_type} | Modified: {timestamp}):`
     - Micro-boost de recência no ranking de busca para desempate límpido de pontuação a favor de versões mais recentes.
  5. **Detecção Ativa de Alterações & Diagnóstico de Saúde de Fontes na TUI**:
     - `DiffResult` no `WorkspaceScanner` agora rastreia `missing_folders: Vec<String>`.
     - Verificação leve periódica na TUI (`check_source_health`):
       * Pastas inacessíveis: exibe `⚠️ [Sync: X folder(s) unreachable]` no Header e no Footer.
       * Mudanças detectadas: exibe `⚡ [Sync: Changes detected (/sync)]` no Header e Footer.
       * Fontes sincronizadas: exibe `✔ Up to date`.
     - Purga de arquivos movidos ou deletados do LanceDB e BM25 durante a execução de `/sync`.
- **Consequências**:
  - Eliminação de erros de pasta não encontrada quando a pasta existe no Google Drive.
  - O LLM agora tem visibilidade total da data e hora de cada documento e aplica a regra de recência com 100% de consistência.
  - Usuário tem feedback visual imediato de integridade e alterações das fontes.

---
