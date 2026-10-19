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

### Modelo único de fonte

Uma tabela `sources` no `settings.db` substitui `workspace_folders`, `workspace_web_urls`
e `workspace_sources` (as colunas sem uso desaparecem com elas):

| coluna | conteúdo |
|---|---|
| `id`, `workspace` | identidade e escopo |
| `kind` | `local`, `web`, `builtin` (futuro: `connector:<nome>`) |
| `target` | caminho ou URL |
| `recursive` | `true` para pasta ou site, `false` para arquivo ou página |
| `display_name`, `created_at` | exibição e auditoria |
| `filters` | ajustes de filtro desta fonte (opcional) |

### Filtros

- Padrões globais numa tabela do `settings.db`, preenchida na criação com a lista atual
  de pastas ignoradas e extensões; cada fonte pode sobrescrever.
- Sintaxe de `.gitignore`, avaliada com o crate `ignore` (o do ripgrep). O `.gitignore` das
  pastas indexadas é respeitado por padrão, com opção para desligar.
- Só entram extensões que algum parser suporta; o usuário pode desligar uma extensão ou
  mapear uma extensão desconhecida para o parser de texto.
- Um arquivo selecionado avulso ignora os filtros de exclusão: o usuário o escolheu.

### Website recursivo

- Fica no mesmo host e dentro do prefixo de caminho da URL dada (`docs.x.com/v2/` não
  sobe para `docs.x.com/`) e respeita o `robots.txt`.
- Teto padrão generoso: **3000 páginas**, configurável globalmente e por fonte.
- Quando o site tem mais páginas que o teto, o usuário é avisado (quantas foram
  indexadas, que há mais a indexar) e recebe o comando ou a configuração que aumenta o
  teto daquela fonte.
- Página avulsa busca só aquela URL, sem seguir links.

### Workspace `Global`

É uma fonte `builtin` (documentação embutida no binário) processada pelo mesmo pipeline
das demais, no primeiro start após instalar ou atualizar — não há mais chunker nem escrita
própria. Sem modelo de embedding disponível nesse momento, o índice lexical é gerado na
hora e os vetores quando houver modelo (ver passo 8).

## 2. Aquisição (download / leitura)

### Decisões

- **Arquivos locais são lidos na origem**, sem cópia, e **uma única vez** por arquivo: o
  mesmo buffer alimenta o hash, o parser e o enriquecimento (hoje são até três leituras).
- **Documentos linkados num site** (PDF, DOCX etc.), dentro do escopo da fonte, são
  baixados e passam pelo pipeline como qualquer arquivo.
- **Re-sync web incremental:** ETag, `Last-Modified` e o `lastmod` do sitemap evitam baixar
  o que não mudou (requisições condicionais).
- **Páginas que somem do site saem do índice.**

### Em aberto

- Conversão HTML → Markdown: Jina Reader (decisão inicial) ou módulo local via ONNX, para o
  AnyContext nascer air-gapped.

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
