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
- **Cada página de PDF é classificada em três estados:** **digital**, **escaneada** ou
  **digital com texto quebrado** (camada de texto existe mas é lixo — fontes sem
  mapeamento Unicode, CID —, comum em sistemas antigos de tribunais, prefeituras e
  hospitais). O terceiro estado é tratado como escaneado. A detecção é independente de
  idioma: proporção de caracteres de substituição, de uso privado e de controle, e
  sequências improváveis; na dúvida, o texto da página é comparado com um OCR rápido da
  mesma página e fica o melhor (medir em vez de adivinhar). PDFs mistos — digitais com anexos escaneados, comuns em
  processos e contabilidade — mandam cada página para o caminho certo (texto ou OCR), em
  vez de descobrir isso no quality gate.
- **Arquivo protegido por senha, corrompido ou de formato sem parser** não é indexado e é
  reportado ao usuário com o motivo — nunca indexado como lixo nem descartado em silêncio.
  Um arquivo protegido pode ser destravado com uma senha do cofre (abaixo).

### Containers e arquivos compactados (auditoria P1)

- **Abertos recursivamente.** Cada item interno vira um **documento filho**, com
  proveniência encadeada (`processo.zip > autos/anexo 3.pdf`,
  `escritorio.pst > Caixa de entrada > Re: contrato > minuta.docx`), e passa pelo pipeline
  normal (classificação pelo conteúdo, parser…).
- **Formatos:** ZIP, 7z, TAR, GZ, BZ2, XZ, MBOX e **PST/OST** (Outlook). **RAR** só para
  leitura, se a licença do descompactador permitir (confirmar juridicamente; comprar uma
  licença fica para avaliação futura); senão é recusado com mensagem clara.
- **Proteção:** limite de profundidade (ex.: 5 níveis), limite de tamanho descompactado e
  de razão de compressão (zip bombs), rejeição de caminhos internos com `../`. O que for
  barrado aparece no relatório.
- **Diff:** o container tem hash próprio e cada filho também; quando o container muda, só
  os filhos alterados são reprocessados.
- **Citação:** mostra o caminho completo dentro do container; "abrir o original" extrai o
  item para uma pasta temporária.
- **Filtros** da fonte (passo 1) valem também dentro do container.

### Cofre de senhas de arquivos

Arquivos e containers protegidos podem ser destravados com senhas que o usuário fornece por
fonte (ou por arquivo). O AnyContext **usa a senha sem conhecê-la** fora do momento exato
de abrir o arquivo:

- **Guardada no cofre do sistema operacional** (Keychain, Windows Credential Manager,
  Secret Service), o mesmo da chave de criptografia (passo 10). O `settings.db` guarda só
  uma referência (`secret_id`) na fonte, nunca a senha.
- **Só o worker de ingestão a lê**, no instante de abrir o arquivo; ela fica só em memória,
  é apagada (zeroize) logo depois, e nunca vai para logs, relatórios, índices ou prompts.
- **O agente e o LLM nunca têm acesso ao cofre**: não existe ferramenta que leia senhas, e
  nenhuma senha entra no contexto de uma conversa.
- **Opção "perguntar sempre"**: a senha não é guardada e é pedida a cada sync que precise
  dela.
- **Sem tentativa e erro:** uma senha errada falha o arquivo e é reportada; o AnyContext
  nunca tenta adivinhar senhas.
- O futuro módulo de usuários pode exigir autenticação para liberar o cofre.

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

### Office e PDF completos (auditoria P6 e P7)

O parser extrai o documento inteiro, não só o texto do corpo. Hoje o DOCX corta seções por
tamanho ("Section 1, 2, 3") e ignora títulos, comentários, notas e controle de alterações;
o PPTX ignora as notas; o XLSX ignora fórmulas e mesclas; o PDF ignora sumário, anotações,
formulários, anexos e assinaturas.

- **Estrutura real:** estilos de título do DOCX, títulos de slide do PPTX e o sumário
  (bookmarks) do PDF viram o caminho de seção (passo 7). Tabelas viram tabelas Markdown;
  listas são preservadas.
- **Controle de alterações:** o texto principal é a **versão atual** (com as inclusões, sem
  as exclusões); cada alteração é indexada como **item próprio** com autor, data e trecho
  incluído ou excluído ("quem alterou a cláusula 5 e quando?").
- **Comentários e anotações** (DOCX, XLSX, PPTX, PDF): itens próprios ligados ao trecho
  comentado, com autor e data.
- **Cabeçalhos e rodapés:** indexados **uma vez por documento**, não repetidos por página.
  Notas de rodapé e de fim ficam ligadas à seção onde aparecem.
- **PPTX:** notas do apresentador junto do slide.
- **XLSX:** o texto indexado é o valor calculado; a fórmula fica como metadado. Células
  mescladas expandidas, intervalos nomeados como rótulos, abas ocultas indexadas e
  marcadas como ocultas.
- **PDF:**
  - campos de formulário (AcroForm) como pares chave–valor;
  - arquivos anexados ao PDF passam pelos containers (passo 4);
  - **assinaturas digitais:** signatário, data e se o documento foi alterado depois de
    assinado. A cadeia de confiança é validada com os certificados raiz do sistema
    operacional, sem regra presa a um país; sem como validar offline, o status é "não
    verificado".
- **Outros formatos:** calendário (`.ics`), contatos (`.vcf`) e MHTML no nível 1 (simples e
  frequentes); Visio (`.vsdx`), XPS e Apple iWork (Pages, Numbers, Keynote — formato
  proprietário e mais difícil) no nível 2.

### Trabalhos longos (planilhas grandes, sites e pastas enormes)

Não há limite de tamanho; quem decide se espera é o usuário.

- **Estimativa antes de começar:** chunks previstos e tempo estimado **nesta máquina**,
  pela velocidade medida nos syncs anteriores com o mesmo `embedding_target` (na primeira
  vez, por uma medição rápida).
- **Confirmação acima de um limite** (ex.: 10 minutos, configurável): indexar tudo,
  indexar só parte (ex.: algumas abas) ou pular.
- **Segundo plano e progressivo:** o que já foi indexado fica buscável na hora; uma
  interrupção não perde trabalho — o diff por chunk (passo 3) retoma de onde parou.
- O AnyContext **não divide nem altera arquivos do usuário**, nem sugere dividir: o
  pipeline já fatia por linhas, então dividir não reduz o trabalho.
- Perguntas agregadas sobre tabelas grandes ("soma das vendas de março") não são bem
  servidas por busca semântica: guardar a tabela num formato consultável e responder por
  consulta é tema do pipeline de recuperação.

## 6. Quality gate e Document AI

### Decisões

- **O quality gate só aplica filtros seguros e independentes de idioma:** vazio ou curto
  demais, sintaxe solta (`}`, `)];`), linha minificada e binário (caracteres de controle).
  Os filtros de entropia de bytes e de densidade lexical saem: descartam texto em
  alfabetos não latinos como "lixo binário" e tabelas ou logs com valores repetidos.
- **Todo chunk descartado é registrado com o motivo** e fica visível no relatório do sync
  (e num comando de inspeção), em vez de só uma contagem.
- **Extração de páginas (auditoria P8).**
  - **Página digital:** a camada de texto é sempre a verdade (é exata). Nenhum modelo
    reescreve esse texto; um modelo pode, no máximo, ajudar a descobrir a estrutura (ordem
    de leitura, tabelas) a partir das coordenadas do texto original.
  - **Página escaneada, com texto quebrado, foto ou manuscrito:** um **modelo de parsing de
    páginas** (VLM pequeno via ONNX) lê a imagem e devolve Markdown com ordem de leitura,
    tabelas, fórmulas e manuscrito, nos perfis que comportam; nos perfis leves fica a
    cascata OCR → layout → heurística 2D:

    | perfil | página escaneada |
    |---|---|
    | Lite | OCR + heurística 2D |
    | Smart Balanced | OCR + modelo de layout |
    | Air-Gapped Power em diante | modelo de parsing de páginas (candidato: Granite-Docling-258M, Apache-2.0) |
    | Workstation Pro, Sovereign Ultra | modelo maior se vencer o benchmark (candidato: PaddleOCR-VL, licença a confirmar) |

  - **Benchmark antes de fixar os modelos:** Granite-Docling, PaddleOCR-VL e a cascata
    OCR + layout medidos num conjunto real de páginas escaneadas, tabelas, formulários e
    manuscritos (qualidade e tempo por página em CPU, por perfil).
  - Documentos escaneados longos usam a estimativa, a confirmação e o processamento
    progressivo dos trabalhos longos (passo 5).
- **Formulários como dados (LSIE).** Formulários (CMR, nota fiscal, holerite, folha de
  ponto, extrato, laudo, formulário judicial…) não podem ter valores ligados ao rótulo
  errado. Teste com um CMR real: o extrator atual gerou um único chunk de tabelas
  quebradas e atribuiu o horário de partida real à chegada real.
  1. **Layout conhecido:** o layout é reconhecido pela geometria (linhas e caixas) e pelos
     rótulos fixos; com o mapa guardado, o **LSIE** (Levi's Scale-Invariant Extraction, do
     DocMapper) extrai de forma determinística — milissegundos, invariante a escala e
     margens, corrige campos que crescem e empurram os demais, sem vazamento entre campos.
  2. **Layout novo — fase 0 automática por perfil**, que gera o mapa (campo, caixa,
     rótulo âncora):
     - perfis leves: heurística de caixas (linhas vetoriais) e tipografia (rótulo pequeno,
       valor maior/negrito), independente de idioma;
     - perfis médios: modelo pequeno que identifica rótulos e valores e entrega o mapa ao
       LSIE;
     - perfis altos: o modelo de parsing de páginas (P8) extrai os pares direto e também
       gera o mapa.
     Licença do modelo de campos entra no benchmark (ex.: LayoutLMv3 é não comercial; os
     YOLO da Ultralytics são AGPL).
  3. **O mapa vira modelo do layout**, revisável e corrigível pelo usuário (tela no estilo
     do Mapping Studio do DocMapper); os próximos documentos do layout usam o LSIE.
  4. **Anomaly gate com modelo local** (no DocMapper é o Gemini): nuvem só por configuração
     explícita.
  5. **Valores ancorados:** em página digital, todo valor precisa existir na camada de
     texto; nenhum modelo inventa um número.
  6. **Saída:** cada campo vira par número do campo + rótulo como no documento + valor; o
     formulário é guardado como registro, e **formulários do mesmo modelo viram uma tabela
     consultável** (uma linha por documento, uma coluna por campo).
  7. **Corpus de testes por setor** com os pares esperados, rodando a cada mudança; começa
     pelo CMR fictício em `crates/any-context-core-rs/tests/fixtures/forms/`. A
     autoavaliação (H1) avisa em produção sobre campos vazios ou de baixa confiança.
  - **Implementação única:** o LSIE é portado para uma crate Rust compartilhada, compilada
    também para WASM e usada pelo DocMapper — um só motor para os dois produtos.
  - **Integração com o DocMapper:** o formato do mapa de layout é comum aos dois produtos
    (um layout mapeado num serve no outro), e o AnyContext pode encaminhar ao DocMapper o
    que é dele (mapeamento visual avançado, carimbo de PDF, exportação de campos). O
    AnyContext funciona sem o DocMapper instalado, e a integração é local, sem nuvem.
- **Document AI local via ONNX.** Nos perfis com cascata, depois do OCR (passo 5), um modelo
  de **layout** reconstrói tabelas e formulários escaneados; a heurística espacial 2D atual
  fica como fallback.
  - LLM de visão (local ou na VPC) só se o usuário configurar explicitamente; desligado
    por padrão. Nunca um serviço externo implícito.
  - Hoje nenhum componente de Document AI está ligado em produção (extrator de layout,
    detector de scan e modo de visão nunca são conectados): todo candidato cai na
    heurística 2D.
- **Imagens que não são texto** (fotos, diagramas, gráficos) recebem uma **descrição gerada
  por um modelo de visão local** (ONNX), indexada junto com o texto de OCR e os metadados.
  A descrição é gerada em inglês (o embedding multilíngue a aproxima de perguntas em
  qualquer idioma).

### Modelo de visão por perfil

O modelo de visão faz parte do kit de modelos. Ponto de partida, **a validar por
benchmark** (conjunto de fotos, diagramas, gráficos e documentos reais; qualidade da
descrição e tempo por imagem em CPU em cada perfil) antes de ser fixado:

| perfil | modelo de visão |
|---|---|
| Lite (<8GB) | Florence-2-base (0,23B, MIT) |
| Smart Balanced (8GB) | SmolVLM-500M (Apache-2.0) |
| Air-Gapped Power (16GB) | SmolVLM-2.2B (Apache-2.0) |
| Workstation Pro (32GB), Sovereign Ultra (64GB+) | o Gemma local do perfil (FEAT-002, multimodal), sem modelo extra; SmolVLM-2.2B se não houver export ONNX |

Processamento progressivo: a imagem fica buscável primeiro pelo OCR e ganha a descrição
em segundo plano.

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

### Decisões

- **Isolamento físico por workspace.** Cada workspace tem as suas tabelas LanceDB (como o
  seu índice lexical, passo 9), no lugar da tabela única `workspace_chunks` filtrada pela
  coluna `workspace` — onde um filtro esquecido vaza dados entre workspaces (BUG-023).
  Apagar um workspace é apagar o seu diretório.
- **Uma tabela por workspace e por `embedding_target`**, com a dimensão real do modelo:
  acaba o preenchimento com zeros até 1536 (IMP-004). Na troca de modelo, a tabela nova é
  preenchida enquanto a antiga continua respondendo, e a antiga é descartada no fim.
- **Manutenção automática:** compactação depois de syncs com muitos deletes ou
  reindexações (IMP-013). As migrações saem do caminho do sync: o schema é versionado e
  migrado uma vez, na abertura da base.
- **Chave de criptografia:** um segredo gerado na instalação e guardado no cofre do sistema
  operacional (Keychain, Windows Credential Manager, Secret Service), com exportação e
  recuperação para backup e troca de máquina. Substitui a chave derivada do id da máquina
  com salt fixo no código, que qualquer um com o id reproduz e que torna a base ilegível em
  outra máquina. A integração com contas de usuário fica para o futuro módulo de usuários
  (fora do escopo da ingestão).

- **Índice vetorial aproximado (ANN) a partir de 100 mil chunks** por workspace e
  `embedding_target`, configurável. Abaixo disso a busca é exaustiva (exata e rápida —
  milhares de arquivos ficam na casa de dezenas de milhares de chunks). Acima, o índice é
  criado automaticamente em segundo plano e reconstruído após muitas mudanças; chunks
  recém-inseridos ainda fora do índice continuam encontrados (busca exaustiva nos novos,
  fundida com o ANN). A pequena perda de recall do ANN é compensada pela busca lexical em
  paralelo e pelo reranker.

- **Vetores protegidos na camada de armazenamento.** A criptografia por campo não serve
  para eles (a busca calcula distâncias sobre os vetores), então os arquivos do LanceDB
  ficam criptografados em disco, por uma camada de armazenamento própria, com a mesma
  chave do cofre do sistema operacional. Como o LanceDB não oferece isso pronto, essa
  camada é construída pelo AnyContext (como o `Directory` criptografado do Tantivy,
  passo 9).
