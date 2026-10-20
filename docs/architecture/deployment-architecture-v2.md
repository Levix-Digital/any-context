# Arquitetura de implantação — design alvo (v2)

Documento transversal: os pipelines de ingestão e de recuperação descrevem **o que**
acontece; este documento descreve **onde** cada peça roda. É decidido antes do backlog,
porque define as costuras que os dois pipelines precisam respeitar.

Motivação: no plano empresarial, o admin pode querer o armazenamento num servidor da
própria VPC, modelos pesados num servidor com GPU, e uma base de conhecimento indexada uma
vez e consultada por todos.

## Princípio

O core fala com **portas** (interfaces). Cada porta tem uma implementação local (padrão) e
uma remota. Decide-se agora as costuras; os modos remotos são construídos depois, como itens
do plano empresarial — sistema distribuído antes de ter clientes empresariais é
complexidade sem retorno.

## Costuras

1. Porta de armazenamento
2. Porta de modelos, por papel
3. Nível de confiança de cada endpoint
4. Identidade do modelo de embedding (o que produz o vetor, não onde roda)
5. Worker de ingestão como papel separável

### 1. Porta de armazenamento

Hoje não há interface: o core usa direto o `NativeLanceStore` (9 arquivos) e três
armazenamentos sem coordenação — LanceDB (vetores e texto), índice lexical e `settings.db`
(que mistura preferências, chaves, histórico, fontes, cache do diff e resumos).

- **Porta única de "base de conhecimento" por workspace**, que esconde os três
  armazenamentos: gravar e remover um documento com toda a árvore (pais, filhos, vetores,
  termos, metadados); busca densa, lexical e por filtros; chunks por id e pai de um chunk;
  consulta a tabelas e formulários; estatísticas. **A porta é responsável pela
  consistência:** cada gravação passa por um diário (journal) e é aplicada de forma
  idempotente nos três lugares; uma falha no meio é completada no próximo sync. Resolve um
  problema que já existe hoje, mesmo sem servidor.
- **O `settings.db` é separado em dois:**
  - **local do usuário** (nunca sai da máquina): preferências, histórico de conversa,
    referências do cofre, perfil de hardware;
  - **do workspace** (acompanha a base de conhecimento): fontes, cache do diff, entidades,
    mapas de layout, resumos, famílias de versões.
- **Sem "LanceDB direto no S3/MinIO"**: o Tantivy precisa de disco de baixa latência, a
  consistência entre os três armazenamentos fica frágil pela rede e o cálculo continua no
  cliente. Armazenamento remoto só pelo AnyContext Server (modo 3), que roda os três
  armazenamentos com disco local e expõe a porta por API.
- **Segurança no servidor:** chave de criptografia no cofre ou serviço de chaves do servidor;
  comunicação com TLS e autenticação mútua.
- **Abrir o arquivo original** (navegar pastas, abrir arquivos, ver sites sem sair do
  AnyContext) fica para a interface em Tauri 2 e não é decidido aqui.

### 2. Porta de modelos, por papel

Hoje só chat e embedding têm interface (`LmProvider` do `actx-lm`: OpenAI, Anthropic,
Gemini, Gemma, mock); os modelos ONNX não têm interface comum, e nenhum consegue rodar num
servidor. As políticas de modelo do Admin Hub não estão ligadas ao core.

- **Papéis explícitos:** embedding, reranker, OCR, parsing de páginas, campos de
  formulário, descrição de imagens, transcrição (com VAD e identificação de falantes),
  entidades e dados pessoais, LLM de contexto e resumos, LLM de chat.
- **Vínculo por papel:** modelo (identificador e versão do catálogo) + **onde roda** (ONNX
  local ou endpoint). Os kits de hardware passam a ser conjuntos de vínculos por papel; um
  kit **"Empresarial / VPC"** aponta os papéis pesados para o servidor.
- **Protocolos remotos (adaptadores):**
  - **APIs nativas dos fornecedores:** OpenAI e Azure OpenAI, Anthropic (Messages) e
    **Google — Gemini API e Vertex AI** (`generateContent`, `embedContent`). A API nativa é
    a preferida para cada fornecedor, porque as camadas "compatíveis com OpenAI" que eles
    oferecem cobrem só parte dos recursos (tipos de tarefa de embedding, entradas
    multimodais, cache de contexto). O Vertex AI no projeto GCP do próprio cliente é o
    caminho de "gateway corporativo" do Google.
  - **Genérico compatível com OpenAI** para servidores que o usam (vLLM, Ollama, LM
    Studio), e os formatos de fato para reranker (`/rerank`) e transcrição
    (`/audio/transcriptions`).
  - **AnyContext Model Server** para os papéis sem padrão (OCR, parsing de páginas, campos,
    entidades): o mesmo runtime ONNX do desktop empacotado como serviço, com o mesmo modelo
    e o mesmo resultado que local.
- **Política do Admin Hub conferida na resolução** (só local, só gateway corporativo,
  híbrido; modelos permitidos): vínculo proibido é recusado com motivo claro, nunca trocado
  em silêncio.
- **Indisponibilidade do servidor:** embedding nunca troca de modelo (o arquivo fica para o
  próximo sync); demais papéis da ingestão entram em fila e são retentados, podendo cair
  para o local **só com o mesmo modelo e versão**; no chat o erro é mostrado e trocar de LLM
  é escolha do usuário, dentro do que a política permite.
- **Adaptadores para serviços de nuvem dos papéis ONNX** (ex.: Google Document AI e
  Speech-to-Text, e equivalentes da Microsoft e da AWS), para clientes que já pagam por
  eles. Seguem o nível de confiança do endpoint (costura 3).

### 3. Nível de confiança de cada endpoint

Hoje não existe essa noção: qualquer provedor configurado recebe o contexto completo.

- **Três níveis:**
  - **local:** a própria máquina;
  - **corporativo:** o perímetro da empresa — VPC, Model Server, AnyContext Server e
    serviços de IA contratados **na conta da própria empresa** (ex.: Vertex AI no projeto
    GCP dela, Azure OpenAI no tenant dela);
  - **externo:** APIs públicas de terceiros.
- **Quem define:** sem Admin Hub, o usuário classifica os endpoints (padrões: `localhost`
  local, API pública externo); com Admin Hub, a política da empresa define e **o usuário não
  pode elevar o nível** de um endpoint.
- **O que o nível decide:**
  - a pseudonimização reversível (ingestão, passo 7) só se aplica ao **externo**;
  - as políticas do Admin Hub viram regras sobre níveis: "só local" → local; "só gateway
    corporativo" → local + corporativo; "híbrido" → todos, dentro dos modelos permitidos.
- **Teto por workspace:** um workspace marcado como **confidencial** (ex.: prontuários,
  processos sob sigilo) nunca envia nada para endpoint externo, mesmo que a política geral
  permita.
- **Outras saídas de dados contam igual:** a busca web (o texto da pergunta enviado a um
  buscador público) recebe o tratamento do externo — pseudonimização e teto do workspace;
  aplicativos que consomem o AnyContext pelo servidor MCP (Cursor, Claude Desktop) são
  consumidores externos, governados pela política.

### 4. Identidade do modelo de embedding

O `embedding_target` fica em cada chunk e há uma tabela por workspace e alvo (ingestão,
passos 7 e 10). Esta costura define quando dois alvos são iguais.

- **A identidade é o que produz o vetor, não onde ele roda:** modelo e versão dos pesos
  (hash do catálogo), dimensão, **precisão** (fp32, fp16, int8) e pré-processamento (versão
  do tokenizador, prefixos de busca e de documento). O mesmo modelo, com a mesma precisão,
  local ou no servidor, é o mesmo alvo — sem reindexação.
- **Teste de impressão digital:** o catálogo traz um conjunto fixo de textos e os vetores
  de referência de cada modelo. Ao configurar um vínculo, e periodicamente, o AnyContext
  embeda esses textos e compara: similaridade acima de um limiar alto (ex.: 0,999) → mesmo
  alvo; abaixo → outro alvo, com a regra de reindexação de sempre.
- O teste também pega mudanças silenciosas (provedor que altera o modelo sem mudar o nome,
  servidor atualizado com outra versão ou precisão) e decide os casos de precisão diferente
  (ex.: fp16 na GPU contra fp32 no desktop).

### 5. Worker de ingestão

Hoje cada sync dispara um processo `actx --sync-worker` (ou uma thread, se o processo não
sobe); progresso, status e cancelamento passam por uma tabela do SQLite; não há fila, e
nada impede dois workers no mesmo workspace.

1. **Fila de trabalhos persistente por workspace**, por prioridade:
   1. **indexação base** (texto, índice lexical, embedding) — o que torna o conteúdo
      buscável;
   2. **enriquecimentos** (contextual retrieval, resumos, descrição de imagens,
      transcrição e falantes);
   3. **autoavaliação** (sondagens, reprocessamento automático);
   4. **manutenção** (compactação, recrawl web, testes de impressão digital).
2. **Trabalhos idempotentes**, ligados ao diário da porta de armazenamento (costura 1):
   uma interrupção é retomada sem duplicar nem perder.
3. **Um único escritor por workspace**, por trava com prazo (lease); se o worker morre, a
   trava expira e outro assume. Leitura e busca nunca são bloqueadas.
4. **O worker é um papel:** processo local nos modos 1 e 2 (com a "receita" compartilhada,
   cada membro roda o seu); no modo 3 roda no AnyContext Server, e os clientes só
   enfileiram pedidos e consultam. O serviço em segundo plano opcional (ingestão, sync
   automático) é esse mesmo worker como serviço do sistema.
5. **Progresso e status por eventos** (início, item processado, erro, fim): eventos nativos
   no Tauri; pela conexão com o servidor no modo 3.
6. **Cancelamento e pausa são comandos na fila**, respeitados entre trabalhos e dentro dos
   longos (ex.: transcrição de 3 horas tem pontos de pausa).

## Modelo de workspace

- **Cada usuário cria os seus workspaces e é o único dono.** Pode compartilhar com outros
  usuários, definindo papéis.
- **O admin pode criar workspaces da organização**, disponíveis para toda a empresa, que
  ninguém além dele edita.
- **Um chat por workspace, nunca misturado.**
- **Nenhuma consulta atravessa workspaces.** Separar contextos ao gosto do usuário é a
  função do workspace. A documentação do AnyContext deixa de ser o workspace `Global` e
  vira a ajuda do sistema, fora dos workspaces.
- **Compartilhamento, com os mesmos papéis em qualquer modo:** **dono**, **editor** (gerencia
  as fontes) e **leitor** (só consulta). O workspace da organização tem o admin como dono e
  todos os demais como leitores.
  - **Modo 1 (sem servidor): compartilha-se a "receita"** — fontes e configuração. Cada
    membro indexa localmente as mesmas fontes, que precisam ser acessíveis a todos (ex.:
    servidor de arquivos, SharePoint).
  - **Modo 3: compartilha-se o índice** — o workspace vive no AnyContext Server e os
    membros consultam o mesmo índice.
- **O chat é de cada usuário**, mesmo num workspace compartilhado: cada um tem o seu
  histórico, e ninguém lê o chat do outro.

## Admin Hub e AnyContext Server

- **Admin Hub (plano de controle):** gestão de frotas, matrículas, políticas, ITSM/SCIM e
  kill-switch. É **zero-knowledge** (nunca vê conteúdo) e é **sempre hospedado no cliente**
  (VPC ou on-premise) — **nunca um SaaS da Levix**. (As especificações iniciais do Admin
  Hub citam "SaaS Levix" como opção de hospedagem; isso fica corrigido.)
- **AnyContext Server (plano de dados, modo 3):** também hospedado na VPC da empresa;
  matriculado e governado pelo Admin Hub como um dispositivo.
- **Regra de perímetro:** o conteúdo (documentos, chunks, vetores, prompts) **nunca sai do
  perímetro da empresa** — no modo 1, nem sai da máquina. O modo 1 continua sendo o padrão.

## Modos de implantação

| modo | descrição | quando |
|---|---|---|
| 1. Desktop | tudo local; padrão | já |
| 2. Desktop + servidor de modelos | índice local, modelos pesados na VPC; só a porta de modelos fica remota | primeiro passo empresarial |
| 3. Cliente + AnyContext Server | índice, ingestão e modelos no servidor; a máquina é cliente; conhecimento compartilhado | plano empresarial completo, com o módulo de usuários e o controle de acesso por fonte (auditoria P23) |
