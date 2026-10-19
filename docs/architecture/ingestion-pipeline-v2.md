# Pipeline de ingestão — design alvo (v2)

Documento vivo da revisão do pipeline de ingestão. Cada passo é preenchido depois de
discutido e decidido; os passos ainda não discutidos ficam marcados como _pendente_.
Este documento é a base para refazer o backlog (`docs/BACKLOG.md`).

## Princípios

- **Multilíngue.** O AnyContext é global: nenhuma decisão de design ou arquitetura se
  prende a um idioma ou alfabeto. Onde for imperativo escolher um idioma (ex.: texto
  gerado pelo próprio pipeline), escolhe-se o inglês.
- **Ação explícita.** Nada é indexado sem o usuário ter escolhido a fonte (passo 1).
- **Air-gapped por padrão.** Nenhuma etapa da ingestão depende de serviço de terceiros;
  capacidades extras vêm de modelos ONNX locais, atualizados pelo catálogo verificado.
- **Nada some em silêncio.** O que não é indexado (formato sem parser, arquivo protegido,
  chunk descartado) é reportado com o motivo.

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

### Web: renderização e conversão para Markdown

O AnyContext nasce air-gapped: nenhuma etapa da aquisição depende de serviço de terceiros.
**O Jina Reader é removido por completo** (código, configuração e `JINA_API_KEY`).

1. **Renderização:** um **Chromium headless embutido** renderiza as páginas, para que sites
   SPA (comuns para o público-alvo: uso doméstico, advogados, contadores) cheguem com
   conteúdo.
2. **Seleção do conteúdo:** um modelo ONNX pequeno de **classificação** (não gerativo)
   marca cada bloco do DOM como conteúdo ou ruído (menu, rodapé, propaganda). Sem o
   modelo instalado, uma heurística no estilo Readability faz o mesmo papel.
3. **Conversão determinística:** os blocos selecionados viram Markdown em Rust,
   preservando headings, tabelas, listas e blocos de código sem reescrever texto.

Um modelo gerativo de HTML → Markdown (ReaderLM-v2) foi descartado: lento demais em CPU,
pode alterar números e código, e a licença precisa ser confirmada para uso comercial.

### Chromium: distribuição e segurança

- **Vai no instalador.** Baixar sob demanda (pelo catálogo verificado dos modelos) fica
  para reavaliação futura, se o instalador ficar pesado demais.
- **É atualizado a cada release do AnyContext**: um navegador desatualizado abrindo sites
  arbitrários é superfície de ataque.
- Roda com sandbox ativo, perfil descartável por execução, downloads desabilitados e
  timeout por página.

### Quando renderizar

Só quando preciso — e a classificação não pode errar. Por isso ela **mede em vez de
adivinhar**, e na dúvida renderiza (o erro aceitável é gastar tempo, nunca perder conteúdo):

1. **Calibração por site:** nas primeiras páginas da fonte (amostra pequena, ex.: 5), o
   pipeline faz as duas coisas — busca estática e renderização — e compara o texto
   extraído. Se a busca estática traz praticamente todo o conteúdo renderizado (ex.: ≥ 95%
   do texto em todas as amostras), o site é **estático**; senão, **renderizado**.
2. **Verificação por página:** mesmo num site estático, uma página cuja busca estática vier
   vazia, muito curta ou com sinais de app JavaScript (ponto de montagem vazio,
   `<noscript>` pedindo JavaScript) é renderizada.
3. **Recalibração:** a classificação fica gravada na fonte e é reconferida a cada sync com
   uma página de amostra; se divergir, o site volta para a calibração.
4. **Transparência:** o modo de cada site (estático ou renderizado) aparece no status da
   fonte, e o usuário pode forçar a renderização.

## 3. Diff incremental

### Decisões

- **Mudança é decidida pelo hash do conteúdo.** mtime + tamanho são só o filtro rápido: se
  não mudaram, o arquivo é pulado sem ler; se mudaram, o hash decide. Um arquivo tocado
  sem mudança de conteúdo só atualiza o cache de mtime.
- **Rename e move são identificados pelo hash do conteúdo** (um arquivo que sumiu e um novo
  com o mesmo hash, na mesma fonte) e só atualizam o caminho dos chunks — sem re-embedar.
  Substitui a heurística atual de nome e tamanho.
- **Diff por chunk.** O id do chunk passa a ser o hash do texto que é embedado (mais o alvo
  de embedding). Numa edição, o arquivo é re-chunkado, os chunks com hash já conhecido
  reaproveitam o vetor e só os novos são embedados; os que deixaram de existir saem.
  - Funciona bem com chunkers que cortam por estrutura (headers, AST), cujas fronteiras não
    se deslocam com uma edição local.
  - O texto embedado inclui o cabeçalho do chunk (passo 7): um contexto que muda para o
    documento inteiro (ex.: um resumo do documento) invalida todos os chunks dele. O
    passo 7 precisa levar isso em conta.
- **Fonte inacessível fica offline, não é apagada.** Um HD externo desconectado ou uma pasta
  de rede fora do ar mantém seus chunks no índice; a fonte é marcada como **offline** no
  status e nas citações, e volta ao diff normal quando reaparecer.
  - Hoje é o contrário: os arquivos de uma pasta inacessível entram como deletados e
    **são removidos do índice** (bug a corrigir).
- **Sem "cura" silenciosa de caminhos.** O path healer atual troca uma pasta sumida por um
  caminho adivinhado (`Documentos/`, `OneDrive/`, pasta irmã) e indexa esse caminho sem
  perguntar. Isso viola a regra de ação explícita (passo 1): no máximo, o AnyContext sugere
  o caminho provável e o usuário confirma.

## 4. Classificação do conteúdo

### Decisões

- **Classificação pelo conteúdo, extensão como dica.** A assinatura binária (magic bytes)
  decide; a extensão só desempata. Um arquivo renomeado, sem extensão ou com extensão
  errada vai para o parser certo.
  - Containers ZIP são inspecionados por dentro: `[Content_Types].xml` separa DOCX, XLSX e
    PPTX; o arquivo `mimetype` separa ODT, ODS, ODP e EPUB.
  - Containers OLE/CFB (Office antigo e Outlook) são separados pelos streams internos:
    `WordDocument` (.doc), `Workbook` (.xls), `PowerPoint Document` (.ppt), `__substg1.0_*`
    (.msg).
  - Texto tem a **codificação detectada** (UTF-8, UTF-16 com BOM e as codificações legadas
    de qualquer idioma, como Windows-125x, Shift_JIS, GB18030) antes de seguir; um texto que é CSV, JSON ou Markdown é
    reconhecido pelo conteúdo.
- **Categorias:**

  | categoria | formatos |
  |---|---|
  | web | páginas renderizadas/convertidas no passo 2 (chegam como Markdown) |
  | HTML local | `.html`, `.htm` — mesmo conversor da web |
  | texto e Markdown | `.txt`, `.md`, `.rst`, logs, scripts |
  | código | linguagens com gramática tree-sitter |
  | estruturado | JSON, JSONL, YAML, TOML, XML |
  | tabular | CSV, TSV, XLSX, XLS, ODS, OFX |
  | PDF | digital ou escaneado, **por página** |
  | Office | DOCX, PPTX, DOC, PPT, ODT, ODP, RTF |
  | e-mail | `.eml`, `.msg` (corpo e anexos, que voltam ao pipeline como arquivos) |
  | e-book | `.epub` |
  | imagem | PNG, JPEG, WebP, TIFF, HEIC |

- **Office antigo e ODF têm parser de verdade** (`.doc`, `.ppt`, `.xls`, `.odt`, `.odp`,
  `.ods`, `.rtf`). Hoje `.doc`/`.ppt` são abertos como ZIP e falham, e `.odt`, `.epub`,
  `.rtf`, `.eml`, `.msg` e `.html` são indexados como texto cru ou falham.
- **PDF digital ou escaneado é decidido na classificação, por página** (camada de texto e
  área coberta por imagem). PDFs mistos — digitais com anexos escaneados, comuns em
  processos e contabilidade — mandam cada página para o caminho certo (texto ou OCR), em
  vez de descobrir isso no quality gate.
- **Arquivo protegido por senha, corrompido ou de formato sem parser** não é indexado e é
  reportado ao usuário com o motivo — nunca indexado como lixo nem descartado em silêncio.

## 5. Parsing e chunking

### Decisões

- **Tamanho medido em tokens**, com um alvo único para todos os parsers (ordem de 512
  tokens) e teto rígido no limite de entrada do modelo de embedding. Hoje o tamanho é em
  caracteres (1800) e o PDF é exceção (até 8000).
- **Overlap só em texto corrido** (prosa, PDF, Office, e-mail, web), da ordem de 10–15%.
  Código, tabelas e dados estruturados são cortados pela estrutura, sem overlap.
- **Chunks hierárquicos (small-to-big).** O chunk pequeno é o que se embeda e se busca; cada
  um guarda o vínculo com o seu **pai**, que é guardado mas não embedado e pode ser
  entregue ao LLM no lugar do trecho:

  | conteúdo | pai |
  |---|---|
  | Markdown, Office, web, e-book | seção (heading) |
  | PDF | página |
  | código | função, classe ou módulo |
  | tabular | planilha + intervalo de linhas |
  | e-mail | mensagem |

- **OCR via modelo ONNX**, no lugar do Tesseract externo (hoje procurado no PATH e muitas
  vezes ausente). Multilíngue: o OCR detecta o sistema de escrita da página e usa o modelo
  de reconhecimento correspondente. Candidato: PaddleOCR exportado para ONNX (Apache-2.0,
  modelos de reconhecimento para dezenas de idiomas e alfabetos); alternativa: docTR.
  Distribuído e atualizado como os demais modelos ONNX.
- **Tabelas repetem o cabeçalho em cada chunk de linhas**, com:
  - localização no chunk (planilha e intervalo de linhas) para citação;
  - tabelas largas (ex.: mais de 15 colunas) em formato de registro `coluna: valor` por
    linha, em vez de linha de tabela Markdown.

### Em aberto

- Limite configurável (com aviso ao usuário) para planilhas com centenas de milhares de
  linhas.

## 6. Quality gate e Document AI

### Decisões

- **O quality gate só aplica filtros seguros e independentes de idioma:** vazio ou curto
  demais, sintaxe solta (`}`, `)];`), linha minificada e binário (caracteres de controle).
  Os filtros de entropia de bytes e de densidade lexical saem: descartam texto em
  alfabetos não latinos como "lixo binário" e tabelas ou logs com valores repetidos.
- **Todo chunk descartado é registrado com o motivo** e fica visível no relatório do sync
  (e num comando de inspeção), em vez de só uma contagem.
- **Document AI local via ONNX.** Depois do OCR (passo 5), um modelo de **layout** reconstrói
  tabelas e formulários escaneados; a heurística espacial 2D atual fica como fallback.
  - LLM de visão (local ou na VPC) só se o usuário configurar explicitamente; desligado
    por padrão. Nunca um serviço externo implícito.
  - Hoje nenhum componente de Document AI está ligado em produção (extrator de layout,
    detector de scan e modo de visão nunca são conectados): todo candidato cai na
    heurística 2D.
- **Imagens que não são texto** (fotos, diagramas, gráficos) recebem uma **descrição gerada
  por um modelo de visão local** (ONNX), indexada junto com o texto de OCR e os metadados.
  A descrição é gerada em inglês (o embedding multilíngue a aproxima de perguntas em
  qualquer idioma).

### Em aberto

- Escolha do modelo de visão local (candidatos com licença permissiva e export ONNX:
  Florence-2, SmolVLM, Moondream; ou o mesmo Gemma local de FEAT-002, se multimodal).

## 7. Cabeçalhos e metadados do chunk

### Decisões

- **Metadados e texto embedado são coisas separadas.**
  - **Metadados** (colunas, nunca embedados): `source_id`, `workspace`, caminho ou URL, tipo
    de conteúdo, localização (página, seção, linhas, intervalo de planilha), `parent_id`
    (passo 5), hashes e datas.
  - **Cabeçalho contextual** (embedado): só **título do documento + caminho da seção** (ex.:
    `Contrato de Locação > Cláusula 5 > Multa`). Curto, estável sob edições do corpo — não
    quebra o diff por chunk (passo 3) — e independente de idioma. No índice lexical o
    cabeçalho tem peso menor que o corpo (ou campo próprio).
- **`embedding_target` em cada chunk** (provedor, modelo, dimensão), em vez de um único
  registro por workspace: permite migrar de modelo aos poucos e saber exatamente o que
  precisa ser reindexado.
- **O envelope atual sai** (resumo das primeiras frases + 7 palavras-chave por frequência,
  prefixado em todo chunk): todos os chunks de um arquivo batiam nas mesmas palavras no
  BM25, uma edição no início do documento invalidava todos os chunks, e a lista de stop
  words era só inglês e português. O breadcrumb `// Context: …` dentro do texto também
  sai — o cabeçalho contextual o substitui.

### Contextual retrieval (LLM local)

Um LLM gera uma frase de contexto por chunk, que entra no cabeçalho embedado. É uma
**capacidade dos kits de modelos (perfis de hardware)**:

- Ligada por padrão nos perfis com folga para um LLM local (ex.: Workstation Pro e
  Sovereign Ultra), desligada nos demais; o usuário pode mudar a configuração.
- Usa o LLM local do próprio perfil (sem modelo extra) e nunca um serviço externo
  implícito; um LLM na VPC só por configuração explícita.
- **Enriquecimento progressivo:** o chunk é indexado primeiro só com título + seção (já
  buscável) e é re-embedado com o contexto em segundo plano, sem bloquear o sync.
- **A identidade do modelo de contexto entra na chave do chunk**, junto com o
  `embedding_target`: ligar, desligar ou trocar o modelo muda o texto embedado, e o
  AnyContext avisa e reindexa os chunks afetados (mesma regra da troca de modelo de
  embedding).

## 8. Embedding

### Decisões

- **O modelo de embedding é independente do modelo de chat** (FEAT-005). É definido pelo
  kit de modelos (perfil de hardware) e o usuário pode trocá-lo. Hoje ele é derivado do
  modelo de chat, e trocar o chat reindexa o workspace inteiro.
- **Padrão local, multilíngue, via ONNX** — o AnyContext nasce air-gapped. O modelo varia
  por perfil; candidatos com licença permissiva: `multilingual-e5-small`/`-base` (MIT),
  `BGE-M3` (MIT; 100+ idiomas, 8192 tokens, também vetores esparsos) e
  `Qwen3-Embedding-0.6B` (Apache-2.0). Provedores na nuvem (OpenAI, Gemini…) só por
  escolha explícita.
- **Nunca troca de modelo em silêncio.** Acaba o fallback para OpenAI e para o provedor
  mock (vetores falsos). Se o modelo escolhido falha, os arquivos falham, o usuário é
  avisado e o próximo sync tenta de novo.
- **Prefixos por modelo:** o catálogo declara os prefixos de busca e de documento de cada
  modelo (ex.: `query:` / `passage:`) e o pipeline os aplica na ingestão e na consulta
  (IMP-014).
- **Lotes limitados por tokens e pelo máximo da API**, com paralelismo controlado nos
  provedores remotos e uso da CPU/GPU disponível no caso local. Hoje é uma chamada por
  arquivo, sem limite, uma de cada vez.

## 9. Indexação lexical (BM25)

### Decisões

- **Tantivy (MIT) substitui a implementação própria.** Índice em disco, incremental e por
  segmentos (sem carregar tudo na memória nem reescrever o arquivo inteiro a cada sync),
  campos com pesos diferentes (cabeçalho com peso menor que o corpo, passo 7), tokenizers
  plugáveis e BM25 nativo.
- **O índice não guarda o texto do chunk**, só os termos e o id; o texto vem do LanceDB.
  Hoje `bm25_index.bin` guarda o texto completo em claro e anula a criptografia do LanceDB
  (IMP-012).
- **Os arquivos do índice ficam criptografados em disco**, por uma camada de armazenamento
  (`Directory`) própria do Tantivy — um índice lexical sempre contém os termos em claro,
  então a proteção é no armazenamento.
- **Um índice por workspace.** As estatísticas (IDF, tamanho médio) de um workspace não
  afetam o ranking de outro; apagar um workspace apaga o seu diretório. Uma consulta que
  abrange vários (ex.: workspace atual + `Global`) consulta cada índice e funde por posição
  (RRF), que não depende de scores comparáveis.
- **Tokenização multilíngue sem tabela por idioma:**
  - segmentação de palavras pelo padrão Unicode (UAX #29), com dicionário para idiomas sem
    espaço entre palavras (chinês, japonês, tailandês, khmer…) — ICU4X, em Rust;
  - remoção de acentos genérica, por normalização Unicode (NFKD sem marcas combinantes),
    para qualquer alfabeto, no lugar da tabela manual só latina;
  - mantida a divisão de identificadores de código (`camelCase`, `snake_case`) junto com o
    identificador inteiro.
- **Sem stemming:** depende de idioma; a busca semântica cobre as variações de palavra.

## 10. Persistência na base vetorial

_Pendente._
