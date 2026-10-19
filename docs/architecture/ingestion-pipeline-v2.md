# Pipeline de ingestão — design alvo (v2)

Documento vivo da revisão do pipeline de ingestão. Cada passo é preenchido depois de
discutido e decidido; os passos ainda não discutidos ficam marcados como _pendente_.
Este documento é a base para refazer o backlog (`docs/BACKLOG.md`).

## Visão geral (caminho feliz)

1. Seleção da fonte
2. Aquisição (download / leitura)
3. Diff incremental
4. Classificação do conteúdo
5. Parsing e chunking
6. Quality gate e Document AI
7. Cabeçalhos e metadados do chunk
8. Embedding
9. Indexação lexical (BM25)
10. Persistência na base vetorial

---

## 1. Seleção da fonte

### Decisões

- **Toda fonte vem de ação explícita do usuário.** Não há fallback implícito: um workspace
  sem fontes não indexa nada (hoje o sync indexa o diretório atual quando o workspace não
  tem pastas — isso sai).
- **Não há unidade padrão; o usuário escolhe** entre:
  - pasta local, indexada recursivamente;
  - arquivo local avulso;
  - website inteiro, rastreado recursivamente;
  - página web avulsa.
- **Conectores (futuro)** seguem a mesma regra: um alvo e o modo avulso ou recursivo.
- **Escopo por workspace mantido.** Cada fonte pertence a um workspace; o workspace
  `Global` (documentação do próprio AnyContext) continua existindo e deve estar indexado
  logo após a instalação e após cada atualização.
- **Filtros de inclusão/exclusão saem do código** e vão para o `settings.db`, com a lista
  atual de pastas ignoradas e extensões como padrão, configurável pelo usuário.
- **Colunas sem uso saem do schema:** em `workspace_web_urls`, nenhum código lê ou grava
  `title`, `last_hash`, `polling_interval_hours`, `last_scraped_at`, `page_count`,
  `root_url` e `scope`.

### Em aberto

- Modelo único de fonte (tabela `sources`) no lugar de `workspace_folders`,
  `workspace_web_urls` e `workspace_sources`.
- Sintaxe dos filtros, `.gitignore` e filtros por fonte.
- Limites do rastreio recursivo de um website.
- Como o `Global` passa pelo pipeline.

## 2. Aquisição (download / leitura)

_Pendente._

## 3. Diff incremental

_Pendente._

## 4. Classificação do conteúdo

_Pendente._

## 5. Parsing e chunking

_Pendente._

## 6. Quality gate e Document AI

_Pendente._

## 7. Cabeçalhos e metadados do chunk

_Pendente._

## 8. Embedding

_Pendente._

## 9. Indexação lexical (BM25)

_Pendente._

## 10. Persistência na base vetorial

_Pendente._
