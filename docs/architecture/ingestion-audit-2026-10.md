# Auditoria da ingestão — "Any Context" (outubro de 2026)

Auditoria do pipeline de ingestão (design alvo em `ingestion-pipeline-v2.md`) contra as
técnicas mais recentes do mercado, com um objetivo: **qualquer tipo de arquivo deve poder
servir de contexto para qualquer profissional** — TI, hospitais, repartições públicas,
universidades, contabilidade, advocacia, engenharia.

Tudo aqui são **propostas** (P1, P2…), ainda não decisões. As aceitas entram no documento
do pipeline; as recusadas ficam registradas aqui com o motivo.

Respeitam os princípios já decididos: multilíngue, ação explícita, air-gapped por padrão
(modelos ONNX locais, por perfil de hardware) e nada some em silêncio.

---

## 1. Harness² (Google Research)

[Harness²](https://github.com/google-research/harness2) (Apache-2.0) **não é um framework
de ingestão**: é um framework de "recursive agent harnessing" — otimiza o sistema em volta
de um modelo (prompt, guardrails, memória, ferramentas, skills…) sem mexer nos pesos. Não
trata de parsing, chunking nem indexação.

Três ideias dele se transferem bem para a ingestão:

| ideia do Harness² | aplicação na ingestão |
|---|---|
| **Propose–Probe–Compose sem gabarito**: propõe variações, testa na própria tarefa, compõe a partir do contraste entre execuções | **H1 — autoavaliação da ingestão** (abaixo) |
| **Experience reflector**: guarda lições comprovadas para tarefas futuras | **H2 — memória de estratégia por fonte** |
| **Skills e componentes editáveis** por domínio | **H3 — pacotes de domínio** |

---

## 2. Cobertura de formatos por público

O design v2 cobre web, HTML, texto, código, estruturado, tabular, PDF, Office, e-mail,
e-book e imagem. Faltam formatos centrais para vários públicos:

| público | formatos que faltam |
|---|---|
| todos | arquivos compactados (ZIP, 7z, TAR), caixas de e-mail (MBOX, PST), áudio e vídeo, calendário (`.ics`), contatos (`.vcf`), MHTML, XPS |
| advocacia, setor público | PDF com assinatura digital, anotações e formulários; DOCX com comentários e controle de alterações; gravações de audiências |
| hospitais | DICOM (metadados, laudos estruturados, PDF embutido), HL7 v2, FHIR, CDA |
| universidades | LaTeX, BibTeX, Jupyter (`.ipynb`), fórmulas em PDF, gravações de aulas |
| engenharia | DXF, IFC (BIM), cabeçalho de STEP/IGES, pranchas em PDF (carimbo), GIS (GeoJSON, KML/KMZ, Shapefile, GeoPackage) |
| contabilidade | XBRL; arquivos de registros delimitados de layout fixo (qualquer país) |
| escritório em geral | Visio (`.vsdx`), Apple iWork (Pages, Numbers, Keynote) |

Fora de alcance sem software proprietário: DWG nativo, OneNote, arquivos com DRM.

---

## 3. Propostas

### Cobertura

- **P1 — Containers e arquivos compactados.** ZIP, 7z, TAR/GZ, MBOX e PST são abertos
  recursivamente; cada item interno vira um documento filho com proveniência
  (`processo.zip > anexo 3.pdf`) e passa pelo pipeline normal. Limites de profundidade e
  de tamanho descompactado (proteção contra zip bomb).
- **P2 — Áudio e vídeo.** Transcrição local multilíngue via ONNX (família Whisper, MIT),
  com timestamps como localizador do chunk; separação de falantes opcional por perfil.
  Vídeo: trilha de áudio + quadros-chave descritos pelo modelo de visão. Audiências,
  consultas, aulas, reuniões.
- **P3 — Acadêmico.** LaTeX por seção, BibTeX como registros, Jupyter por célula (código +
  saída), fórmulas de PDF reconhecidas como LaTeX (via P8).
- **P4 — Engenharia e GIS.** DXF (textos, camadas, carimbo), IFC (entidades e
  propriedades), cabeçalho de STEP/IGES, pranchas em PDF (texto vetorial + carimbo), GIS
  (atributos das feições e metadados espaciais).
- **P5 — Saúde.** DICOM (tags, laudos estruturados, PDF embutido; pixels pelo modelo de
  visão opcional), HL7 v2, FHIR e CDA com chunkers que entendem a estrutura de cada um.
- **P6 — Office completo.** DOCX: comentários, controle de alterações, notas de rodapé,
  cabeçalhos e rodapés. PPTX: notas do apresentador. XLSX: fórmulas, células mescladas,
  intervalos nomeados, abas ocultas. Mais Visio, iWork, MHTML, XPS, `.ics` e `.vcf`.
- **P7 — PDF além do texto.** Sumário/bookmarks como caminho de seção, anotações, campos de
  formulário (chave–valor), arquivos anexados ao PDF (pelo P1) e metadados de assinatura
  digital (signatário, data, validade) — essenciais em processos e atos públicos.

### Qualidade de parsing

- **P8 — Modelo de parsing de documentos (VLM pequeno, ONNX).** Um modelo único que lê a
  imagem da página e devolve Markdown com ordem de leitura, tabelas, fórmulas e código:
  [Granite-Docling-258M](https://infoq.com/news/2025/10/granite-docling-ibm) (Apache-2.0,
  há conversão ONNX) ou [PaddleOCR-VL](https://arxiv.org/pdf/2510.14528) (0,9B, 109
  idiomas; licença a confirmar). Substitui a soma OCR + modelo de layout + heurística 2D
  nos perfis que comportam; nos perfis leves fica a cascata OCR + layout. A escolha exige
  benchmark (ex.: OmniDocBench).
- **P9 — Detecção de camada de texto quebrada.** PDFs digitais cujo texto extraído é lixo
  (fontes sem mapeamento Unicode, CID) são tratados como escaneados e vão para OCR/P8.
  Detecção independente de idioma: proporção de caracteres de substituição, de uso
  privado e de sequências improváveis, conferida contra uma amostra de OCR.
- **P10 — Recuperação visual de páginas.** Para documentos visualmente ricos (gráficos,
  pranchas, formulários), embeddings multivetoriais da imagem da página, na linha do
  [ColPali](https://arxiv.org/abs/2407.01449v6), como índice adicional nos perfis altos.
  Licenças variam por modelo (algumas são Gemma). Depende do pipeline de recuperação.
- **P11 — Tabelas como dados consultáveis.** Toda tabela (planilha, CSV, tabela de PDF ou
  DOCX) é guardada também num formato consultável, além dos chunks, para perguntas
  agregadas. Já anotado como pendência da recuperação; aqui estende-se a tabelas dentro
  de documentos.

### Enriquecimento

- **P12 — Metadados de documento independentes de idioma.** Idioma do documento, data do
  documento extraída do conteúdo (não o mtime), tipo, título e autores; mais **extração de
  entidades zero-shot** com tipos configuráveis por workspace (ex.: "número do processo",
  "paciente", "número da prancha", "identificador fiscal") via
  [GLiNER](https://pypi.org/project/GLiNER/) multilíngue (Apache-2.0, exportável para
  ONNX). Vira filtro e faceta na recuperação.
- **P13 — Dados pessoais e sensíveis.** O mesmo modelo marca chunks com categorias de
  dados pessoais; a política do workspace decide: só marcar, mascarar na resposta ou não
  indexar. Hospitais e setor público (LGPD, GDPR, HIPAA — sem regra presa a um país).
- **P14 — Resumos hierárquicos.** Resumos de documento e de pasta gerados pelo LLM local
  do perfil, em segundo plano, guardados como **nós próprios** buscáveis (não prefixados
  nos chunks, como o envelope removido). Respondem a "do que trata este contrato?" e
  perguntas de visão geral.
- **P15 — Late chunking.** Com o modelo de embedding local de contexto longo (ex.: BGE-M3,
  8192 tokens), o documento é embedado inteiro e cada chunk recebe o vetor com contexto do
  documento, sem custo de LLM — o "contextual retrieval barato" para perfis leves.
  [Ganho reportado modesto](https://arxiv.org/html/2409.04701v3) (≈ 3% nDCG@10). Efeito no
  diff por chunk: uma edição re-embeda a janela em volta dela, não só o chunk editado.
- **P16 — Grafo de entidades e relações.** Relações entre documentos (partes ↔ processos,
  componentes ↔ pranchas, pacientes ↔ exames). Caro; avaliar só para perfis altos e depois
  de P12.
- **P17 — Versões e quase-duplicatas.** MinHash/SimHash entre documentos: agrupa versões
  (`contrato_v3_final_FINAL.docx`), marca a mais recente e guarda conteúdo idêntico uma só
  vez. Evita respostas que misturam versões.

### Armazenamento e eficiência

- **P18 — Vetores compactos.** Truncamento Matryoshka e quantização int8/binária com
  reordenação pelos vetores completos: índice várias vezes menor e mais rápido, por
  perfil.
- **P19 — Vetores esparsos aprendidos.** Os vetores esparsos do BGE-M3 como complemento ao
  BM25, com melhor comportamento multilíngue. Opcional.

### Operação

- **P20 — Sync automático das fontes escolhidas.** Observação do sistema de arquivos nas
  fontes que o usuário selecionou (continua ação explícita) e recrawl agendado para
  fontes web. Hoje o sync só roda quando pedido.
- **P21 — Governança de recursos.** Ingestão em segundo plano com prioridade baixa, pausa
  com bateria fraca ou uso intenso, uso de GPU e paralelismo por perfil.
- **P22 — Localizadores precisos.** Página + região (bounding box) no PDF, timestamp no
  áudio, célula na planilha, linha no código — para abrir o original no ponto exato
  (FEAT-016). Confiança para advocacia e saúde.
- **P23 — Controle de acesso por fonte.** Quem pode ver cada fonte; depende do futuro
  módulo de usuários. Só registrar agora.

### Inspiradas no Harness²

- **H1 — Autoavaliação da ingestão (propose–probe–compose).** Para cada fonte, o LLM local
  gera perguntas sintéticas a partir de uma amostra de chunks e a recuperação é executada:
  a taxa de acerto vira um **score de qualidade da ingestão** por arquivo e por fonte. Score
  baixo dispara estratégias alternativas (OCR em vez da camada de texto, outro tamanho de
  chunk, renderização web) e fica a melhor. Arquivos mal ingeridos aparecem no relatório.
- **H2 — Memória de estratégia por fonte.** A estratégia vencedora (parser, OCR,
  renderização, tamanho de chunk) é lembrada por fonte e tipo de arquivo e reaproveitada —
  generaliza a calibração de renderização web já decidida.
- **H3 — Pacotes de domínio.** Configurações por área (saúde, jurídico, engenharia,
  contabilidade, acadêmico, TI): tipos de entidade do P12, parsers habilitados, metadados
  de filtro, política de dados sensíveis do P13. Escolhidos por workspace, **ortogonais aos
  perfis de hardware** e independentes de idioma.

---

## 4. Prioridade sugerida

| prioridade | propostas | por quê |
|---|---|---|
| 1 — base | P8, P9, P1, P6, P7, P12, P13, P17, P22, H1 | qualidade de extração e confiança para todos os públicos |
| 2 — públicos específicos | P2, P3, P4, P5, P14, P15, P18, P20, H2, H3 | cobertura por área e eficiência |
| 3 — avaliar depois | P10, P16, P19, P21, P23 | custo alto, dependem da recuperação ou do módulo de usuários |

---

## 5. Decisões

| proposta | decisão | onde foi registrada |
|---|---|---|
| P8 | aceita: texto digital é a verdade; modelo de parsing de páginas para escaneadas nos perfis que comportam; benchmark antes de fixar | `ingestion-pipeline-v2.md`, passo 6 |
| P9 | aceita: terceiro estado de página "texto quebrado", com verificação contra OCR na dúvida | `ingestion-pipeline-v2.md`, passo 4 |
| P1 | aceita, com PST/OST no nível 1, RAR só leitura (licença a confirmar) e cofre de senhas de arquivos | `ingestion-pipeline-v2.md`, passo 4 |
| P6, P7 | aceitas; controle de alterações com texto atual + alterações como itens próprios; `.ics`, `.vcf`, MHTML no nível 1, Visio, XPS e iWork no nível 2 | `ingestion-pipeline-v2.md`, passo 5 |
| Formulários | aceito: LSIE (DocMapper) como motor, fase 0 automática por perfil, mapas de layout revisáveis, crate Rust única para AnyContext e DocMapper, integração local com o DocMapper, corpus de testes iniciado pelo CMR fictício | `ingestion-pipeline-v2.md`, passo 6 |
| P12, P13 | aceitas: metadados e entidades configuráveis por workspace; dados pessoais mascarados por padrão com LLM na nuvem e só marcados com LLM local; segredos nunca indexados | `ingestion-pipeline-v2.md`, passo 7 |
| P17, P22 | aceitas, respeitando a hierarquia pai–filhos: duplicata exata guarda a árvore uma vez; filhos nunca fundidos entre pais diferentes; famílias de versões na raiz do documento; localizador preciso no filho e extensão no pai | `ingestion-pipeline-v2.md`, passos 3 e 7 |
