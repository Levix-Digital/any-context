# Pipeline de recuperação — design alvo (v2)

Documento vivo da revisão do pipeline de recuperação, no mesmo método do
`ingestion-pipeline-v2.md`. A seção "Estado atual" registra como o código funciona hoje
(levantado em 2026-10-20); o design alvo é preenchido passo a passo depois de discutido.

## Estado atual

1. **Roteamento da pergunta.** Um classificador por palavras-chave fixas em português e
   inglês ("arquitetura", "compare", "onde está"…), somado ao tamanho da pergunta, decide
   entre três caminhos:
   - **Fast (1-hop):** uma busca e uma única chamada ao LLM.
   - **ReAct:** o LLM chama ferramentas em loop, até 10 turnos.
   - **Deep Search:** decompõe a pergunta em até 4 subperguntas, busca, reflete sobre o que
     falta, busca de novo e sintetiza.

   Perguntas de status ("o sync terminou?") vão para um caminho próprio. O usuário pode
   forçar o modo com `/fast` e `/deep`.
2. **Pré-processamento da consulta.**
   - Detecta nomes de arquivo na pergunta e datas (formatos ISO, numéricos e meses em
     inglês).
   - Os arquivos citados entram com score máximo fixo (1,0). As datas viram filtros por
     trechos do caminho do arquivo (`/2025/03/`), também com score fixo (0,95).
   - A pergunta é usada como está: não é reescrita com o contexto da conversa (IMP-009).
3. **Busca densa.** A pergunta é embedada com o modelo registrado do workspace e buscada no
   LanceDB, no workspace atual e no `Global`. Se o embedding falha, a busca continua só com
   o BM25 e a falha é registrada.
4. **Busca lexical:** BM25 no mesmo workspace.
5. **Fusão RRF** das duas listas (k = 60). Candidatos que só vieram do BM25 têm o texto
   buscado no LanceDB.
6. **Score mínimo:** corte no score do RRF, que varia por preset (Turbo, Balanced, Deep).
7. **Reranking:** cross-encoder multilíngue local via ONNX reordena até 30 candidatos. Sem
   corte por relevância depois disso (IMP-007).
8. **Diversificação e orçamento.**
   - No máximo 2, 3 ou 5 chunks por arquivo, conforme o preset.
   - Top-k de 10, 20 ou 40.
   - Teto de 20, 40 ou 60 mil caracteres de contexto (em caracteres, não em tokens).
9. **Gate de "zero contexto".** Se nenhum chunk passa de um score mínimo, responde "nada
   encontrado" sem chamar o LLM.
10. **Montagem do prompt.** Cada chunk numa tag XML com origem, versão e localização; ordem
    em formato de U (mais relevantes nas pontas); texto recuperado escapado.
11. **Geração.**
    - **Fast:** histórico da conversa + prompt numa única chamada.
    - **ReAct e Deep Search:** prompts próprios, com o mesmo formatador de contexto.
12. **Resposta e histórico:** a resposta e a pergunta original, sem o XML, vão para o
    histórico da sessão no SQLite.

### Pendências herdadas da ingestão

- Perguntas agregadas sobre tabelas grandes ("soma das vendas de março"): guardar a tabela
  num formato consultável e responder por consulta (ingestão, passo 5).
- Uso dos chunks "pai" (small-to-big) na montagem do contexto (ingestão, passo 5).
- **Nenhuma consulta atravessa workspaces.** Hoje toda pergunta busca no workspace atual e
  no `Global` e mistura os resultados; a documentação do AnyContext vira a ajuda do
  sistema, consultada só em perguntas sobre o próprio AnyContext, por rota ou ferramenta
  própria (ingestão, passo 1).
- Um chat por workspace, individual de cada usuário mesmo em workspace compartilhado
  (`deployment-architecture-v2.md`).
- Famílias de versões: preferir a versão vigente por padrão e usar a família inteira em
  perguntas sobre mudanças entre versões (ingestão, passo 3).
- Entidades e metadados do documento como filtros e facetas; busca sem máscara e
  pseudonimização reversível do contexto enviado a um endpoint externo, com os valores reais
  devolvidos localmente na resposta (ingestão, passo 7; níveis de confiança em
  `deployment-architecture-v2.md`).
- Teto "confidencial" por workspace e busca web tratada como saída para o externo
  (`deployment-architecture-v2.md`, costura 3).
- Resumos de documento e de pasta para perguntas de visão geral e para escolher em quais
  documentos procurar (ingestão, passo 7).
- Orientações de resposta dos pacotes de domínio (ingestão, seção transversal H3).
- Formulários do mesmo modelo de layout como tabela consultável (ingestão, passo 6).
- Conjunto de avaliação por workspace formado pelas perguntas sintéticas da autoavaliação
  (ingestão, seção transversal).

## Design alvo

### Princípio: configuração mínima

Decisões de arquitetura não viram opção do usuário, nem do avançado. Reranker, tamanho de
chunk, overlap, top-K, limiares, pesos do RRF e número de candidatos são decididos pelo
AnyContext e ajustados automaticamente pela avaliação. O usuário configura só o que é dele:
fontes e filtros de inclusão, modelo de chat, Fast ou Deep, pacote de domínio, políticas de
privacidade do workspace, teto de crawl de um site.

### Visão geral (caminho feliz)

Pergunta do usuário → Fast ou Deep → (Deep: refinamento da pergunta) → busca densa + lexical
→ RRF → seleção dos melhores chunks → ordem em U + XML → modelo de inferência. Os passos
abaixo detalham esse caminho e acrescentam os passos que o código já tem e as pendências
da ingestão.

1. Entrada da pergunta
2. Roteamento
3. Entendimento da pergunta
4. Busca
5. Fusão
6. Reranking
7. Seleção
8. Montagem do contexto
9. Geração e citações
10. Histórico e avaliação

### 1. Entrada da pergunta

Hoje a pergunta é buscada como foi digitada (o histórico vai para o LLM, não para a busca
— IMP-009); o histórico é uma janela de 30 turnos; a documentação do AnyContext é
misturada em toda busca; perguntas de status são detectadas por palavras-chave fixas em
PT/EN ("qual o status do processo 123?" cai na rota de status do AnyContext).

- **Reescrita da continuação, só para a busca.** Uma pergunta que depende da conversa ("e o
  prazo da segunda?") é reescrita como pergunta completa e autônoma antes de buscar; a
  original é a que fica no histórico e que o LLM de resposta vê. Quem reescreve: o LLM local
  do perfil; sem ele, o LLM de chat, respeitando o nível de confiança e a pseudonimização.
  Sem LLM local e com chat externo, a alternativa sem LLM é buscar com a pergunta anterior
  do usuário somada à atual. Funciona em qualquer idioma.
- **Histórico por orçamento de tokens:** mensagens recentes inteiras; as antigas viram um
  resumo da conversa, gerado em segundo plano.
- **Ajuda e status são ferramentas, não rotas por palavra-chave:** `anycontext_help`
  (documentação do AnyContext) e `workspace_status` (estado do sync e das fontes), chamadas
  pelo LLM em qualquer idioma; a interface também oferece painel de ajuda e status das
  fontes na árvore.
- **Sem anexo temporário no chat.** O contexto é o que está no workspace — um conceito só.
  **Arrastar um arquivo para o chat é um atalho para adicioná-lo ao workspace como arquivo
  avulso**, com confirmação curta (ação explícita), progresso da indexação visível e, para
  arquivos grandes, a estimativa de tempo da ingestão. O arquivo fica disponível nas
  próximas conversas.
- **`@fonte` na pergunta** restringe a busca àquela fonte, sempre dentro do workspace
  (detalhado no passo 3).
- **Fila de perguntas.** Enquanto o agente responde, o usuário pode enviar novas perguntas,
  que ficam enfileiradas:
  - processadas **em ordem, uma de cada vez** — cada uma pode depender da resposta
    anterior, e a reescrita da continuação usa essa resposta;
  - a fila fica visível no chat; perguntas ainda não iniciadas podem ser editadas,
    removidas ou reordenadas; há "parar a atual e enviar agora";
  - a fila é do chat do workspace e sobrevive ao fechar o app (como rascunhos locais);
  - cada pergunta conta para as cotas de tokens da política (Admin Hub);
  - parar a pergunta atual não apaga a fila.
- **Parar e editar.** Parar (botão ou `Esc`) cancela busca, turnos do Deep e streaming; a
  pergunta volta ao composer com o texto original para editar e reenviar; a pergunta
  interrompida e a resposta parcial saem do histórico.
- **Ramos da conversa.** Editar uma pergunta já respondida cria um **ramo novo** a partir
  dela, preservando o anterior; a mensagem editada mostra "‹ 2/3 ›" para alternar entre
  versões. Resumo da conversa, reescrita da continuação e fila seguem o ramo ativo.
  **Fundação:** o histórico é guardado desde já como árvore (cada mensagem com `parent_id`,
  o chat com o ramo ativo), mesmo que a interface de ramos chegue num lançamento posterior
  — evita migrar os chats existentes.

### 2. Roteamento

Hoje há três modos (Fast, ReAct, Deep Search); no automático, um classificador por
palavras-chave fixas em PT/EN e tamanho da pergunta decide; as ferramentas só existem no
ReAct.

- **Dois modos: Fast e Deep.** O ReAct deixa de ser um modo; as **ferramentas** (ajuda,
  status, consulta a tabelas, busca web quando permitida) ficam disponíveis nos dois.
  - **Fast:** a busca vem primeiro; o LLM responde e só chama ferramenta se precisar, com
    limite pequeno (ex.: até 2 chamadas).
  - **Deep:** turnos de refinamento e busca (abaixo).
- **Automático: começa Fast e escala para Deep** quando a evidência é insuficiente (melhores
  trechos com relevância baixa no reranker, ou o LLM dizendo que o contexto não basta), com
  indicador discreto. Sem palavras-chave, em qualquer idioma. O usuário pode fixar Fast ou
  Deep no seletor do composer.
- **Deep:** cada turno (1) refina ou decompõe a pergunta com base no que já foi encontrado,
  (2) busca e (3) avalia o que falta — refinar sem ver o encontrado é refinar às cegas.
  - **Máximo padrão de 3 turnos**, com parada antecipada quando a evidência basta (a
    maioria para no 1º ou 2º); **teto de tokens** além do teto de turnos.
  - O máximo pode subir até 5 **pelo pacote de domínio** (ex.: jurídico, acadêmico), nunca
    por opção do usuário (princípio da configuração mínima).
  - O padrão é ajustado com dados: o conjunto de avaliação mede o ganho de cada turno; se o
    3º não trouxer ganho, o padrão cai para 2.

### 3. Entendimento da pergunta

Hoje nomes de arquivo (regex com extensões fixas) entram com score fixo 1,0 e datas (regex
com meses só em inglês) viram filtros por trechos do caminho com score fixo 0,95; não há
filtro por tipo, entidade ou versão, nem distinção entre tipos de pergunta.

**Planejador da pergunta**, na mesma chamada de LLM da reescrita da continuação (passo 1),
que devolve uma busca estruturada:

1. **Texto de busca:** a pergunta autônoma; no Deep, também as subperguntas.
2. **Filtros:** fonte (`@arquivo`, pasta citada — sempre dentro do workspace); **datas pela
   data do documento** (ingestão, passo 7), em qualquer idioma; tipo de documento;
   entidades (com tolerância a grafias diferentes); versão (vigente por padrão, todas em
   perguntas sobre mudanças).
3. **Tipo de pergunta**, que decide o caminho:
   - pontual → busca nos trechos;
   - visão geral → resumos de documento e pasta;
   - agregada sobre tabela ou formulário → consulta à tabela;
   - comparação → família de versões, normalmente no Deep;
   - ajuda ou status → ferramentas do passo 1.

**Regras dos filtros:**

- **Sem scores inventados:** filtros restringem ou reforçam a busca.
- **Explícito restringe, inferido reforça:** o que o usuário disse (`@arquivo`, "de março
  de 2025") restringe; o que o planejador deduziu só dá preferência.
- **Filtro que zera o resultado é relaxado**, com aviso na resposta ("não encontrei
  contratos de março de 2025; estes são os mais próximos").
- Com chat externo, a pergunta passa pela pseudonimização antes do planejador. Alternativa
  sem LLM, com menos recursos: `@fonte`, datas por analisador multilíngue e entidades pelo
  índice de entidades.

### 4. Busca

Hoje: busca densa exaustiva no workspace atual e no `Global`, BM25 com a pergunta
"expandida" por datas, 50/100/150 candidatos por lista; o único filtro real é o
workspace; falha do embedding da pergunta deixa só o BM25 (registrado, sem aviso); não há
consulta a tabelas.

1. **Buscadores em paralelo, com os filtros dentro da busca (pré-filtro)** — densa e
   lexical juntas, para cada pergunta ou subpergunta do Deep; filtrar depois de pegar os
   candidatos pode deixar zero.
2. **O que é buscado:** os **chunks filhos** (os pais entram na seleção, passo 7); os
   **resumos** de documento e pasta só nas perguntas de visão geral. A densa usa o prefixo
   de consulta do modelo e a tabela do alvo de embedding (durante uma troca de modelo, a
   tabela antiga responde até a nova ficar completa); a lexical usa os campos com pesos
   (cabeçalho com peso menor que o corpo).
3. **30 candidatos por lista**, ajustado pelo conjunto de avaliação (sobe se 50 melhorar a
   taxa de acerto de forma mensurável).
4. **Consulta a tabelas e formulários:** para perguntas agregadas, o LLM gera **SQL somente
   leitura** sobre o esquema das tabelas do workspace (colunas, tipos, exemplos de valores),
   executado pelo motor SQL que o LanceDB já traz (DataFusion). Salvaguardas: só leitura e
   só as tabelas daquele workspace; limite de tempo e de linhas; **a consulta e as linhas
   usadas aparecem na citação**; falha ou ambiguidade cai para a busca normal, com aviso.
5. **Falha do embedding da pergunta:** busca só lexical, com **aviso discreto** na resposta.
6. **Buscadores futuros entram como listas extras** (busca visual de páginas, vetores
   esparsos — auditoria P10 e P19), sem mudar o resto do pipeline.

### 5. Fusão

Hoje: RRF (k = 60) das listas densa e lexical com pesos iguais; a lista densa vem com os
resultados de arquivo e data injetados com score inventado; corte por score mínimo do RRF
(0,012/0,008/0,005) — arbitrário, porque o score do RRF mede posição nas listas, não
relevância; no Deep, as subperguntas são juntadas por deduplicação com soma de score.

1. **RRF com k = 60 sobre listas limpas**, e **todas as listas numa fusão só**: no Deep,
   cada subpergunta gera duas listas e todas entram no mesmo RRF (um chunk achado por várias
   subperguntas sobe naturalmente); buscadores futuros entram igual.
2. **Pesos iguais por padrão**, ajustados por workspace pelo conjunto de avaliação (ex.:
   lexical num workspace de código, densa num de textos jurídicos), de forma automática e
   interna.
3. **Sem corte pelo score do RRF:** a decisão de relevância passa para o reranker (passo 6),
   e o gate de "nada encontrado" também (passo 7).
4. **Quantos seguem para o reranker — por orçamento de tempo, não número fixo:**
   - **Fast: 30**; **Deep: 50** (a fusão do Deep junta várias subperguntas e o usuário já
     aceita esperar mais);
   - ajustado pelo perfil de hardware (ex.: 20 no Fast em perfis leves; até 100 no Deep em
     perfis fortes ou no Model Server com GPU), com um orçamento de tempo do reranking (ex.:
     até cerca de 1 segundo no Fast);
   - a avaliação mede em que posição da fusão o trecho certo costuma estar e ajusta o
     número por workspace (cai se ele está quase sempre entre os 20 primeiros; sobe, dentro
     do orçamento, se aparece com frequência depois do 30º).

### 6. Reranking

Hoje: cross-encoder ONNX multilíngue (`mmarco-reranker` padrão; `bge-reranker-v2-m3`
INT8), escolhido ou desligado pelo usuário com `/rerank`; reordena até 30, os demais vão
para o fim sem avaliação; sem corte por relevância (IMP-007); o Deep pede ao LLM um
"sim/não" chunk a chunk (uma chamada por chunk), duplicando o cross-encoder.

1. **O cross-encoder é o único juiz de relevância, nos dois modos.** O julgamento chunk a
   chunk pelo LLM no Deep sai; o LLM do Deep avalia o conjunto ("o que ainda falta?").
2. **Modelo vem do kit de hardware** pela porta de modelos (pequeno nos perfis leves,
   `bge-reranker-v2-m3` nos médios e fortes, Model Server no modo empresarial). **Sempre
   ligado — decisão de arquitetura, sem opção para o usuário;** o comando `/rerank` sai.
3. **Entrada:** o texto do chunk filho com o cabeçalho contextual (o mesmo texto embedado),
   truncado no limite útil do modelo. Resumos (visão geral) passam pelo reranker;
   resultados de SQL não (são exatos).
4. **Score calibrado em três faixas** — relevante, talvez, irrelevante —, com limiares por
   modelo no catálogo, refinados por workspace pela avaliação. Alimentam o gate de "nada
   encontrado" (passo 7), a escalada do Fast para o Deep (passo 2) e o descarte: trechos
   irrelevantes não vão para o LLM, mesmo que sejam os "melhores" (resolve o IMP-007).
5. **Candidatos além do orçamento são descartados**, não empurrados para o fim sem
   avaliação.

### 7. Seleção

_Pendente._

### 8. Montagem do contexto

_Pendente._

### 9. Geração e citações

_Pendente._

### 10. Histórico e avaliação

_Pendente._
